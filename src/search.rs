use std::collections::HashMap;
use tantivy::collector::{Count, FacetCollector, MultiCollector, TopDocs};
use tantivy::query::{AllQuery, QueryParser};
use tantivy::{self, Index};

pub fn search(index: &Index, text: String) -> tantivy::Result<()> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let schema = index.schema();
    let path_field = schema.get_field("path").unwrap();
    let tags_field = schema.get_field("tags").unwrap();

    let query_parser = QueryParser::for_index(&index, vec![path_field]);

    let query = query_parser.parse_query(&text)?;

    let limit = 10;
    let exclude_count = false;
    let exclude_docs = false;
    let facet_prefixes = vec!["/file_type"];

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
        println!("Match count: {}", count);
    }

    // facet
    if let Some(fh) = facet_handle {
        let facet_counts = fh.extract(&mut multi_fruit);
        let mut facet_kv: HashMap<String, u64> = HashMap::new();
        for facet_prefix in &facet_prefixes {
            for (facet_key, facet_value) in facet_counts.get(facet_prefix) {
                println!("{}: {}", facet_key.to_string(), facet_value);
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
            println!("score: {} doc: {}", score, schema.to_json(&doc));
        }
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
