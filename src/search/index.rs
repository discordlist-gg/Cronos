use std::fs;
use std::path::Path;
use tantivy::schema::Schema;
use anyhow::Result;
use tantivy::{Directory, Index, IndexReader, ReloadPolicy};
use tantivy::directory::MmapDirectory;
use tokio::task::JoinHandle;


pub async fn open_or_create(
    path: &Path,
    schema: Schema,
    num_readers: usize,
) -> Result<(IndexReader, writer)> {
    fs::create_dir_all(path)?;

    let dir = MmapDirectory::open(path)?;

    let index = if dir.exists(path)? {
        tantivy::Index::open(dir)
    } else {
        tantivy::Index::open_or_create(dir, schema)
    }?;

    let reader = index.reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .num_searchers(num_readers)
        .try_into()?;

    let writer = super::writer::start_writer(index).await?;

    Ok((reader, writer))
}
