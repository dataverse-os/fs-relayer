use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::Utc;
use dataverse_ceramic::event::{Event, EventValue, VerifyOption};
use dataverse_ceramic::{StreamId, StreamState};
use dataverse_core::store::dapp;
use dataverse_core::stream::{Stream, StreamStore};
use int_enum::IntEnum;

use crate::file::errors::FileClientError;
use crate::file::status::Status;

use super::index_file::IndexFile;
use super::index_folder::IndexFolder;
use super::FileModel;
use super::{operator::StreamFileLoader, StreamFile};

pub struct Client {
	pub operator: Arc<dyn StreamFileLoader>,
	pub stream_store: Arc<dyn StreamStore>,
}

impl Client {
	pub fn new(operator: Arc<dyn StreamFileLoader>, stream_store: Arc<dyn StreamStore>) -> Self {
		Self {
			operator,
			stream_store,
		}
	}
}

impl Client {
	pub async fn get_file_model(
		&self,
		app_id: &uuid::Uuid,
		model: FileModel,
	) -> anyhow::Result<dataverse_core::store::dapp::Model> {
		dapp::get_model_by_name(app_id, &model.to_string()).await
	}

	pub async fn load_stream_by_app_id(
		&self,
		app_id: &uuid::Uuid,
		stream_id: &StreamId,
	) -> anyhow::Result<StreamState> {
		let ceramic = dapp::get_dapp_ceramic(app_id).await?;

		self.operator
			.load_stream_state(&ceramic, stream_id, None)
			.await
	}

	pub async fn load_streams_auto_model(
		&self,
		account: Option<String>,
		model_id: &StreamId,
	) -> anyhow::Result<Vec<StreamState>> {
		let model = dapp::get_model(model_id).await?;
		let ceramic = model.ceramic().await?;
		self.operator
			.load_stream_states(&ceramic, account, model_id)
			.await
	}
}

#[async_trait::async_trait]
pub trait StreamFileTrait {
	async fn load_file(&self, dapp_id: &uuid::Uuid, stream_id: &StreamId) -> Result<StreamFile>;

	async fn load_stream(&self, dapp_id: &uuid::Uuid, stream_id: &StreamId) -> Result<StreamState>;

	async fn load_files(
		&self,
		account: Option<String>,
		model_id: &StreamId,
		options: Vec<LoadFilesOption>,
	) -> anyhow::Result<Vec<StreamFile>>;
}

pub enum LoadFilesOption {
	Signal(serde_json::Value),
	None,
}

#[async_trait::async_trait]
impl StreamFileTrait for Client {
	async fn load_file(&self, dapp_id: &uuid::Uuid, stream_id: &StreamId) -> Result<StreamFile> {
		let ceramic = dapp::get_dapp_ceramic(dapp_id).await?;
		let stream_state = self
			.operator
			.load_stream_state(&ceramic, stream_id, None)
			.await?;
		let model_id = &stream_state.must_model()?;
		let model = dapp::get_model(model_id).await?;
		if model.dapp_id != *dapp_id {
			anyhow::bail!(FileClientError::StreamWithModelNotInDapp(stream_id.clone(), model_id.clone(), *dapp_id));
		}
		match model.name.as_str() {
			"indexFile" => {
				let index_file = serde_json::from_value::<IndexFile>(stream_state.content.clone())?;
				let mut file = StreamFile::new_with_file(stream_state)?;
				if let Ok(content_id) = &index_file.content_id.parse() {
					let content_state = self
						.operator
						.load_stream_state(&ceramic, content_id, None)
						.await?;
					file.write_content(content_state)?;
				}
				Ok(file)
			}
			"actionFile" => StreamFile::new_with_file(stream_state),
			"indexFolder" | "contentFolder" => StreamFile::new_with_content(stream_state),
			_ => {
				let mut file = StreamFile::new_with_content(stream_state)?;
				let index_file_model_id = self
					.get_file_model(dapp_id, FileModel::IndexFile)
					.await?
					.id;

				let index_file = self
					.operator
					.load_index_file_by_content_id(
						&ceramic,
						&index_file_model_id,
						&stream_id.to_string(),
					)
					.await;

				match index_file {
					Ok((file_state, _)) => {
						file.write_file(file_state)?;
					}
					Err(err) => {
						tracing::error!(
							model_id = index_file_model_id.to_string(),
							stream_id = stream_id.to_string(),
							"failed load index file model: {}",
							err
						);
						let desc = format!("failed load index file model: {}", err);
						file.write_status(Status::NakedStream, desc);
					}
				}
				Ok(file)
			}
		}
	}

	async fn load_stream(
		&self,
		dapp_id: &uuid::Uuid,
		stream_id: &StreamId,
	) -> anyhow::Result<StreamState> {
		let ceramic = dapp::get_dapp_ceramic(dapp_id).await?;
		self.operator
			.load_stream_state(&ceramic, stream_id, None)
			.await
	}

