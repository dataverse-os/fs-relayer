use ceramic_core::StreamId;
use uuid::Uuid;

#[derive(Debug)]
pub enum ModelStoreError {
	DappNotFound(Uuid),
    CeramicNotInNetworks,
    ModelNotInDapp(String, Uuid),
    ModelIDNotInDapp(StreamId),
}

impl std::fmt::Display for ModelStoreError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
            Self::DappNotFound(dapp_id) => write!(f, "dapp {} not found", dapp_id),
            Self::CeramicNotInNetworks => write!(f,"ceramic not in networks"),
            Self::ModelNotInDapp(model_name, dapp_id) => write!(f, "model with name `{}` not found in dapp {}", model_name, dapp_id),
            Self::ModelIDNotInDapp(model_id) => write!(f,"model with id `{}` not found in dapp table", model_id),
        }
	}
}

impl std::error::Error for ModelStoreError {}