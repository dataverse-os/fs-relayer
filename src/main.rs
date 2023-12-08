mod config;
mod response;
mod state;

use crate::{config::Config, response::JsonResponse};
use state::*;

use std::{str::FromStr, sync::Arc};

use actix_web::{get, post, put};
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use dataverse_ceramic::kubo::message::MessageSubscriber;
use dataverse_ceramic::{commit, kubo, StreamId};
use futures::future::join_all;
use serde::Deserialize;
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
    model_id: StreamId,
    account: Option<String>,
}

#[get("/dataverse/streams")]
async fn load_streams(
    query: web::Query<LoadFilesQuery>,
    state: web::Data<AppState>,
) -> impl Responder {
    match state
        .load_files(query.account.clone(), &query.model_id)
        .await
    {
        Ok(file) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(file),
        Err(err) => {
            tracing::warn!(
                model_id = query.model_id.to_string(),
                account = query.account.clone(),
                "load streams error: {}",
                err
            );
            HttpResponse::BadRequest().json_error(err.to_string())
        }
    }
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
    let kubo_client = kubo::new(&cfg.kubo_path);
    let kubo_client = Arc::new(kubo_client);
    let kubo_client_clone = kubo_client.clone();

    let operator = Arc::new(kubo::Cached::new(kubo_client, cfg.cache_size)?);
    let data_path = cfg.data_path()?;
    let key = dataverse_iroh_store::SecretKey::from_str(&cfg.iroh.key)?;
    let iroh_store =
        dataverse_iroh_store::Client::new(data_path, key, cfg.iroh.into(), operator).await?;
    let iroh_store = Arc::new(iroh_store);

    let mut subscribers = Vec::new();
    for network in cfg.networks {
        let iroh_store = iroh_store.clone();
        let kubo_client_clone = kubo_client_clone.clone();
        let sub = tokio::task::spawn(async move {
            tracing::info!(?network, "subscribe to kubo topic");
            if let Err(err) = kubo_client_clone.subscribe(iroh_store, network).await {
                tracing::error!(?network, "subscribe error: {}", err);
            };
        });
        subscribers.push(sub);
    }

    let state = AppState::new(iroh_store);
    let addrs = ("0.0.0.0", 8080);

    let web = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(state.clone()))
            .service(load_stream)
            .service(load_streams)
            .service(post_create_stream)
            .service(put_update_stream)
    })
    .bind(addrs)?
    .run();
    tracing::info!("start server on {}:{}", addrs.0, addrs.1);

    if let Err(err) = web.await {
        tracing::error!("server error: {}", err);
    };

    join_all(subscribers).await;

    Ok(())
}
