use std::str::FromStr;

use anyhow::Result;
use async_std::task;
use ceramic_core::Cid;
use chrono::{DateTime, Utc};
use dataverse_ceramic::{self as ceramic, StreamId};
use dataverse_core::store::dapp;
use int_enum::IntEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{file::errors::IndexFileError, policy::Policy};

use super::{
	access_control::AccessControl,
	common::decode_base64,
	content_type::{ContentType, ContentTypeResourceType},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexFile {
	/// file name, encrypted when payable type
	pub file_name: String,
	pub file_type: u64,
	// stream_id or ipfs cid
	pub content_id: String,
	pub created_at: DateTime<Utc>,
	pub fs_version: Option<String>,
	pub updated_at: DateTime<Utc>,
	pub content_type: String,
	pub access_control: Option<String>,
	pub deleted: Option<bool>,
	pub reserved: Option<String>,
}

impl IndexFile {
	pub fn content_type(&self) -> anyhow::Result<ContentType> {
		Ok(serde_json::from_slice(&decode_base64(&self.content_type)?)?)
	}

	pub fn access_control(&self) -> anyhow::Result<Option<AccessControl>> {
		match &self.access_control {
			Some(acl) => {
				let decoded = decode_base64(acl)?;
				Ok(serde_json::from_slice(&decoded)?)
			}
			None => Ok(None),
		}
	}
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, IntEnum)]
pub enum IndexFileType {
	Public = 0,
	Private = 1,
	Payable = 2,
}

struct IndexFileProcessor {
	pub state: ModelState,
}

struct ModelState {
	dapp_id: uuid::Uuid,
}

#[async_trait::async_trait]
impl Policy for IndexFileProcessor {
	async fn effect_at(&self, state: &ceramic::StreamState) -> Result<bool> {
		// check model_name is indexfile
		let model_id = state.must_model()?;
		let model = dapp::get_model(&model_id).await?;
		Ok(model.name == "indexFile")
	}

	async fn validate_data(
		&self,
		_state: &ceramic::StreamState,
		data: serde_json::Value,
	) -> Result<()> {
		let content: IndexFile = serde_json::from_value(data)?;
		let content_type = content.content_type()?;

		// validate content id
		self.validate_content(&content.content_id, &content_type)
			.await?;
		// check acl
		if let Some(acl) = content.access_control()? {
			self.validate_acl(&acl).await?;
		};
		Ok(())
	}

	async fn validate_patch_add_or_replace(
		&self,
		data: &Value,
		path: &str,
		value: &Value,
	) -> Result<()> {
		match path {
			"/accessControl" => {
				let data = value.as_str().unwrap();
				let acl: AccessControl = AccessControl::from_str(data)?;
				task::block_on(self.validate_acl(&acl))
			}
			"/fileType" => IndexFileProcessor::validate_file_type_modify_constraint(data, value),
			_ => Ok(()),
		}
	}

	fn protected_fields(&self) -> Vec<String> {
		vec!["contentId".to_string(), "contentType".to_string()]
	}
}

impl IndexFileProcessor {
	pub fn validate_file_type_modify_constraint(
		data: &Value,
		_value: &Value,
	) -> anyhow::Result<()> {
		let index_file: IndexFile = serde_json::from_value(data.clone())?;
		if index_file.file_type == IndexFileType::Payable as u64 {
			anyhow::bail!(IndexFileError::FileTypeUnchangeable);
		}
		Ok(())
	}

	#[allow(dead_code)]
	pub async fn validate_content_id(&self, content_id: &str) -> anyhow::Result<()> {
		if let Ok(_stream_id) = StreamId::from_str(content_id) {
			// let state = self.stream_store.get_stream(&stream_id).await?;
			// let model_id = state.must_model()?;
			// let model = dapp::get_model(&model_id).await?;
			// if model.dapp_id != self.state.dapp_id {
			// 	anyhow::bail!("stream not in same app");
			// }
			// TODO check streamId not fs stream
			// TODO check streamId is Dapp stream
			// TODO check streamId can get from ceramic
		}
		Ok(())
	}

	pub async fn validate_content(
		&self,
		content_id: &str,
		content_type: &ContentType,
	) -> Result<()> {
		match content_type.resource {
			ContentTypeResourceType::IPFS => {
				let cid = Cid::from_str(content_id)?;
				log::debug!("content_id {} is ipfs cid", cid);
			}
			ContentTypeResourceType::CERAMIC => {
				if let Some(_resource_id) = &content_type.resource_id {
					// let model_id: StreamId = resource_id.parse()?;
					// let content_id: StreamId = content_id.parse()?;
					// let content = self.stream_store.get_stream(&content_id).await?;
					// if model_id != content.must_model()? {
					// 	anyhow::bail!("resourceId not match contentId")
					// }
				}
			}
			_ => {}
		};
		Ok(())
	}

