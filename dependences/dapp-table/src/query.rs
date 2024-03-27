use anyhow::{Context, Result};

use graphql_client::{GraphQLQuery, Response};


use crate::errors::DappLookupError;

pub const DEFAULT_DAPP_TABLE_BACKEND: &str = "https://gateway.dataverse.art/v1/dapp-table/graphql";

pub struct Client {
	pub backend: String,
}

impl Client {
	pub fn new(backend: Option<String>) -> Self {
		Self {
			backend: backend.unwrap_or(DEFAULT_DAPP_TABLE_BACKEND.to_string()),
		}
	}

	pub async fn lookup_dapp_by_dapp_id(
		&self,
		dapp_id: &String,
	) -> Result<get_dapp::GetDappGetDapp> {
		let request_body = GetDapp::build_query(get_dapp::Variables {
			dapp_id: Some(dapp_id.to_string()),
			model_id: None,
		});

		let client = reqwest::Client::new();
		let res = client
			.post(&self.backend)
			.json(&request_body)
			.send()
			.await?;
		let response_body: Response<get_dapp::ResponseData> = res.json().await?;
		let dapp = response_body
			.data
			.context(DappLookupError::MissingResponseData(dapp_id.into()))?
			.get_dapp;
		Ok(dapp)
	}

	pub async fn lookup_dapp(
		&self,
		variables: get_dapp::Variables,
	) -> Result<get_dapp::GetDappGetDapp> {
		let request_body = GetDapp::build_query(variables);

		let client = reqwest::Client::new();
		let res = client
			.post(&self.backend)
			.json(&request_body)
			.send()
			.await?;
		let response_body: Response<get_dapp::ResponseData> = res.json().await?;
		let dapp = response_body
			.data
			.context(DappLookupError::MissingResponseData(
				request_body.operation_name.into(),
			))?
			.get_dapp;
		Ok(dapp)
	}

	pub async fn lookup_dapps(&self) -> Result<Vec<get_dapps::GetDappsGetDapps>> {
		let request_body = GetDapps::build_query(get_dapps::Variables {});

		let client = reqwest::Client::new();
		let res = client
			.post(&self.backend)
			.json(&request_body)
			.send()
			.await?;
		let response_body: Response<get_dapps::ResponseData> = res.json().await?;
		let dapp = response_body
			.data
			.context(DappLookupError::MissingResponseData(
				request_body.operation_name.into(),
			))?
			.get_dapps;
		Ok(dapp)
	}
}

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the schema
#[derive(GraphQLQuery, Clone, Copy)]
#[graphql(
	schema_path = "gql/schema.graphql",
	query_path = "gql/query.graphql",
	response_derives = "Debug"
)]
pub struct GetDapp;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the schema
#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "gql/schema.graphql",
	query_path = "gql/query.graphql",
	response_derives = "Debug"
)]
pub struct GetDapps;

#[cfg(test)]
mod tests {

	use super::*;

	#[tokio::test]
	async fn test_lookup_dapp_by_app_id() {
		let client = Client::new(None);
		let variables = get_dapp::Variables {
			dapp_id: Some("00d21b01-5166-4e22-acc2-10fc2c6be6a8".to_string()),
			model_id: None,
		};
		let resp = client.lookup_dapp(variables).await;
		assert!(resp.is_ok());
		let dapp = resp.unwrap();
		assert_eq!(dapp.id, "00d21b01-5166-4e22-acc2-10fc2c6be6a8");
	}

	#[tokio::test]
	async fn test_lookup_dapp_by_model_id() {
		let client = Client::new(None);
		let variables = get_dapp::Variables {
			dapp_id: None,
			model_id: Some(
				"kjzl6hvfrbw6c5m98besslbjufnwxk9t1uzebyu1gevzr17tq65sbe3vv8oq53b".to_string(),
			),
		};
		let resp = client.lookup_dapp(variables).await;
		assert!(resp.is_ok());
		let dapp = resp.unwrap();
		assert_eq!(dapp.id, "f329831c-d9c9-4a71-b98b-8235b57f04a6");
		assert!(dapp.models.iter().any(|m| {
			m.streams.iter().any(|m| {
				m.model_id == "kjzl6hvfrbw6c5m98besslbjufnwxk9t1uzebyu1gevzr17tq65sbe3vv8oq53b"
			})
		}));
	}

	#[tokio::test]
	async fn test_lookup_dapps() {
		let client = Client::new(None);
		let dapps = client.lookup_dapps().await;
		log::debug!("{:?}", dapps);
		assert!(dapps.is_ok());
		let dapps = dapps.unwrap();
		assert!(!dapps.is_empty());
	}
}
