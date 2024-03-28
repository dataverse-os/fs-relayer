mod config;
mod migration;
mod response;
mod state;
mod client;
mod error;

use crate::{config::Config, response::JsonResponse};
use anyhow::Context;
use dataverse_file_types::core::dapp_store::get_model_by_name;
use migration::migration;
use serde_json::Value;

use std::net::SocketAddrV4;
use std::{str::FromStr, sync::Arc};

use actix_web::{get, post, put};
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use dataverse_ceramic::kubo::message::MessageSubscriber;
use dataverse_ceramic::network::Network;
use dataverse_ceramic::{commit, kubo, StreamId, StreamOperator};
use dataverse_file_types::core::stream::StreamStore;
use dataverse_file_types::file::StreamFileLoader;
use crate::client::LoadFilesOption;
use dataverse_file_types::task as fs_task;
use futures::future;
use serde::Deserialize;
use state::*;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Deserialize)]
struct LoadFileQuery {
    stream_id: StreamId,
    dapp_id: uuid::Uuid,
    format: Option<String>,
}

#[get("/dataverse/stream")]
async fn load_stream(
    query: web::Query<LoadFileQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    if let Some(format) = &query.format {
        if format == "ceramic" {
            return match state.load_stream(&query.dapp_id, &query.stream_id).await {
                Ok(file) => HttpResponse::Ok()
                    .insert_header(header::ContentType::json())
                    .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                    .json(file),
                Err(err) => {
                    tracing::warn!(
                        format = "ceramic",
                        stream_id = query.stream_id.to_string(),
                        dapp_id = query.dapp_id.to_string(),
                        "load stream error: {}",
                        err
                    );
                    HttpResponse::BadRequest().json_error(err.to_string())
                }
            };
        }
    }
    match state.load_file(&query.dapp_id, &query.stream_id).await {
        Ok(file) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(file),
        Err(err) => {
            tracing::warn!(
                format = "dataverse",
                stream_id = query.stream_id.to_string(),
                dapp_id = query.dapp_id.to_string(),
                "load stream error: {}",
                err
            );
            HttpResponse::BadRequest().json_error(err.to_string())
        }
    }
}

#[derive(Deserialize)]
struct LoadFilesQuery {
    account: Option<String>,
    model_id: Option<StreamId>,

    stream_ids: Option<String>,
    dapp_id: Option<uuid::Uuid>,
}

#[derive(Deserialize)]
struct LoadFilesPayload {
    signals: Vec<Value>,
}

#[post("/dataverse/streams")]
async fn post_load_streams(
    mut query: web::Query<LoadFilesQuery>,
    payload: web::Json<LoadFilesPayload>,
    state: web::Data<AppState>,
) -> impl Responder {
    // if signals is not empty and dapp_id is not empty, then override model_id
    if !payload.signals.is_empty() {
        if let Some(dapp_id) = &query.dapp_id {
            match get_model_by_name(dapp_id, "indexFolder").await {
                Ok(model) => query.model_id = Some(model.id),
                Err(err) => return HttpResponse::BadRequest().json_error(err.to_string()),
            };
        }
    }

    if let Some(model_id) = &query.model_id {
        return load_streams_by_model_id(
            state,
            model_id.clone(),
            query.account.clone(),
            payload.signals.clone(),
        )
        .await;
    }

    if let (Some(stream_ids_str), Some(dapp_id)) = (&query.stream_ids, &query.dapp_id) {
        let mut stream_ids = Vec::new();
        for stream_id_str in stream_ids_str.split(",") {
            match StreamId::from_str(stream_id_str).context("invalid stream id") {
                Ok(stream_id) => stream_ids.push(stream_id),
                Err(err) => return HttpResponse::BadRequest().json_error(err.to_string()),
            };
        }
        return load_streams_by_stream_ids(state, stream_ids.clone(), dapp_id.clone()).await;
    }

    return HttpResponse::BadRequest().json_error("invalid query".to_string());
}

#[get("/dataverse/streams")]
async fn get_load_streams(
    query: web::Query<LoadFilesQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    if let Some(model_id) = &query.model_id {
        return load_streams_by_model_id(state, model_id.clone(), query.account.clone(), vec![])
            .await;
    }

    if let (Some(stream_ids_str), Some(dapp_id)) = (&query.stream_ids, &query.dapp_id) {
        let mut stream_ids = Vec::new();
        for stream_id_str in stream_ids_str.split(",") {
            match StreamId::from_str(stream_id_str).context("invalid stream id") {
                Ok(stream_id) => stream_ids.push(stream_id),
                Err(err) => return HttpResponse::BadRequest().json_error(err.to_string()),
            };
        }
        return load_streams_by_stream_ids(state, stream_ids.clone(), dapp_id.clone()).await;
    }

    return HttpResponse::BadRequest().json_error("invalid query".to_string());
}

