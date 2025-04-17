mod config_builder;
mod web;
use tokio;
use web::web::start_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_server().await?;
    Ok(())
}
