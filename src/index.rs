use tantivy::schema::*;
use tantivy::{self, Index};

pub struct IndexWriter {
    writer: tantivy::IndexWriter,
    path: Field,
    tags: Field,
}

pub fn create(index_path: &String) -> tantivy::Result<IndexWriter> {
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
    pub fn add(&mut self, entry: &str) -> tantivy::Result<()> {
        let mut doc = Document::new();
        doc.add_text(self.path, entry);
        doc.add_facet(self.tags, Facet::from("/dummy"));
        self.writer.add_document(doc);

        Ok(())
    }
    pub fn commit(&mut self) -> tantivy::Result<u64> {
        self.writer.commit()
    }
}
