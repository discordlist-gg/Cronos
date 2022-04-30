use std::collections::HashMap;
use tantivy::collector::{Collector, SegmentCollector};
use tantivy::fastfield::{DynamicFastFieldReader, FastFieldReader};
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, FAST, INDEXED, TEXT};
use tantivy::{doc, Index, Score, SegmentReader};
use tantivy::aggregation::metric::Stats;


struct DistributionsCollector {
    field: Field,
}

impl DistributionsCollector {
    fn with_field(field: Field) -> Self {
        Self { field }
    }
}

impl Collector for DistributionsCollector {
    // That's the type of our result.
    // Our standard deviation will be a float.
    type Fruit = HashMap<String, usize>;

    type Child = StatsSegmentCollector;

    fn for_segment(
        &self,
        _segment_local_id: u32,
        segment_reader: &SegmentReader,
    ) -> tantivy::Result<StatsSegmentCollector> {
        let fast_field_reader = segment_reader.fast_fields().u64(self.field)?;
        Ok(StatsSegmentCollector {
            fast_field_reader,
            stats: Default::default(),
        })
    }

    fn requires_scoring(&self) -> bool {
        // this collector does not care about score.
        false
    }

    fn merge_fruits(&self, segment_stats: Vec<Option<Stats>>) -> tantivy::Result<Self::Fruit> {
        let mut stats = Default::default();
        for segment_stats in segment_stats.into_iter().flatten() {
            todo!()
        }
        Ok(stats)
    }
}

struct StatsSegmentCollector {
    fast_field_reader: DynamicFastFieldReader<u64>,
    stats: HashMap<String, usize>,
}

impl SegmentCollector for StatsSegmentCollector {
    type Fruit = HashMap<String, usize>;

    fn collect(&mut self, doc: u32, _score: Score) {
        let value = self.fast_field_reader.get(doc);

        todo!()
    }

    fn harvest(self) -> <Self as SegmentCollector>::Fruit {
        self.stats
    }
}