use anyhow::Context;
use ceramic_box::event::{Event, EventsLoader, EventsUploader};
use ceramic_box::Ceramic;
use ceramic_box::{Cid, StreamId};
use dataverse_file_types::core::stream::FileSlotStore;
use dataverse_file_types::file::FileLoader;

use crate::errors::IrohClientError;
use crate::Client;

impl FileLoader for Client {}

#[async_trait]
impl EventsUploader for Client {
    async fn upload_event(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        commit: Event,
    ) -> anyhow::Result<()> {
        self.operator.upload_event(ceramic, stream_id, commit).await
    }
}

#[async_trait]
impl EventsLoader for Client {
    async fn load_events(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        tip: Option<Cid>,
    ) -> anyhow::Result<Vec<Event>> {
        let tip = match tip {
            Some(tip) => tip,
            None => {
                self.load_stream(stream_id)
                    .await?
                    .context(IrohClientError::StreamNotFound(stream_id.clone()))?
                    .tip
            }
        };
        self.operator
            .load_events(ceramic, stream_id, Some(tip))
            .await
    }
}
