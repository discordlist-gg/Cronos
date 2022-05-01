use std::collections::BTreeMap;
use backend_common::tags::Flag;
use scylla::IntoTypedRows;

pub async fn refresh_bot_tags() -> anyhow::Result<()> {
    let tags = fetch_bot_tags().await?;

    backend_common::tags::set_bot_tags(tags);
    Ok(())
}

pub async fn refresh_pack_tags() -> anyhow::Result<()> {
    let tags = fetch_pack_tags().await?;

    backend_common::tags::set_pack_tags(tags);
    Ok(())
}

async fn fetch_bot_tags() -> anyhow::Result<BTreeMap<String, Flag>> {
    let items = super::connection::session()
        .query_prepared("SELECT name, category FROM bot_tags;", &[])
        .await?
        .rows
        .unwrap_or_default()
        .into_typed()
        .collect::<Result<Vec<(String, String)>, _>>()?;

    let mut map = BTreeMap::new();
    for (name, category) in items {
        map.insert(name, Flag {
            category,
        });
    }

    Ok(map)
}

async fn fetch_pack_tags() -> anyhow::Result<BTreeMap<String, Flag>> {
    let items = super::connection::session()
        .query_prepared("SELECT name FROM pack_tags;", &[])
        .await?
        .rows
        .unwrap_or_default()
        .into_typed()
        .collect::<Result<Vec<(String,)>, _>>()?;

    let mut map = BTreeMap::new();
    for (name,) in items {
        map.insert(name, Flag {
            category: "".to_string(),
        });
    }

    Ok(map)
}