use ceramic_core::StreamId;
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::serde::{Deserialize, Serialize};
use fang::typetag;
use fang::AsyncRunnable;
use fang::FangError;

use crate::EventsUploader;
use crate::{Ceramic, Event};

#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct EventUploadHandler {
	pub ceramic: Ceramic,
	pub stream_id: StreamId,
	pub commit: Event,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for EventUploadHandler {
	async fn run(&self, _client: &mut dyn AsyncQueueable) -> Result<(), FangError> {
		let http_operator = super::Client::new();

		let result = http_operator
			.upload_event(&self.ceramic, &self.stream_id, self.commit.clone())
			.await;
		let stream_id = self.stream_id.to_string();
		let cid = self.commit.cid.to_string();
		match result {
			Err(err) => {
				tracing::warn!(stream_id, cid, ?err, "failed to upload event via http");
				Err(FangError {
					description: format!("failed to upload block: {:?}", err),
				})
			}
			Ok(_) => {
				tracing::info!(stream_id, cid, "upload event via http");
				Ok(())
			}
		}
	}

	fn uniq(&self) -> bool {
		true
	}
}
