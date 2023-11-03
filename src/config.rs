use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

static APP_NAME: &str = "dataverse-node";

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    data_path: Option<String>,

    pub ceramic: String,
    pub iroh: IrohConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct IrohConfig {
    pub key: String,
    pub author: String,
    pub model: String,
    pub streams: String,
    pub patch: String,
    pub genesis: String,
}

impl Into<dataverse_iroh_store::KeySet> for IrohConfig {
    fn into(self) -> dataverse_iroh_store::KeySet {
        dataverse_iroh_store::KeySet {
            author: self.author,
            model: self.model,
            streams: self.streams,
            patch: self.patch,
            genesis: self.genesis,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let file = confy::get_configuration_file_path(APP_NAME, None)?;
        println!("The configuration file path is: {:#?}", file);
        let cfg: Self = confy::load(APP_NAME, None)?;
        println!("The data path is: {:#?}", cfg.data_path()?);
        Ok(cfg)
    }

    pub fn data_path(&self) -> anyhow::Result<PathBuf> {
        let project =
            ProjectDirs::from("rs", "", APP_NAME).context("Failed to get project dirs")?;
        Ok(project.data_dir().to_path_buf())
    }
}
