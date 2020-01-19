use anyhow::{Context, Result};
use failure::ResultExt;
use regex::{Captures, Regex};
use tantivy::collector::{Count, FacetCollector, MultiCollector, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{Facet, Field, IndexRecordOption};
use tantivy::{self, DocAddress, Document, Index, Searcher, Snippet, SnippetGenerator, Term};

/// Create query with [Tantivy Query parser](https://docs.rs/tantivy/0.11.3/tantivy/query/struct.QueryParser.html)
///
/// Search term example: `path:csv OR path:pdf`
pub fn raw_query(index: &Index, text: &str) -> Result<Box<dyn Query>> {
    let path_field = index
        .schema()
        .get_field("path")
        .context("Field 'path' not found")?;
    let query_parser = QueryParser::for_index(&index, vec![path_field]);

    Ok(query_parser.parse_query(text).compat()?)
}

lazy_static! {
    static ref TAG_REGEX: Regex = Regex::new(r"(:[A-Za-z0-9_\-.]+)+").unwrap();
}

/// Create basic doctags query
///
/// Search term example: `:file_type:file html png`
pub fn doctags_query(index: &Index, text: &String) -> Result<Box<dyn Query>> {
    let mut tag_query = Vec::new();
    let tags_field = index
        .schema()
        .get_field("tags")
        .context("Field 'tags' not found")?;
    let mut raw = TAG_REGEX.replace_all(text, |caps: &Captures| {
        let facet = caps[0].replace(":", "/");
        let query: Box<dyn Query> = Box::new(TermQuery::new(
            Term::from_facet(tags_field, &Facet::from(&facet)),
            IndexRecordOption::Basic,
        ));
        tag_query.push(query);
        // Remove from raw query string
        ""
    });
    if raw.trim().is_empty() {
        raw = std::borrow::Cow::Borrowed("*"); // match all
    }
    let path_query = raw_query(index, &raw)?;
    let query = if tag_query.is_empty() {
        path_query
    } else {
        let query_vec: Vec<(Occur, Box<dyn Query>)> = vec![path_query]
            .iter()
            .chain(tag_query.iter())
            .map(|q| (Occur::Must, q.box_clone()))
            .collect();
        Box::new(BooleanQuery::from(query_vec))
    };
    Ok(query)
}

pub fn search(index: &Index, text: String, limit: usize) -> Result<()> {
    let limit = if limit == 0 { 100_000 } else { limit };
    let exclude_count = true;
    let exclude_docs = false;

    let reader = index.reader().compat()?;

    let searcher = reader.searcher();

    let schema = index.schema();
    let path_field = index
        .schema()
        .get_field("path")
        .context("Field 'path' not found")?;

    let query = doctags_query(&index, &text)?;

    let mut multi_collector = MultiCollector::new();
    let count_handle = if exclude_count {
        None
    } else {
        Some(multi_collector.add_collector(Count))
    };
    let top_docs_handle = if exclude_docs {
        None
    } else {
        Some(multi_collector.add_collector(TopDocs::with_limit(limit as usize)))
    };

    // search index
    let mut multi_fruit = searcher.search(&query, &multi_collector).compat()?;

    // count
    if let Some(ch) = count_handle {
        let count = ch.extract(&mut multi_fruit) as i64;
        debug!("Match count: {}", count);
    }

    // docs
    if let Some(tdh) = top_docs_handle {
        let top_docs = tdh.extract(&mut multi_fruit);
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address).compat()?;
            // let named_doc = schema.to_named_doc(&doc);
            debug!("score: {} doc: {}", score, schema.to_json(&doc));
            println!(
                "{}",
                doc.get_first(path_field)
                    .context("No 'path' entry in doc")?
                    .text()
                    .context("Couldn't convert 'path' entry to text")?
            );
        }
    }

    Ok(())
}

pub struct Match {
    pub text: String,
    pub snippet: Snippet,
}

pub fn search_matches(index: &Index, input: &String, max_results: usize) -> Result<Vec<Match>> {
    let path_field = index
        .schema()
        .get_field("path")
        .context("Field 'path' not found")?;

    let reader = index.reader().compat()?;
    let searcher = reader.searcher();
    let query = doctags_query(&index, &input)?;

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(max_results))
        .compat()?;

    let snippet_generator = SnippetGenerator::create(&searcher, &query, path_field).compat()?;

    let lines: Result<Vec<Match>> = top_docs
        .iter()
        .map(|(_score, doc_address)| {
            formatted_match(&searcher, doc_address, &snippet_generator, &path_field)
        })
        .collect();

    lines
}

fn formatted_match(
    searcher: &Searcher,
    doc_address: &DocAddress,
    snippet_generator: &SnippetGenerator,
    path_field: &Field,
) -> Result<Match> {
    let doc = searcher.doc(*doc_address).compat()?;
    let text = doc
        .get_first(*path_field)
        .context("No 'path' entry in doc")?
        .text()
        .context("Couldn't convert 'path' entry to text")?
        .to_string();
    let snippet = snippet_generator.snippet_from_doc(&doc);
    Ok(Match { text, snippet })
}

pub fn doc_from_id(index: &Index, id: u64) -> Result<Option<Document>> {
    let id_field = index
        .schema()
        .get_field("id")
        .context("Field 'id' not found")?;
    let term = Term::from_field_u64(id_field, id);
    let term_query = TermQuery::new(term, IndexRecordOption::Basic);
    search_single_doc(index, &term_query)
}

fn search_single_doc(index: &Index, query: &TermQuery) -> Result<Option<Document>> {
    let reader = index.reader().compat()?;
    let searcher = reader.searcher();
    let top_docs = searcher.search(query, &TopDocs::with_limit(1)).compat()?;
    if let Some((_score, doc_address)) = top_docs.first() {
        let doc = searcher.doc(*doc_address).compat()?;
        Ok(Some(doc))
    } else {
        Ok(None)
    }
}

pub fn doc_from_path(index: &Index, path: &String) -> Result<Option<Document>> {
    let reader = index.reader().compat()?;

    let searcher = reader.searcher();

    let path_field = index
        .schema()
        .get_field("path")
        .context("Field 'path' not found")?;

    let term = Term::from_field_text(path_field, &path);
    let term_query = TermQuery::new(term, IndexRecordOption::Basic);
    // FIXME: TermQuery gives empty result. Workaround:
    let query_parser = QueryParser::for_index(&index, vec![path_field]);
    let term_query = query_parser
        .parse_query(&path)
        .unwrap_or(Box::new(term_query));
    let top_docs = searcher
        .search(&term_query, &TopDocs::with_limit(1))
        .compat()?;

    if let Some((_score, doc_address)) = top_docs.first() {
        let doc = searcher.doc(*doc_address).compat()?;
        Ok(Some(doc))
    } else {
        dbg!("doc_from_path not found", path);
        Ok(None)
    }
}

pub fn stats(index: &Index) -> Result<()> {
    let reader = index.reader().compat()?;

    let searcher = reader.searcher();

    let count = searcher.search(&AllQuery, &Count).compat()?;

    println!("Total documents: {}", &count);

    let tags = index
        .schema()
        .get_field("tags")
        .context("Field 'tags' not found")?;
    let mut facet_collector = FacetCollector::for_field(tags);
    facet_collector.add_facet("/");

    let facet_counts = searcher.search(&AllQuery, &facet_collector).compat()?;
    for (facet, count) in facet_counts.get("/") {
        println!("{}: {}", &facet, count);
    }

    Ok(())
}
