#[derive(Debug)]
pub enum EventError {
	MissingId,
	UnsupportedCodecError(u64),
	InvalidGenesisError,
	InvalidPreviousCid(String, String),
	MissingLastLog,
}

impl std::fmt::Display for EventError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingId => write!(f, "Missing Id in event"),
			Self::UnsupportedCodecError(id) => write!(f, "Unsupported codec {}", id),
			Self::InvalidGenesisError => write!(f, "invalid genesis event"),
			Self::InvalidPreviousCid(prev, tip) => {
				write!(f, "invalid prev cid: {} != {}", prev, tip)
			}
			Self::MissingLastLog => write!(f, "missing last log"),
		}
	}
}

impl std::error::Error for EventError {}

#[derive(Debug)]
pub enum JwsError {
	NoLink,
}

impl std::fmt::Display for JwsError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoLink => write!(f, "JWS does not have a link"),
		}
	}
}

impl std::error::Error for JwsError {}

#[derive(Debug)]
pub enum SignedValueError {
	NoLink,
}

impl std::fmt::Display for SignedValueError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoLink => write!(f, "JWS does not have a link"),
		}
	}
}

impl std::error::Error for SignedValueError {}
