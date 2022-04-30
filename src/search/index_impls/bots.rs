use std::path::Path;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tantivy::schema::{FAST, Field, INDEXED, Schema, SchemaBuilder, STORED, TEXT};
use tantivy::Term;
use once_cell::sync::OnceCell;
use tokio::sync::Semaphore;

use crate::models::bots::{Bot, remove_bot_from_live, update_live_data};
use crate::search::writer::Writer;
use crate::search::index;
use crate::search::readers::bots;

pub static ID_FIELD: &str = "id";
pub static FEATURES_FIELD: &str = "features";
pub static USERNAME_FIELD: &str = "username";
pub static DESCRIPTION_FIELD: &str = "brief_description";
pub static CATEGORIES_FIELD: &str = "categories";
pub static TAGS_FIELD: &str = "tags";

static BOT_INDEX: OnceCell<BotIndex> = OnceCell::new();

pub async fn init_index(path: &Path, limiter: Arc<Semaphore>, max_concurrency: usize) -> Result<()> {
    let index = BotIndex::create(path, limiter, max_concurrency).await?;
    let _ = BOT_INDEX.set(index);

    Ok(())
}

pub fn writer() -> &'static BotIndex {
    BOT_INDEX.get().unwrap()
}

pub struct BotIndex {
    id_field: Field,
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

        Ok(Self { id_field, writer, schema })
    }

    pub async fn remove_bot(&self, bot_id: i64) -> Result<()> {
        let term = Term::from_field_i64(self.id_field, bot_id);
        self.writer.remove_docs(term).await?;

        remove_bot_from_live(bot_id);

        Ok(())
    }

    pub async fn upsert_bot(&self, bot_id: i64) -> Result<()> {
        let bot = Bot::fetch(bot_id)
            .await?
            .ok_or_else(|| anyhow!("Bot does not exist!"))?;

        let doc = bot.as_tantivy_doc(&self.schema);
        self.writer.add_document(doc).await?;

        update_live_data(bot);

        Ok(())
    }
}


fn default_schema() -> Schema {
    let mut builder = SchemaBuilder::new();

    builder.add_i64_field(ID_FIELD, INDEXED | FAST | STORED);
    builder.add_u64_field(FEATURES_FIELD, INDEXED | FAST);
    builder.add_text_field(USERNAME_FIELD, TEXT);
    builder.add_text_field(DESCRIPTION_FIELD, TEXT);
    builder.add_text_field(TAGS_FIELD, TEXT);
    builder.add_text_field(CATEGORIES_FIELD, TEXT);

    builder.build()
}

