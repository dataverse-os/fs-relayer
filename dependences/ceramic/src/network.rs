use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::Context;
use ethers_core::types::{Block, Transaction};
use ethers_providers::{Http, Middleware, Provider};
use futures_util::FutureExt;
use int_enum::IntEnum;
use once_cell::sync::Lazy;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::event::AnchorProof;

#[repr(u64)]
#[derive(Debug, Clone, Copy, IntEnum, PartialEq, Eq, Hash)]
pub enum Chain {
	EthereumMainnet = 1,
	EthereumGnosis = 100,
	EthereumGoerli = 5,
	None = 0,
}

impl Chain {
	pub fn chain_id(&self) -> String {
		match self {
			Chain::None => "none".to_string(),
			_ => format!("eip155:{}", self.int_value()),
		}
	}

	pub fn network(&self) -> Network {
		match self {
			Chain::EthereumMainnet => Network::Mainnet,
			Chain::EthereumGnosis => Network::TestnetClay,
			Chain::EthereumGoerli => Network::DevUnstable,
			Chain::None => Network::InMemory,
		}
	}
}

impl FromStr for Chain {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"eip155:1" => Ok(Chain::EthereumMainnet),
			"eip155:100" => Ok(Chain::EthereumGnosis),
			"eip155:5" => Ok(Chain::EthereumGoerli),
			_ => anyhow::bail!("invalid chain"),
		}
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Network {
	/// Production network
	Mainnet,
	/// Test network
	TestnetClay,
	/// Developement network
	DevUnstable,
	/// Local network with unique id
	Local(u32),
	/// Singleton network in memory
	InMemory,
}

impl From<ceramic_core::Network> for Network {
	fn from(network: ceramic_core::Network) -> Self {
		match network {
			ceramic_core::Network::Mainnet => Network::Mainnet,
			ceramic_core::Network::TestnetClay => Network::TestnetClay,
			ceramic_core::Network::DevUnstable => Network::DevUnstable,
			ceramic_core::Network::Local(i) => Network::Local(i),
			ceramic_core::Network::InMemory => Network::InMemory,
		}
	}
}

impl Network {
	pub fn public(&self) -> bool {
		match self {
			Network::Mainnet => true,
			Network::TestnetClay => true,
			Network::DevUnstable => true,
			Network::Local(_) => false,
			Network::InMemory => false,
		}
	}

	pub fn kubo_topic(&self) -> String {
		multibase::encode(multibase::Base::Base64Url, self.pubsub_topic())
	}

	pub fn pubsub_topic(&self) -> String {
		match self {
			Network::Mainnet => "/ceramic/mainnet".to_string(),
			Network::TestnetClay => "/ceramic/testnet-clay".to_string(),
			Network::DevUnstable => "/ceramic/dev-unstable".to_string(),
			Network::Local(i) => format!("/ceramic/local-{}", i),
			Network::InMemory => "/ceramic/inmemory".to_owned(),
		}
	}

	pub fn chain(&self) -> Chain {
		match self {
			Network::Mainnet => Chain::EthereumMainnet,
			Network::TestnetClay => Chain::EthereumGnosis,
			Network::DevUnstable => Chain::EthereumGoerli,
			Network::Local(_) => Chain::None,
			Network::InMemory => Chain::None,
		}
	}
}

static PROVIDERS: Lazy<Mutex<Providers>> = Lazy::new(|| Mutex::new(Providers::default()));

pub async fn provider(chain: Chain) -> anyhow::Result<ProviderMiddleware> {
	PROVIDERS.lock().await.provider(chain)
}

#[derive(Debug, Clone)]
pub struct Providers {
	pub providers: HashMap<Chain, ProviderMiddleware>,
}

impl Default for Providers {
	fn default() -> Self {
		let mut rpcs: HashMap<Chain, &str> = HashMap::new();
		rpcs.insert(Chain::EthereumMainnet, "https://eth.llamarpc.com");
		rpcs.insert(Chain::EthereumGnosis, "https://rpc.gnosis.gateway.fm");

		let providers_future = Self::new(rpcs).boxed();
		

		tokio::task::block_in_place(|| futures::executor::block_on(providers_future)).unwrap()
	}
}

