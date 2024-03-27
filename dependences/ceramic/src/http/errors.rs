#[derive(Debug)]
pub enum HttpError {
	InvalidLogType,
	StreamLoadError,
	CeramicNotInNetworkError,
	NullSignerSignError,
}

impl std::fmt::Display for HttpError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			HttpError::InvalidLogType => write!(f, "invalid log type"),
			HttpError::CeramicNotInNetworkError => write!(f, "ceramic not in networks"),
			HttpError::StreamLoadError => write!(f, "Failed to load stream"),
			HttpError::NullSignerSignError => write!(f, "NullSigner cannot sign"),
		}
	}
}

impl std::error::Error for HttpError {}
