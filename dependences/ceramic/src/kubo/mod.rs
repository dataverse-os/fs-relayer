pub mod cache;
pub mod message;
pub mod pubsub;
pub mod store;
pub mod task;

pub use cache::Cached;
pub use store::Store;

use crate::StreamId;
use ceramic_core::Cid;
use ceramic_kubo_rpc_server::models;
use ceramic_kubo_rpc_server::{ApiNoContext, ContextWrapperExt};
use ceramic_kubo_rpc_server::{BlockGetPostResponse, BlockPutPostResponse};
use int_enum::IntEnum;
use swagger::{AuthData, ByteArray, ContextBuilder, EmptyContext, Push, XSpanIdString};

use crate::event::{self, Event, EventsLoader, EventsUploader};
use crate::types::jws::ToCid;
use crate::{Ceramic, StreamLoader, StreamState};

use self::message::MessageUpdatePublisher;

pub type ClientContext = swagger::make_context_ty!(
    ContextBuilder,
    EmptyContext,
    Option<AuthData>,
    XSpanIdString
);

pub type Client = Box<dyn ApiNoContext<ClientContext> + Send + Sync>;

pub fn new(base_path: &str) -> Client {
    let context: ClientContext = swagger::make_context!(
        ContextBuilder,
        EmptyContext,
        None as Option<AuthData>,
        XSpanIdString::default()
    );

    // Using HTTP
    let client = Box::new(
        ceramic_kubo_rpc_server::Client::try_new_http(base_path)
            .expect("Failed to create HTTP client"),
    );
    Box::new(client.with_context(context))
}

#[async_trait]
pub trait CidLoader {
    async fn load_cid(&self, cid: &Cid) -> anyhow::Result<Vec<u8>>;

    async fn load_cid_retry_3_times(&self, cid: &Cid) -> anyhow::Result<Vec<u8>> {
        self.load_cid_with_retry(cid, 3).await
    }

    async fn load_cid_with_retry(&self, cid: &Cid, max_retries: u32) -> anyhow::Result<Vec<u8>> {
        let mut retries = 0;

        loop {
            match self.load_cid(cid).await {
                Ok(result) => return Ok(result),
                Err(err) if retries < max_retries => {
                    retries += 1;
                    tracing::warn!(
                        cid = cid.to_string(),
                        ?err,
                        "Failed to load CID, retrying... ({}/{})",
                        retries,
                        max_retries
                    );
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[async_trait]
impl CidLoader for Client {
    async fn load_cid(&self, cid: &Cid) -> anyhow::Result<Vec<u8>> {
        let timeout = Some("2s".into());

        let res = self.block_get_post(cid.to_string(), timeout, None).await?;

        let result = match res {
            BlockGetPostResponse::Success(bytes) => bytes.to_vec(),
            BlockGetPostResponse::BadRequest(err) => {
                tracing::warn!(?err, cid = cid.to_string(), "bad request");
                anyhow::bail!("bad request: {:?}", err);
            }
            BlockGetPostResponse::InternalError(err) => {
                tracing::warn!(?err, cid = cid.to_string(), "internal error");
                anyhow::bail!("internal error: {:?}", err);
            }
        };

        Ok(result)
    }
}

#[async_trait]
pub trait BlockUploader {
    async fn block_upload(&self, cid: Cid, block: Vec<u8>) -> anyhow::Result<()>;
}

#[async_trait]
impl BlockUploader for Client {
    async fn block_upload(&self, _cid: Cid, block: Vec<u8>) -> anyhow::Result<()> {
        let mhtype = Some(models::Multihash::Sha2256);
        let file = ByteArray(block.clone());
        let res = self.block_put_post(file, None, mhtype, None).await?;

        match res {
            BlockPutPostResponse::Success(res) => {
                tracing::info!(res.key, res.size, "block uploaded");
                Ok(())
            }
            BlockPutPostResponse::BadRequest(err) => {
                tracing::warn!(error = err.message, "bailed to post block: {:?}", err);
                anyhow::bail!("Failed to post block: {:?}", err)
            }
        }
    }
}

#[async_trait]
impl<T: BlockUploader + AnchorRuester + MessageUpdatePublisher + Send + Sync> EventsUploader for T {
    async fn upload_event(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        commit: Event,
    ) -> anyhow::Result<()> {
        match &commit.value {
            event::EventValue::Signed(signed) => {
                if let Some(cacao_block) = &signed.cacao_block {
                    self.block_upload(signed.cacao_link()?, cacao_block.clone())
                        .await?;
                }
                if let Some(linked_block) = &signed.linked_block {
                    self.block_upload(signed.payload_link()?, linked_block.clone())
                        .await?;
                }
                self.block_upload(commit.cid, signed.jws.to_vec()?).await?;
            }
            // anchor commit generate by ceramic node default
            // don't need to upload it
            event::EventValue::Anchor(_) => {}
        }
        self.request_anchor(ceramic, stream_id, commit).await?;
        Ok(())
    }

    async fn upload_events(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        commits: Vec<Event>,
    ) -> anyhow::Result<()> {
        let tip = match commits.last() {
            Some(commit) => commit.cid,
            None => anyhow::bail!("input commits of {} is empty", stream_id),
        };

        let state = StreamState::make(stream_id.r#type.int_value(), commits.clone()).await?;
        let model = state.must_model()?;

        for commit in commits {
            self.upload_event(ceramic, stream_id, commit).await?;
        }

        self.publish_update(ceramic, stream_id, &tip, &model)
            .await?;
        Ok(())
    }
}

impl StreamLoader for Client {}

#[async_trait]
impl<T: CidLoader + Send + Sync> EventsLoader for T {
    async fn load_events(
        &self,
        _ceramic: &Ceramic,
        _stream_id: &StreamId,
        tip: Option<Cid>,
    ) -> anyhow::Result<Vec<Event>> {
        let mut commits = Vec::new();
        let mut cid = match tip {
            Some(tip) => tip,
            None => {
                log::warn!("kubo not support query tip, should input tip");
                todo!("query latest tip of stream")
            }
        };
        loop {
            let bytes = self.load_cid_retry_3_times(&cid).await?;
            let mut commit = event::Event::decode(cid, bytes.to_vec())?;
            match &mut commit.value {
                event::EventValue::Signed(signed) => {
                    signed.linked_block =
                        Some(self.load_cid_retry_3_times(&signed.payload_link()?).await?);
                    signed.cacao_block = Some(self.load_cid_retry_3_times(&signed.cap()?).await?);
                }
                event::EventValue::Anchor(anchor) => {
                    anchor.proof_block = Some(self.load_cid_retry_3_times(&anchor.proof).await?)
                }
            }
            commits.insert(0, commit.clone());
            match commit.prev()? {
                Some(prev) => cid = prev,
                None => break,
            };
        }
        Ok(commits)
    }
}

#[async_trait]
pub trait AnchorRuester {
    async fn request_anchor(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        event: Event,
    ) -> anyhow::Result<()>;
}

#[async_trait]
impl AnchorRuester for Client {
    async fn request_anchor(
        &self,
        ceramic: &Ceramic,
        stream_id: &StreamId,
        event: Event,
    ) -> anyhow::Result<()> {
        let http_operator = crate::http::Client::new();

        let result = http_operator.upload_event(ceramic, stream_id, event).await;
        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!("failed to request for anchor: {}", err);
                Err(err)
            }
        }
    }
}
