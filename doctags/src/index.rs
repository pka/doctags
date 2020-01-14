use crate::walk;
use anyhow::{Context, Result};
use failure::ResultExt;
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

pub fn create_and_write(basedirs: &Vec<String>, index_path: &String) -> Result<()> {
    let mut index_writer = create(index_path)?;
    walk::find(basedirs, |id, parent_id, path, tags| {
        index_writer.add(id, parent_id, path, tags).unwrap() // TODO
    })?;
    index_writer.commit()?;
    Ok(())
}

pub fn create(index_path: &String) -> Result<IndexWriter> {
    if Path::new(index_path).exists() {
        if Path::new(index_path).join(".managed.json").exists() {
            debug!("Recreating index at {}", index_path);
            fs::remove_dir_all(index_path)?;
        } else {
            return Err(anyhow!("Couldn't find Tantivy index in '{}'", &index_path));
        }
    } else {
        info!("Creating index at {}", index_path);
    }
    std::fs::create_dir_all(index_path)?;

    let schema = build_schema();

    let id = schema.get_field("id").context("Field 'id' not found")?;
    let parent_id = schema
        .get_field("parent_id")
        .context("Field 'parent_id' not found")?;
    let path = schema.get_field("path").context("Field 'path' not found")?;
    let tags = schema.get_field("tags").context("Field 'tags' not found")?;

    let index = tantivy::Index::create_in_dir(&index_path, schema).compat()?;

    let writer = index.writer(50_000_000).compat()?;

    Ok(IndexWriter {
        writer,
        id,
        parent_id,
        path,
        tags,
    })
}

pub fn create_in_ram() -> Result<(Index, IndexWriter)> {
    let schema = build_schema();

    let id = schema.get_field("id").context("Field 'id' not found")?;
    let parent_id = schema
        .get_field("parent_id")
        .context("Field 'parent_id' not found")?;
    let path = schema.get_field("path").context("Field 'path' not found")?;
    let tags = schema.get_field("tags").context("Field 'tags' not found")?;

    let index = Index::create_in_ram(schema);

    let writer = index.writer(6_000_000).compat()?;
    let index_writer = IndexWriter {
        writer,
        id,
        parent_id,
        path,
        tags,
    };

    Ok((index, index_writer))
}

pub fn open(index_path: &String) -> Result<Index> {
    Ok(Index::open_in_dir(index_path).compat()?)
}

impl IndexWriter {
    pub fn add(&mut self, id: u64, parent_id: u64, path: &str, tags: &Vec<&String>) -> Result<()> {
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
    pub fn commit(&mut self) -> Result<u64> {
        Ok(self.writer.commit().compat()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::collector::Count;
    use tantivy::doc;
    use tantivy::query::AllQuery;

    #[test]
    fn create_index() -> Result<()> {
        let (index, mut idx) = create_in_ram()?;

        idx.writer
            .add_document(doc!(idx.id => 3u64, idx.parent_id => 2u64, idx.path => "/root"));
        let _ = idx.commit();

        let reader = index.reader().compat()?;
        let searcher = reader.searcher();
        let count = searcher.search(&AllQuery, &Count).compat()?;
        assert_eq!(count, 1);
        Ok(())
    }
}