async fn load_streams_by_model_id(
    state: web::Data<AppState>,
    model_id: StreamId,
    account: Option<String>,
    signals: Vec<Value>,
) -> HttpResponse {
    let mut opts = vec![];
    for ele in signals {
        opts.push(LoadFilesOption::Signal(ele))
    }
    match state.load_files(account.clone(), &model_id, opts).await {
        Ok(file) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(file),
        Err(err) => {
            tracing::warn!(
                model_id = model_id.to_string(),
                account = account.clone(),
                "load streams error: {}",
                err
            );
            HttpResponse::BadRequest().json_error(err.to_string())
        }
    }
}

async fn load_streams_by_stream_ids(
    state: web::Data<AppState>,
    stream_ids: Vec<StreamId>,
    dapp_id: uuid::Uuid,
) -> HttpResponse {
    let mut files = Vec::new();

    for stream_id in stream_ids {
        match state.load_file(&dapp_id, &stream_id).await {
            Ok(file) => files.push(file),
            Err(err) => {
                tracing::warn!(
                    stream_id = stream_id.to_string(),
                    dapp_id = dapp_id.to_string(),
                    "load stream error: {}",
                    err
                );
                return HttpResponse::BadRequest().json_error(err.to_string());
            }
        }
    }
    HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
        .json(files)
}

#[derive(Deserialize)]
struct DappQuery {
    dapp_id: uuid::Uuid,
}

#[post("/dataverse/stream")]
async fn post_create_stream(
    query: web::Query<DappQuery>,
    payload: web::Json<commit::Genesis>,
    state: web::Data<AppState>,
) -> impl Responder {
    match state.create_stream(&query.dapp_id, payload.0).await {
        Ok(stream) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(stream),
        Err(err) => {
            tracing::warn!(
                dapp_id = query.dapp_id.to_string(),
                "create stream error: {}",
                err
            );
            HttpResponse::BadRequest().json_error(err.to_string())
        }
    }
}

#[put("/dataverse/stream")]
async fn put_update_stream(
    query: web::Query<DappQuery>,
    payload: web::Json<commit::Data>,
    state: web::Data<AppState>,
) -> impl Responder {
    match state.update_stream(&query.dapp_id, payload.0).await {
        Ok(stream) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(stream),
        Err(err) => {
            tracing::warn!(
                dapp_id = query.dapp_id.to_string(),
                "update stream error: {}",
                err
            );
            HttpResponse::BadRequest().json_error(err.to_string())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cfg = Config::load()?;
    let mut futures: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();

    // setup task queue
    let queue = task_queue(&cfg).await?;

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

async fn task_queue(cfg: &Config) -> anyhow::Result<fs_task::Queue> {
    // init kubo client for kubo task queue
    kubo::task::init_kubo(&cfg.kubo_path);
    let queue: fs_task::Queue = fs_task::new_queue(&cfg.queue_dsn, cfg.queue_pool).await?;
    let mut pool = fs_task::build_pool(queue.clone(), cfg.queue_worker);
    tracing::info!("starting queue");
    pool.start().await;
    return Ok(queue);
}

async fn init_store(
    cfg: &Config,
    operator: Arc<dyn StreamOperator>,
) -> anyhow::Result<(
    Arc<dyn kubo::Store>,
    Arc<dyn StreamFileLoader>,
    Arc<dyn StreamStore>,
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
    let data_path = cfg.data_path()?;
    let key = dataverse_iroh_store::SecretKey::from_str(&cfg.iroh.key)?;
    let key_set = cfg.iroh.clone().into();
    let iroh_store = dataverse_iroh_store::Client::new(data_path, key, key_set, operator).await?;
    Ok(Arc::new(iroh_store))
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

fn web_server(state: AppState, addr: SocketAddrV4) -> anyhow::Result<JoinHandleWithError> {
    tracing::info!("start server on {}", addr);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(state.clone()))
            .service(load_stream)
            .service(get_load_streams)
            .service(post_load_streams)
            .service(post_create_stream)
            .service(put_update_stream)
    })
    .bind(addr)?
    .run();

    let web = tokio::spawn(async { server.await.context("server error") });
    return Ok(web);
}

type JoinHandleWithError = JoinHandle<anyhow::Result<()>>;
