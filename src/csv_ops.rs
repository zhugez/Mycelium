use csv::Writer;
use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::models::{Plugin, DownloadResult};
use crate::fs_ops::get_plugin_version;

pub async fn save_plugins_to_csv(
    plugins: &[Plugin],
    results: &[DownloadResult],
    csv_path: &str,
) -> Result<(), io::Error> {
    let mut wtr = Writer::from_path(csv_path)?;
    wtr.write_record(["name", "version", "slug", "status"])?;

    let results_map: HashMap<&String, &DownloadResult> = results
        .iter()
        .filter(|r| r.success)
        .map(|r| (&r.slug, r))
        .collect();

    for p in plugins {
        let status = if results_map.contains_key(&p.slug) {
            format!("downloaded")
        } else {
            "not downloaded".to_string()
        };
        wtr.write_record([
            &p.name,
            "",
            &p.slug,
            &status,
        ])?;
    }

    wtr.flush()?;
    println!("Saved plugin info to {}", csv_path);
    Ok(())
}

pub fn save_plugins_csv_with_versions(
    plugins: &[Plugin],
    results: &[DownloadResult],
    extract_dir: &Path,
    csv_path: &str,
) -> Result<(), io::Error> {
    let mut wtr = Writer::from_path(csv_path)?;
    wtr.write_record(["name", "version", "slug", "status"])?;

    let results_map: HashMap<&String, &DownloadResult> = results
        .iter()
        .filter(|r| r.success)
        .map(|r| (&r.slug, r))
        .collect();

    for p in plugins {
        let (status, version) = if results_map.contains_key(&p.slug) {
            let ver = get_plugin_version(extract_dir, &p.slug)
                .unwrap_or_default();
            ("extracted".to_string(), ver)
        } else {
            ("not downloaded".to_string(), String::new())
        };

        wtr.write_record([
            &p.name,
            &version,
            &p.slug,
            &status,
        ])?;
    }

    wtr.flush()?;
    println!("Saved plugin info with versions to {}", csv_path);
    Ok(())
}
