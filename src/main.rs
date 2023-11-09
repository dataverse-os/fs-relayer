mod config;
mod response;
mod state;

use crate::config::Config;
use state::*;

use actix_web::{
    error, get, http::header, middleware::Logger, post, put, web, App, HttpResponse, HttpServer,
    Responder,
};
use dataverse_types::ceramic::StreamId;
use env_logger::Env;
use serde::Deserialize;
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
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    if let Some(format) = &query.format {
        if format == "ceramic" {
            return match state.load_stream(&query.dapp_id, &query.stream_id).await {
                Ok(file) => HttpResponse::Ok()
                    .insert_header(header::ContentType::json())
                    .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                    .json(file),
                Err(err) => error::ErrorInternalServerError(err.to_string()).error_response(),
            };
        }
    }
    match state.load_file(&query.dapp_id, &query.stream_id).await {
        Ok(file) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(file),
        Err(err) => error::ErrorInternalServerError(err.to_string()).error_response(),
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
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    match state.load_files(&query.account, &query.model_id).await {
        Ok(file) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(file),
        Err(err) => error::ErrorInternalServerError(err.to_string()).error_response(),
    }
}

#[derive(Deserialize)]
struct DappQuery {
    dapp_id: uuid::Uuid,
}

#[post("/dataverse/stream")]
async fn post_create_stream(
    query: web::Query<DappQuery>,
    payload: web::Json<dataverse_iroh_store::commit::Genesis>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    match state.create_stream(&query.dapp_id, payload.0).await {
        Ok(stream) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(stream),
        Err(err) => HttpResponse::BadRequest().json(err.to_string()),
    }
}

#[put("/dataverse/stream")]
async fn put_update_stream(
    query: web::Query<DappQuery>,
    payload: web::Json<dataverse_iroh_store::commit::Data>,
    state: web::Data<AppState<'_>>,
) -> impl Responder {
    match state.update_stream(&query.dapp_id, payload.0).await {
        Ok(stream) => HttpResponse::Ok()
            .insert_header(header::ContentType::json())
            .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
            .json(stream),
        Err(err) => HttpResponse::BadRequest().json(err.to_string()),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;

    let data_path = cfg.data_path()?;

    let key = dataverse_iroh_store::SecretKey::from_str(&cfg.iroh.key)?;
    let iroh_store = dataverse_iroh_store::Client::new(data_path, key, cfg.iroh.into()).await?;
    let status = iroh_store.iroh.node.status().await?;
    println!("iroh status: {:?}", status);

    println!("finish init database");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let state = AppState::new(iroh_store);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(state.clone()))
            .service(load_stream)
            .service(load_streams)
            .service(post_create_stream)
            .service(put_update_stream)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;
    Ok(())
}
