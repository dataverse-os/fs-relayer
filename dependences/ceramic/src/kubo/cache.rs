extern crate lru;

use ceramic_core::{Cid, StreamId};
use fang::{AsyncQueue, AsyncQueueable};
use postgres_openssl::MakeTlsConnector;
use std::{sync::Arc};
use redis::{AsyncCommands};
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
    pub redis_cli: Arc<redis::Client>,
    pub exp_seconds: u64,
}

impl Cached {
    pub fn new(
        client: Arc<Client>,
        queue: Arc<Mutex<AsyncQueue<MakeTlsConnector>>>,
        redis_cli: Arc<redis::Client>,
        exp_seconds : u64,
    ) -> Self {
        Self {
            client,
            queue,
            redis_cli,
            exp_seconds
        }
    }
}

impl StreamLoader for Cached {}


#[async_trait::async_trait]
impl CidLoader for Cached {
    async fn load_cid(&self, cid: &Cid) -> anyhow::Result<Vec<u8>> {
        let mut conn = self.redis_cli.get_multiplexed_async_connection().await?;
        let data_opt : Option<Vec<u8>> = conn.get(cid.to_string()).await.ok();
        if let Some(data) = data_opt {
            return Ok(data);
        }
        match self.client.load_cid(cid).await {
            Ok(data) => {
                let _: () = conn.set_ex(cid.to_string(), data.clone(), self.exp_seconds).await?;
                Ok(data)
            }
            Err(err) => Err(err),
        }
    }
}

#[async_trait::async_trait]
impl BlockUploader for Cached {
    async fn block_upload(&self, cid: Cid, block: Vec<u8>) -> anyhow::Result<()> {
        let mut conn = self.redis_cli.get_multiplexed_async_connection().await?;
        let _ : () = conn.set_ex(cid.to_string(), block.clone(), self.exp_seconds).await?;
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
