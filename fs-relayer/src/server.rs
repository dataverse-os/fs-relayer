use crate::error::AppError;

use actix_web::error::{JsonPayloadError, QueryPayloadError};
use actix_web::web::{JsonConfig, QueryConfig};
use actix_web::{get, post, put, HttpRequest};
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use ceramic_box::{commit, StreamId};
use dataverse_file_types::core::client::LoadFilesOption;
use dataverse_file_types::core::dapp_store::get_model_by_name;
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddrV4;
use std::str::FromStr;

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
) -> Result<impl Responder, AppError> {
    if let Some(format) = &query.format {
        if format == "ceramic" {
            let file = state.load_stream(&query.dapp_id, &query.stream_id).await?;
            return Ok(HttpResponse::Ok().json(file));
        }
    }
    let file = state.load_file(&query.dapp_id, &query.stream_id).await?;
    return Ok(HttpResponse::Ok().json(file));
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
) -> Result<impl Responder, AppError> {
    // if signals is not empty and dapp_id is not empty, then override model_id
    if !payload.signals.is_empty() {
        if let Some(dapp_id) = &query.dapp_id {
            query.model_id = Some(get_model_by_name(dapp_id, "indexFolder").await?.id);
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
        for stream_id_str in stream_ids_str.split(',') {
            stream_ids.push(StreamId::from_str(stream_id_str).context("invalid stream id")?);
        }
        return load_streams_by_stream_ids(state, stream_ids.clone(), *dapp_id).await;
    }

    Err(AppError::InvalidQuery)
}

#[get("/dataverse/streams")]
async fn get_load_streams(
    query: web::Query<LoadFilesQuery>,
    state: web::Data<crate::state::AppState>,
) -> Result<impl Responder, AppError> {
    if let Some(model_id) = &query.model_id {
        return load_streams_by_model_id(state, model_id.clone(), query.account.clone(), vec![])
            .await;
    }

    if let (Some(stream_ids_str), Some(dapp_id)) = (&query.stream_ids, &query.dapp_id) {
        let mut stream_ids = Vec::new();
        for stream_id_str in stream_ids_str.split(',') {
            stream_ids.push(StreamId::from_str(stream_id_str).context("invalid stream id")?);
        }
        return load_streams_by_stream_ids(state, stream_ids.clone(), *dapp_id).await;
    }

    Err(AppError::InvalidQuery)
}

async fn load_streams_by_model_id(
    state: web::Data<crate::state::AppState>,
    model_id: StreamId,
    account: Option<String>,
    signals: Vec<Value>,
) -> Result<HttpResponse, AppError> {
    let mut opts = vec![];
    for ele in signals {
        opts.push(LoadFilesOption::Signal(ele))
    }
    let files = state.load_files(account.clone(), &model_id, opts).await?;

    return Ok(HttpResponse::Ok().json(files));
}

async fn load_streams_by_stream_ids(
    state: web::Data<crate::state::AppState>,
    stream_ids: Vec<StreamId>,
    dapp_id: uuid::Uuid,
) -> Result<HttpResponse, AppError> {
    let mut files = Vec::new();

    for stream_id in stream_ids {
        files.push(state.load_file(&dapp_id, &stream_id).await?);
    }
    Ok(HttpResponse::Ok().json(files))
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
) -> Result<impl Responder, AppError> {
    let stream = state.create_stream(&query.dapp_id, payload.0).await?;
    Ok(HttpResponse::Ok().json(stream))
}

#[put("/dataverse/stream")]
async fn put_update_stream(
    query: web::Query<DappQuery>,
    payload: web::Json<commit::Data>,
    state: web::Data<crate::state::AppState>,
) -> Result<impl Responder, AppError> {
    let stream = state.update_stream(&query.dapp_id, payload.0).await?;
    return Ok(HttpResponse::Ok().json(stream));
}

pub fn web_server(
    state: crate::state::AppState,
    addr: SocketAddrV4,
) -> anyhow::Result<crate::JoinHandleWithError> {
    tracing::info!("start server on {}", addr);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(QueryConfig::default().error_handler(query_error_handler))
            .app_data(JsonConfig::default().error_handler(json_error_handler))
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
    Ok(web)
}

fn query_error_handler(err: QueryPayloadError, _req: &HttpRequest) -> actix_web::Error {
    AppError::BadQuery(err.to_string()).into()
}

fn json_error_handler(err: JsonPayloadError, _req: &HttpRequest) -> actix_web::Error {
    AppError::BadJson(err.to_string()).into()
}
