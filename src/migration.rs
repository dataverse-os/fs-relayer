use std::sync::Arc;

use dataverse_ceramic::StreamOperator;
use dataverse_core::stream::StreamStore;

use crate::{config::Config, iroh_store, pgsql_store};

pub async fn migration(cfg: &Config, operator: Arc<dyn StreamOperator>) -> anyhow::Result<()> {
    let migration = std::env::var("MIGRATION")?;
    let split: Vec<_> = migration.split(",").collect();
    if split.iter().any(|&s| s == "stream_store") {
        let pgsql_store = pgsql_store(cfg, operator.clone()).await?;
        let iroh_store = iroh_store(cfg, operator).await?;
        migration_stream_store(iroh_store, pgsql_store).await?;
    }
    Ok(())
}

async fn migration_stream_store(
    from: Arc<dataverse_iroh_store::Client>,
    to: Arc<dataverse_pgsql_store::Client>,
) -> anyhow::Result<()> {
    let streams = from.list_all_streams().await?;
    for stream in streams {
        to.save_stream(&stream).await?;
    }
    Ok(())
}
