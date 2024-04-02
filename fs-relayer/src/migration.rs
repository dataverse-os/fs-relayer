use std::sync::Arc;

use anyhow::Context;
use ceramic_box::StreamOperator;
use dataverse_file_types::core::{dapp_store, stream::StreamStore};

use crate::{config::Config, iroh_store, pgsql_store};

pub async fn migration(cfg: &Config, operator: Arc<dyn StreamOperator>) -> anyhow::Result<()> {
    if let Ok(migration) = std::env::var("MIGRATION") {
        let split: Vec<_> = migration.split(',').collect();
        if split.iter().any(|&s| s == "stream_store") {
            let pgsql_store = pgsql_store(cfg, operator.clone()).await?;
            let iroh_store = iroh_store(cfg, operator.clone()).await?;
            migration_stream_store(operator, iroh_store, pgsql_store).await?;
        }
    };
    Ok(())
}

async fn migration_stream_store(
    operator: Arc<dyn StreamOperator>,
    from: Arc<dyn StreamStore>,
    to: Arc<dyn StreamStore>,
) -> anyhow::Result<()> {
    let streams = from.list_all_streams().await?;
    for mut stream in streams {
        let stream_id = stream.stream_id()?;
        let ceramic = dapp_store::get_dapp_ceramic(&stream.dapp_id).await?;
        let state = operator
            .load_stream_state(&ceramic, &stream_id, Some(stream.tip))
            .await?;
        stream.account = Some(
            state
                .controllers()
                .first()
                .context("no controllers")?
                .clone(),
        );
        stream.model = state.model()?;
        stream.content = state.content;
        to.save_stream(&stream).await?;
    }
    Ok(())
}
