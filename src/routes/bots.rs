use std::collections::HashMap;
use poem_openapi::{OpenApi, Object, ApiResponse};
use poem_openapi::payload::Json;
use poem::Result;

use crate::models::bots::Bot;

#[derive(Debug, Object)]
pub struct BotSearchPayload {
    /// The query to be searched.
    ///
    /// If null this will be a wild card search.
    #[oai(validator(min_length = 1, max_length = 50))]
    query: Option<String>,

    /// How many documents to return.
    ///
    /// Defaults to 20 results.
    #[oai(validator(minimum(value="1"), maximum(value="50")))]
    limit: Option<usize>,

    /// How many documents to skip first.
    #[oai(validator(maximum(value="40000")), default)]
    offset: usize,

    /// A set of filter rules.
    #[oai(default)]
    filter: BotFilter,
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
    hits: Vec<Bot>,

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
    tag_distribution: HashMap<String, HashMap<String, usize>>
}

pub struct BotApi;

#[OpenApi]
impl BotApi {
    #[oai(path = "/bots/search", method = "post")]
    pub async fn search(&self, payload: Json<BotSearchPayload>) -> Result<Json<BotSearchResult>> {
        todo!()
    }
}
