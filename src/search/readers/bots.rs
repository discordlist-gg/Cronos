use std::sync::Arc;

use anyhow::Result;
use backend_common::types::JsSafeBigInt;
use once_cell::sync::OnceCell;
use poem_openapi::{Enum, Object};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{DocAddress, IndexReader, Searcher, Term};
use tokio::sync::{oneshot, Semaphore};

use crate::models::bots;
use crate::search::index_impls::bots::TAGS_AGG_FIELD;
use crate::search::readers::{extract_search_data, Order, SearchResult};
use crate::search::FromTantivyDoc;

static BOT_READER: OnceCell<InnerReader> = OnceCell::new();

pub fn reader() -> &'static InnerReader {
    BOT_READER.get().unwrap()
}

pub fn init(
    ctx: FieldContext,
    search_fields: Vec<Field>,
    reader: IndexReader,
    concurrency_limiter: Arc<Semaphore>,
) {
    BOT_READER.get_or_init(|| {
        InnerReader::new(ctx, search_fields, reader, concurrency_limiter)
    });
}

#[derive(Enum, Debug, Copy, Clone)]
#[oai(rename_all = "lowercase")]
pub enum BotsSortBy {
    /// Sort by relevance.
    Relevancy,

    /// Sort by votes.
    Votes,

    /// Sort by the trending score.
    Trending,

    /// How many servers the bot is in.
    Popularity,

    /// Premium Bots.
    Premium,
}

impl Default for BotsSortBy {
    fn default() -> Self {
        Self::Relevancy
    }
}

#[derive(Default, Debug, Object)]
pub struct BotFilter {
    /// A set of tags to filter results by.
    #[oai(validator(max_items = 10, unique_items), default)]
    tags: Vec<String>,

    /// The set of features to filter by.
    features: Option<JsSafeBigInt>,

    /// If the bot should be premium or not.
    premium: Option<bool>,
}

#[derive(Debug, Copy, Clone)]
pub struct FieldContext {
    pub id_field: Field,
    pub premium_field: Field,
    pub tags_agg_field: Field,
    pub features_field: Field,
}

pub struct InnerReader {
    ctx: FieldContext,
    reader: IndexReader,
    concurrency_limiter: Arc<Semaphore>,
    search_fields: Arc<Vec<Field>>,
}

impl InnerReader {
    fn new(
        ctx: FieldContext,
        search_fields: Vec<Field>,
        reader: IndexReader,
        concurrency_limiter: Arc<Semaphore>,
    ) -> Self {
        Self {
            ctx,
            reader,
            concurrency_limiter,
            search_fields: search_fields.into(),
        }
    }

    pub async fn search<T>(
        &self,
        query: Option<String>,
        filter: BotFilter,
        limit: usize,
        offset: usize,
        sort_by: BotsSortBy,
        order: Order,
    ) -> Result<SearchResult<T>>
    where
        T: FromTantivyDoc + Sync + Send + 'static,
    {
        let _permit = self.concurrency_limiter.acquire().await?;
        let (waker, rx) = oneshot::channel();

        let searcher = self.reader.searcher();
        let ctx = self.ctx;
        let fields = self.search_fields.clone();

        rayon::spawn(move || {
            let state = execute_search(
                ctx,
                filter,
                fields.as_ref(),
                &searcher,
                query,
                limit,
                offset,
                sort_by,
                order,
            );

            let _ = waker.send(state);
        });

        rx.await?
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_search<T>(
    ctx: FieldContext,
    filter: BotFilter,
    search_fields: &[Field],
    searcher: &Searcher,
    query: Option<String>,
    limit: usize,
    offset: usize,
    sort_by: BotsSortBy,
    order: Order,
) -> Result<SearchResult<T>>
where
    T: FromTantivyDoc + Sync + Send + 'static,
{
    let query_stages =
        crate::search::queries::parse_query(query.as_deref(), search_fields);
    let mut result_addresses = vec![];
    let features_filter = filter.features.map(|v| *v as u64);
    for stage in query_stages {
        let stage = apply_filter(ctx, &filter, stage);

        search_docs(
            ctx,
            &mut result_addresses,
            searcher,
            stage,
            limit + offset,
            sort_by,
            order,
            features_filter,
        )?;

        if result_addresses.len() == (limit + offset) {
            break;
        }
    }

    let query =
        crate::search::queries::distribution_query(query.as_deref(), search_fields);
    let query = apply_filter(ctx, &filter, query);

    let filter =
        features_filter.map(|flags| (ctx.features_field, move |v| (v & flags) != 0));

    let (count, dist) =
        super::search_aggregate(query, TAGS_AGG_FIELD.to_string(), searcher, filter)?;

    let docs = result_addresses.into_iter().skip(offset);
    let loaded = extract_search_data(searcher, ctx.id_field, docs)?;

    Ok((count, dist, loaded))
}

#[allow(clippy::too_many_arguments)]
fn search_docs(
    ctx: FieldContext,
    results: &mut Vec<DocAddress>,
    searcher: &Searcher,
    query: Box<dyn Query>,
    limit: usize,
    sort_by: BotsSortBy,
    order: Order,
    features_filter: Option<u64>,
) -> Result<()> {
    let collector = TopDocs::with_limit(limit);
    let filter =
        features_filter.map(|flags| (ctx.features_field, move |v| (v & flags) != 0));
    match sort_by {
        BotsSortBy::Relevancy => super::execute_basic_search(
            searcher, query, results, collector, order, filter,
        ),
        BotsSortBy::Popularity => super::execute_search(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            bots::get_bot_guild_count,
            order,
            filter,
        ),
        BotsSortBy::Premium => super::execute_search(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            bots::get_bot_premium,
            order,
            filter,
        ),
        BotsSortBy::Trending => super::execute_search(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            bots::get_bot_trending_score,
            order,
            filter,
        ),
        BotsSortBy::Votes => super::execute_search(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            bots::get_bot_votes,
            order,
            filter,
        ),
    }?;

    Ok(())
}

fn apply_filter(
    ctx: FieldContext,
    filter: &BotFilter,
    existing_query: Box<dyn Query>,
) -> Box<dyn Query> {
    if filter.tags.is_empty() {
        return existing_query;
    }

    let mut parts = filter
        .tags
        .iter()
        .map(|v| {
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(ctx.tags_agg_field, v),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            )
        })
        .collect::<Vec<(Occur, Box<dyn Query>)>>();

    if let Some(premium) = filter.premium {
        parts.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_u64(ctx.premium_field, premium as u64),
                IndexRecordOption::Basic,
            )),
        ));
    }

    Box::new(BooleanQuery::new(vec![
        (Occur::Must, existing_query),
        (Occur::Must, Box::new(BooleanQuery::new(parts))),
    ]))
}
