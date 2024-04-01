#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate serde;

pub mod did;
pub mod event;
pub mod http;
pub mod kubo;
pub mod network;
pub mod stream;
pub mod types;

pub use ceramic_core::StreamId;
pub use ceramic_core::Cid;

pub use event::commit;
pub use event::{Event, EventValue, EventsLoader, EventsUploader};
use serde::{Deserialize, Serialize};
pub use stream::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ceramic {
    pub endpoint: String,
    pub network: network::Network,
}

impl Ceramic {
    pub async fn new(endpoint: &str) -> anyhow::Result<Self> {
        let network = http::Client::network(endpoint).await?;
        let endpoint = endpoint.into();
        Ok(Self { endpoint, network })
    }
}