	async fn load_files(
		&self,
		account: Option<String>,
		model_id: &StreamId,
		options: Vec<LoadFilesOption>,
	) -> Result<Vec<StreamFile>> {
		let model = dapp::get_model(model_id).await?;
		let app_id = model.dapp_id;
		let ceramic = model.ceramic().await?;

		let stream_states = self
			.operator
			.load_stream_states(&ceramic, account.clone(), model_id)
			.await?;

		match model.name.as_str() {
			"indexFile" => {
				let mut files: Vec<StreamFile> = vec![];
				for state in stream_states {
					let index_file: IndexFile = serde_json::from_value(state.content.clone())?;
					let mut file = StreamFile::new_with_file(state)?;
					file.content_id = Some(index_file.content_id.clone());

					if let Ok(stream_id) = &index_file.content_id.parse() {
						let content_state = self
							.operator
							.load_stream_state(&ceramic, stream_id, None)
							.await?;
						if let Err(err) = file.write_content(content_state) {
							let desc = format!("failed load content file model {}", err);
							file.write_status(Status::BrokenContent, desc);
						};
					}
					files.push(file);
				}

				Ok(files)
			}
			"actionFile" => stream_states
				.into_iter()
				.map(StreamFile::new_with_file)
				.collect(),
			"indexFolder" => {
				let files = stream_states
					.into_iter()
					.filter_map(|state| {
						let mut file = StreamFile::new_with_content(state.clone()).ok()?;
						let index_folder =
							match serde_json::from_value::<IndexFolder>(state.content.clone()) {
								Err(err) => {
									file.write_status(
										Status::BrokenFolder,
										format!("Failed to asset content as index_folder: {}", err),
									);
									return Some(file);
								}
								Ok(index_folder) => index_folder,
							};

						let maybe_options = match index_folder.options() {
							Ok(options) => options,
							Err(err) => {
								file.write_status(
									Status::BrokenFolder,
									format!("Failed to decode folder options: {}", err),
								);
								return Some(file);
							}
						};

						// check if index_folder access control is valid
						// if let Err(err) = index_folder.access_control() {
						// 	file.write_status(
						// 		Status::BrokenFolder,
						// 		format!("access control error: {}", err),
						// 	);
						// 	return Some(file);
						// }

						// check if index_folder options contains every signals
						let required_signals: Vec<_> = options
							.iter()
							.filter_map(|option| match option {
								LoadFilesOption::Signal(signal) => Some(signal.clone()),
								_ => None,
							})
							.collect();

						let all_signals_present = required_signals.iter().all(|signal| {
							maybe_options
								.as_ref()
								.map_or(false, |options| options.signals.contains(signal))
						});

						if !all_signals_present {
							return None;
						}
						Some(file)
					})
					.collect();
				Ok(files)
			}
			"contentFolder" => stream_states
				.into_iter()
				.map(StreamFile::new_with_content)
				.collect(),
			_ => {
				let model_index_file = self.get_file_model(&app_id, FileModel::IndexFile).await?;

				let file_query_edges = self
					.operator
					.load_stream_states(&ceramic, account, &model_index_file.id)
					.await?;

				let mut file_map: HashMap<String, StreamFile> = HashMap::new();
				for state in stream_states {
					let content_id = state.stream_id()?;
					let file = StreamFile::new_with_content(state)?;
					file_map.insert(content_id.to_string(), file);
				}

				for node in file_query_edges {
					let index_file = serde_json::from_value::<IndexFile>(node.content.clone());
					if let Ok(index_file) = index_file {
						if let Some(stream_file) = file_map.get_mut(&index_file.content_id) {
							stream_file.file_model_id = Some(model_index_file.id.clone());
							stream_file.file_id = Some(node.stream_id()?);
							stream_file.file = Some(node.content);
						}
					}
				}

				// set verified_status to -1 if file_id is None (illegal file)
				let files = file_map.into_values().map(|mut file| {
						if file.file_id.is_none() {
							if let Some(content_id) = file.content_id.clone() {
								let desc = format!("file_id is None, content_id: {}", content_id);
								file.write_status(Status::NakedStream, desc);
							}
						}
						file
					})
					.collect();

				Ok(files)
			}
		}
	}
}

#[async_trait::async_trait]
pub trait StreamEventSaver {
	async fn save_event(
		&self,
		dapp_id: &uuid::Uuid,
		stream_id: &StreamId,
		event: &Event,
	) -> Result<StreamState>;
}

#[async_trait::async_trait]
impl StreamEventSaver for Client {
	async fn save_event(
		&self,
		dapp_id: &uuid::Uuid,
		stream_id: &StreamId,
		event: &Event,
	) -> Result<StreamState> {
		let ceramic = dapp::get_dapp_ceramic(dapp_id).await?;
		match &event.value {
			EventValue::Signed(signed) => {
				let (mut stream, mut commits) = {
					let stream = self.stream_store.load_stream(stream_id).await;
					match stream.ok().flatten() {
						Some(stream) => (
							stream.clone(),
							self.operator
								.load_events(&ceramic, stream_id, Some(stream.tip))
								.await?,
						),
						None => {
							if !signed.is_gensis() {
								anyhow::bail!(FileClientError::CommitStreamIdNotFoundOnStore(stream_id.clone()));
							}
							(
								Stream::new(dapp_id, stream_id.r#type.int_value(), event, None)?,
								vec![],
							)
						}
					}
				};
				// check if commit already exists
				if commits.iter().any(|ele| ele.cid == event.cid) {
					return stream.state(commits).await;
				}

				if let Some(prev) = event.prev()? {
					if commits.iter().all(|ele| ele.cid != prev) {
						anyhow::bail!(FileClientError::NoPrevCommitFound);
					}
				}
				commits.push(event.clone());
				let state = stream.state(commits).await?;

				let model = state.must_model()?;
				let opts = vec![
					VerifyOption::ResourceModelsContain(model.clone()),
					VerifyOption::ExpirationTimeBefore(Utc::now()),
				];
				event.verify_signature(opts)?;

				stream = Stream {
					model: Some(model),
					account: state.controllers().first().map(Clone::clone),
					tip: event.cid,
					content: state.content.clone(),
					..stream
				};

				self.stream_store.save_stream(&stream).await?;
				self.operator
					.upload_event(&ceramic, stream_id, event.clone())
					.await?;

				Ok(state)
			}
			EventValue::Anchor(_) => {
				anyhow::bail!(FileClientError::AnchorCommitUnsupported);
			}
		}
	}
}
