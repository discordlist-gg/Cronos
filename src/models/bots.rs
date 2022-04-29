use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use backend_common::tags::BotTags;
use backend_common::types::{JsSafeBigInt, JsSafeInt, Set, Timestamp};
use backend_common::FieldNamesAsArray;
use once_cell::sync::Lazy;
use poem_openapi::Object;
use scylla::FromRow;

use crate::derive_fetch_by_id;
use crate::models::connection::session;
use crate::models::utils::{process_rows, VoteStats};

#[derive(Object, FromRow, FieldNamesAsArray, Debug, Clone)]
#[oai(rename_all = "camelCase")]
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

static VOTE_INFO: Lazy<ArcSwap<HashMap<i64, VoteStats>>> =
    Lazy::new(|| ArcSwap::from_pointee(HashMap::new()));

#[inline]
pub fn vote_stats(id: i64) -> VoteStats {
    VOTE_INFO.load().get(&id).copied().unwrap_or_default()
}

pub async fn refresh_latest_votes() -> Result<()> {
    let iter = session()
        .query_iter("SELECT id, votes, all_time_votes FROM bot_votes;", &[])
        .await?;

    VOTE_INFO.store(Arc::new(process_rows(iter).await));

    Ok(())
}

#[inline]
pub fn get_bot_votes(bot_id: i64) -> u64 {
    let guard = VOTE_INFO.load();
    guard.get(&bot_id).map(|v| v.votes()).unwrap_or_default()
}

#[inline]
pub fn get_bot_premium(_bot_id: i64) -> bool {
    false
}

#[inline]
pub fn get_bot_trending_score(_bot_id: i64) -> f64 {
    0.0
}

#[inline]
pub fn get_bot_guild_count(_bot_id: i64) -> u64 {
    0
}
