use std::cmp::Reverse;

use poem_openapi::Enum;
use tantivy::collector::TopDocs;
use tantivy::fastfield::FastFieldReader;
use tantivy::query::Query;
use tantivy::schema::Field;
use tantivy::{DocAddress, DocId, Score, Searcher, SegmentReader};

pub mod bots;
pub mod packs;

#[derive(Enum, Debug, Copy, Clone)]
pub enum Order {
    /// Order in descending order.
    Desc,

    /// Order in ascending order.
    Asc,
}

impl Default for Order {
    fn default() -> Self {
        Self::Desc
    }
}

pub(crate) fn execute_search<T>(
    searcher: &Searcher,
    query: Box<dyn Query>,
    results: &mut Vec<DocAddress>,
    field: Field,
    collector: TopDocs,
    cb: fn(i64) -> T,
    order: Order,
) -> anyhow::Result<()>
where
    T: PartialOrd + Clone + Send + Sync + 'static,
{
    match order {
        Order::Desc => {
            collector_for_id_desc(searcher, query, results, field, collector, cb)
        },
        Order::Asc => {
            collector_for_id_asc(searcher, query, results, field, collector, cb)
        },
    }
}

pub(crate) fn collector_for_id_desc<T>(
    searcher: &Searcher,
    query: Box<dyn Query>,
    results: &mut Vec<DocAddress>,
    field: Field,
    collector: TopDocs,
    cb: fn(i64) -> T,
) -> anyhow::Result<()>
where
    T: PartialOrd + Clone + Send + Sync + 'static,
{
    let collector = collector.tweak_score(move |segment_reader: &SegmentReader| {
        let reader = segment_reader.fast_fields().i64(field).unwrap();

        // We can now define our actual scoring function
        move |doc: DocId, original_score: Score| {
            let entity_id: i64 = reader.get(doc);

            (cb(entity_id), original_score)
        }
    });

    let docs = searcher.search(&query, &collector)?;
    results.extend(docs.into_iter().map(|v| v.1));

    Ok(())
}

pub(crate) fn collector_for_id_asc<T>(
    searcher: &Searcher,
    query: Box<dyn Query>,
    results: &mut Vec<DocAddress>,
    field: Field,
    collector: TopDocs,
    cb: fn(i64) -> T,
) -> anyhow::Result<()>
where
    T: PartialOrd + Clone + Send + Sync + 'static,
{
    let collector = collector.tweak_score(move |segment_reader: &SegmentReader| {
        let reader = segment_reader.fast_fields().i64(field).unwrap();

        // We can now define our actual scoring function
        move |doc: DocId, original_score: Score| {
            let entity_id: i64 = reader.get(doc);

            (Reverse(cb(entity_id)), original_score)
        }
    });

    let docs = searcher.search(&query, &collector)?;
    results.extend(docs.into_iter().map(|v| v.1));

    Ok(())
}
