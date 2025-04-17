use ammonia::clean;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::model::{Entry, Link, Text};
use feed_rs::parser as feed_parser;
use mediatype::MediaTypeBuf;
use reqwest::get as fetch;
use rss::Channel;
use std::default::Default;
use std::io::Cursor;
use std::str;
use tokio::task::spawn_blocking;
use tokio::time::{Duration, timeout};

pub async fn fetch_feed(url: &str) -> Result<Vec<Entry>> {
    let response = timeout(Duration::from_secs(10), fetch(url))
        .await
        .context(format!("Failed to fetch feed from {}", url))??;
    let headers = response.headers().clone();
    let content_type = headers.get("Content-Type");
    if !content_type.map_or(false, |ct| ct.to_str().unwrap_or("").contains("xml")) {
        eprintln!("Expected XML but got Content-Type: {:?}", content_type);
    }
    let body = response
        .bytes()
        .await
        .context("Failed to read response bytes")?;

    if let Ok(feed) = feed_parser::parse(&body[..]) {
        return Ok(sanitize_and_validate_entries(feed.entries));
    }
    return fallback_to_rss(&body[..]).await;
}

async fn fallback_to_rss(body: &[u8]) -> Result<Vec<Entry>> {
    let data = body.to_vec();

    let entries = spawn_blocking(move || -> anyhow::Result<Vec<Entry>> {
        let cursor = Cursor::new(data);
        let channel = Channel::read_from(cursor).context("rss fallback parse failed")?;

        let content_type =
            MediaTypeBuf::from_string("text/plain".to_string()).expect("Valid media type");

        let raw_entries: Vec<Entry> = channel
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

        Ok(raw_entries)
    })
    .await
    .context("spawn_blocking join error")??;

    Ok(sanitize_and_validate_entries(entries))
}

fn sanitize_and_validate_entries(entries: Vec<Entry>) -> Vec<Entry> {
    entries
        .into_iter()
        .map(|mut entry| {
            if let Some(mut title) = entry.title {
                title.content = clean(&title.content).to_string();
                entry.title = Some(title);
            }
            if let Some(mut summary) = entry.summary {
                summary.content = clean(&summary.content).to_string();
                entry.summary = Some(summary);
            }

            let validated_links: Vec<Link> = entry
                .links
                .into_iter()
                .filter_map(|mut link| {
                    validated_url(&link.href).map(|valid| {
                        link.href = valid;
                        link
                    })
                })
                .collect();
            entry.links = validated_links;

            if let Some(mut media) = entry.media.first().cloned() {
                let validated_thumbnails = media
                    .thumbnails
                    .into_iter()
                    .filter_map(|mut thumb| {
                        validated_url(&thumb.image.uri).map(|valid| {
                            thumb.image.uri = valid;
                            thumb
                        })
                    })
                    .collect();
                media.thumbnails = validated_thumbnails;
                entry.media = vec![media];
            }

            entry
        })
        .collect()
}

fn validated_url(raw: &str) -> Option<String> {
    url::Url::parse(raw)
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .map(|u| u.to_string())
}
