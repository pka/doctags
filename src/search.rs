use std::collections::HashMap;
use tantivy::collector::{Count, FacetCollector, MultiCollector, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{Facet, IndexRecordOption};
use tantivy::{self, Index, Term};

/// Create query with [Tantivy Query parser](https://docs.rs/tantivy/0.11.3/tantivy/query/struct.QueryParser.html)
///
/// Search term example: `path:csv OR path:pdf`
pub fn raw_query(index: &Index, text: &str) -> Box<dyn Query> {
    let path_field = index.schema().get_field("path").unwrap();
    let query_parser = QueryParser::for_index(&index, vec![path_field]);

    query_parser.parse_query(text).unwrap()
}

/// Create simple doctags query
///
/// Search term example: `:file_type:file html png`
pub fn doctags_query(index: &Index, text: &String) -> Box<dyn Query> {
    if text.starts_with(":") {
        let v: Vec<&str> = text.splitn(2, ' ').collect();
        let path_query = raw_query(index, v[1]);
        let facet = v[0].replace(":", "/");
        let tags_field = index.schema().get_field("tags").unwrap();
        let tag_query: Box<dyn Query> = Box::new(TermQuery::new(
            Term::from_facet(tags_field, &Facet::from(&facet)),
            IndexRecordOption::Basic,
        ));
        let faceted_query =
            BooleanQuery::from(vec![(Occur::Must, path_query), (Occur::Must, tag_query)]);
        Box::new(faceted_query)
    } else {
        raw_query(index, text)
    }
}

pub fn search(index: &Index, text: String, limit: usize) -> tantivy::Result<()> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let schema = index.schema();
    let path_field = index.schema().get_field("path").unwrap();

    let query = doctags_query(&index, &text);

    let limit = if limit == 0 { 100_000 } else { limit };
    let exclude_count = false;
    let exclude_docs = false;
    let facet_prefixes: Vec<&str> = vec![];

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
    let facet_handle = if facet_prefixes.is_empty() {
        None
    } else {
        let tags_field = schema.get_field("tags").unwrap();
        let mut facet_collector = FacetCollector::for_field(tags_field);
        for facet_prefix in &facet_prefixes {
            facet_collector.add_facet(facet_prefix);
        }
        Some(multi_collector.add_collector(facet_collector))
    };

    // search index
    let mut multi_fruit = searcher.search(&query, &multi_collector).unwrap();

    // count
    if let Some(ch) = count_handle {
        let count = ch.extract(&mut multi_fruit) as i64;
        debug!("Match count: {}", count);
    }

    // facet
    if let Some(fh) = facet_handle {
        let facet_counts = fh.extract(&mut multi_fruit);
        let mut facet_kv: HashMap<String, u64> = HashMap::new();
        for facet_prefix in &facet_prefixes {
            for (facet_key, facet_value) in facet_counts.get(facet_prefix) {
                debug!("{}: {}", facet_key.to_string(), facet_value);
                facet_kv.insert(facet_key.to_string(), facet_value);
            }
        }
    }

    // docs
    if let Some(tdh) = top_docs_handle {
        let top_docs = tdh.extract(&mut multi_fruit);
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address).unwrap();
            // let named_doc = schema.to_named_doc(&doc);
            debug!("score: {} doc: {}", score, schema.to_json(&doc));
            println!("{}", doc.get_first(path_field).unwrap().text().unwrap());
        }
    }

    Ok(())
}

pub fn count(index: &Index, text: String) -> tantivy::Result<()> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let query = if text.is_empty() {
        Box::new(AllQuery)
    } else {
        raw_query(&index, &text)
    };

    let count = searcher.search(&query, &Count).unwrap();

    println!("Match count: {}", &count);

    let tags = index.schema().get_field("tags").unwrap();
    let mut facet_collector = FacetCollector::for_field(tags);
    facet_collector.add_facet("/");

    let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
    for (facet, count) in facet_counts.get("/") {
        println!("{}: {}", &facet, count);
    }

    Ok(())
}
