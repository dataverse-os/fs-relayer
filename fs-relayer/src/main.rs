#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate serde;

mod client;
mod config;
mod error;
mod migration;
mod response;
mod server;
mod state;
mod task;

use crate::config::Config;
use anyhow::Context;

use migration::migration;

use std::{str::FromStr, sync::Arc};

use crate::server::web_server;
use crate::task as fs_task;

use ceramic_box::kubo::message::MessageSubscriber;
use ceramic_box::network::Network;
use ceramic_box::{kubo, StreamOperator};

use dataverse_file_types::core::stream::FileSlotStore;
use dataverse_file_types::file::FileLoader;
use futures::future;

use state::*;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cfg = Config::load()?;
    let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

    // setup task queue
    let queue = fs_task::task_queue(&cfg).await?;

    // setup kubo operator
    let (kubo_client, operator) = kubo_operator(&cfg, queue).await?;

    // running migration
    migration(&cfg, operator.clone()).await?;

    // setup file system store
    let (kubo_store, operator, stream_store) = init_store(&cfg, operator).await?;
    // let iroh_store = iroh_store(&cfg, operator).await?;

    // setup network subscription
    for network in cfg.networks {
        futures.push(network_subscribe(
            network,
            kubo_store.clone(),
            kubo_client.clone(),
        ));
    }

    // setup web server
    let state = AppState::new(operator, stream_store);
    let addr = "0.0.0.0:8080";
    futures.push(web_server(state, addr.parse()?)?);
    let futures: Vec<_> = futures.into_iter().map(Box::pin).collect();

    if let (Err(err), idx, remaining) = future::select_all(futures).await {
        tracing::error!("error in {}: {}", idx, err);
        for future in remaining {
            future.abort();
        }
        anyhow::bail!("error: {}", err);
    }

    Ok(())
}

async fn init_store(
    cfg: &Config,
    operator: Arc<dyn StreamOperator>,
) -> anyhow::Result<(
    Arc<dyn kubo::Store>,
    Arc<dyn FileLoader>,
    Arc<dyn FileSlotStore>,
)> {
    match cfg.pgsql_dsn.is_some() {
        true => {
            let pgsql_store = pgsql_store(cfg, operator).await?;
            Ok((pgsql_store.clone(), pgsql_store.clone(), pgsql_store))
        }
        false => {
            let iroh_store = iroh_store(cfg, operator).await?;
            Ok((iroh_store.clone(), iroh_store.clone(), iroh_store))
        }
    }
}

async fn pgsql_store(
    cfg: &Config,
    operator: Arc<dyn StreamOperator>,
) -> anyhow::Result<Arc<dataverse_pgsql_store::Client>> {
    if let Some(dsn) = &cfg.pgsql_dsn {
        let iroh_store = dataverse_pgsql_store::Client::new(operator, dsn)?;
        return Ok(Arc::new(iroh_store));
    }
    anyhow::bail!("pgsql_dsn is not set");
}

async fn iroh_store(
    cfg: &Config,
    operator: Arc<dyn StreamOperator>,
) -> anyhow::Result<Arc<dataverse_iroh_store::Client>> {
    if let Some(iroh) = &cfg.iroh {
        let data_path = cfg.data_path()?;
        let key = dataverse_iroh_store::SecretKey::from_str(&iroh.key)?;
        let key_set = iroh.clone().into();
        let iroh_store =
            dataverse_iroh_store::Client::new(data_path, key, key_set, operator).await?;
        return Ok(Arc::new(iroh_store));
    }
    anyhow::bail!("iroh is not set");
}

async fn kubo_operator(
    cfg: &Config,
    queue: fs_task::Queue,
) -> anyhow::Result<(Arc<kubo::Client>, Arc<dyn StreamOperator>)> {
    let kubo: Arc<kubo::Client> = Arc::new(kubo::new(&cfg.kubo_path));
    let queue = Arc::new(Mutex::new(queue));

    let operator: Arc<dyn StreamOperator> =
        Arc::new(kubo::Cached::new(kubo.clone(), queue, cfg.cache_size)?);
    Ok((kubo, operator))
}

fn network_subscribe(
    network: Network,
    store: Arc<dyn kubo::Store>,
    kubo: Arc<kubo::Client>,
) -> JoinHandleWithError {
    tokio::spawn(async move {
        tracing::info!(?network, "subscribe to kubo topic");
        kubo.subscribe(store, network)
            .await
            .context("subscribe error")
    })
}

type JoinHandleWithError = JoinHandle<anyhow::Result<()>>;
