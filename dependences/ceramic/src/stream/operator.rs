use crate::event::{Event, EventsLoader, EventsUploader};
use crate::{AnchorStatus, Ceramic, StreamState};
use ceramic_core::{Cid, StreamId};
use int_enum::IntEnum;

#[async_trait::async_trait]
pub trait StreamOperator: StreamLoader + EventsUploader + Send + Sync {}

impl<T: StreamLoader + EventsUploader> StreamOperator for T {}

#[async_trait::async_trait]
pub trait StreamsLoader: StreamLoader {
	async fn load_stream_states(
		&self,
		ceramic: &Ceramic,
		account: Option<String>,
		model_id: &StreamId,
	) -> anyhow::Result<Vec<StreamState>>;
}

#[async_trait::async_trait]
pub trait StreamLoader: EventsLoader + Sync + Send {
	async fn load_stream_state(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: Option<Cid>,
	) -> anyhow::Result<StreamState> {
		let events = self.load_events(ceramic, stream_id, tip).await?;
		StreamState::make(stream_id.r#type.int_value(), events).await
	}
}

#[async_trait::async_trait]
pub trait StreamStateSaver {
	async fn save_stream_state(&self, state: &StreamState) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait StreamAnchorRequester {
	async fn request_anchor(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
	) -> anyhow::Result<AnchorStatus>;
}

pub struct CachedStreamLoader<T: StreamLoader> {
	loader: T,
	cache: std::collections::HashMap<String, StreamState>,
}

impl<T: StreamLoader> CachedStreamLoader<T> {
	pub fn new(loader: T) -> Self {
		Self {
			loader,
			cache: std::collections::HashMap::new(),
		}
	}
}

#[async_trait::async_trait]
impl<T: StreamLoader + Send + Sync> EventsLoader for CachedStreamLoader<T> {
	async fn load_events(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: Option<Cid>,
	) -> anyhow::Result<Vec<Event>> {
		self.loader.load_events(ceramic, stream_id, tip).await
	}
}

#[async_trait::async_trait]
impl<T: StreamLoader + Send + Sync> StreamLoader for CachedStreamLoader<T> {
	async fn load_stream_state(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: Option<Cid>,
	) -> anyhow::Result<StreamState> {
		if let Some(stream) = self.cache.get(&stream_id.to_string()) {
			return Ok(stream.clone());
		}

		let stream = self
			.loader
			.load_stream_state(ceramic, stream_id, tip)
			.await?;
		// TODO: insert data into cache
		Ok(stream)
	}
}

#[async_trait::async_trait]
impl<T: StreamsLoader + Send + Sync> StreamsLoader for CachedStreamLoader<T> {
	async fn load_stream_states(
		&self,
		ceramic: &Ceramic,
		account: Option<String>,
		model_id: &StreamId,
	) -> anyhow::Result<Vec<StreamState>> {
		self.loader
			.load_stream_states(ceramic, account, model_id)
			.await
	}
}
