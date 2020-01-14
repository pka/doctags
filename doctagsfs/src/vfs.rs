use anyhow::{Context, Result};
use doctags::search::{doc_from_id, doc_from_path};
use failure::ResultExt;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use tantivy::collector::{FacetCollector, TopDocs};
use tantivy::query::{AllQuery, TermQuery};
use tantivy::schema::*;
use tantivy::{self, Document, Index};

#[derive(Clone, PartialEq, Debug)]
pub enum FsEntry {
    Tag(String),
    Path(String),
}

#[derive(Clone, Debug)]
pub struct VfsEntry {
    pub id: u64,
    pub entry: FsEntry,
}

pub struct DoctagsFS {
    index: Index,
    /// id -> entry
    entries: HashMap<u64, VfsEntry>,
    /// parent_id -> entry ids
    children: HashMap<u64, Vec<u64>>,
    /// parent_id -> query
    queries: HashMap<u64, String>,
}

impl DoctagsFS {
    pub fn new(index: Index) -> DoctagsFS {
        DoctagsFS {
            index,
            entries: HashMap::new(),
            children: HashMap::new(),
            queries: HashMap::new(),
        }
    }

    fn add_tag_entry(&mut self, id: u64, parent_id: u64, tag: &str) {
        self.entries.insert(
            id,
            VfsEntry {
                id: id,
                entry: FsEntry::Tag(tag.to_string()),
            },
        );
        self.children
            .entry(parent_id)
            .or_insert(Vec::new())
            .push(id);
    }

    pub fn create_vfs_tree(&mut self) -> Result<()> {
        let reader = self.index.reader().compat()?;

        let searcher = reader.searcher();

        self.entries.insert(
            fuse::FUSE_ROOT_ID,
            VfsEntry {
                id: fuse::FUSE_ROOT_ID,
                entry: FsEntry::Tag("FUSEROOT".to_string()),
            },
        );
        let mut id: u64 = std::u64::MAX; // we use ids down from std::u64::MAX
        let parent_id: u64 = fuse::FUSE_ROOT_ID;
        self.add_tag_entry(id, parent_id, "_");

        let tags_field = self
            .index
            .schema()
            .get_field("tags")
            .context("Field 'tag' not found")?;
        let mut facet_collector = FacetCollector::for_field(tags_field);
        facet_collector.add_facet("/");
        let facet_counts = searcher.search(&AllQuery, &facet_collector).compat()?;
        let mut facets = HashMap::new();
        for (facet, _count) in facet_counts.get("/") {
            id -= 1;
            facets.insert(id, facet.to_string());
            let name: &str = &facet.to_string()[1..];
            self.add_tag_entry(id, parent_id, name);
        }

        // TODO: support depth > 2 and combinations of tags
        let mut facets2 = Vec::new();
        for (parent_id, facetstr) in &facets {
            let mut facet_collector = FacetCollector::for_field(tags_field);
            facet_collector.add_facet(&facetstr);
            let facet_counts = searcher.search(&AllQuery, &facet_collector).compat()?;
            for (facet, _count) in facet_counts.get("/") {
                id -= 1;
                facets2.push((id, facet.to_string()));
                let facetpath = facet.to_string();
                let name = Path::new(&facetpath)
                    .file_name()
                    .context("Couldn't determine file name")?
                    .to_str()
                    .context("Couldn't convert filename to str")?;
                self.add_tag_entry(id, *parent_id, name);
            }
        }
        for (id, facet) in facets2 {
            facets.insert(id, facet);
        }

        // Add facet query for each leaf in entries tree
        for id in self.entries.keys() {
            if !self.children.contains_key(id) {
                if let Some(facet) = facets.get(id) {
                    self.queries.insert(*id, facet.to_string());
                }
            }
        }
        Ok(())
    }

    fn path_from_doc(&self, doc: Document) -> Result<String> {
        let path_field = self
            .index
            .schema()
            .get_field("path")
            .context("Field 'path' not found")?;
        let path = doc
            .get_first(path_field)
            .context("No 'path' entry in doc")?
            .text()
            .context("Couldn't convert 'path' entry to text")?
            .to_string();
        Ok(path)
    }

    pub fn entry_from_id(&self, id: u64) -> Result<Option<VfsEntry>> {
        if let Some(entry) = self.entries.get(&id) {
            return Ok(Some(entry.clone()));
        } else if let Ok(Some(doc)) = doc_from_id(&self.index, id) {
            return Ok(Some(VfsEntry {
                id,
                entry: FsEntry::Path(self.path_from_doc(doc)?),
            }));
        }
        Ok(None)
    }

