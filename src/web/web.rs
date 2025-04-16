use super::html_render::{
    render_feed_entries, render_feed_error, render_feed_title, render_page_end, render_page_start,
};
use super::parser::{fallback_to_rss, fetch_feed};
use crate::config::AppConfig;
use anyhow::{Context, Result};
use axum::{Router, http::StatusCode, response::Html, routing::get, routing::post};
use axum_server::Server;
use feed_rs::model::Entry;
use feed_rs::parser as feed_parser;
use std::sync::Arc;
use tokio::fs;

pub async fn start_server(config: AppConfig) -> anyhow::Result<()> {
    let state = Arc::new(config);

    let app = Router::new()
        .route("/", get(index))
        .route("/refresh", post(refresh_feeds))
        .with_state(state);

    let addr = "127.0.0.1:3000".parse()?;
    Server::bind(addr).serve(app.into_make_service()).await?;

    Ok(())
}

pub async fn load_config(path: &str) -> anyhow::Result<Arc<AppConfig>> {
    let config_str = fs::read_to_string(path)
        .await
        .context("Failed to read configuration file")?;
    let config: AppConfig =
        toml::de::from_str(&config_str).context("Failed to parse configuration")?;
    let state = Arc::new(config);
    Ok(state)
}

pub async fn fetch_and_parse_feed(url: &str) -> Result<Vec<Entry>> {
    let body = fetch_feed(url).await?;

    if let Ok(feed) = feed_parser::parse(&body[..]) {
        return Ok(feed.entries);
    }
    return fallback_to_rss(&body[..]).await;
}

async fn refresh_feeds() -> Result<Html<String>, StatusCode> {
    let updated_config = load_config("feeds.toml").await.map_err(|err| {
        eprintln!("Error loading config: {:?}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let updated_html = generate_html_from_config(updated_config).await;

    Ok(Html(updated_html))
}

async fn generate_html_from_config(config: Arc<AppConfig>) -> String {
    let mut html = render_page_start();

    for feed in &config.feeds {
        let feed_title = feed.title.clone().unwrap_or(feed.url.clone());
        html.push_str(&render_feed_title(&feed_title));

        match fetch_and_parse_feed(&feed.url).await {
            Ok(entries) => {
                html.push_str(&render_feed_entries(&entries));
            }
            Err(err) => {
                html.push_str(&render_feed_error(&err.to_string()));
            }
        }
    }

    html.push_str(&render_page_end());
    html
}

async fn index() -> Html<String> {
    let config = load_config("feeds.toml").await.unwrap();
    let html = generate_html_from_config(config.clone()).await;
    Html(html)
}
