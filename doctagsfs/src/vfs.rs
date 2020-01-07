use doctags::search::{doc_from_id, doc_from_path};
use std::ffi::OsStr;
use std::path::Path;
use tantivy::collector::{FacetCollector, TopDocs};
use tantivy::query::{AllQuery, TermQuery};
use tantivy::schema::*;
use tantivy::{self, Index};

#[derive(PartialEq, Debug)]
enum FsEntry {
    Tag(String),
}

pub struct VfsEntry {
    id: u64,
    parent_id: u64,
    fs_entry: FsEntry,
}

pub struct DoctagsFS {
    index: Index,
    entries: Vec<VfsEntry>,
}

impl DoctagsFS {
    pub fn create_vfs_tree(&self) -> Vec<VfsEntry> {
        let reader = self.index.reader().unwrap();

        let searcher = reader.searcher();

        let tags_field = self.index.schema().get_field("tags").unwrap();

        let mut id: u64 = std::u64::MAX;
        let parent_id: u64 = 1;
        let mut facet_collector = FacetCollector::for_field(tags_field);
        facet_collector.add_facet("/");
        let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
        let mut entries = Vec::new();
        let mut facets = Vec::new();
        for (facet, _count) in facet_counts.get("/") {
            id -= 1;
            facets.push((id, parent_id, facet.to_string()));
            let name: &str = &facet.to_string()[1..];
            entries.push(VfsEntry {
                id,
                parent_id,
                fs_entry: FsEntry::Tag(name.to_string()),
            });
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
                entries.push(VfsEntry {
                    id,
                    parent_id: *parent_id,
                    fs_entry: FsEntry::Tag(name.to_string()),
                });
            }
        }
        facets.append(&mut facets2);
        dbg!(facets);

        entries
    }

    pub fn file_from_id(&self, id: u64) -> tantivy::Result<Option<(u64, String)>> {
        let path_field = self.index.schema().get_field("path").unwrap();
        if let Ok(Some(doc)) = doc_from_id(&self.index, id) {
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

    pub fn files_from_parent_id(&self, parent_id: u64) -> tantivy::Result<Vec<(u64, String)>> {
        let reader = self.index.reader()?;

        let searcher = reader.searcher();

        let schema = self.index.schema();
        let path_field = self.index.schema().get_field("path").unwrap();
        let id_field = self.index.schema().get_field("id").unwrap();
        let parent_id_field = self.index.schema().get_field("parent_id").unwrap();

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
        &self,
        parent_id: u64,
        name: &OsStr,
    ) -> tantivy::Result<Option<(u64, String)>> {
        let path_field = self.index.schema().get_field("path").unwrap();
        let id_field = self.index.schema().get_field("id").unwrap();

        if let Ok(Some(doc)) = doc_from_id(&self.index, parent_id) {
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
            if let Ok(Some(doc)) = doc_from_path(&self.index, &path) {
                let id = doc.get_first(id_field).unwrap().u64_value();
                return Ok(Some((id, path)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doctags::index;
    use doctags::walk;

    #[test]
    fn vfs_tree_generation() -> tantivy::Result<()> {
        let (index, mut index_writer) = index::create_in_ram().unwrap();
        walk::find(&"../..", |id, parent_id, path, tags| {
            index_writer.add(id, parent_id, path, tags).unwrap()
        });
        let _ = index_writer.commit();

        let fs = DoctagsFS {
            index,
            entries: vec![],
        };
        let entries = fs.create_vfs_tree();
        assert_eq!(entries.len(), 9);
        assert_eq!(entries[0].fs_entry, FsEntry::Tag("author".to_string()));
        Ok(())
    }
}
