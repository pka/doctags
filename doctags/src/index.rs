use crate::walk;
use std::fs;
use std::path::Path;
use tantivy::schema::*;
use tantivy::{self, Index};

pub struct IndexWriter {
    writer: tantivy::IndexWriter,
    id: Field,
    parent_id: Field,
    path: Field,
    tags: Field,
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    schema_builder.add_u64_field("id", INDEXED | STORED);
    schema_builder.add_u64_field("parent_id", INDEXED);
    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_facet_field("tags");

    schema_builder.build()
}

pub fn create_and_write(basedir: &str, index_path: &String) {
    let mut index_writer = create(index_path).unwrap();
    walk::find(&basedir, |id, parent_id, path, tags| {
        index_writer.add(id, parent_id, path, tags).unwrap()
    });
    let _ = index_writer.commit();
}

pub fn create(index_path: &String) -> tantivy::Result<IndexWriter> {
    if Path::new(index_path).exists() {
        if Path::new(index_path).join(".managed.json").exists() {
            debug!("Recreating index at {}", index_path);
            fs::remove_dir_all(index_path).unwrap();
        } else {
            return Err(tantivy::Error::IndexAlreadyExists);
        }
    } else {
        info!("Creating index at {}", index_path);
    }
    std::fs::create_dir_all(index_path).unwrap();

    let schema = build_schema();

    let index = tantivy::Index::create_in_dir(&index_path, schema.clone())?;

    let writer = index.writer(50_000_000)?;

    let id = schema.get_field("id").unwrap();
    let parent_id = schema.get_field("parent_id").unwrap();
    let path = schema.get_field("path").unwrap();
    let tags = schema.get_field("tags").unwrap();

    Ok(IndexWriter {
        writer,
        id,
        parent_id,
        path,
        tags,
    })
}

pub fn open(index_path: &String) -> tantivy::Result<Index> {
    Index::open_in_dir(index_path)
}

impl IndexWriter {
    pub fn add(
        &mut self,
        id: u64,
        parent_id: u64,
        path: &str,
        tags: &Vec<&String>,
    ) -> tantivy::Result<()> {
        let mut doc = Document::new();
        doc.add_u64(self.id, id);
        doc.add_u64(self.parent_id, parent_id);
        doc.add_text(self.path, path);
        for tag in tags {
            doc.add_facet(self.tags, Facet::from(tag.as_str()));
        }
        self.writer.add_document(doc);

        Ok(())
    }
    pub fn commit(&mut self) -> tantivy::Result<u64> {
        self.writer.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::collector::Count;
    use tantivy::doc;
    use tantivy::query::AllQuery;

    #[test]
    fn create_index() -> tantivy::Result<()> {
        let schema = build_schema();
        let id = schema.get_field("id").unwrap();
        let parent_id = schema.get_field("parent_id").unwrap();
        let path = schema.get_field("path").unwrap();

        let index = Index::create_in_ram(build_schema());
        let mut writer = index.writer(6_000_000)?;
        writer.add_document(doc!(id => 3u64, parent_id => 2u64, path => "/root"));
        writer.commit()?;

        let reader = index.reader()?;
        let searcher = reader.searcher();
        let count = searcher.search(&AllQuery, &Count)?;
        assert_eq!(count, 1);
        Ok(())
    }
}
