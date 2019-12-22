use tantivy::collector::{Count, FacetCollector, TopDocs};
use tantivy::query::{AllQuery, QueryParser};
use tantivy::{self, Index};

pub fn search(index: &Index, text: String) -> tantivy::Result<()> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let path = index.schema().get_field("path").unwrap();

    let query_parser = QueryParser::for_index(&index, vec![path]);

    let query = query_parser.parse_query(&text)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address)?;
        println!("{}", index.schema().to_json(&retrieved_doc));
    }

    Ok(())
}

pub fn count(index: &Index, text: String) -> tantivy::Result<()> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let path = index.schema().get_field("path").unwrap();

    let query = if text.is_empty() {
        Box::new(AllQuery)
    } else {
        let query_parser = QueryParser::for_index(&index, vec![path]);

        query_parser.parse_query(&text)?
    };

    let count = searcher.search(&query, &Count).unwrap();

    println!("Match count: {}", &count);

    let tags = index.schema().get_field("tags").unwrap();
    let mut facet_collector = FacetCollector::for_field(tags);
    facet_collector.add_facet("/file_type");

    let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
    for (facet, count) in facet_counts.get("/file_type") {
        println!("{}: {}", &facet, count);
    }

    Ok(())
}
