use std::cmp::Reverse;
use std::collections::HashMap;

use poem_openapi::Enum;
use tantivy::aggregation::agg_req::{
    Aggregation,
    Aggregations,
    BucketAggregation,
    BucketAggregationType,
};
use tantivy::aggregation::agg_result::{AggregationResult, BucketResult};
use tantivy::aggregation::bucket::TermsAggregation;
use tantivy::aggregation::AggregationCollector;
use tantivy::collector::{Count, FilterCollector, TopDocs};
use tantivy::fastfield::FastFieldReader;
use tantivy::query::Query;
use tantivy::schema::Field;
use tantivy::{DocAddress, DocId, Score, Searcher, SegmentReader};

use crate::search::FromTantivyDoc;

pub mod bots;
pub mod packs;

pub(crate) type SearchResult<T> = (usize, HashMap<String, usize>, Vec<T>);

#[derive(Enum, Debug, Copy, Clone)]
#[oai(rename_all = "lowercase")]
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_search<T>(
    searcher: &Searcher,
    query: Box<dyn Query>,
    results: &mut Vec<DocAddress>,
    field: Field,
    collector: TopDocs,
    cb: fn(i64) -> T,
    order: Order,
    filter: Option<(Field, fn(u64) -> bool)>,
) -> anyhow::Result<()>
where
    T: PartialOrd + Clone + Send + Sync + 'static,
{
    match order {
        Order::Desc => {
            collector_for_id_desc(searcher, query, results, field, collector, cb, filter)
        },
        Order::Asc => {
            collector_for_id_asc(searcher, query, results, field, collector, cb, filter)
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
    filter: Option<(Field, fn(u64) -> bool)>,
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

    let docs = match filter {
        None => searcher.search(&query, &collector),
        Some((field, pred)) => {
            info!("filtering???");
            let filter = FilterCollector::new(field, pred, collector);
            searcher.search(&query, &filter)
        },
    }?;

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
    filter: Option<(Field, fn(u64) -> bool)>,
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

    let docs = match filter {
        None => searcher.search(&query, &collector),
        Some((field, pred)) => {
            let filter = FilterCollector::new(field, pred, collector);
            searcher.search(&query, &filter)
        },
    }?;

    results.extend(docs.into_iter().map(|v| v.1));

    Ok(())
}

pub(crate) fn extract_search_data<T>(
    searcher: &Searcher,
    id_field: Field,
    address: impl Iterator<Item = DocAddress>,
) -> anyhow::Result<Vec<T>>
where
    T: FromTantivyDoc + Sync + Send + 'static,
{
    let mut loaded = vec![];
    for doc in address {
        let doc = searcher.doc(doc)?;
        if let Some(doc) = T::from_doc(id_field, doc) {
            loaded.push(doc);
        }
    }

    Ok(loaded)
}

fn search_aggregate(
    query: Option<&str>,
    field_name: String,
    fields: &[Field],
    searcher: &Searcher,
) -> anyhow::Result<(usize, HashMap<String, usize>)> {
    let distribution_query = crate::search::queries::distribution_query(query, fields);

    let terms = TermsAggregation {
        field: field_name,
        size: Some(1000),
        ..Default::default()
    };

    let aggs: Aggregations = vec![(
        "tags".to_string(),
        Aggregation::Bucket(BucketAggregation {
            bucket_agg: BucketAggregationType::Terms(terms),
            sub_aggregation: Aggregations::default(),
        }),
    )]
    .into_iter()
    .collect();
    let collector = AggregationCollector::from_aggs(aggs);

    let (count, terms) = searcher.search(&distribution_query, &(Count, collector))?;

    let (_, first_agg) = terms.0.into_iter().next().unwrap();
    let mut distributions = HashMap::new();
    if let AggregationResult::BucketResult(BucketResult::Terms { buckets, .. }) =
        first_agg
    {
        distributions.extend(
            buckets
                .into_iter()
                .map(|v| (v.key.to_string(), v.doc_count as usize)),
        );
    }

    Ok((count, distributions))
}
