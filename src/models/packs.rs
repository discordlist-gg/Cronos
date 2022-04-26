use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use backend_common::tags::PackTags;
use backend_common::types::{JsSafeBigInt, Set, Timestamp};
use backend_common::FieldNamesAsArray;
use once_cell::sync::Lazy;
use poem_openapi::Object;
use scylla::FromRow;

use crate::derive_fetch_by_id;
use crate::models::connection::session;
use crate::models::utils::{process_rows, VoteStats};

#[derive(Object, FromRow, FieldNamesAsArray, Debug, Clone)]
#[oai(rename_all = "camelCase")]
pub struct Pack {
    /// The ID of the pack.
    pub id: JsSafeBigInt,

    /// The name of the pack.
    pub name: String,

    /// The description of the pack.
    pub description: String,

    /// The timestamp of when the pack was created.
    pub created_on: Timestamp,

    /// The tag associated with this pack.
    pub tag: PackTags,

    /// The bots that this pack contains. (In the form of IDs)
    pub bots: Vec<JsSafeBigInt>,

    /// If true the pack is removed from any public viewing.
    ///
    /// The owner is free to enable/disable this however they please.
    pub is_hidden: bool,

    /// If true the pack is removed from any public viewing.
    ///
    /// Only Dlist admins are able to enable/disable this.
    pub is_forced_into_hiding: bool,

    /// The primary owner of this pack.
    pub owner_id: JsSafeBigInt,

    /// The set of co-owners of this pack.
    pub co_owner_ids: Set<JsSafeBigInt>,
}
derive_fetch_by_id!(Pack, table = "packs");

static VOTE_INFO: Lazy<ArcSwap<HashMap<i64, VoteStats>>> =
    Lazy::new(|| ArcSwap::from_pointee(HashMap::new()));

#[inline]
pub fn vote_stats(id: i64) -> VoteStats {
    VOTE_INFO.load().get(&id).copied().unwrap_or_default()
}

pub async fn refresh_latest_votes() -> Result<()> {
    let iter = session()
        .query_iter("SELECT id, votes, all_time_votes FROM pack_votes;", &[])
        .await?;

    VOTE_INFO.store(Arc::new(process_rows(iter).await));

    Ok(())
}
