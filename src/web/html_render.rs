use super::parser::fetch_feed;
use crate::config_builder::AppConfig;
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse};
use feed_rs::model::Entry;
use std::sync::Arc;

pub async fn index() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; img-src 'self' data: https:; script-src 'self';"
            .parse()
            .unwrap(),
    );

    let html = match AppConfig::load_config_state().await {
        Ok(config) => generate_html_from_config(config).await,
        Err(err) => format!(
            "<html><body><h1>Error loading config</h1><p>{}</p></body></html>",
            err
        ),
    };

    (headers, Html(html))
}

async fn generate_html_from_config(config: Arc<AppConfig>) -> String {
    let mut html = render_page_start();

    for feed in &config.feeds {
        let feed_title = feed.title.clone().unwrap_or(feed.url.clone());
        html.push_str(&render_feed_title(&feed_title));

        match fetch_feed(&feed.url).await {
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

fn render_page_start() -> String {
    r#"
    <html>
    <head>
        <title>RSS Reader</title>
    </head>
    <body>
        <h1>Feeds</h1>
        <button onclick="refreshFeeds()">Refresh Feeds</button>
        <script>
            function refreshFeeds() {
                fetch('/refresh', { method: 'POST' })
                    .then(() => {
                        location.reload();
                    })
                    .catch(error => alert('Error refreshing feeds: ' + error));
            }
        </script>
        <ul>
    "#
    .to_string()
}

fn render_page_end() -> String {
    "</ul></body></html>".to_string()
}

fn render_feed_title(title: &str) -> String {
    format!(r#"<li><h2>{}</h2>"#, title)
}

fn render_feed_error(err: &str) -> String {
    format!(r#"<p>Error loading feed: {}</p></li>"#, err)
}

fn render_feed_entries(entries: &[Entry]) -> String {
    let mut html = String::from("<ul>");
    for entry in entries.iter().take(10) {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or("No Title".to_string());
        let link = entry
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or("#".to_string());
        let summary = entry
            .summary
            .as_ref()
            .map(|s| s.content.clone())
            .unwrap_or_default();
        let date = entry
            .published
            .map(|d| d.to_rfc2822())
            .unwrap_or("No date".to_string());

        let thumbnail = entry
            .media
            .first()
            .and_then(|media| media.thumbnails.first())
            .map(|thumb| {
                format!(
                    "<img src=\"{}\" alt=\"thumbnail\" style=\"max-width:200px;\"><br>",
                    thumb.image.uri
                )
            })
            .unwrap_or_default();

        html.push_str(&format!(
            "<li>{}<a href=\"{}\"><strong>{}</strong></a><br><em>{}</em><br><p>{}</p></li>",
            thumbnail, link, title, date, summary
        ));
    }
    html.push_str("</ul></li>");
    html
}
