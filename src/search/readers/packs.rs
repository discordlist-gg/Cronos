use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::OnceCell;
use poem_openapi::{Enum, Object};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{DocAddress, IndexReader, Searcher, Term};
use tokio::sync::{oneshot, Semaphore};

use crate::models::packs;
use crate::search::index_impls::packs::TAG_AGG_FIELD;
use crate::search::readers::{extract_search_data, Order, SearchResult};
use crate::search::FromTantivyDoc;

static PACK_READER: OnceCell<InnerReader> = OnceCell::new();

pub fn reader() -> &'static InnerReader {
    PACK_READER.get().unwrap()
}

pub fn init(
    ctx: FieldContext,
    search_fields: Vec<Field>,
    reader: IndexReader,
    concurrency_limiter: Arc<Semaphore>,
) {
    PACK_READER.get_or_init(|| {
        InnerReader::new(ctx, search_fields, reader, concurrency_limiter)
    });
}

#[derive(Enum, Debug, Copy, Clone)]
#[oai(rename_all = "lowercase")]
pub enum PacksSortBy {
    /// Sort by relevance.
    Relevance,

    /// Sort by votes.
    Likes,

    /// Sort by the trending score.
    Trending,

    /// How many bots the pack is in.
    NumBots,
}

impl Default for PacksSortBy {
    fn default() -> Self {
        Self::Relevance
    }
}

#[derive(Default, Debug, Object)]
pub struct PackFilter {
    /// A specific category to filter out results.
    #[oai(validator(min_length = 2, max_length = 32))]
    category: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub struct FieldContext {
    pub id_field: Field,
    pub tag_agg_field: Field,
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
        filter: PackFilter,
        limit: usize,
        offset: usize,
        sort_by: PacksSortBy,
        order: Order,
    ) -> Result<SearchResult<T>>
    where
        T: FromTantivyDoc + Sync + Send + 'static,
    {
        let _permit = self.concurrency_limiter.acquire().await?;
        let (waker, rx) = oneshot::channel();

        let searcher = self.reader.searcher();
        let fields = self.search_fields.clone();
        let ctx = self.ctx;

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
    filter: PackFilter,
    search_fields: &[Field],
    searcher: &Searcher,
    query: Option<String>,
    limit: usize,
    offset: usize,
    sort_by: PacksSortBy,
    order: Order,
) -> Result<SearchResult<T>>
where
    T: FromTantivyDoc + Sync + Send + 'static,
{
    let query_stages =
        crate::search::queries::parse_query(query.as_deref(), search_fields);
    let mut result_addresses = vec![];

    for stage in query_stages {
        let stage = apply_filter(ctx.tag_agg_field, &filter, stage);

        search_docs(
            ctx,
            &mut result_addresses,
            searcher,
            stage,
            limit + offset,
            sort_by,
            order,
        )?;

        if result_addresses.len() == (limit + offset) {
            break;
        }
    }

    let query =
        crate::search::queries::distribution_query(query.as_deref(), search_fields);
    let query = apply_filter(ctx.tag_agg_field, &filter, query);
    let (count, dist) = super::search_aggregate::<fn(u64) -> bool>(
        query,
        TAG_AGG_FIELD.to_string(),
        searcher,
        None,
    )?;

    let docs = result_addresses.into_iter().skip(offset);
    let loaded = extract_search_data(searcher, ctx.id_field, docs)?;

    Ok((count, dist, loaded))
}

fn search_docs(
    ctx: FieldContext,
    results: &mut Vec<DocAddress>,
    searcher: &Searcher,
    query: Box<dyn Query>,
    limit: usize,
    sort_by: PacksSortBy,
    order: Order,
) -> Result<()> {
    let collector = TopDocs::with_limit(limit);

    match sort_by {
        PacksSortBy::Relevance => super::execute_basic_search::<fn(u64) -> bool>(
            searcher, query, results, collector, order, None,
        ),
        PacksSortBy::NumBots => super::execute_search::<_, fn(u64) -> bool>(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            packs::get_pack_bot_count,
            order,
            None,
        ),
        PacksSortBy::Trending => super::execute_search::<_, fn(u64) -> bool>(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            packs::get_pack_trending_score,
            order,
            None,
        ),
        PacksSortBy::Likes => super::execute_search::<_, fn(u64) -> bool>(
            searcher,
            query,
            results,
            ctx.id_field,
            collector,
            packs::get_pack_likes,
            order,
            None,
        ),
    }?;

    Ok(())
}

fn apply_filter(
    tag_field: Field,
    filter: &PackFilter,
    existing_query: Box<dyn Query>,
) -> Box<dyn Query> {
    let parts = filter.category.as_ref().map(|v| {
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(tag_field, v),
                IndexRecordOption::Basic,
            )) as Box<dyn Query>,
        )
    });

    match parts {
        None => existing_query,
        Some(parts) => Box::new(BooleanQuery::new(vec![
            parts,
            (Occur::Must, existing_query),
        ])),
    }
}
