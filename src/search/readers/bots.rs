use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::OnceCell;
use poem_openapi::Enum;
use tantivy::collector::TopDocs;
use tantivy::query::Query;
use tantivy::schema::Field;
use tantivy::{DocAddress, IndexReader, Searcher};
use tokio::sync::{oneshot, Semaphore};

use crate::models::bots;
use crate::search::readers::Order;
use crate::search::FromTantivyDoc;

static BOT_READER: OnceCell<InnerReader> = OnceCell::new();

pub fn reader() -> &'static InnerReader {
    BOT_READER.get().unwrap()
}

pub fn init(
    id_field: Field,
    search_fields: Vec<Field>,
    reader: IndexReader,
    concurrency_limiter: Arc<Semaphore>,
) {
    BOT_READER.get_or_init(|| InnerReader::new(id_field, search_fields, reader, concurrency_limiter));
}

#[derive(Enum, Debug, Copy, Clone)]
pub enum BotsSortBy {
    /// Sort by relevance.
    Relevance,

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
        Self::Relevance
    }
}

pub struct InnerReader {
    id_field: Field,
    reader: IndexReader,
    concurrency_limiter: Arc<Semaphore>,
    search_fields: Arc<Vec<Field>>,
}

impl InnerReader {
    fn new(
        id_field: Field,
        search_fields: Vec<Field>,
        reader: IndexReader,
        concurrency_limiter: Arc<Semaphore>,
    ) -> Self {
        Self {
            id_field,
            reader,
            concurrency_limiter,
            search_fields: search_fields.into(),
        }
    }

    pub async fn search<T>(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        sort_by: BotsSortBy,
        order: Order,
    ) -> Result<Vec<T>>
    where
        T: FromTantivyDoc + Sync + Send + 'static,
    {
        let _permit = self.concurrency_limiter.acquire().await?;
        let (waker, rx) = oneshot::channel();

        let query = query.to_string();
        let searcher = self.reader.searcher();
        let id = self.id_field;
        let fields = self.search_fields.clone();

        rayon::spawn(move || {
            let state = execute_search(
                id,
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
fn execute_search<T: FromTantivyDoc>(
    id_field: Field,
    search_fields: &[Field],
    searcher: &Searcher,
    query: String,
    limit: usize,
    offset: usize,
    sort_by: BotsSortBy,
    order: Order,
) -> Result<Vec<T>> {
    let distribution_query = crate::search::queries::distribution_query(&query, search_fields);
    let query_stages = crate::search::queries::parse_query(&query, search_fields);
    let mut result_addresses = vec![];

    for stage in query_stages {
        search_docs(
            id_field,
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

    let docs = result_addresses.into_iter().skip(offset);

    let mut loaded = vec![];
    for doc in docs {
        let doc = searcher.doc(doc)?;
        loaded.push(T::from_doc(id_field, doc)?);
    }

    Ok(loaded)
}

fn search_docs(
    id_field: Field,
    results: &mut Vec<DocAddress>,
    searcher: &Searcher,
    query: Box<dyn Query>,
    limit: usize,
    sort_by: BotsSortBy,
    order: Order,
) -> Result<()> {
    let collector = TopDocs::with_limit(limit);

    match sort_by {
        BotsSortBy::Relevance => {
            let docs = searcher.search(&query, &collector)?;
            results.extend(docs.into_iter().map(|v| v.1));
            Ok(())
        },
        BotsSortBy::Popularity => super::execute_search(
            searcher,
            query,
            results,
            id_field,
            collector,
            bots::get_bot_guild_count,
            order,
        ),
        BotsSortBy::Premium => super::execute_search(
            searcher,
            query,
            results,
            id_field,
            collector,
            bots::get_bot_premium,
            order,
        ),
        BotsSortBy::Trending => super::execute_search(
            searcher,
            query,
            results,
            id_field,
            collector,
            bots::get_bot_trending_score,
            order,
        ),
        BotsSortBy::Votes => super::execute_search(
            searcher,
            query,
            results,
            id_field,
            collector,
            bots::get_bot_votes,
            order,
        ),
    }?;

    Ok(())
}
