use std::collections::HashMap;

use anyhow::Context;
use ceramic_core::StreamId;
use dataverse_ceramic::Ceramic;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::store::errors::ModelStoreError;

#[derive(Debug, Clone)]
pub struct Model {
	pub id: StreamId,
	pub name: String,
	pub dapp_id: uuid::Uuid,
	pub encryptable: Vec<String>,
	pub version: i32,
	pub latest: bool,
}

impl Model {
	pub async fn ceramic(&self) -> anyhow::Result<Ceramic> {
		get_dapp_ceramic(&self.dapp_id).await
	}
}

static MODEL_STORE: Lazy<Mutex<ModelStore>> = Lazy::new(|| Mutex::new(ModelStore::new()));

pub struct ModelStore {
	client: dapp_table_client::Client,
	models: HashMap<String, Model>,
	ceramic: HashMap<String, Ceramic>,
	dapp_ceramic: HashMap<uuid::Uuid, String>,
}

pub async fn get_dapp_ceramic(dapp_id: &uuid::Uuid) -> anyhow::Result<Ceramic> {
	let mut store = MODEL_STORE.lock().await;
	store.get_dapp_ceramic(dapp_id, true).await
}

pub async fn get_ceramic(ceramic_str: &String) -> anyhow::Result<Ceramic> {
	MODEL_STORE.lock().await.get_ceramic(ceramic_str).await
}

pub async fn get_model_by_name(dapp_id: &uuid::Uuid, model_name: &str) -> anyhow::Result<Model> {
	let mut store = MODEL_STORE.lock().await;
	store.get_model_by_name(dapp_id, model_name, true).await
}

pub async fn get_model(model_id: &StreamId) -> anyhow::Result<Model> {
	MODEL_STORE.lock().await.get_model(model_id).await
}

pub async fn get_models(dapp_id: &uuid::Uuid, offline: bool) -> anyhow::Result<Vec<Model>> {
	MODEL_STORE.lock().await.get_models(dapp_id, offline).await
}

impl ModelStore {
	fn new() -> Self {
		let backend = std::env::var("DAPP_TABLE_BACKEND").ok();
		ModelStore {
			models: Default::default(),
			dapp_ceramic: Default::default(),
			ceramic: Default::default(),
			client: dapp_table_client::Client::new(backend),
		}
	}

	async fn get_dapp_ceramic(
		&mut self,
		dapp_id: &uuid::Uuid,
		online: bool,
	) -> anyhow::Result<Ceramic> {
		if let Some(ceramic) = self.dapp_ceramic.get(dapp_id) {
			return self.get_ceramic(&ceramic.clone()).await;
		}
		if online {
			match self.load_dapp(dapp_id).await {
				Ok((ceramic, _)) => return Ok(ceramic),
				Err(err) => log::warn!("load dapp error: {}", err),
			};
		}

		anyhow::bail!(ModelStoreError::DappNotFound(*dapp_id))
	}

	async fn get_ceramic(&mut self, ceramic_str: &String) -> anyhow::Result<Ceramic> {
		if let Some(ceramic) = self.ceramic.get(ceramic_str) {
			return Ok(ceramic.clone());
		}

		let chains = dataverse_ceramic::http::Client::chains(ceramic_str).await?;
		let ceramic = Ceramic {
			endpoint: ceramic_str.clone(),
			network: chains.first().context(ModelStoreError::CeramicNotInNetworks)?.network(),
		};
		self.ceramic.insert(ceramic_str.clone(), ceramic.clone());
		Ok(ceramic)
	}

	async fn get_models(
		&mut self,
		dapp_id: &uuid::Uuid,
		online: bool,
	) -> anyhow::Result<Vec<Model>> {
		if !online {
			let models = self
				.models.values().filter(|&x| x.dapp_id == *dapp_id).cloned()
				.collect();
			return Ok(models);
		}
		let (_, models) = self.load_dapp(dapp_id).await?;
		Ok(models)
	}

	async fn load_dapp(&mut self, dapp_id: &uuid::Uuid) -> anyhow::Result<(Ceramic, Vec<Model>)> {
		log::info!("lookup dapp with dapp_id: {}", dapp_id);
		let dapp = self
			.client
			.lookup_dapp_by_dapp_id(&dapp_id.to_string())
			.await?;
		self.dapp_ceramic
			.insert(*dapp_id, dapp.ceramic.clone());
		let ceramic = self.get_ceramic(&dapp.ceramic).await?;
		let models = self.store_dapp_models(dapp)?;
		Ok((ceramic, models))
	}

	fn store_dapp_models(
		&mut self,
		dapp: dapp_table_client::get_dapp::GetDappGetDapp,
	) -> anyhow::Result<Vec<Model>> {
		let mut result = vec![];
		for model in dapp.models {
			for (idx, ele) in model.streams.iter().enumerate() {
				let model = Model {
					id: ele.model_id.parse()?,
					dapp_id: dapp.id.parse()?,
					encryptable: ele.encryptable.clone(),
					name: model.model_name.clone(),
					version: idx as i32,
					latest: ele.latest,
				};
				self.models.insert(model.id.to_string(), model.clone());
				result.push(model)
			}
		}
		Ok(result)
	}

	async fn get_model_by_name(
		&mut self,
		dapp_id: &uuid::Uuid,
		model_name: &str,
		online: bool,
	) -> anyhow::Result<Model> {
		for model in self.models.values() {
			if model.name == model_name && model.dapp_id == *dapp_id && model.latest {
				return Ok(model.clone());
			}
		}

		if online {
			let (_, models) = self.load_dapp(dapp_id).await?;
			for model in models {
				if model.name == model_name && model.dapp_id == *dapp_id && model.latest {
					return Ok(model.clone());
				}
			}
		}

		anyhow::bail!(ModelStoreError::ModelNotInDapp(model_name.into(), *dapp_id))
	}

	pub async fn get_model(&mut self, model_id: &StreamId) -> anyhow::Result<Model> {
		if let Some(model) = self.models.get(&model_id.to_string()) {
			return Ok(model.clone());
		}

		let variables = dapp_table_client::get_dapp::Variables {
			dapp_id: None,
			model_id: Some(model_id.to_string()),
		};
		log::info!("lookup dapp with model_id: {}", model_id);
		let dapp = self.client.lookup_dapp(variables).await?;

		let models = self.store_dapp_models(dapp)?;
		for model in models {
			if model.id == *model_id {
				return Ok(model);
			}
		}
		anyhow::bail!(ModelStoreError::ModelIDNotInDapp(model_id.clone()))
	}
}
