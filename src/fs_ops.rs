use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;
use regex::Regex;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use zip::ZipArchive;

use crate::error::AppError;
use crate::models::DownloadResult;
use crate::api::get_download_link;

const USER_AGENT: &str = "mycelium/1.0 (+contact: you@example.com)";

async fn download_file(url: &str, dest: &Path, timeout: u64, retries: usize) -> Result<(), AppError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let ext = dest.extension().unwrap_or_default().to_str().unwrap_or("");
    let tmp_path = dest.with_extension(format!("{}.part", ext));

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
                let bytes = response.bytes().await?;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(&tmp_path)
                    .await?;

                file.write_all(&bytes).await?;
                file.sync_all().await?;

                tokio::fs::rename(&tmp_path, dest).await?;
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                let backoff = 1.0 * (2.0_f64).powi(attempt as i32);
                sleep(Duration::from_secs_f64(backoff)).await;
            }
        }
    }

    Err(AppError::Reqwest(last_err.unwrap()))
}

pub async fn download_one(slug: &str, out_dir: &Path) -> DownloadResult {
    match get_download_link(slug).await {
        Ok((Some(dl), version)) => {
            let dest = out_dir.join(format!("{}.zip", slug));
            if dest.exists() && fs::metadata(&dest).map(|m| m.len() > 0).unwrap_or(false) {
                return DownloadResult {
                    slug: slug.to_string(),
                    success: true,
                    message: "already exists".to_string(),
                    version,
                };
            }

            match download_file(&dl, &dest, 60, 4).await {
                Ok(_) => DownloadResult {
                    slug: slug.to_string(),
                    success: true,
                    message: "downloaded".to_string(),
                    version,
                },
                Err(e) => DownloadResult {
                    slug: slug.to_string(),
                    success: false,
                    message: format!("download failed: {}", e),
                    version,
                },
            }
        }
        Ok((None, version)) => DownloadResult {
            slug: slug.to_string(),
            success: false,
            message: "no download_link".to_string(),
            version,
        },
        Err(e) => DownloadResult {
            slug: slug.to_string(),
            success: false,
            message: format!("error: {}", e),
            version: None,
        },
    }
}

pub fn extract_one_zip(zip_path: &Path, extract_dir: &Path) -> Result<String, String> {
    let slug = zip_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let file = File::open(zip_path)
        .map_err(|e| format!("cannot open: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)
        .map_err(|e| format!("invalid zip: {}", e))?;

    archive.extract(extract_dir)
        .map_err(|e| format!("extract failed: {}", e))?;

    Ok(slug.to_string())
}

pub fn extract_all_zips(zip_dir: &str, extract_dir: &str) -> Vec<String> {
    let zip_path = Path::new(zip_dir);
    let extract_path = Path::new(extract_dir);
    let mut extracted_slugs = Vec::new();

    if let Err(e) = fs::create_dir_all(extract_path) {
        eprintln!("Cannot create extract dir: {}", e);
        return extracted_slugs;
    }

    let zip_files: Vec<_> = match fs::read_dir(zip_path) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|ext| ext == "zip").unwrap_or(false))
            .collect(),
        Err(e) => {
            eprintln!("Cannot read zip dir: {}", e);
            return extracted_slugs;
        }
    };

    if zip_files.is_empty() {
        println!("No zip files found in {}", zip_path.display());
        return extracted_slugs;
    }

    println!("\n--- Extracting {} plugins ---", zip_files.len());

    let mut ok = 0;
    let mut fail = 0;

    for zip_file in &zip_files {
        let slug = zip_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        match extract_one_zip(zip_file, extract_path) {
            Ok(_) => {
                ok += 1;
                extracted_slugs.push(slug.to_string());
                println!("[OK] {}: extracted successfully", slug);
            }
            Err(e) => {
                fail += 1;
                println!("[FAIL] {}: {}", slug, e);
            }
        }
    }

    println!("\nExtraction done. OK={} FAIL={} -> folder: {}", ok, fail, extract_path.display());
    extracted_slugs
}

/// Extract version from plugin's readme.txt or main PHP file
pub fn get_plugin_version(extract_dir: &Path, slug: &str) -> Option<String> {
    let plugin_dir = extract_dir.join(slug);

    // Try readme.txt first (Stable tag: x.x.x)
    let readme_path = plugin_dir.join("readme.txt");
    if readme_path.exists() {
        if let Ok(file) = File::open(&readme_path) {
            let reader = BufReader::new(file);
            let stable_tag_re = Regex::new(r"(?i)^\s*stable\s+tag:\s*(.+)").unwrap();
            for line in reader.lines().take(50) {
                if let Ok(line) = line {
                    if let Some(caps) = stable_tag_re.captures(&line) {
                        let version = caps.get(1).map(|m| m.as_str().trim().to_string());
                        if let Some(v) = version {
                            if !v.is_empty() && v != "trunk" {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }

    // Try main PHP file (Version: x.x.x in plugin header)
    let main_php = plugin_dir.join(format!("{}.php", slug));
    if main_php.exists() {
        if let Ok(file) = File::open(&main_php) {
            let reader = BufReader::new(file);
            let version_re = Regex::new(r"(?i)^\s*\*?\s*version:\s*(.+)").unwrap();
            for line in reader.lines().take(100) {
                if let Ok(line) = line {
                    if let Some(caps) = version_re.captures(&line) {
                        let version = caps.get(1).map(|m| m.as_str().trim().to_string());
                        if let Some(v) = version {
                            if !v.is_empty() {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }

    // Search for any PHP file with plugin header
    if let Ok(entries) = fs::read_dir(&plugin_dir) {
        let version_re = Regex::new(r"(?i)^\s*\*?\s*version:\s*(.+)").unwrap();
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|e| e == "php").unwrap_or(false) {
                if let Ok(file) = File::open(&path) {
                    let reader = BufReader::new(file);
                    for line in reader.lines().take(100) {
                        if let Ok(line) = line {
                            if let Some(caps) = version_re.captures(&line) {
                                let version = caps.get(1).map(|m| m.as_str().trim().to_string());
                                if let Some(v) = version {
                                    if !v.is_empty() {
                                        return Some(v);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
