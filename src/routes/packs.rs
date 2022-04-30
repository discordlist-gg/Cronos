use std::collections::HashMap;
use backend_common::tags::{BotTags, PackTags};
use backend_common::types::{JsSafeBigInt, JsSafeInt, Set, Timestamp};
use poem_openapi::{OpenApi, Object};
use poem_openapi::payload::Json;
use tantivy::Document;
use tantivy::schema::Field;
use poem::Result;

use crate::models::bots::get_bot_data;
use crate::models::packs::get_pack_data;
use crate::routes::bots::BotHit;
use crate::search::{FromTantivyDoc, readers};
use crate::search::readers::Order;
use crate::search::readers::packs::PacksSortBy;

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct PackHit {
    /// The ID of the pack.
    pub id: JsSafeBigInt,

    /// The name of the pack.
    pub name: String,

    /// The description of the pack.
    pub description: String,

    /// The timestamp of when the pack was created.
    pub created_on: Timestamp,

    /// The tag associated with this pack.
    pub tag: String,

    /// The bots that this pack contains.
    pub bots: Vec<BotHit>,

    /// The primary owner of this pack.
    pub owner_id: JsSafeBigInt,

    /// The set of co-owners of this pack.
    pub co_owner_ids: Set<JsSafeBigInt>,
}

impl FromTantivyDoc for PackHit {
    fn from_doc(id_field: Field, doc: Document) -> Option<Self> {
        let id = doc.get_first(id_field)?.as_i64()?;
        let pack = get_pack_data(id)?;
        let bots = pack.bots
            .iter()
            .filter_map(|v| get_bot_data(v.0))
            .map(|v| BotHit::from(v))
            .collect();

        Some(Self {
            id: pack.id,
            name: pack.name,
            created_on: pack.created_on,
            owner_id: pack.owner_id,
            co_owner_ids: pack.co_owner_ids,
            description: pack.description,
            bots,
            tag: pack.tag
                .first()
                .map(|v| v.name.to_string())
                .unwrap_or_default()
        })
    }
}

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct PackSearchPayload {
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
    filter: PackFilter,

    /// How to sort results.
    #[oai(default)]
    sort_by: PacksSortBy,

    /// Order results Asc or Desc.
    #[oai(default)]
    order: Order,
}

#[derive(Default, Debug, Object)]
pub struct PackFilter {
    /// A specific category to filter out results.
    #[oai(validator(min_length = 2, max_length = 32))]
    category: Option<String>,
}

#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct PackSearchResult {
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

pub struct PackApi;

#[OpenApi]
impl PackApi {
    #[oai(path = "/packs/search", method = "post")]
    pub async fn search(
        &self,
        payload: Json<PackSearchPayload>,
    ) -> Result<Json<PackSearchResult>> {
        let limit = payload.0.limit.unwrap_or(20);
        let offset = payload.0.offset;
        let query = payload.0.query.clone();

        let (num_hits, dist, hits) = readers::packs::reader()
            .search::<BotHit>(
                payload.0.query,
                limit,
                offset,
                payload.0.sort_by,
                payload.0.order,
            )
            .await?;

        let result = PackSearchResult {
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