    pub fn entries_from_parent_id(&self, parent_id: u64) -> Result<Vec<VfsEntry>> {
        if let Some(query) = self.queries.get(&parent_id) {
            let reader = self.index.reader().compat()?;

            let searcher = reader.searcher();

            let schema = self.index.schema();
            let id_field = schema.get_field("id").context("Field 'id' not found")?;
            let tags_field = schema.get_field("tags").context("Field 'tags' not found")?;

            let term = Term::from_facet(tags_field, &Facet::from(&query));
            let term_query = TermQuery::new(term, IndexRecordOption::Basic);
            // TODO: order by parent_id and limit to first sub level
            let top_docs = searcher
                .search(&term_query, &TopDocs::with_limit(100))
                .compat()?;

            let mut entries = Vec::new();
            for (_score, doc_address) in top_docs {
                let doc = searcher.doc(doc_address).compat()?;
                // debug!("doc: {}", schema.to_json(&doc));
                let id = doc
                    .get_first(id_field)
                    .context("No 'id' entry in doc")?
                    .u64_value();
                entries.push(VfsEntry {
                    id,
                    entry: FsEntry::Path(self.path_from_doc(doc)?),
                });
            }
            Ok(entries)
        } else if let Some(ids) = self.children.get(&parent_id) {
            Ok(ids.iter().map(|id| self.entries[id].clone()).collect())
        } else {
            let reader = self.index.reader().compat()?;

            let searcher = reader.searcher();

            let schema = self.index.schema();
            let id_field = schema.get_field("id").context("Field 'id' not found")?;
            let parent_id_field = schema
                .get_field("parent_id")
                .context("Field 'parent_id' not found")?;

            let term = Term::from_field_u64(parent_id_field, parent_id);
            let term_query = TermQuery::new(term, IndexRecordOption::Basic);
            let top_docs = searcher
                .search(&term_query, &TopDocs::with_limit(100))
                .compat()?;

            let mut entries = Vec::new();
            for (_score, doc_address) in top_docs {
                let doc = searcher.doc(doc_address).compat()?;
                // debug!("doc: {}", schema.to_json(&doc));
                let id = doc
                    .get_first(id_field)
                    .context("No 'id' entry in doc")?
                    .u64_value();
                entries.push(VfsEntry {
                    id,
                    entry: FsEntry::Path(self.path_from_doc(doc)?),
                });
            }
            Ok(entries)
        }
    }

    pub fn entry_from_dir_entry(&self, parent_id: u64, name: &OsStr) -> Result<Option<VfsEntry>> {
        if let Some(ids) = self.children.get(&parent_id) {
            let entry = ids
                .iter()
                .map(|id| &self.entries[id])
                .find(|e| e.entry == FsEntry::Tag(name.to_string_lossy().to_string()))
                .map(|e| e.clone());
            return Ok(entry);
        } else if let Some(_query) = self.queries.get(&parent_id) {
            // FIXME Return dummy file
            return Ok(Some(VfsEntry {
                id: 2,
                entry: FsEntry::Path("/home/pi/code/rust/doctags".to_string()),
            }));
        } else if self.entries.contains_key(&parent_id) {
            // special parent '_' for all files
            // TODO: return base path
            return Ok(Some(VfsEntry {
                id: 2,
                entry: FsEntry::Tag(name.to_string_lossy().to_string()),
            }));
        } else {
            let id_field = self
                .index
                .schema()
                .get_field("id")
                .context("No 'id' entry in doc")?;

            if let Ok(Some(doc)) = doc_from_id(&self.index, parent_id) {
                let parent_path = self.path_from_doc(doc)?;
                let path = Path::new(&parent_path)
                    .join(name)
                    .to_str()
                    .context("Couldn't convert path to str")?
                    .to_string();
                if let Ok(Some(doc)) = doc_from_path(&self.index, &path) {
                    let id = doc
                        .get_first(id_field)
                        .context("No 'id' entry in doc")?
                        .u64_value();
                    return Ok(Some(VfsEntry {
                        id,
                        entry: FsEntry::Path(path),
                    }));
                }
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
    fn vfs_tree_generation() -> Result<()> {
        let (index, mut index_writer) = index::create_in_ram()?;
        walk::find(
            &vec![format!("{}/..", env!("CARGO_MANIFEST_DIR"))],
            |id, parent_id, path, tags| index_writer.add(id, parent_id, path, tags).unwrap(), // TODO
        )?;
        index_writer.commit()?;

        let mut fs = DoctagsFS::new(index);
        fs.create_vfs_tree()?;
        assert_eq!(fs.entries.len(), 9);
        assert_eq!(
            fs.entries[&(std::u64::MAX - 1)].entry,
            FsEntry::Tag("author".to_string())
        );
        assert_eq!(fs.children.len(), 4);
        assert_eq!(fs.children[&1].len(), 5);
        assert_eq!(fs.queries.len(), 4);
        Ok(())
    }
}
