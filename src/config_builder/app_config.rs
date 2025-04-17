use ammonia::clean;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use url::Url;

const CONFIG_DIR: &str = "config";
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
        ensure_config_exists().await?;
        let config_str = fs::read_to_string(CONFIG_PATH)
            .await
            .context("Failed to read configuration file")?;
        if config_str.trim().is_empty() {
            return Ok(Arc::new(AppConfig { feeds: vec![] }));
        }
        let config: AppConfig =
            toml::de::from_str(&config_str).context("Failed to parse configuration")?;

        Ok(Arc::new(sanitize_config(config)))
    }
}

fn sanitize_config(config: AppConfig) -> AppConfig {
    let sanitized_feeds = config
        .feeds
        .into_iter()
        .filter(|feed| is_valid_url(&feed.url))
        .map(|feed| Feed {
            url: feed.url,
            title: feed.title.map(|t| clean(&t)),
            tags: feed.tags.into_iter().map(|tag| clean(tag.trim())).collect(),
        })
        .collect();

    AppConfig {
        feeds: sanitized_feeds,
    }
}

async fn ensure_config_exists() -> Result<()> {
    if !fs::metadata(CONFIG_DIR).await.is_ok() {
        fs::create_dir_all(CONFIG_DIR)
            .await
            .context("Failed to create config directory")?;
    }
    if !fs::metadata(CONFIG_PATH).await.is_ok() {
        fs::write(CONFIG_PATH, "")
            .await
            .context("Failed to create default configuration file")?;
    }

    Ok(())
}

fn is_valid_url(url: &str) -> bool {
    Url::parse(url)
        .map(|u| matches!(u.scheme(), "http" | "https"))
        .unwrap_or(false)
}
