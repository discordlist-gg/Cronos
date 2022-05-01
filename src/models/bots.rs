use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use backend_common::tags::BotTags;
use backend_common::types::{JsSafeBigInt, JsSafeInt, Set, Timestamp};
use backend_common::FieldNamesAsArray;
use futures::StreamExt;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use scylla::FromRow;
use tantivy::schema::Schema;

use crate::models::bots::flags::PREMIUM;
use crate::models::connection::session;
use crate::models::utils::{process_rows, VoteStats};
use crate::search::index_impls::bots::{
    DESCRIPTION_FIELD,
    FEATURES_FIELD,
    ID_FIELD,
    PREMIUM_FIELD,
    TAGS_FIELD,
    USERNAME_FIELD,
};
use crate::{derive_fetch_by_id, derive_fetch_iter};

pub mod flags {
    pub const PREMIUM: i64 = 1 << 0;
}

#[derive(FromRow, FieldNamesAsArray, Debug, Clone)]
pub struct Bot {
    /// The snowflake ID of the bot.
    pub id: JsSafeBigInt,

    /// The bot's username.
    pub username: String,

    /// The bot's avatar hash if applicable.
    pub avatar: Option<String>,

    /// The bot's discriminator i.e `0001`
    pub discriminator: JsSafeInt,

    /// The bot's given prefix.
    pub prefix: Option<String>,

    /// Is the bot able to be put in a pack?
    pub is_packable: bool,

    /// The bot's custom slug which can be used to access the bot.
    pub slug: Option<String>,

    /// The given Dlist flags.
    pub flags: JsSafeBigInt,

    /// The bot's given list of features.
    ///
    /// This is stored in the form of a bitflag(s).
    pub features: JsSafeBigInt,

    /// The bot's required invite permissions.
    pub permissions: JsSafeBigInt,

    /// The bot's associated tags.
    pub tags: BotTags,

    /// The timestamp that the bot was first created on.
    pub created_on: Timestamp,

    /// Is the guild temporarily hidden from the public listing.
    pub is_hidden: bool,

    /// Is the guild forced into being hidden by a Dlist admin.
    pub is_forced_into_hiding: bool,

    /// The bot's primary owner.
    pub owner_id: JsSafeBigInt,

    /// The bot's secondary/co-owners
    pub co_owner_ids: Set<JsSafeBigInt>,

    /// The amount of guilds the bot is in.
    pub guild_count: Option<JsSafeInt>,

    /// The short description of the bot.
    pub brief_description: String,
}
derive_fetch_by_id!(Bot, table = "bots");
derive_fetch_iter!(Bot, table = "bots");

impl Bot {
    pub fn as_tantivy_doc(&self, schema: &Schema) -> tantivy::Document {
        let mut document = tantivy::Document::new();

        let id_field = schema.get_field(ID_FIELD).unwrap();
        let premium_field = schema.get_field(PREMIUM_FIELD).unwrap();
        let username_field = schema.get_field(USERNAME_FIELD).unwrap();
        let description_field = schema.get_field(DESCRIPTION_FIELD).unwrap();
        let features_field = schema.get_field(FEATURES_FIELD).unwrap();
        let tags_field = schema.get_field(TAGS_FIELD).unwrap();

        document.add_i64(id_field, *self.id);
        document.add_u64(
            premium_field,
            if (*self.flags & PREMIUM) == 0 { 0 } else { 1 },
        );
        document.add_text(username_field, &self.username);
        document.add_text(description_field, &self.brief_description);
        document.add_u64(features_field, *self.features as u64);

        for tag in self.tags.iter() {
            document.add_text(tags_field, &tag.name);
        }

        document
    }
}

static VOTE_INFO: Lazy<ArcSwap<HashMap<i64, VoteStats>>> =
    Lazy::new(|| ArcSwap::from_pointee(HashMap::new()));

#[inline]
pub fn vote_stats(id: i64) -> VoteStats {
    VOTE_INFO.load().get(&id).copied().unwrap_or_default()
}

pub async fn refresh_latest_votes() -> Result<()> {
    let iter = session()
        .query_iter("SELECT id, votes FROM bot_votes;", &[])
        .await?;

    VOTE_INFO.store(Arc::new(process_rows(iter).await));

    Ok(())
}

static LIVE_DATA: Lazy<RwLock<HashMap<i64, Bot>>> = Lazy::new(Default::default);

#[inline]
pub fn get_bot_data(id: i64) -> Option<Bot> {
    let txn = LIVE_DATA.read();
    txn.get(&id).cloned()
}

#[inline]
pub fn remove_bot_from_live(bot_id: i64) {
    let mut txn = LIVE_DATA.write();
    txn.remove(&bot_id);
}

#[inline]
pub fn update_live_data(bot: Bot) {
    let mut txn = LIVE_DATA.write();
    txn.insert(*bot.id, bot);
}

pub fn all_bots() -> Vec<Bot> {
    let txn = LIVE_DATA.read();
    txn.iter().map(|(_, v)| v.clone()).collect()
}

pub async fn refresh_latest_data() -> Result<()> {
    let mut iter = Bot::iter_rows().await?.into_typed::<Bot>();

    let mut bots = HashMap::new();
    while let Some(Ok(row)) = iter.next().await {
        if row.is_hidden || row.is_forced_into_hiding {
            continue;
        }

        bots.insert(*row.id, row);
    }

    let mut lock = LIVE_DATA.write();
    (*lock) = bots;

    Ok(())
}

#[inline]
pub fn get_bot_votes(bot_id: i64) -> u64 {
    vote_stats(bot_id).votes()
}

#[inline]
pub fn get_bot_premium(bot_id: i64) -> bool {
    match get_bot_data(bot_id) {
        None => false,
        Some(b) => (*b.flags & flags::PREMIUM) != 0,
    }
}

#[inline]
pub fn get_bot_trending_score(_bot_id: i64) -> f64 {
    0.0
}

#[inline]
pub fn get_bot_guild_count(_bot_id: i64) -> u64 {
    0
}
