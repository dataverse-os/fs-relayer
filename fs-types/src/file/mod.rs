pub mod client;
pub mod common;
pub mod operator;
pub mod status;

pub mod access_control;
pub mod action_file;
pub mod content_folder;
pub mod content_type;
pub mod index_file;
pub mod index_folder;

mod errors;

pub use index_file::*;

use std::fmt::Display;

use anyhow::Context;
pub use client::*;
use dataverse_ceramic::StreamState;
pub use operator::*;

use ceramic_core::StreamId;
use errors::StreamFileError;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use self::status::Status;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct StreamFile {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_id: Option<StreamId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file_model_id: Option<StreamId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub file: Option<Value>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub content_id: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub model_id: Option<StreamId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content: Option<Value>,

	pub controller: String,
	pub verified_status: Status,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub verified_status_desc: Option<String>,
}



impl StreamFile {
	pub fn new_with_file(state: StreamState) -> anyhow::Result<Self> {
		let mut file = Self::default();
		file.write_file(state)?;
		Ok(file)
	}

	pub fn write_file(&mut self, state: StreamState) -> anyhow::Result<()> {
		self.file = Some(state.content.clone());
		self.file_id = Some(state.stream_id()?);
		self.file_model_id = Some(state.must_model()?);
		self.controller = state
			.controllers()
			.first()
			.context(StreamFileError::NoControllerError)?
			.clone();
		Ok(())
	}

	pub fn new_with_content(state: StreamState) -> anyhow::Result<Self> {
		let mut file = Self::default();
		file.write_content(state)?;
		Ok(file)
	}

	pub fn write_content(&mut self, state: StreamState) -> anyhow::Result<()> {
		self.content = Some(state.content.clone());
		self.content_id = Some(state.stream_id()?.to_string());
		self.model_id = Some(state.must_model()?);
		self.controller = state
			.controllers()
			.first()
			.context(StreamFileError::NoControllerError)?
			.clone();
		Ok(())
	}

	pub fn write_status(&mut self, status: Status, desc: String) {
		self.verified_status = status;
		self.verified_status_desc = Some(format!("{:?}: {}", status, desc));
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileModel {
	IndexFile,
	ActionFile,
	IndexFolder,
	ContentFolder,
}

impl Display for FileModel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let str = match self {
			FileModel::IndexFile => "indexFile",
			FileModel::ActionFile => "actionFile",
			FileModel::IndexFolder => "indexFolder",
			FileModel::ContentFolder => "contentFolder",
		};
		write!(f, "{}", str)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn file_model() -> anyhow::Result<()> {
		assert_eq!(FileModel::IndexFile.to_string(), "indexFile".to_string());
		Ok(())
	}
}
