mod config;
mod web;
use config::AppConfig;
use tokio;
use web::web::start_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load("feeds.toml")?;
    start_server(config).await?;
    Ok(())
}
