use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::model::{Entry, Link, Text};
use mediatype::MediaTypeBuf;
use reqwest::get as fetch;
use rss::Channel;
use std::default::Default;
use std::io::Cursor;
use std::str;
use tokio::task::spawn_blocking;

pub async fn fetch_feed(url: &str) -> Result<bytes::Bytes> {
    let response = fetch(url)
        .await
        .context(format!("Failed to fetch feed from {}", url))?;
    let headers = response.headers().clone();
    let content_type = headers.get("Content-Type");
    if !content_type.map_or(false, |ct| ct.to_str().unwrap_or("").contains("xml")) {
        eprintln!("Expected XML but got Content-Type: {:?}", content_type);
    }
    return response
        .bytes()
        .await
        .context("Failed to read response bytes");
}

pub async fn fallback_to_rss(body: &[u8]) -> Result<Vec<Entry>> {
    let data = body.to_vec();
    let entries = spawn_blocking(move || -> anyhow::Result<Vec<Entry>> {
        let cursor = Cursor::new(data);
        let channel = Channel::read_from(cursor).context("rss fallback parse failed")?;

        let content_type =
            MediaTypeBuf::from_string("text/plain".to_string()).expect("Valid media type");

        let entries = channel
            .items()
            .iter()
            .map(|item| Entry {
                title: item.title().map(|t| Text {
                    content: t.to_string(),
                    content_type: content_type.clone(),
                    src: None,
                }),
                links: item
                    .link()
                    .map(|href| {
                        vec![Link {
                            href: href.to_string(),
                            rel: None,
                            media_type: None,
                            href_lang: None,
                            title: None,
                            length: None,
                        }]
                    })
                    .unwrap_or_default(),
                summary: item.description().map(|desc| Text {
                    content: desc.to_string(),
                    content_type: content_type.clone(),
                    src: None,
                }),
                published: item.pub_date().and_then(|d| {
                    DateTime::parse_from_rfc2822(d)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                ..Default::default()
            })
            .collect();

        Ok(entries)
    })
    .await
    .context("spawn_blocking join error")??;

    Ok(entries)
}
