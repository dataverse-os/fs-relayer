use chrono::{DateTime, Utc};
use int_enum::IntEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::*;

use super::{access_control::AccessControl, common::decode_base64};
use crate::file::errors::IndexFolderError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexFolder {
	pub folder_name: String,
	pub folder_type: FolderType,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
	pub fs_version: String,

	pub access_control: Option<String>,
	pub content_folder_ids: Vec<String>,

	pub options: Option<String>,
	pub deleted: Option<bool>,
	pub reserved: Option<String>,
}

impl IndexFolder {
	pub fn options(&self) -> anyhow::Result<Option<FolderOptions>> {
		match &self.options {
			Some(options) => {
				let decoded = decode_base64(options)?;
				Ok(serde_json::from_slice(&decoded)?)
			}
			None => Ok(None),
		}
	}

	pub fn access_control(&self) -> anyhow::Result<Option<AccessControl>> {
		match &self.access_control {
			Some(access_control) => {
				let decoded = decode_base64(access_control)?;
				serde_json::from_slice(&decoded).map_err(Into::into)
			}
			None => {
				if self.folder_type != FolderType::PublicFolderType {
					anyhow::bail!(IndexFolderError::AccessControlMissing)
				}
				Ok(None)
			}
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FolderOptions {
	pub folder_description: Option<String>,
	#[serde(default = "Vec::new")]
	pub signals: Vec<Value>,
}

#[repr(u64)]
#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy, IntEnum, PartialEq, Eq, Hash)]
pub enum FolderType {
	PublicFolderType = 0,
	PrivateFolderType = 1,
	UnionFolderType = 2,
}

#[cfg(test)]
mod tests {
	use super::*;

	use serde_json::json;

	#[test]
	fn decode_index_folder() {
		let value = json!({
				"accessControl": "eyJlbmNyeXB0aW9uUHJvdmlkZXIiOnsicHJvdG9jb2wiOiJMaXQiLCJlbmNyeXB0ZWRTeW1tZXRyaWNLZXkiOiI3YjIyNjM2OTcwNjg2NTcyNzQ2NTc4NzQyMjNhMjI3MjM5NjIzODYyNzU0ZDM5NmU3MTc1NmMyYjRjMmI2NTYxNjQ1MjZlNzc1YTcxNzU0MTJmMzc3MTU3MmI1NDZkNjQ1Mjc5NDI2NDJmNGE1Mjc1NTgyYjM5NDU1NjY3NzY1MjRhNWE0MTQzNDc2YjRlNjM3MTQ1NzY2OTcxNTI0YjRmMmYzNjczNzc2NTU5NDg0NTJiNTY0NzYyNjY0NDRiMzQ1MjU4NTMzODZkNjQ1NTcyMzY1NDQ4MzI0Yjc1NzA2YTQxNGY2ZDMyNzA0ZDU4NGQ3NDM1NDI0YTQ2NDM2MzUwNDM1ODRhNjU2ZTc0Mzk3MjRlNzM0MzY1NWE0MzY5MzU1MDMwNDQ1OTUxNzg2Njc5NjU2YTRmNTk1NDU3NTk0ODQ1NjQzODUzNDE0YzM1NDU2NTUxNmM3YTY5MzE0ODU2Mzg3ODM5NmQ1MDY1NjI0NDUwMzI2NzZhMmY0ODcxNjk2NjZjMzUzNzY3NDc3NjcxNGQzOTY0NDU0ZjZkNDMzMzZkNDk0NDIyMmMyMjY0NjE3NDYxNTQ2ZjQ1NmU2MzcyNzk3MDc0NDg2MTczNjgyMjNhMjI2MTY2MzI2NjMxMzM2MzMyMzczMzM1MzUzNzM1MzgzOTM4Mzc2NjMxMzAzNzM1NjIzNDM1NjY2NDY2MzEzNTM4MzAzNDM1MzQzMzM4NjUzNTMzMzYzNDM4Mzk2MTMyMzM2NjY1MzQ2MzM4NjQzODMwNjUzNzYyNjMzMjMzNjEzMDIyN2QiLCJkZWNyeXB0aW9uQ29uZGl0aW9ucyI6W3siY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiIiLCJjaGFpbiI6ImV0aGVyZXVtIiwibWV0aG9kIjoiIiwicGFyYW1ldGVycyI6WyI6dXNlckFkZHJlc3MiXSwicmV0dXJuVmFsdWVUZXN0Ijp7ImNvbXBhcmF0b3IiOiI9IiwidmFsdWUiOiIweENlZGY2MmRmMTk0NTQyYjNmYjNFMzc2ODQ4Zjg3Y0U5YWZkM0NkRGUifX0seyJvcGVyYXRvciI6ImFuZCJ9LHsiY29uZGl0aW9uVHlwZSI6ImV2bUJhc2ljIiwiY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiJTSVdFIiwiY2hhaW4iOiJldGhlcmV1bSIsIm1ldGhvZCI6IiIsInBhcmFtZXRlcnMiOlsiOnJlc291cmNlcyJdLCJyZXR1cm5WYWx1ZVRlc3QiOnsiY29tcGFyYXRvciI6ImNvbnRhaW5zIiwidmFsdWUiOiJjZXJhbWljOi8vKj9tb2RlbD1ranpsNmh2ZnJidzZjNnEyOWNvODNuOGF2OGtybHN0MzV3Ym9qdWh4eHMybmZ1eWtpcmJxNG5uZGpqdmR1M3EifX0seyJvcGVyYXRvciI6ImFuZCJ9LHsiY29uZGl0aW9uVHlwZSI6ImV2bUJhc2ljIiwiY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiJTSVdFIiwiY2hhaW4iOiJldGhlcmV1bSIsIm1ldGhvZCI6IiIsInBhcmFtZXRlcnMiOlsiOnJlc291cmNlcyJdLCJyZXR1cm5WYWx1ZVRlc3QiOnsiY29tcGFyYXRvciI6ImNvbnRhaW5zIiwidmFsdWUiOiJjZXJhbWljOi8vKj9tb2RlbD1ranpsNmh2ZnJidzZjODg3amhqeW45a3oxNXg2anduYTNrdml1cDh4NGxzMmEwdjZ4YjEzcXZpaXBiOHk3bWcifX0seyJvcGVyYXRvciI6ImFuZCJ9LHsiY29uZGl0aW9uVHlwZSI6ImV2bUJhc2ljIiwiY29udHJhY3RBZGRyZXNzIjoiIiwic3RhbmRhcmRDb250cmFjdFR5cGUiOiJTSVdFIiwiY2hhaW4iOiJldGhlcmV1bSIsIm1ldGhvZCI6IiIsInBhcmFtZXRlcnMiOlsiOnJlc291cmNlcyJdLCJyZXR1cm5WYWx1ZVRlc3QiOnsiY29tcGFyYXRvciI6ImNvbnRhaW5zIiwidmFsdWUiOiJjZXJhbWljOi8vKj9tb2RlbD1ranpsNmh2ZnJidzZjOHA1Y29jdHdxZjhmb2hrZWJ0dzBoaXhoMzR5emN3YW9od21xbmRlMG1oczdwdms0NGUifX1dLCJkZWNyeXB0aW9uQ29uZGl0aW9uc1R5cGUiOiJBY2Nlc3NDb250cm9sQ29uZGl0aW9uIn19",
				"contentFolderIds": [
						"kjzl6kcym7w8y50i8sgcbbr4ev2x55bzv88srrg1dmlt9tv8g0x7nnj9soli1t5"
				],
				"createdAt": "2024-03-04T11:09:46.769Z",
				"folderName": "Oc8-lTGwrvm+YK3kpf6pA8j+_PB24Pw+SfQgubDOZkA",
				"folderType": 1,
				"fsVersion": "0.11",
				"options": "e30",
				"updatedAt": "2024-03-04T11:09:46.769Z"
		});

		let index_folder = serde_json::from_value::<IndexFolder>(value);
		assert!(index_folder.is_ok());
		let index_folder = index_folder.unwrap();
		assert!(index_folder.options().is_ok());
		assert!(index_folder.access_control().is_ok());
	}

	#[test]
	fn test_decode_folder_options() {
		let encoded_data = "eyJzaWduYWxzIjpbeyJ0eXBlIjoyLCJpZCI6IjB4YmRiNmYwZmViYTMwM2RiMTcyYjU1NzcxMmNiZjI3YTYzM2MzYzZiM2NiNjg2YjI1ZGFjMTIxZTk2ODFjZmQ1NSJ9XX0";
		let decoded = decode_base64(encoded_data);
		assert!(decoded.is_ok());
		let decoded = decoded.unwrap();

		let folder_options: FolderOptions = serde_json::from_slice(&decoded).unwrap();

		assert_eq!(
			folder_options.signals,
			vec![
				json!({"type":2, "id":"0xbdb6f0feba303db172b557712cbf27a633c3c6b3cb686b25dac121e9681cfd55"})
			]
		);

		assert_eq!(
			folder_options.signals,
			vec![
				json!({"id":"0xbdb6f0feba303db172b557712cbf27a633c3c6b3cb686b25dac121e9681cfd55", "type":2})
			]
		);
	}
}
