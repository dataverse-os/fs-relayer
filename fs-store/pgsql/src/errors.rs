use std::error::Error;

use ceramic_core::{Cid, StreamId};

#[derive(Debug)]
pub enum ConnectionPoolError {
	PoolInitializationError(String),
}

impl std::fmt::Display for ConnectionPoolError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::PoolInitializationError(err) => {
				write!(f, "PoolInitializationError: {}", err)
			}
		}
	}
}

impl Error for ConnectionPoolError {}

#[derive(Debug)]
pub enum PgSqlClientError {
	MissingGenesis,
	MissingEventForStream(Cid, StreamId),
	DbExecError,
}

impl std::fmt::Display for PgSqlClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingGenesis => write!(f, "missing genesis"),
			Self::MissingEventForStream(cid, stream_id) => {
				write!(f, "missing event {} for stream {}", cid, stream_id)
			}
			Self::DbExecError => write!(f, "db exec error"),
		}
	}
}

impl Error for PgSqlClientError {}

#[derive(Debug)]
pub enum PgSqlEventError {
	UnsupportedCodecError(u64),
}

impl std::fmt::Display for PgSqlEventError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::UnsupportedCodecError(codec) => write!(f, "unsupported codec {}", codec),
		}
	}
}

impl Error for PgSqlEventError {}
