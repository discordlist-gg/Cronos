use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use tantivy::schema::{Field, Schema, SchemaBuilder, FAST, INDEXED, STORED, TEXT};
use tantivy::Term;
use tokio::sync::Semaphore;

use crate::models;
use crate::models::packs::{remove_pack_from_live, update_live_data, Pack};
use crate::search::index;
use crate::search::readers::packs;
use crate::search::writer::Writer;

pub static ID_FIELD: &str = "id";
pub static NAME_FIELD: &str = "name";
pub static DESCRIPTION_FIELD: &str = "description";
pub static TAG_FIELD: &str = "tag";

static PACK_INDEX: OnceCell<PackIndex> = OnceCell::new();

pub async fn init_index(
    path: &Path,
    limiter: Arc<Semaphore>,
    max_concurrency: usize,
) -> Result<()> {
    let index = PackIndex::create(path, limiter, max_concurrency).await?;
    let _ = PACK_INDEX.set(index);

    Ok(())
}

pub fn writer() -> &'static PackIndex {
    PACK_INDEX.get().unwrap()
}

pub struct PackIndex {
    id_field: Field,
    writer: Writer,
    schema: Schema,
}

impl PackIndex {
    pub async fn create(
        path: &Path,
        limiter: Arc<Semaphore>,
        max_concurrency: usize,
    ) -> Result<Self> {
        let (reader, schema, writer) =
            index::open_or_create(path, default_schema(), max_concurrency).await?;

        let id_field = schema.get_field(ID_FIELD).unwrap();
        let search_fields = vec![
            schema.get_field(NAME_FIELD).unwrap(),
            schema.get_field(DESCRIPTION_FIELD).unwrap(),
            schema.get_field(TAG_FIELD).unwrap(),
        ];

        packs::init(id_field, search_fields, reader, limiter);

        Ok(Self {
            id_field,
            writer,
            schema,
        })
    }

    pub async fn remove_pack(&self, pack_id: i64) -> Result<()> {
        let term = Term::from_field_i64(self.id_field, pack_id);
        self.writer.remove_docs(term).await?;

        remove_pack_from_live(pack_id);

        Ok(())
    }

    pub async fn upsert_pack(&self, pack_id: i64) -> Result<()> {
        let pack = Pack::fetch(pack_id)
            .await?
            .ok_or_else(|| anyhow!("Bot does not exist!"))?;

        let doc = pack.as_tantivy_doc(&self.schema);
        self.writer.add_document(doc).await?;

        update_live_data(pack);

        Ok(())
    }

    pub async fn full_refresh(&self) -> Result<()> {
        self.writer.clear_all_docs().await?;
        models::packs::refresh_latest_data().await?;

        for pack in models::packs::all_packs() {
            self.writer
                .add_document(pack.as_tantivy_doc(&self.schema))
                .await?;
        }

        Ok(())
    }
}

fn default_schema() -> Schema {
    let mut builder = SchemaBuilder::new();

    builder.add_i64_field(ID_FIELD, INDEXED | FAST | STORED);
    builder.add_text_field(NAME_FIELD, TEXT);
    builder.add_text_field(DESCRIPTION_FIELD, TEXT);
    builder.add_text_field(TAG_FIELD, TEXT);

    builder.build()
}
