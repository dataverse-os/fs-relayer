use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use ceramic_box::StreamId;
use uuid::Uuid;

#[derive(Debug)]
pub enum AppError {
    StreamWithModelNotInDapp(StreamId, StreamId, Uuid),
    AnchorCommitUnsupported,
    NoPrevCommitFound,
    CommitStreamIdNotFoundOnStore(StreamId),
    InvalidQuery,
    BadQuery(String),
    BadJson(String),

    Any(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::AnchorCommitUnsupported => write!(f, "anchor commit not supported"),
            AppError::NoPrevCommitFound => write!(f, "donot have previous commit"),
            AppError::CommitStreamIdNotFoundOnStore(stream_id) => write!(
                f,
                "publishing commit with stream_id {} not found in store",
                stream_id
            ),
            AppError::StreamWithModelNotInDapp(stream_id, model_id, dapp_id) => write!(
                f,
                "stream_id {} with model_id {} not belong to dapp {}",
                stream_id, model_id, dapp_id
            ),
            AppError::InvalidQuery => write!(f, "invalid query"),
            AppError::BadQuery(msg) => write!(f, "bad query: {}", msg),
            AppError::BadJson(msg) => write!(f, "bad json: {}", msg),

            AppError::Any(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

#[derive(Serialize)]
struct AppErrorResponse {
    pub error: String,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::AnchorCommitUnsupported => StatusCode::BAD_REQUEST,
            AppError::NoPrevCommitFound => StatusCode::BAD_REQUEST,
            AppError::CommitStreamIdNotFoundOnStore(_) => StatusCode::BAD_REQUEST,
            AppError::StreamWithModelNotInDapp(_, _, _) => StatusCode::BAD_REQUEST,
            AppError::InvalidQuery => StatusCode::BAD_REQUEST,
            AppError::BadQuery(_) => StatusCode::BAD_REQUEST,
            AppError::BadJson(_) => StatusCode::BAD_REQUEST,

            AppError::Any(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        actix_web::HttpResponse::build(self.status_code())
            .json(AppErrorResponse {
                error: self.to_string(),
            })
            .into()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> AppError {
        AppError::Any(error.to_string())
    }
}
