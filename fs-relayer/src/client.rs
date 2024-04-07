use crate::error::FileClientError;
use anyhow::Result;
use ceramic_box::event::{Event, EventValue, VerifyOption};
use ceramic_box::{StreamId, StreamState};
use chrono::Utc;
pub use dataverse_file_types::core::client::*;
use dataverse_file_types::core::dapp_store;
use dataverse_file_types::core::stream::{FileSlot, FileSlotStore};
use dataverse_file_types::file::index_file::IndexFile;
use dataverse_file_types::file::index_folder::IndexFolder;
use dataverse_file_types::file::status::Status;
use dataverse_file_types::file::FileModel;
use dataverse_file_types::file::{operator::FileLoader, File};
use int_enum::IntEnum;
use std::{collections::HashMap, sync::Arc};

pub struct Client {
    pub operator: Arc<dyn FileLoader>,
    pub file_store: Arc<dyn FileSlotStore>,
}

impl Client {
    pub fn new(operator: Arc<dyn FileLoader>, file_store: Arc<dyn FileSlotStore>) -> Self {
        Self {
            operator,
            file_store,
        }
    }
}

impl Client {
    pub async fn get_file_model(
        &self,
        app_id: &uuid::Uuid,
        model: FileModel,
    ) -> anyhow::Result<dapp_store::Model> {
        dapp_store::get_model_by_name(app_id, &model.to_string()).await
    }
}

#[async_trait]
impl FileTrait for Client {
    async fn load_file(&self, dapp_id: &uuid::Uuid, stream_id: &StreamId) -> Result<File> {
        let ceramic = dapp_store::get_dapp_ceramic(dapp_id).await?;
        let stream_state = self
            .operator
            .load_stream_state(&ceramic, stream_id, None)
            .await?;
        let model_id = &stream_state.must_model()?;
        let model = dapp_store::get_model(model_id).await?;
        if model.dapp_id != *dapp_id {
            anyhow::bail!(FileClientError::StreamWithModelNotInDapp(
                stream_id.clone(),
                model_id.clone(),
                *dapp_id
            ));
        }

        let file_model_name = serde_json::from_str::<FileModel>(&model.name);
        match file_model_name {
            Ok(model_name) => match model_name {
                FileModel::IndexFile => {
                    let index_file =
                        serde_json::from_value::<IndexFile>(stream_state.content.clone())?;
                    let mut file = File::new_with_file(stream_state)?;
                    if let Ok(content_id) = &index_file.content_id.parse() {
                        let content_state = self
                            .operator
                            .load_stream_state(&ceramic, content_id, None)
                            .await?;
                        file.write_content(content_state)?;
                    }
                    Ok(file)
                }
                FileModel::ActionFile => File::new_with_file(stream_state),
                FileModel::IndexFolder | FileModel::ContentFolder => {
                    File::new_with_content(stream_state)
                }
            },
            _ => {
                let mut file = File::new_with_content(stream_state)?;
                let index_file_model_id =
                    self.get_file_model(dapp_id, FileModel::IndexFile).await?.id;

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
                        file.write_status(Status::NakedFile, desc);
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
        let ceramic = dapp_store::get_dapp_ceramic(dapp_id).await?;
        self.operator
            .load_stream_state(&ceramic, stream_id, None)
            .await
    }

    async fn load_files(
        &self,
        account: Option<String>,
        model_id: &StreamId,
        options: Vec<LoadFilesOption>,
    ) -> Result<Vec<File>> {
        let model = dapp_store::get_model(model_id).await?;
        let app_id = model.dapp_id;
        let ceramic = model.ceramic().await?;

        let stream_states = self
            .operator
            .load_stream_states(&ceramic, account.clone(), model_id)
            .await?;

        match model.name.as_str() {
            "indexFile" => {
                let mut files: Vec<File> = vec![];
                for state in stream_states {
                    let index_file: IndexFile = serde_json::from_value(state.content.clone())?;
                    let mut file = File::new_with_file(state)?;
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
            "actionFile" => stream_states.into_iter().map(File::new_with_file).collect(),
            "indexFolder" => {
                let files = stream_states
                    .into_iter()
                    .filter_map(|state| {
                        let mut file = File::new_with_content(state.clone()).ok()?;
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
                .map(File::new_with_content)
                .collect(),
            _ => {
                let model_index_file = self.get_file_model(&app_id, FileModel::IndexFile).await?;

                let file_query_edges = self
                    .operator
                    .load_stream_states(&ceramic, account, &model_index_file.id)
                    .await?;

                let mut file_map: HashMap<String, File> = HashMap::new();
                for state in stream_states {
                    let content_id = state.stream_id()?;
                    let file = File::new_with_content(state)?;
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
                let files = file_map
                    .into_values()
                    .map(|mut file| {
                        if file.file_id.is_none() {
                            if let Some(content_id) = file.content_id.clone() {
                                let desc = format!("file_id is None, content_id: {}", content_id);
                                file.write_status(Status::NakedFile, desc);
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

#[async_trait]
impl EventSaver for Client {
    async fn save_event(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
        event: &Event,
    ) -> Result<StreamState> {
        let ceramic = dapp_store::get_dapp_ceramic(dapp_id).await?;
        match &event.value {
            EventValue::Signed(signed) => {
                let (mut stream, mut commits) = {
                    let stream = self.file_store.load_stream(stream_id).await;
                    match stream.ok().flatten() {
                        Some(stream) => (
                            stream.clone(),
                            self.operator
                                .load_events(&ceramic, stream_id, Some(stream.tip))
                                .await?,
                        ),
                        None => {
                            if !signed.is_gensis() {
                                anyhow::bail!(FileClientError::CommitStreamIdNotFoundOnStore(
                                    stream_id.clone()
                                ));
                            }
                            (
                                FileSlot::new(dapp_id, stream_id.r#type.int_value(), event, None)?,
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

                stream = FileSlot {
                    model: Some(model),
                    account: state.controllers().first().cloned(),
                    tip: event.cid,
                    content: state.content.clone(),
                    ..stream
                };

                self.file_store.save_stream(&stream).await?;
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
