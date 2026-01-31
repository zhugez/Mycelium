mod error;
mod models;
mod api;
mod fs_ops;
mod csv_ops;

use std::path::Path;
use clap::Parser;
use tokio::fs;

use crate::error::AppError;
use crate::models::{Plugin, DownloadResult};
use crate::api::fetch_popular_parallel;
use crate::fs_ops::{download_one, extract_all_zips};
use crate::csv_ops::{save_plugins_to_csv, save_plugins_csv_with_versions};

#[derive(Parser, Debug)]
#[command(name = "mycelium")]
#[command(about = "Download and analyze popular WordPress plugins from wordpress.org")]
struct Args {
    /// Minimum active installs filter
    #[arg(short = 'm', long, default_value_t = 10_000)]
    min_active: i64,

    /// Number of pages to fetch
    #[arg(short = 'p', long, default_value_t = 50)]
    pages: i32,

    /// Plugins per page
    #[arg(long, default_value_t = 100)]
    per_page: i32,

    /// Output directory for downloaded zips
    #[arg(short = 'o', long, default_value = "wp_zips")]
    output_dir: String,

    /// Number of parallel download workers
    #[arg(short = 'w', long, default_value_t = 5)]
    workers: usize,

    /// Output CSV file path
    #[arg(short = 'c', long, default_value = "plugins.csv")]
    csv_path: String,

    /// Skip downloading, only list plugins
    #[arg(long, default_value_t = false)]
    list_only: bool,

    /// Extract all zips after download
    #[arg(short = 'e', long, default_value_t = false)]
    extract: bool,

    /// Directory for extracted plugins
    #[arg(long, default_value = "wp_extracted")]
    extract_dir: String,
}

async fn download_plugins(
    plugins: &[Plugin],
    out_dir: &str,
    download_workers: usize,
) -> Vec<DownloadResult> {
    let out_path = Path::new(out_dir);
    fs::create_dir_all(out_path).await.ok();

    let slugs: Vec<String> = plugins.iter().map(|p| p.slug.clone()).collect();
    let mut ok = 0;
    let mut fail = 0;
    let mut results = Vec::new();

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(download_workers));
    let mut handles = Vec::new();

    for slug in slugs {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let out_path = out_path.to_path_buf();

        let handle = tokio::spawn(async move {
            let result = download_one(&slug, &out_path).await;
            drop(permit);
            result
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(result) => {
                results.push(result.clone());
                if result.success {
                    ok += 1;
                    println!("[OK] {}: {}", result.slug, result.message);
                } else {
                    fail += 1;
                    println!("[FAIL] {}: {}", result.slug, result.message);
                }
            }
            Err(e) => {
                fail += 1;
                eprintln!("Task error: {:?}", e);
            }
        }
    }

    println!("\nDownload done. OK={} FAIL={} -> folder: {}", ok, fail, out_path.display());
    results
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    println!("Fetching popular plugins (min_active={}, pages={}, per_page={}...",
        args.min_active, args.pages, args.per_page);
    
    let plugins = fetch_popular_parallel(args.min_active, args.pages, args.per_page).await?;

    println!("Unique plugins: {}", plugins.len());
    for p in plugins.iter().take(30) {
        println!("{} {} - {} (page {})", p.active_installs, p.slug, p.name, p.page);
    }

    if !args.list_only {
        println!("\n--- Downloading zips (stable) ---");
        let results = download_plugins(&plugins, &args.output_dir, args.workers).await;

        // Save initial CSV status
        let _ = save_plugins_to_csv(&plugins, &results, &args.csv_path).await;

        if args.extract {
            let _extracted = extract_all_zips(&args.output_dir, &args.extract_dir);
            let extract_path = Path::new(&args.extract_dir);
            let _ = save_plugins_csv_with_versions(&plugins, &results, extract_path, &args.csv_path);
        }
    }

    Ok(())
}