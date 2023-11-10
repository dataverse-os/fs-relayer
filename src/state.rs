use futures::executor::block_on;
use std::{sync::Arc, thread};

use dataverse_file_system::{
    file, file::StreamFile, file::StreamFileTrait, stream::StreamPublisher,
};
use dataverse_iroh_store::commit::{Data, Genesis};
use dataverse_types::ceramic::{StreamId, StreamState};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppState<'a> {
    pub iroh_store: Arc<dataverse_iroh_store::Client>,
    pub file_client: Arc<file::Client<'a>>,
}

impl AppState<'_> {
    pub fn new(cache_client: dataverse_iroh_store::Client) -> Self {
        let iroh_store = Arc::new(cache_client);
        Self {
            iroh_store: iroh_store.clone(),
            file_client: Arc::new(file::Client::new(Some(iroh_store))),
        }
    }

    pub async fn create_stream(
        &self,
        dapp_id: &uuid::Uuid,
        genesis: Genesis,
    ) -> anyhow::Result<StreamStateResponse> {
        let (stream, state) = self
            .iroh_store
            .save_genesis_commit(dapp_id, genesis)
            .await?;
        let iroh_store = self.iroh_store.clone();
        let stream = stream.clone();
        thread::spawn(move || {
            block_on(async move {
                log::debug!(
                    "thread::spawn: {:?}",
                    iroh_store.publish_stream(stream).await
                );
            })
        });
        state.try_into()
    }

    pub async fn update_stream(
        &self,
        dapp_id: &uuid::Uuid,
        data: Data,
    ) -> anyhow::Result<StreamStateResponse> {
        let (stream, state) = self.iroh_store.save_data_commit(dapp_id, data).await?;
        let iroh_store = self.iroh_store.clone();
        let stream = stream.clone();
        thread::spawn(move || {
            block_on(async move {
                log::debug!(
                    "thread::spawn: {:?}",
                    iroh_store.publish_stream(stream).await
                );
            })
        });
        state.try_into()
    }

    pub async fn load_stream(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamStateResponse> {
        self.file_client
            .load_stream(&dapp_id, &stream_id)
            .await?
            .try_into()
    }

    pub async fn load_file(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamFile> {
        self.file_client.load_file(&dapp_id, &stream_id).await
    }

    pub async fn load_files(
        &self,
        account: &Option<String>,
        model_id: &StreamId,
    ) -> anyhow::Result<Vec<StreamFile>> {
        self.file_client.load_files(&account, &model_id).await
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
