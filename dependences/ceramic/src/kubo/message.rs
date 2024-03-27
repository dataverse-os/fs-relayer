use std::sync::Arc;

use base64::Engine;
use bytes::Bytes;
use ceramic_core::{Cid, StreamId};
use ceramic_kubo_rpc_server::{IdPostResponse, PubsubPubPostResponse, PubsubSubPostResponse};
use futures_util::StreamExt;
use libipld::cbor::DagCborCodec;
use libipld::codec::Codec;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{network::Network, Ceramic};

use super::{pubsub::Message, store, Client};

#[async_trait::async_trait]
pub trait MessageSubscriber: MessageResponsePublisher {
	async fn subscribe(&self, store: Arc<dyn store::Store>, network: Network)
		-> anyhow::Result<()>;

	async fn kubo_message_handler(
		&self,
		kubo_id: Arc<String>,
		network: Network,
		store: Arc<dyn store::Store>,
		event: Result<Bytes, Box<dyn std::error::Error + Send + Sync>>,
	) -> () {
		let msg_resp = match event {
			Ok(data) => match serde_json::from_slice::<MessageResponse>(&data) {
				Ok(msg) => msg,
				Err(err) => {
					let data = String::from_utf8_lossy(&data);
					tracing::error!(?data, "failed to decode as MessageResponse: {}", err);
					return;
				}
			},
			Err(err) => {
				tracing::error!("kubo sub error: {}", err);
				return;
			}
		};
		if msg_resp.from == *kubo_id {
			return;
		}

		if let Ok((_, msg_data)) = multibase::decode(msg_resp.data) {
			if let Ok(msg) = serde_json::from_slice::<Message>(&msg_data) {
				tracing::info!(?network, ?msg, "kubo sub receive msg");
				if let Err(err) = self
					.ceramic_message_handler(network, store, msg.clone())
					.await
				{
					tracing::error!(?network, ?msg, "ceramic message handler error: {}", err)
				};
			}
		};
	}

	async fn ceramic_message_handler(
		&self,
		network: Network,
		store: Arc<dyn store::Store>,
		msg: Message,
	) -> anyhow::Result<()> {
		match msg {
			Message::Query { id, stream } => {
				let stream_id: StreamId = stream.parse()?;
				if let Some(tip) = store.get(Some(id.clone()), Some(stream_id.clone())).await? {
					tracing::info!(?network, ?id, ?stream, ?tip, "query stored response");
					if let Err(err) = self.publish_response(&network, &id, &stream_id, &tip).await {
						tracing::error!(?network, ?id, ?stream, "publish response error: {}", err)
					}
				}
			}
			Message::Response { id, tips } => {
				if store.get(Some(id.clone()), None).await?.is_some() {
					for (stream_id, tip) in tips {
						let push = store
							.push(Some(id.clone()), Some(stream_id.parse()?), tip.parse()?)
							.await;
						if let Err(err) = push {
							tracing::error!(stream_id = id, "store push error: {}", err)
						}
					}
				};
			}
			Message::Update {
				stream,
				tip,
				model: _,
			} => {
				if let Some(tip_old) = store.get(None, Some(stream.parse()?)).await? {
					if tip_old.to_string() == tip {
						tracing::info!(?network, ?stream, ?tip, "update tip not changed");
						return Ok(());
					}
					if let Err(err) = store.push(None, Some(stream.parse()?), tip.parse()?).await {
						tracing::error!(stream, tip, "store push error: {}", err)
					}
				}
			}
			_ => (),
		}
		Ok(())
	}
}

#[async_trait::async_trait]
impl MessageSubscriber for Client {
	async fn subscribe(
		&self,
		store: Arc<dyn store::Store>,
		network: Network,
	) -> anyhow::Result<()> {
		let sub = self.pubsub_sub_post(network.kubo_topic()).await?;
		let kubo_id = self.id_post(None).await?;
		tracing::info!(?kubo_id, "subscribe on kubo id");
		let kube_id = match kubo_id {
			IdPostResponse::Success(id) => id.id,
			IdPostResponse::BadRequest(err) => {
				tracing::error!(?err, "failed to get kubo id");
				anyhow::bail!("failed to get kubo id: {}", err.message)
			}
		};

		if let PubsubSubPostResponse::Success(body) = sub {
			let store = Arc::clone(&store);
			let kube_id = Arc::new(kube_id);
			let handler = body.for_each_concurrent(None, move |event| {
				self.kubo_message_handler(kube_id.clone(), network, store.clone(), event)
			});
			handler.await;
			return Ok(());
		}
		anyhow::bail!("subscribe failed")
	}
}

