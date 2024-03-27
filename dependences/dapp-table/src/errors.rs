#[derive(Debug)]
pub enum DappLookupError {
	MissingResponseData(String),
}

impl std::fmt::Display for DappLookupError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingResponseData(dapp_id) => {
				write!(f, "Missing Response data for '{}'", dapp_id)
			}
		}
	}
}

impl std::error::Error for DappLookupError {}
