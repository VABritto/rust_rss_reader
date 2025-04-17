use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;

const CONFIG_PATH: &str = "config/feeds.toml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub feeds: Vec<Feed>,
}

impl AppConfig {
    pub async fn load_config_state() -> anyhow::Result<Arc<AppConfig>> {
        let config_str = fs::read_to_string(CONFIG_PATH)
            .await
            .context("Failed to read configuration file")?;
        let config: AppConfig =
            toml::de::from_str(&config_str).context("Failed to parse configuration")?;
        Ok(Arc::new(config))
    }
}
