use std::fs;
use std::path::Path;
use tantivy::schema::*;
use tantivy::{self, Index};

pub struct IndexWriter {
    writer: tantivy::IndexWriter,
    path: Field,
    tags: Field,
}

pub fn create(index_path: &String) -> tantivy::Result<IndexWriter> {
    if Path::new(index_path).exists() {
        if Path::new(index_path).join(".managed.json").exists() {
            fs::remove_dir_all(index_path).unwrap();
        } else {
            return Err(tantivy::Error::IndexAlreadyExists);
        }
    }
    std::fs::create_dir_all(index_path).unwrap();

    let mut schema_builder = Schema::builder();

    schema_builder.add_text_field("path", TEXT | STORED);
    schema_builder.add_facet_field("tags");

    let schema = schema_builder.build();

    let index = tantivy::Index::create_in_dir(&index_path, schema.clone())?;

    let writer = index.writer(50_000_000)?;

    let path = schema.get_field("path").unwrap();
    let tags = schema.get_field("tags").unwrap();

    Ok(IndexWriter { writer, path, tags })
}

pub fn open(index_path: String) -> tantivy::Result<Index> {
    Index::open_in_dir(&index_path)
}

impl IndexWriter {
    pub fn add(&mut self, path: &str, is_dir: bool) -> tantivy::Result<()> {
        let mut doc = Document::new();
        doc.add_text(self.path, path);
        let facet = if is_dir {
            "/file_type/dir"
        } else {
            "/file_type/file"
        };
        doc.add_facet(self.tags, Facet::from(facet));
        self.writer.add_document(doc);

        Ok(())
    }
    pub fn commit(&mut self) -> tantivy::Result<u64> {
        self.writer.commit()
    }
}
