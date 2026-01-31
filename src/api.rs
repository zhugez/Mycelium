use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use crate::error::AppError;
use crate::models::Plugin;

const BASE_URL: &str = "https://api.wordpress.org/plugins/info/1.2/";
const USER_AGENT: &str = "mycelium/1.0 (+contact: you@example.com)";

async fn get_json(url: &str, timeout: u64, retries: usize) -> Result<serde_json::Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;

    let mut last_err = None;

    for attempt in 0..retries {
        match client.get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        {
            Ok(response) => {
                let text = response.text().await?;
                return Ok(serde_json::from_str(&text)?);
            }
            Err(e) => {
                last_err = Some(e);
                let backoff = 1.0 * (2.0_f64).powi(attempt as i32);
                sleep(Duration::from_secs_f64(backoff)).await;
            }
        }
    }

    Err(AppError::Reqwest(last_err.unwrap()))
}

fn build_query_url(page: i32, per_page: i32) -> String {
    format!(
        "{}?action=query_plugins&request[browse]=popular&request[per_page]={}&request[page]={}",
        BASE_URL, per_page, page
    )
}

async fn fetch_page(page: i32, per_page: i32) -> Result<(i32, Vec<serde_json::Value>), AppError> {
    let url = build_query_url(page, per_page);
    let data = get_json(&url, 25, 4).await?;
    let plugins = data.get("plugins").and_then(|p| p.as_array()).unwrap_or(&Vec::new()).clone();
    Ok((page, plugins))
}

pub async fn fetch_popular_parallel(
    min_active_installs: i64,
    pages: i32,
    per_page: i32,
) -> Result<Vec<Plugin>, AppError> {
    let mut by_slug: HashMap<String, Plugin> = HashMap::new();
    let mut handles = Vec::new();

    for p in 1..=pages {
        let handle = tokio::spawn(fetch_page(p, per_page));
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await?;
        match result {
            Ok((page, plugins)) => {
                for p in plugins {
                    let slug = p.get("slug").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    if slug.is_empty() {
                        continue;
                    }

                    let ai = p.get("active_installs")
                        .and_then(|a| a.as_i64())
                        .unwrap_or(0);

                    if ai < min_active_installs {
                        continue;
                    }

                    let name = p.get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();

                    let last_updated = p.get("last_updated")
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());

                    let entry = Plugin {
                        slug: slug.clone(),
                        name,
                        active_installs: ai,
                        last_updated,
                        page,
                    };

                    // Keep the entry with highest active installs if duplicates appear (rare but possible)
                    if let Some(prev) = by_slug.get(&slug) {
                        if ai > prev.active_installs {
                            by_slug.insert(slug, entry);
                        }
                    } else {
                        by_slug.insert(slug, entry);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching page: {:?}", e);
            }
        }
    }

    let mut plugins: Vec<Plugin> = by_slug.into_values().collect();
    plugins.sort_by(|a, b| b.active_installs.cmp(&a.active_installs));
    Ok(plugins)
}

pub async fn get_download_link(slug: &str) -> Result<(Option<String>, Option<String>), AppError> {
    let url = format!(
        "{}?action=plugin_information&request[slug]={}",
        BASE_URL,
        slug
    );
    let info = get_json(&url, 25, 4).await?;
    let dl = info.get("download_link").and_then(|d| d.as_str()).map(|s| s.to_string());
    let version = info.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());

    if dl.as_ref().map(|s| s.starts_with("http")).unwrap_or(false) {
        Ok((dl, version))
    } else {
        Ok((None, version))
    }
}
