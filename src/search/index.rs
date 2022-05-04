use std::fs;
use std::path::Path;

use anyhow::Result;
use tantivy::directory::MmapDirectory;
use tantivy::schema::Schema;
use tantivy::tokenizer::RawTokenizer;
use tantivy::{IndexReader, ReloadPolicy};

use crate::search::tokenizer::SimpleUnicodeTokenizer;
use crate::search::writer::Writer;

pub async fn open_or_create(
    path: &Path,
    schema: Schema,
    num_readers: usize,
) -> Result<(IndexReader, Schema, Writer)> {
    fs::create_dir_all(path)?;

    let dir = MmapDirectory::open(path)?;

    let index = if tantivy::Index::exists(&dir)? {
        tantivy::Index::open(dir)
    } else {
        tantivy::Index::open_or_create(dir, schema)
    }?;

    index
        .tokenizers()
        .register("default", SimpleUnicodeTokenizer::default());

    index.tokenizers().register("raw", RawTokenizer);

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .num_searchers(num_readers)
        .try_into()?;

    let schema = index.schema();
    let writer = super::writer::start_writer(index).await?;

    Ok((reader, schema, writer))
}
