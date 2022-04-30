use std::collections::HashMap;

use backend_common::types::{JsSafeBigInt, JsSafeInt, Set, Timestamp};
use poem::Result;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use poem_openapi::param::Path;
use tantivy::schema::Field;
use tantivy::Document;

use crate::models::bots::{get_bot_data, Bot};
use crate::routes::StandardResponse;
use crate::search::readers::bots::BotsSortBy;
use crate::search::readers::Order;
use crate::search::{readers, FromTantivyDoc, index_impls};

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct BotHit {
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

    /// The given Dlist flags.
    pub flags: JsSafeBigInt,

    /// The bot's given list of features.
    ///
    /// This is stored in the form of a bitflag(s).
    pub features: JsSafeBigInt,

    /// The bot's required invite permissions.
    pub permissions: JsSafeBigInt,

    /// The bot's associated tags.
    pub tags: Vec<String>,

    /// The timestamp that the bot was first created on.
    pub created_on: Timestamp,

    /// The bot's primary owner.
    pub owner_id: JsSafeBigInt,

    /// The bot's secondary/co-owners
    pub co_owner_ids: Set<JsSafeBigInt>,

    /// The amount of guilds the bot is in.
    pub guild_count: Option<JsSafeInt>,

    /// The short description of the bot.
    pub brief_description: String,
}

impl From<Bot> for BotHit {
    fn from(bot: Bot) -> Self {
        Self {
            id: bot.id,
            username: bot.username,
            avatar: bot.avatar,
            discriminator: bot.discriminator,
            prefix: bot.prefix,
            flags: bot.features,
            features: bot.features,
            permissions: bot.permissions,
            tags: bot.tags.iter().map(|v| v.name.to_string()).collect(),
            created_on: bot.created_on,
            owner_id: bot.owner_id,
            co_owner_ids: bot.co_owner_ids,
            guild_count: bot.guild_count,
            brief_description: bot.brief_description,
        }
    }
}

impl FromTantivyDoc for BotHit {
    fn from_doc(id_field: Field, doc: Document) -> Option<Self> {
        let id = doc.get_first(id_field)?.as_i64()?;
        let bot = get_bot_data(id)?;

        Some(Self::from(bot))
    }
}

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct BotSearchPayload {
    /// The query to be searched.
    ///
    /// If null this will be a wild card search.
    #[oai(validator(min_length = 1, max_length = 50))]
    query: Option<String>,

    /// How many documents to return.
    ///
    /// Defaults to 20 results.
    #[oai(validator(minimum(value = "1"), maximum(value = "50")))]
    limit: Option<usize>,

    /// How many documents to skip first.
    #[oai(validator(maximum(value = "40000")), default)]
    offset: usize,

    /// A set of filter rules.
    #[oai(default)]
    filter: BotFilter,

    /// How to sort results.
    #[oai(default)]
    sort_by: BotsSortBy,

    /// Order results Asc or Desc.
    #[oai(default)]
    order: Order,
}

#[derive(Default, Debug, Object)]
pub struct BotFilter {
    /// A specific category to filter out results.
    #[oai(validator(min_length = 2, max_length = 32))]
    category: Option<String>,

    /// A set of tags to filter results by.
    #[oai(validator(max_items = 10, unique_items), default)]
    tags: Vec<String>,
}

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct BotSearchResult {
    /// The search results themselves.
    hits: Vec<BotHit>,

    /// The maximum amount of docs that could get returned.
    limit: usize,

    /// The number of skipped documents.
    offset: usize,

    /// The original query used to search results.
    query: String,

    /// The total number of documents that matched the query.
    ///
    /// This is a best-guess estimate.
    nb_hits: usize,

    /// The distribution of tags/categories across the results.
    tag_distribution: HashMap<String, usize>,
}


pub struct BotApi;

#[OpenApi]
impl BotApi {
    /// Update Bot Data
    ///
    /// This internally pulls data from the database.
    #[oai(path = "/bots/:id", method = "post", tag = "crate::ApiTags::Bots")]
    pub async fn update_bot(&self, id: Path<u64>) -> Result<StandardResponse> {
        index_impls::bots::writer()
            .upsert_bot(*id as i64)
            .await?;

        Ok(StandardResponse::Ok)
    }

    /// Remove Bot Data
    #[oai(path = "/bots/:id", method = "delete", tag = "crate::ApiTags::Bots")]
    pub async fn remove_bot(&self, id: Path<u64>) -> Result<StandardResponse> {
        index_impls::bots::writer()
            .remove_bot(*id as i64)
            .await?;

        Ok(StandardResponse::Ok)
    }

    /// Refresh Bot Data
    #[oai(path = "/bots/refresh", method = "post", tag = "crate::ApiTags::Bots")]
    pub async fn refresh_bots(&self) -> Result<StandardResponse> {
        index_impls::bots::writer()
            .full_refresh()
            .await?;

        Ok(StandardResponse::Ok)
    }

    /// Search Bots
    #[oai(path = "/bots/search", method = "post", tag = "crate::ApiTags::Bots")]
    pub async fn search(
        &self,
        payload: Json<BotSearchPayload>,
    ) -> Result<Json<BotSearchResult>> {
        let limit = payload.0.limit.unwrap_or(20);
        let offset = payload.0.offset;
        let query = payload.0.query.clone();

        let (num_hits, dist, hits) = readers::bots::reader()
            .search::<BotHit>(
                payload.0.query,
                limit,
                offset,
                payload.0.sort_by,
                payload.0.order,
            )
            .await?;

        let result = BotSearchResult {
            hits,
            limit,
            offset,
            query: query.unwrap_or_else(|| "*".to_string()),
            nb_hits: num_hits,
            tag_distribution: dist,
        };

        Ok(Json(result))
    }
}
