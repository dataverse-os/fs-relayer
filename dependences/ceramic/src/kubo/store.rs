use ceramic_core::{Cid, StreamId};

#[async_trait::async_trait]
pub trait Store: Sync + Send {
	async fn get(
		&self,
		id: Option<String>,
		stream_id: Option<StreamId>,
	) -> anyhow::Result<Option<Cid>>;
	async fn push(
		&self,
		id: Option<String>,
		stream_id: Option<StreamId>,
		tip: Cid,
	) -> anyhow::Result<()>;
}
