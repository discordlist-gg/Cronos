use std::path::Path;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tantivy::schema::{FAST, INDEXED, IndexRecordOption, Schema, SchemaBuilder, STORED, TEXT, TextFieldIndexing, TextOptions};
use tokio::sync::Semaphore;

use crate::models::bots::Bot;
use crate::search::writer::Writer;
use crate::search::index;
use crate::search::readers::bots;

pub static ID_FIELD: &str = "id";
pub static FEATURES_FIELD: &str = "features";
pub static USERNAME_FIELD: &str = "username";
pub static DESCRIPTION_FIELD: &str = "brief_description";
pub static CATEGORIES_FIELD: &str = "categories";
pub static TAGS_FIELD: &str = "tags";

pub struct BotIndex {
    writer: Writer,
    schema: Schema,
}

impl BotIndex {
    pub async fn create(path: &Path, limiter: Arc<Semaphore>, max_concurrency: usize) -> Result<Self> {
        let (
            reader,
            schema,
            writer,
        ) = index::open_or_create(path,default_schema(), max_concurrency).await?;

        let id_field = schema.get_field(ID_FIELD).unwrap();
        let search_fields = vec![
            schema.get_field(USERNAME_FIELD).unwrap(),
            schema.get_field(DESCRIPTION_FIELD).unwrap(),
            schema.get_field(TAGS_FIELD).unwrap(),
            schema.get_field(CATEGORIES_FIELD).unwrap(),
        ];

        bots::init(id_field, search_fields, reader, limiter);

        Ok(Self { writer, schema })
    }

    pub async fn upsert_bot(&self, bot_id: i64) -> Result<()> {
        let bot = Bot::fetch(bot_id)
            .await?
            .ok_or_else(|| anyhow!("Bot does not exist!"))?;

        let doc = bot.as_tantivy_doc(&self.schema);
        self.writer.add_documents()
    }
}


fn default_schema() -> Schema {
    let mut builder = SchemaBuilder::new();

    builder.add_i64_field(ID_FIELD, INDEXED | FAST);
    builder.add_u64_field(FEATURES_FIELD, INDEXED | FAST);
    builder.add_text_field(USERNAME_FIELD, TEXT);
    builder.add_text_field(DESCRIPTION_FIELD, TEXT);
    builder.add_text_field(TAGS_FIELD, TEXT);
    builder.add_text_field(CATEGORIES_FIELD, TEXT);

    builder.build()
}

