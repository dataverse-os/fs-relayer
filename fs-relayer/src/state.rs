use std::sync::Arc;

use crate::client::*;
use ceramic_box::{commit, event::Event, StreamId, StreamState};
use dataverse_file_types::core::stream::StreamStore;
use dataverse_file_types::file::{StreamFile, StreamFileLoader};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState {
    pub file_client: Arc<Client>,
}

impl AppState {
    pub fn new(operator: Arc<dyn StreamFileLoader>, stream_store: Arc<dyn StreamStore>) -> Self {
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
        let state = self
            .file_client
            .save_event(dapp_id, &stream_id, &commit)
            .await?;
        state.try_into()
    }

    pub async fn update_stream(
        &self,
        dapp_id: &uuid::Uuid,
        data: commit::Data,
    ) -> anyhow::Result<StreamStateResponse> {
        let commit: Event = data.commit.try_into()?;
        let state = self
            .file_client
            .save_event(dapp_id, &data.stream_id, &commit)
            .await?;
        state.try_into()
    }

    pub async fn load_stream(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamStateResponse> {
        self.file_client
            .load_stream(dapp_id, stream_id)
            .await?
            .try_into()
    }

    pub async fn load_file(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamFile> {
        self.file_client.load_file(dapp_id, stream_id).await
    }

    pub async fn load_files(
        &self,
        account: Option<String>,
        model_id: &StreamId,
        options: Vec<LoadFilesOption>,
    ) -> anyhow::Result<Vec<StreamFile>> {
        self.file_client
            .load_files(account, model_id, options)
            .await
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
