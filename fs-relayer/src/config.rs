use std::path::PathBuf;

use anyhow::Context;
use ceramic_box::network::Network;
use directories::ProjectDirs;

static APP_NAME: &str = "dataverse-file-relayer";

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    data_path: Option<String>,
    pub kubo_path: String,
    pub networks: Vec<Network>,

    pub queue_dsn: String,
    pub queue_pool: u32,
    pub queue_worker: u32,

    pub cache_size: usize,
    pub index_models: IndexModels,
    pub ceramic: String,

    pub pgsql_dsn: Option<String>,
    pub iroh: IrohConfig,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct IrohConfig {
    pub key: String,
    author: String,
    model: String,
    streams: String,
}

impl Into<dataverse_iroh_store::KeySet> for IrohConfig {
    fn into(self) -> dataverse_iroh_store::KeySet {
        dataverse_iroh_store::KeySet {
            author: self.author,
            model: self.model,
            streams: self.streams,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexModels {
    None,
    All,
    Dapps(Vec<uuid::Uuid>),
}

impl Default for IndexModels {
    fn default() -> Self {
        IndexModels::All
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let file = confy::get_configuration_file_path(APP_NAME, None)?;
        tracing::info!("use config path: {:#?}", file);
        let cfg: Self = confy::load(APP_NAME, None)?;
        tracing::info!("use data path: {:#?}", cfg.data_path()?);
        Ok(cfg)
    }

    pub fn data_path(&self) -> anyhow::Result<PathBuf> {
        let project =
            ProjectDirs::from("rs", "", APP_NAME).context("Failed to get project dirs")?;
        Ok(project.data_dir().to_path_buf())
    }
}
