use crate::migration::migration;
use crate::{config::Config, response::JsonResponse};
use anyhow::Context;
use dataverse_file_types::core::dapp_store::get_model_by_name;
use serde_json::Value;

use std::net::SocketAddrV4;
use std::{str::FromStr, sync::Arc};

use crate::state::*;
use crate::task as fs_task;
use actix_web::{get, post, put};
use actix_web::{http::header, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use ceramic_box::kubo::message::MessageSubscriber;
use ceramic_box::network::Network;
use ceramic_box::{commit, kubo, StreamId};
use dataverse_file_types::core::client::LoadFilesOption;
use futures::future;
use serde::Deserialize;
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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
    state: web::Data<crate::state::AppState>,
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

pub fn web_server(
    state: crate::state::AppState,
    addr: SocketAddrV4,
) -> anyhow::Result<crate::JoinHandleWithError> {
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
