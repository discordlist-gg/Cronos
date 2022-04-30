use std::collections::HashMap;

use futures::StreamExt;
use scylla::frame::value::Counter;
use scylla::transport::iterator::RowIterator;

#[macro_export]
macro_rules! derive_fetch_by_id {
    ($slf:ident, table = $tbl:expr) => {
        impl $slf {
            pub async fn fetch(id: i64) -> anyhow::Result<Option<$slf>> {
                use scylla::IntoTypedRows;

                use super::connection::session;

                let qry = format!(
                    "SELECT {} FROM {} WHERE id = ?",
                    $slf::FIELD_NAMES_AS_ARRAY.join(", "),
                    $tbl,
                );

                session()
                    .query_prepared(&qry, (id,))
                    .await?
                    .rows
                    .unwrap_or_default()
                    .into_typed::<$slf>()
                    .next()
                    .transpose()
                    .map_err(anyhow::Error::from)
            }
        }
    };
}

#[macro_export]
macro_rules! derive_fetch_iter {
    ($slf:ident, table = $tbl:expr) => {
        impl $slf {
            pub async fn iter_rows() -> Result<scylla::transport::iterator::RowIterator> {
                use super::connection::session;

                let qry = format!(
                    "SELECT {} FROM {};",
                    $slf::FIELD_NAMES_AS_ARRAY.join(", "),
                    $tbl,
                );

                session()
                    .query_iter(&qry, &[])
                    .await
                    .map_err(anyhow::Error::from)
            }
        }
    };
}

#[derive(Debug, Default, Copy, Clone)]
pub struct VoteStats {
    votes: u64,
    all_time_votes: u64,
}

impl VoteStats {
    pub fn new(votes: i64, all_time_votes: i64) -> Self {
        Self {
            votes: votes as u64,
            all_time_votes: all_time_votes as u64,
        }
    }

    #[inline]
    pub fn votes(&self) -> u64 {
        self.votes
    }

    #[inline]
    pub fn all_time_votes(&self) -> u64 {
        self.all_time_votes
    }
}

pub async fn process_rows(iter: RowIterator) -> HashMap<i64, VoteStats> {
    let mut iter = iter.into_typed::<(i64, Counter, Counter)>();

    let mut processed_changes = HashMap::new();
    while let Some(Ok((id, Counter(votes), Counter(all_time_votes)))) = iter.next().await
    {
        processed_changes.insert(id, VoteStats::new(votes, all_time_votes));
    }

    processed_changes
}