impl Providers {
	pub async fn new(rpcs: HashMap<Chain, &str>) -> anyhow::Result<Self> {
		let mut providers = HashMap::new();
		for (chain, rpc) in rpcs {
			let provider = Provider::<Http>::try_from(rpc)?;
			let chain_id = provider.get_chainid().await?;
			if chain_id.as_u64() != chain.int_value() {
				anyhow::bail!("chain id mismatch for {:?} {}", chain, rpc);
			}
			providers.insert(chain, ProviderMiddleware(chain, provider.into()));
		}
		Ok(Self { providers })
	}

	fn provider(&self, chain: Chain) -> anyhow::Result<ProviderMiddleware> {
		if let Some(p) = self.providers.get(&chain) {
			return Ok(p.clone());
		}
		anyhow::bail!("no rpc for chain {:?}", chain)
	}
}

static TRANSACTION_STORE: Lazy<Mutex<HashMap<H256, Transaction>>> =
	Lazy::new(|| Mutex::new(Default::default()));

static BLOCK_STORE: Lazy<Mutex<HashMap<H256, Block<H256>>>> =
	Lazy::new(|| Mutex::new(Default::default()));

#[derive(Debug, Clone)]
pub struct ProviderMiddleware(pub Chain, pub Arc<Provider<Http>>);

impl ProviderMiddleware {
	pub async fn get_transaction(&self, tx_hash: H256) -> anyhow::Result<Transaction> {
		let mut store = TRANSACTION_STORE.lock().await;
		if let Some(transaction) = store.get(&tx_hash) {
			return Ok(transaction.clone());
		}
		tracing::info!(
			transaction_hash = tx_hash.to_string(),
			"fetching transaction"
		);
		let tx = match self.1.get_transaction(tx_hash).await? {
			Some(tx) => tx,
			None => {
				tracing::warn!(
					transaction_hash = tx_hash.to_string(),
					"transaction not found with rpc"
				);
				anyhow::bail!("transaction not found: {}", tx_hash)
			}
		};
		store.insert(tx_hash, tx.clone());
		Ok(tx)
	}

	pub async fn get_block(&self, block_hash: H256) -> anyhow::Result<Block<H256>> {
		let mut store = BLOCK_STORE.lock().await;
		if let Some(block) = store.get(&block_hash) {
			return Ok(block.clone());
		}
		tracing::info!(block_hash = block_hash.to_string(), "fetching block");
		let block = match self.1.get_block(block_hash).await? {
			Some(block) => block,
			None => {
				tracing::warn!(
					block_hash = block_hash.to_string(),
					"block not found with rpc"
				);
				anyhow::bail!("block not found block_hash: {}", block_hash)
			}
		};
		store.insert(block_hash, block.clone());
		Ok(block)
	}
}

pub async fn timestamp(proof: AnchorProof) -> anyhow::Result<i64> {
	let provider = provider(proof.chain()?).await?;
	let tx_hash = proof.tx_hash()?;

	let tx = provider.get_transaction(tx_hash).await?;
	let block_hash = tx.block_hash.context("no block hash")?;
	let block = provider.get_block(block_hash).await?;
	Ok(block.timestamp.as_u64() as i64)
}

#[cfg(test)]
mod tests {
	use std::vec;

	use super::*;

	#[tokio::test]
	async fn test_chain_id() {
		let chain = Chain::EthereumMainnet;
		assert_eq!(chain.chain_id(), "eip155:1".to_string());

		let chain = Chain::EthereumGnosis;
		assert_eq!(chain.chain_id(), "eip155:100".to_string());

		let chain = Chain::EthereumGoerli;
		assert_eq!(chain.chain_id(), "eip155:5".to_string());

		let chain = Chain::None;
		assert_eq!(chain.chain_id(), "none".to_string());
	}

	#[tokio::test]
	async fn network() -> anyhow::Result<()> {
		let ceramic = "https://dataverseceramicdaemon.com";
		let http_client = crate::http::Client::init(ceramic)?;
		let chains = http_client.chains().await?;
		assert_eq!(chains.supported_chains, vec!["eip155:1"]);
		let chain = chains.supported_chains[0].parse::<Chain>();
		assert!(chain.is_ok());
		let network = chain.unwrap();
		assert_eq!(network, Chain::EthereumMainnet);
		Ok(())
	}

	#[tokio::test]
	async fn test_providers_new() {
		let mut rpcs = HashMap::new();
		rpcs.insert(Chain::EthereumMainnet, "https://eth.llamarpc.com");
		rpcs.insert(
			Chain::EthereumGnosis,
			"https://rpc.gnosis.gateway.fm",
		);
		let providers = Providers::new(rpcs).await;
		assert!(providers.is_ok());
	}
}
