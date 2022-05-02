use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use tantivy::schema::{Field, Schema, SchemaBuilder, FAST, INDEXED, STORED, TEXT};
use tantivy::Term;
use tokio::sync::Semaphore;

use crate::models;
use crate::models::bots::{remove_bot_from_live, update_live_data, Bot};
use crate::search::index;
use crate::search::readers::bots;
use crate::search::readers::bots::FieldContext;
use crate::search::writer::Writer;

pub static ID_FIELD: &str = "id";
pub static PREMIUM_FIELD: &str = "premium";
pub static FEATURES_FIELD: &str = "features";
pub static USERNAME_FIELD: &str = "username";
pub static DESCRIPTION_FIELD: &str = "brief_description";
pub static TAGS_FIELD: &str = "tags";

static BOT_INDEX: OnceCell<BotIndex> = OnceCell::new();

pub async fn init_index(
    path: &Path,
    limiter: Arc<Semaphore>,
    max_concurrency: usize,
) -> Result<()> {
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
    pub async fn create(
        path: &Path,
        limiter: Arc<Semaphore>,
        max_concurrency: usize,
    ) -> Result<Self> {
        let (reader, schema, writer) =
            index::open_or_create(path, default_schema(), max_concurrency).await?;

        let id_field = schema.get_field(ID_FIELD).unwrap();
        let premium_field = schema.get_field(PREMIUM_FIELD).unwrap();
        let tags_field = schema.get_field(TAGS_FIELD).unwrap();
        let features_field = schema.get_field(FEATURES_FIELD).unwrap();
        let search_fields = vec![
            schema.get_field(USERNAME_FIELD).unwrap(),
            schema.get_field(DESCRIPTION_FIELD).unwrap(),
            tags_field,
        ];

        let ctx = FieldContext {
            id_field,
            premium_field,
            tags_field,
            features_field,
        };

        bots::init(ctx, search_fields, reader, limiter);

        Ok(Self {
            id_field,
            writer,
            schema,
        })
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

    pub async fn full_refresh(&self) -> Result<()> {
        self.writer.clear_all_docs().await?;
        models::bots::refresh_latest_data().await?;

        for bot in models::bots::all_bots() {
            self.writer
                .add_document(bot.as_tantivy_doc(&self.schema))
                .await?;
        }

        Ok(())
    }
}

fn default_schema() -> Schema {
    let mut builder = SchemaBuilder::new();

    builder.add_i64_field(ID_FIELD, INDEXED | FAST | STORED);
    builder.add_u64_field(FEATURES_FIELD, INDEXED | FAST);
    builder.add_u64_field(PREMIUM_FIELD, INDEXED | FAST);
    builder.add_text_field(USERNAME_FIELD, TEXT);
    builder.add_text_field(DESCRIPTION_FIELD, TEXT);
    builder.add_text_field(TAGS_FIELD, TEXT | FAST);

    builder.build()
}
