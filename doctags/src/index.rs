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

pub fn create_and_write(basedirs: &Vec<String>, index_path: &String) {
    let mut index_writer = create(index_path).unwrap();
    walk::find(basedirs, |id, parent_id, path, tags| {
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

    let id = schema.get_field("id").unwrap();
    let parent_id = schema.get_field("parent_id").unwrap();
    let path = schema.get_field("path").unwrap();
    let tags = schema.get_field("tags").unwrap();

    let index = tantivy::Index::create_in_dir(&index_path, schema)?;

    let writer = index.writer(50_000_000)?;

    Ok(IndexWriter {
        writer,
        id,
        parent_id,
        path,
        tags,
    })
}

pub fn create_in_ram() -> tantivy::Result<(Index, IndexWriter)> {
    let schema = build_schema();

    let id = schema.get_field("id").unwrap();
    let parent_id = schema.get_field("parent_id").unwrap();
    let path = schema.get_field("path").unwrap();
    let tags = schema.get_field("tags").unwrap();

    let index = Index::create_in_ram(schema);

    let writer = index.writer(6_000_000)?;
    let index_writer = IndexWriter {
        writer,
        id,
        parent_id,
        path,
        tags,
    };

    Ok((index, index_writer))
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
        let (index, mut idx) = create_in_ram().unwrap();

        idx.writer
            .add_document(doc!(idx.id => 3u64, idx.parent_id => 2u64, idx.path => "/root"));
        let _ = idx.commit();

        let reader = index.reader()?;
        let searcher = reader.searcher();
        let count = searcher.search(&AllQuery, &Count)?;
        assert_eq!(count, 1);
        Ok(())
    }
}
