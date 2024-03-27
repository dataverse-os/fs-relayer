use ceramic_core::Cid;
use fang::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::serde::{Deserialize, Serialize};
use fang::typetag;
use fang::AsyncRunnable;
use fang::FangError;
use std::sync::OnceLock;

use super::message::MessagePublisher;
use super::{BlockUploader, Client};

static KUBO: OnceLock<Client> = OnceLock::new();

pub fn init_kubo(base_path: &str) {
	KUBO.get_or_init(|| super::new(base_path));
}

async fn get_kubo() -> Result<&'static Client, FangError> {
	match KUBO.get() {
		Some(kubo) => Ok(kubo),
		None => {
			tracing::error!("Kubo client not initialized");
			Err(FangError {
				description: "Kubo client not initialized".to_string(),
			})
		}
	}
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct BlockUploadHandler {
	pub cid: Cid,
	pub block: Vec<u8>,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for BlockUploadHandler {
	async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
		let kubo = get_kubo().await?;

		match kubo.block_upload(self.cid, self.block.clone()).await {
			Ok(_) => {
				tracing::info!(cid = self.cid.to_string(), "uploading block");
				Ok(())
			}
			Err(err) => {
				tracing::warn!(cid = self.cid.to_string(), ?err, "uploading block");
				Err(FangError {
					description: format!("Failed to upload block: {:?}", err),
				})
			}
		}
	}

	fn uniq(&self) -> bool {
		true
	}
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct UpdateMessagePublishHandler {
	pub topic: String,
	pub msg: Vec<u8>,
}

#[async_trait]
#[typetag::serde]
impl AsyncRunnable for UpdateMessagePublishHandler {
	async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
		let kubo = get_kubo().await?;

		let res = kubo.publish_message(&self.topic, self.msg.clone()).await;
		match res {
			Ok(_) => {
				tracing::info!(topic = self.topic, "publishing message");
				Ok(())
			}
			Err(err) => {
				tracing::warn!(topic = self.topic, ?err, "publishing message");
				Err(FangError {
					description: format!("Failed to publish message: {:?}", err),
				})
			}
		}
	}

	fn uniq(&self) -> bool {
		true
	}
}
