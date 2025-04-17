use ammonia::clean;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use url::Url;

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

fn is_valid_url(url: &str) -> bool {
    Url::parse(url)
        .map(|u| matches!(u.scheme(), "http" | "https"))
        .unwrap_or(false)
}
