use std::fmt::Debug;

use anyhow::{Error, Result};
use once_cell::sync::OnceCell;
use scylla::frame::value::ValueList;
use scylla::query::Query;
use scylla::transport::iterator::RowIterator;
use scylla::{QueryResult, SessionConfig};

static CONN: OnceCell<Session> = OnceCell::new();

#[inline]
/// Get the currently active session.
pub(crate) fn session() -> &'static Session {
    CONN.get().unwrap()
}

/// Establishes a connection with the Scylla cluster.
pub async fn connect(nodes: &[impl AsRef<str>], init_tables: bool) -> Result<()> {
    let mut cfg = SessionConfig::new();
    cfg.add_known_nodes(nodes);

    let session = scylla::Session::connect(cfg).await?;
    let _ = session.query("CREATE KEYSPACE discordlist WITH replication = {'class': 'SimpleStrategy', 'replication_factor' : 1};", &[]).await;
    session.use_keyspace("discordlist", false).await?;

    let session = Session::from(session);

    let _ = CONN.set(session);

    if init_tables {
        create_tables().await?;
    }

    Ok(())
}

async fn create_tables() -> Result<()> {
    for table in include_str!("./scripts/test-tables.cql").split(';') {
        let table = table.trim();
        if table.is_empty() {
            continue;
        }

        session().query(table, &[]).await?;
    }

    Ok(())
}

pub struct Session(scylla::CachingSession);

impl From<scylla::Session> for Session {
    fn from(s: scylla::Session) -> Self {
        Self(scylla::CachingSession::from(s, 100))
    }
}

impl Session {
    #[instrument(skip(self, query), level = "debug")]
    pub async fn query(
        &self,
        query: &str,
        values: impl ValueList + Debug,
    ) -> Result<QueryResult> {
        debug!("executing query {}", query);
        self.0.execute(query, &values).await.map_err(|e| {
            error!("Failed to execute query {} with error {:?}", query, e);
            Error::from(e)
        })
    }

    #[instrument(skip(self, query), level = "debug")]
    pub async fn query_iter(
        &self,
        query: &str,
        values: impl ValueList + Debug,
    ) -> Result<RowIterator> {
        debug!("preparing and paging new statement: {}", query);
        self.0
            .execute_iter(Query::from(query), &values)
            .await
            .map_err(|e| {
                error!("Failed to execute query {} with error {:?}", query, e);
                Error::from(e)
            })
    }

    #[instrument(skip(self, query), level = "debug")]
    pub async fn query_prepared(
        &self,
        query: &str,
        values: impl ValueList + Debug,
    ) -> Result<QueryResult> {
        debug!("preparing and executing statement: {}", query);
        self.0
            .execute(Query::from(query), &values)
            .await
            .map_err(|e| {
                error!("Failed to execute query {} with error {:?}", query, e);
                Error::from(e)
            })
    }
}
