use std::sync::Arc;

use crate::client::*;
use ceramic_box::{commit, event::Event, StreamId, StreamState};
use dataverse_file_types::core::stream::FileSlotStore;
use dataverse_file_types::file::{File, FileLoader};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub file_client: Arc<Client>,
}

impl AppState {
    pub fn new(operator: Arc<dyn FileLoader>, stream_store: Arc<dyn FileSlotStore>) -> Self {
        let data = Client::new(operator, stream_store);
        Self {
            file_client: Arc::new(data),
        }
    }

    pub async fn create_stream(
        &self,
        dapp_id: &uuid::Uuid,
        genesis: commit::Genesis,
    ) -> anyhow::Result<StreamStateResponse> {
        let stream_id = genesis.stream_id()?;
        let commit: Event = genesis.genesis.try_into()?;
        let result = self
            .file_client
            .save_event(dapp_id, &stream_id, &commit)
            .await;
        if let Err(err) = &result {
            tracing::warn!(
                dapp_id = dapp_id.to_string(),
                "create stream error: {}",
                err
            );
        }
        result?.try_into()
    }

    pub async fn update_stream(
        &self,
        dapp_id: &uuid::Uuid,
        data: commit::Data,
    ) -> anyhow::Result<StreamStateResponse> {
        let commit: Event = data.commit.try_into()?;
        let result = self
            .file_client
            .save_event(dapp_id, &data.stream_id, &commit)
            .await;
        if let Err(err) = &result {
            tracing::warn!(
                dapp_id = dapp_id.to_string(),
                "update stream error: {}",
                err
            );
        }
        result?.try_into()
    }

    pub async fn load_stream(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamStateResponse> {
        let result = self.file_client.load_stream(dapp_id, stream_id).await;
        if let Err(err) = &result {
            tracing::warn!(
                format = "ceramic",
                stream_id = stream_id.to_string(),
                dapp_id = dapp_id.to_string(),
                "load stream error: {}",
                err
            );
        }
        result?.try_into()
    }

    pub async fn load_file(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<File> {
        let result = self.file_client.load_file(dapp_id, stream_id).await;
        if let Err(err) = &result {
            tracing::warn!(
                format = "dataverse",
                stream_id = stream_id.to_string(),
                dapp_id = dapp_id.to_string(),
                "load stream error: {}",
                err
            );
        }
        result
    }

    pub async fn load_files(
        &self,
        account: Option<String>,
        model_id: &StreamId,
        options: Vec<LoadFilesOption>,
    ) -> anyhow::Result<Vec<File>> {
        let result = self
            .file_client
            .load_files(account.clone(), model_id, options)
            .await;
        if let Err(err) = &result {
            tracing::warn!(
                model_id = model_id.to_string(),
                account = account,
                "load streams error: {}",
                err
            );
        }
        return result;
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStateResponse {
    pub stream_id: StreamId,
    pub state: StreamState,
}

impl TryFrom<StreamState> for StreamStateResponse {
    type Error = anyhow::Error;

    fn try_from(value: StreamState) -> Result<Self, Self::Error> {
        Ok(Self {
            stream_id: value.stream_id()?,
            state: value,
        })
    }
}