#[async_trait::async_trait]
pub trait MessagePublisher {
	async fn publish_message(&self, topic: &str, msg: Vec<u8>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl MessagePublisher for Client {
	async fn publish_message(&self, topic: &str, msg: Vec<u8>) -> anyhow::Result<()> {
		let en_topic = multibase::encode(multibase::Base::Base64Url, topic);
		let file = swagger::ByteArray(msg);
		let res = self.pubsub_pub_post(en_topic, file).await?;
		match res {
			PubsubPubPostResponse::BadRequest(resp) => {
				tracing::warn!(topic, ?resp, "failed to post pub msg to kubo");
				anyhow::bail!(resp.message);
			}
			PubsubPubPostResponse::Success => Ok(()),
		}
	}
}

#[async_trait::async_trait]
pub trait MessageUpdatePublisher {
	async fn publish_update(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: &Cid,
		model: &StreamId,
	) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<T: MessagePublisher + Send + Sync> MessageUpdatePublisher for T {
	async fn publish_update(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: &Cid,
		model: &StreamId,
	) -> anyhow::Result<()> {
		let msg = json!({
			"typ": 0,
			"stream": stream_id.to_string(),
			"tip": tip.to_string(),
			"model": model.to_string(),
		});
		let file = serde_json::to_vec(&msg)?;

		self.publish_message(&ceramic.network.pubsub_topic(), file)
			.await
	}
}

#[async_trait::async_trait]
pub trait MessageResponsePublisher {
	async fn publish_response(
		&self,
		network: &Network,
		id: &str,
		stream_id: &StreamId,
		tip: &Cid,
	) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<T: MessagePublisher + Send + Sync> MessageResponsePublisher for T {
	async fn publish_response(
		&self,
		network: &Network,
		id: &str,
		stream_id: &StreamId,
		tip: &Cid,
	) -> anyhow::Result<()> {
		let msg = json!({
			"typ": 2,
			"id": id,
			"tips": {
				stream_id.to_string(): tip.to_string(),
			}
		});
		let file = serde_json::to_vec(&msg)?;
		self.publish_message(&network.pubsub_topic(), file).await
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
	pub from: String,
	pub data: String,
	pub seqno: String,
	#[serde(rename = "topicIDs")]
	pub topic_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, libipld::DagCbor, PartialEq, Eq)]
pub struct MessageQuery {
	#[ipld]
	pub tpy: i32,
	#[ipld]
	pub stream: String,
}

#[async_trait::async_trait]
pub trait TipQueryer {
	async fn query_last_tip(&self, network: Network, stream_id: &StreamId) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<T: MessagePublisher + Send + Sync> TipQueryer for T {
	async fn query_last_tip(&self, network: Network, stream_id: &StreamId) -> anyhow::Result<()> {
		let stream_id_str = stream_id.to_string();
		let id = message_hash(1, stream_id_str.to_string())?;
		let msg = json!({
			"typ": 1,
			"id": id,
			"stream": stream_id_str,
		});
		let file = serde_json::to_vec(&msg)?;
		self.publish_message(&network.pubsub_topic(), file).await
	}
}

pub fn message_hash(tpy: i32, stream: String) -> anyhow::Result<String> {
	let obj = MessageQuery { tpy, stream };
	let res = DagCborCodec.encode(&obj)?;
	let mut hasher = Sha256::new();
	hasher.update(res);
	let mut id: Vec<u8> = hasher.finalize().to_vec();
	let mut digest = vec![0x12, id.len() as u8];
	digest.append(&mut id);
	Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest))
}