	pub async fn validate_acl(&self, acl: &AccessControl) -> Result<()> {
		if let Some(p) = &acl.encryption_provider {
			let linked_ceramic_models = p.linked_ceramic_models()?;
			for ele in linked_ceramic_models {
				let model = dapp::get_model(&ele).await?;
				if model.dapp_id != self.state.dapp_id {
					anyhow::bail!(IndexFileError::LinkedModelNotInApp);
				}
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn test_decode_index_file() -> anyhow::Result<()> {
		let content = json!({
		  "fileName": "lfcMzQrSOjIdBDupp2Or9Gdp1qrnrcQcCov2t9m34ec",
		  "fileType": 2,
		  "contentId": "kjzl6kcym7w8y8syiams0kvm3qwfnutk2szi0wlhvf6rr9lalzpibxed0qvotuy",
		  "createdAt": "2023-09-01T07:03:23.313Z",
		  "fsVersion": "0.11",
		  "updatedAt": "2023-09-01T07:55:37.537Z",
		  "contentType": "eyJyZXNvdXJjZSI6IkNFUkFNSUMiLCJyZXNvdXJjZUlkIjoia2p6bDZodmZyYnc2Y2F0ZWszNmgzcGVwMDlrOWd5bWZubGE5azZvamxncm13am9ndmpxZzhxM3pweWJsMXl1In0",
		  "accessControl": "eyJlbmNyeXB0aW9uUHJvdmlkZXIiOnsicHJvdG9jb2wiOiJMaXQiLCJlbmNyeXB0ZWRTeW1tZXRyaWNLZXkiOiI1ODczNjBmMjc3MjUwM2FiZDI0Y2Y2M2RhMjI1MDAwNWNhYjc3ZDlhNjY4NTUyZTdiZDM3MjhlOGE3M2UzMGQ0YzQ2Mjc5NjExZDI5ZDgwN2JmZWVlNThjMGY4ZDFlMGRjNGJhOWI5MWMxMTMwYWUxMWZlZGViZDdlYzdmODkzNGJjZWNkZGQ3MTdlMjRhOTkyNDU1OTY3MjhjNTAxZGI5MjU1YjhiYTFmN2ZhYWIxOWFiOTk2ZjZkZjAzYWI3OTQwZWVmMmVlZGU0ZDMxODIxYTE4NGY5YzVjYmFkMjVlNWViYjE0OTczNjM0NjJlZGUyZmZmNTU1Yjk3MDQ0MzhhMDAwMDAwMDAwMDAwMDAyMGRjNTAzZjExZjdjNmU3MGM0NDMyZWY5ZjdhYjZhM2E4ZDgwNWZhY2YxNjlkMmFlNmYwYjY2MmZhY2VmM2E0YTk1ZDczMGY5OTFlZTBmMjhiZjk5N2ViODcxMDIwMDBiNiIsImRlY3J5cHRpb25Db25kaXRpb25zIjpbeyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmNhZ3Q2OTRpaW0yd3VlY3U3ZXVtZWRzN3FkMHA2dXptOGRucXNxNjlsbDdrYWNtMDVndSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmM3Z3U4OGc2NnoyOG44MWxjcGJnNmh1MnQ4cHUycHVpMHNmbnB2c3JocW4za3hoOXhhaSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmM4Nmd0OWo0MTV5dzJ4OHN0bWtvdGNyenBldXRyYmtwNDJpNHo5MGdwNWlicHR6NHNzbyJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmNhdGVrMzZoM3BlcDA5azlneW1mbmxhOWs2b2psZ3Jtd2pvZ3ZqcWc4cTN6cHlibDF5dSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0sW3siY29uZGl0aW9uVHlwZSI6ImV2bUJhc2ljIiwiY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiIiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6dXNlckFkZHJlc3MiXSwicmV0dXJuVmFsdWVUZXN0Ijp7ImNvbXBhcmF0b3IiOiI9IiwidmFsdWUiOiIweDMxMmVBODUyNzI2RTNBOWY2MzNBMDM3N2MwZWE4ODIwODZkNjY2NjYifX0seyJvcGVyYXRvciI6Im9yIn0seyJjb250cmFjdEFkZHJlc3MiOiIweDg2NzNmMjFCMzQzMTlCRDA3MDlBN2E1MDFCRDBmZEI2MTRBMGE3QTEiLCJjb25kaXRpb25UeXBlIjoiZXZtQ29udHJhY3QiLCJmdW5jdGlvbk5hbWUiOiJpc0NvbGxlY3RlZCIsImZ1bmN0aW9uUGFyYW1zIjpbIjp1c2VyQWRkcmVzcyJdLCJmdW5jdGlvbkFiaSI6eyJpbnB1dHMiOlt7ImludGVybmFsVHlwZSI6ImFkZHJlc3MiLCJuYW1lIjoidXNlciIsInR5cGUiOiJhZGRyZXNzIn1dLCJuYW1lIjoiaXNDb2xsZWN0ZWQiLCJvdXRwdXRzIjpbeyJpbnRlcm5hbFR5cGUiOiJib29sIiwibmFtZSI6IiIsInR5cGUiOiJib29sIn1dLCJzdGF0ZU11dGFiaWxpdHkiOiJ2aWV3IiwidHlwZSI6ImZ1bmN0aW9uIn0sImNoYWluIjoibXVtYmFpIiwicmV0dXJuVmFsdWVUZXN0Ijp7ImtleSI6IiIsImNvbXBhcmF0b3IiOiI9IiwidmFsdWUiOiJ0cnVlIn19XV0sImRlY3J5cHRpb25Db25kaXRpb25zVHlwZSI6IlVuaWZpZWRBY2Nlc3NDb250cm9sQ29uZGl0aW9uIn0sIm1vbmV0aXphdGlvblByb3ZpZGVyIjp7InByb3RvY29sIjoiTGVucyIsImJhc2VDb250cmFjdCI6IjB4NzU4MjE3N0Y5RTUzNmFCMGI2YzcyMWUxMWYzODNDMzI2RjJBZDFENSIsInVuaW9uQ29udHJhY3QiOiIweDc1ODIxNzdGOUU1MzZhQjBiNmM3MjFlMTFmMzgzQzMyNkYyQWQxRDUiLCJjaGFpbklkIjo4MDAwMSwiZGF0YXRva2VuSWQiOiIweDg2NzNmMjFCMzQzMTlCRDA3MDlBN2E1MDFCRDBmZEI2MTRBMGE3QTEifX0"
		});
		let index_file = serde_json::from_value::<IndexFile>(content);
		assert!(index_file.is_ok());
		let index_file = index_file.unwrap();
		assert_eq!(
			index_file.file_name,
			"lfcMzQrSOjIdBDupp2Or9Gdp1qrnrcQcCov2t9m34ec"
		);
		let content_type = index_file.content_type();
		assert!(content_type.is_ok());
		println!("{:?}", content_type.unwrap());

		// let access_control = index_file.access_control();
		// assert!(access_control.is_ok());
		// println!("{:?}", access_control.unwrap());

		Ok(())
	}

	#[test]
	fn parse_index_file() {
		let index_file = json!({
		  "fileName": "lfcMzQrSOjIdBDupp2Or9Gdp1qrnrcQcCov2t9m34ec",
		  "fileType": 2,
		  "contentId": "kjzl6kcym7w8y8syiams0kvm3qwfnutk2szi0wlhvf6rr9lalzpibxed0qvotuy",
		  "createdAt": "2023-09-01T07:03:23.313Z",
		  "fsVersion": "0.11",
		  "updatedAt": "2023-09-01T07:55:37.537Z",
		  "contentType": "eyJyZXNvdXJjZSI6IkNFUkFNSUMiLCJyZXNvdXJjZUlkIjoia2p6bDZodmZyYnc2Y2F0ZWszNmgzcGVwMDlrOWd5bWZubGE5azZvamxncm13am9ndmpxZzhxM3pweWJsMXl1In0",
		  "accessControl": "eyJlbmNyeXB0aW9uUHJvdmlkZXIiOnsicHJvdG9jb2wiOiJMaXQiLCJlbmNyeXB0ZWRTeW1tZXRyaWNLZXkiOiI1ODczNjBmMjc3MjUwM2FiZDI0Y2Y2M2RhMjI1MDAwNWNhYjc3ZDlhNjY4NTUyZTdiZDM3MjhlOGE3M2UzMGQ0YzQ2Mjc5NjExZDI5ZDgwN2JmZWVlNThjMGY4ZDFlMGRjNGJhOWI5MWMxMTMwYWUxMWZlZGViZDdlYzdmODkzNGJjZWNkZGQ3MTdlMjRhOTkyNDU1OTY3MjhjNTAxZGI5MjU1YjhiYTFmN2ZhYWIxOWFiOTk2ZjZkZjAzYWI3OTQwZWVmMmVlZGU0ZDMxODIxYTE4NGY5YzVjYmFkMjVlNWViYjE0OTczNjM0NjJlZGUyZmZmNTU1Yjk3MDQ0MzhhMDAwMDAwMDAwMDAwMDAyMGRjNTAzZjExZjdjNmU3MGM0NDMyZWY5ZjdhYjZhM2E4ZDgwNWZhY2YxNjlkMmFlNmYwYjY2MmZhY2VmM2E0YTk1ZDczMGY5OTFlZTBmMjhiZjk5N2ViODcxMDIwMDBiNiIsImRlY3J5cHRpb25Db25kaXRpb25zIjpbeyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmNhZ3Q2OTRpaW0yd3VlY3U3ZXVtZWRzN3FkMHA2dXptOGRucXNxNjlsbDdrYWNtMDVndSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmM3Z3U4OGc2NnoyOG44MWxjcGJnNmh1MnQ4cHUycHVpMHNmbnB2c3JocW4za3hoOXhhaSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmM4Nmd0OWo0MTV5dzJ4OHN0bWtvdGNyenBldXRyYmtwNDJpNHo5MGdwNWlicHR6NHNzbyJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0seyJjb25kaXRpb25UeXBlIjoiZXZtQmFzaWMiLCJjb250cmFjdEFkZHJlc3MiOiIiLCJzdGFuZGFyZENvbnRyYWN0VHlwZSI6IlNJV0UiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6cmVzb3VyY2VzIl0sInJldHVyblZhbHVlVGVzdCI6eyJjb21wYXJhdG9yIjoiY29udGFpbnMiLCJ2YWx1ZSI6ImNlcmFtaWM6Ly8qP21vZGVsPWtqemw2aHZmcmJ3NmNhdGVrMzZoM3BlcDA5azlneW1mbmxhOWs2b2psZ3Jtd2pvZ3ZqcWc4cTN6cHlibDF5dSJ9fSx7Im9wZXJhdG9yIjoiYW5kIn0sW3siY29uZGl0aW9uVHlwZSI6ImV2bUJhc2ljIiwiY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiIiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6dXNlckFkZHJlc3MiXSwicmV0dXJuVmFsdWVUZXN0Ijp7ImNvbXBhcmF0b3IiOiI9IiwidmFsdWUiOiIweDMxMmVBODUyNzI2RTNBOWY2MzNBMDM3N2MwZWE4ODIwODZkNjY2NjYifX0seyJvcGVyYXRvciI6Im9yIn0seyJjb250cmFjdEFkZHJlc3MiOiIweDg2NzNmMjFCMzQzMTlCRDA3MDlBN2E1MDFCRDBmZEI2MTRBMGE3QTEiLCJjb25kaXRpb25UeXBlIjoiZXZtQ29udHJhY3QiLCJmdW5jdGlvbk5hbWUiOiJpc0NvbGxlY3RlZCIsImZ1bmN0aW9uUGFyYW1zIjpbIjp1c2VyQWRkcmVzcyJdLCJmdW5jdGlvbkFiaSI6eyJpbnB1dHMiOlt7ImludGVybmFsVHlwZSI6ImFkZHJlc3MiLCJuYW1lIjoidXNlciIsInR5cGUiOiJhZGRyZXNzIn1dLCJuYW1lIjoiaXNDb2xsZWN0ZWQiLCJvdXRwdXRzIjpbeyJpbnRlcm5hbFR5cGUiOiJib29sIiwibmFtZSI6IiIsInR5cGUiOiJib29sIn1dLCJzdGF0ZU11dGFiaWxpdHkiOiJ2aWV3IiwidHlwZSI6ImZ1bmN0aW9uIn0sImNoYWluIjoibXVtYmFpIiwicmV0dXJuVmFsdWVUZXN0Ijp7ImtleSI6IiIsImNvbXBhcmF0b3IiOiI9IiwidmFsdWUiOiJ0cnVlIn19XV0sImRlY3J5cHRpb25Db25kaXRpb25zVHlwZSI6IlVuaWZpZWRBY2Nlc3NDb250cm9sQ29uZGl0aW9uIn0sIm1vbmV0aXphdGlvblByb3ZpZGVyIjp7InByb3RvY29sIjoiTGVucyIsImJhc2VDb250cmFjdCI6IjB4NzU4MjE3N0Y5RTUzNmFCMGI2YzcyMWUxMWYzODNDMzI2RjJBZDFENSIsInVuaW9uQ29udHJhY3QiOiIweDc1ODIxNzdGOUU1MzZhQjBiNmM3MjFlMTFmMzgzQzMyNkYyQWQxRDUiLCJjaGFpbklkIjo4MDAwMSwiZGF0YXRva2VuSWQiOiIweDg2NzNmMjFCMzQzMTlCRDA3MDlBN2E1MDFCRDBmZEI2MTRBMGE3QTEifX0"
		});

		let index_file = serde_json::from_value::<IndexFile>(index_file);
		assert!(index_file.is_ok());
		let index_file = index_file.unwrap();
		assert_eq!(
			index_file.file_name,
			"lfcMzQrSOjIdBDupp2Or9Gdp1qrnrcQcCov2t9m34ec"
		);
	}
}
