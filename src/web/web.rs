use super::html_render::index;
use crate::config_builder::AppConfig;
use axum::{Router, routing::get, routing::post};
use axum_server::Server;

pub async fn start_server() -> anyhow::Result<()> {
    let state = AppConfig::load_config_state().await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/refresh", post(index))
        .with_state(state);

    let addr = "127.0.0.1:3000".parse()?;
    Server::bind(addr).serve(app.into_make_service()).await?;

    Ok(())
}
