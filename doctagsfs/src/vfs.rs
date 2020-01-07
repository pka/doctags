use doctags::search::{doc_from_id, doc_from_path};
use std::ffi::OsStr;
use std::path::Path;
use tantivy::collector::{FacetCollector, TopDocs};
use tantivy::query::{AllQuery, TermQuery};
use tantivy::schema::*;
use tantivy::{self, doc, Index};

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    schema_builder.add_u64_field("id", INDEXED | STORED);
    schema_builder.add_u64_field("parent_id", INDEXED);
    schema_builder.add_text_field("name", TEXT | STORED);

    schema_builder.build()
}

pub fn create_vfs_tree(index: &Index, vfs_index: &mut Index) -> tantivy::Result<()> {
    let reader = index.reader()?;
    let mut writer = vfs_index.writer(6_000_000)?;

    let id_field = vfs_index.schema().get_field("id").unwrap();
    let parent_id_field = vfs_index.schema().get_field("parent_id").unwrap();
    let name_field = vfs_index.schema().get_field("name").unwrap();

    let searcher = reader.searcher();

    let tags_field = index.schema().get_field("tags").unwrap();

    let mut id: u64 = std::u64::MAX;
    let parent_id: u64 = 1;
    let mut facet_collector = FacetCollector::for_field(tags_field);
    facet_collector.add_facet("/");
    let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
    let mut facets = Vec::new();
    for (facet, _count) in facet_counts.get("/") {
        id -= 1;
        facets.push((id, parent_id, facet.to_string()));
        let name: &str = &facet.to_string()[1..];
        writer.add_document(doc!(id_field => id, parent_id_field => parent_id, name_field => name));
    }

    let mut facets2 = Vec::new();
    for (parent_id, _, facetstr) in &facets {
        let mut facet_collector = FacetCollector::for_field(tags_field);
        facet_collector.add_facet(&facetstr);
        let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
        for (facet, _count) in facet_counts.get("/") {
            id -= 1;
            facets2.push((id, *parent_id, facet.to_string()));
            let facetpath = facet.to_string();
            let name = Path::new(&facetpath).file_name().unwrap().to_str().unwrap();
            writer.add_document(
                doc!(id_field => id, parent_id_field => *parent_id, name_field => name),
            );
        }
    }
    facets.append(&mut facets2);
    dbg!(facets);

    writer.commit()?;

    Ok(())
}

pub fn file_from_id(index: &Index, id: u64) -> tantivy::Result<Option<(u64, String)>> {
    let path_field = index.schema().get_field("path").unwrap();
    if let Ok(Some(doc)) = doc_from_id(index, id) {
        let path = doc
            .get_first(path_field)
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        return Ok(Some((id, path)));
    }
    Ok(None)
}

pub fn files_from_parent_id(index: &Index, parent_id: u64) -> tantivy::Result<Vec<(u64, String)>> {
    let reader = index.reader()?;

    let searcher = reader.searcher();

    let schema = index.schema();
    let path_field = index.schema().get_field("path").unwrap();
    let id_field = index.schema().get_field("id").unwrap();
    let parent_id_field = index.schema().get_field("parent_id").unwrap();

    let term = Term::from_field_u64(parent_id_field, parent_id);
    let term_query = TermQuery::new(term, IndexRecordOption::Basic);
    let top_docs = searcher.search(&term_query, &TopDocs::with_limit(100))?;

    let mut docs = Vec::new();
    for (_score, doc_address) in top_docs {
        let doc = searcher.doc(doc_address)?;
        debug!("doc: {}", schema.to_json(&doc));
        let id = doc.get_first(id_field).unwrap().u64_value();
        let path = doc
            .get_first(path_field)
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        docs.push((id, path));
    }
    Ok(docs)
}

pub fn file_from_dir_entry(
    index: &Index,
    parent_id: u64,
    name: &OsStr,
) -> tantivy::Result<Option<(u64, String)>> {
    let path_field = index.schema().get_field("path").unwrap();
    let id_field = index.schema().get_field("id").unwrap();

    if let Ok(Some(doc)) = doc_from_id(index, parent_id) {
        let parent_path = doc
            .get_first(path_field)
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        let path = Path::new(&parent_path)
            .join(name)
            .to_str()
            .unwrap()
            .to_string();
        if let Ok(Some(doc)) = doc_from_path(index, &path) {
            let id = doc.get_first(id_field).unwrap().u64_value();
            return Ok(Some((id, path)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use doctags::index;
    use doctags::walk;
    use tantivy::collector::Count;
    use tantivy::query::AllQuery;

    #[test]
    fn create_index() -> tantivy::Result<()> {
        let schema = build_schema();
        let id = schema.get_field("id").unwrap();
        let parent_id = schema.get_field("parent_id").unwrap();
        let name = schema.get_field("name").unwrap();

        let index = Index::create_in_ram(schema);
        let mut writer = index.writer(6_000_000)?;
        writer.add_document(doc!(id => 3u64, parent_id => 2u64, name => "lang"));
        writer.commit()?;

        let reader = index.reader()?;
        let searcher = reader.searcher();
        let count = searcher.search(&AllQuery, &Count)?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn vfs_tree_generation() -> tantivy::Result<()> {
        let (index, mut index_writer) = index::create_in_ram().unwrap();
        walk::find(&"../..", |id, parent_id, path, tags| {
            index_writer.add(id, parent_id, path, tags).unwrap()
        });
        let _ = index_writer.commit();

        let vfs_schema = build_schema();
        let name_field = vfs_schema.get_field("name").unwrap();
        let mut vfs_index = Index::create_in_ram(vfs_schema.clone());

        let _ = create_vfs_tree(&index, &mut vfs_index);
        let reader = vfs_index.reader()?;
        let searcher = reader.searcher();
        let count = searcher.search(&AllQuery, &Count)?;
        assert_eq!(count, 9);
        let entries = searcher.search(&AllQuery, &TopDocs::with_limit(2))?;
        let (_score, doc_address) = entries.first().unwrap();
        let doc = searcher.doc(*doc_address)?;
        dbg!(&vfs_schema.to_json(&doc));
        let name = doc
            .get_first(name_field)
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        assert_eq!(name, "author");
        Ok(())
    }
}
