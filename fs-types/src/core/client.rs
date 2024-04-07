use ceramic_box::event::Event;
use ceramic_box::{StreamId, StreamState};

#[async_trait]
pub trait FileTrait {
    async fn load_file(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<crate::file::File>;

    async fn load_stream(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
    ) -> anyhow::Result<StreamState>;

    async fn load_files(
        &self,
        account: Option<String>,
        model_id: &StreamId,
        options: Vec<LoadFilesOption>,
    ) -> anyhow::Result<Vec<crate::file::File>>;
}

pub enum LoadFilesOption {
    Signal(serde_json::Value),
    None,
}

#[async_trait]
pub trait EventSaver {
    async fn save_event(
        &self,
        dapp_id: &uuid::Uuid,
        stream_id: &StreamId,
        event: &Event,
    ) -> anyhow::Result<StreamState>;
}
