extern crate lru;

use ceramic_core::{Cid, StreamId};
use fang::{AsyncQueue, AsyncQueueable};
use lru::LruCache;
use postgres_openssl::MakeTlsConnector;
use std::{num::NonZeroUsize, sync::Arc};
use tokio::sync::Mutex;

use crate::{http, Ceramic, Event, EventValue, StreamLoader};

use super::{
	message::MessagePublisher,
	task::{BlockUploadHandler, UpdateMessagePublishHandler},
	AnchorRuester, BlockUploader, CidLoader, Client,
};

pub struct Cached {
	pub client: Arc<Client>,
	pub queue: Arc<Mutex<AsyncQueue<MakeTlsConnector>>>,
	pub cache: Arc<Mutex<LruCache<Cid, Vec<u8>>>>,
}

impl Cached {
	pub fn new(
		client: Arc<Client>,
		queue: Arc<Mutex<AsyncQueue<MakeTlsConnector>>>,
		cache_size: usize,
	) -> anyhow::Result<Self> {
		let cap = match NonZeroUsize::new(cache_size) {
			Some(cap) => cap,
			None => anyhow::bail!("{} is not a valid cache size", cache_size),
		};
		Ok(Self {
			client,
			queue,
			cache: Arc::new(Mutex::new(LruCache::new(cap))),
		})
	}
}

impl StreamLoader for Cached {}

#[async_trait::async_trait]
impl CidLoader for Cached {
	async fn load_cid(&self, cid: &Cid) -> anyhow::Result<Vec<u8>> {
		let data_opt;
		{
			let mut cache = self.cache.lock().await;
			data_opt = cache.get(cid).map(|data| data.to_vec());
		}
		if let Some(data) = data_opt {
			return Ok(data);
		}
		match self.client.load_cid(cid).await {
			Ok(data) => {
				let mut cache = self.cache.lock().await;
				cache.put(*cid, data.to_vec());
				Ok(data)
			}
			Err(err) => Err(err),
		}
	}
}

#[async_trait::async_trait]
impl BlockUploader for Cached {
	async fn block_upload(&self, cid: Cid, block: Vec<u8>) -> anyhow::Result<()> {
		self.cache.lock().await.put(cid, block.clone());
		let task = BlockUploadHandler { cid, block };
		if let Err(err) = self.queue.lock().await.insert_task(&task).await {
			log::error!("failed to insert task: {}", err);
		};
		Ok(())
	}
}

#[async_trait::async_trait]
impl MessagePublisher for Cached {
	async fn publish_message(&self, topic: &str, msg: Vec<u8>) -> anyhow::Result<()> {
		let task = UpdateMessagePublishHandler {
			topic: topic.into(),
			msg,
		};
		if let Err(err) = self.queue.lock().await.insert_task(&task).await {
			log::error!("failed to insert task: {}", err);
		};
		Ok(())
	}
}

#[async_trait::async_trait]
impl AnchorRuester for Cached {
	async fn request_anchor(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		event: Event,
	) -> anyhow::Result<()> {
		if let EventValue::Signed(_) = &event.value {
			let task = http::EventUploadHandler {
				ceramic: ceramic.clone(),
				stream_id: stream_id.clone(),
				commit: event,
			};
			if let Err(err) = self.queue.lock().await.insert_task(&task).await {
				log::error!("failed to insert task: {}", err);
			};
		}
		Ok(())
	}
}
