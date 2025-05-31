use std::process::Command;
use log::{info, error};
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use std::path::Path;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

const GITHUB_REPO: &str = "whoamikiddie/rust-zlt";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn check_for_updates() -> Result<bool, Box<dyn std::error::Error>> {
    info!("Checking for updates...");
    
    let client = reqwest::Client::new();
    let releases_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    let response = client
        .get(&releases_url)
        .header("User-Agent", "rust-zlt-updater")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Ok(false);
    }
    
    let release: Release = response.json().await?;
    let latest_version = release.tag_name.trim_start_matches('v');
    
    info!("Current version: {}, Latest version: {}", CURRENT_VERSION, latest_version);
    
    Ok(version_compare::compare(latest_version, CURRENT_VERSION).unwrap_or(version_compare::Cmp::Lt) == version_compare::Cmp::Gt)
}

pub async fn auto_update_service() {
    loop {
        match check_for_updates().await {
            Ok(true) => {
                info!("New version available. Starting update process...");
                if let Err(e) = perform_update().await {
                    error!("Update failed: {}", e);
                }
            }
            Ok(false) => {
                info!("No updates available");
            }
            Err(e) => {
                error!("Error checking for updates: {}", e);
            }
        }
        
        // Check for updates every 6 hours
        sleep(Duration::from_secs(6 * 60 * 60)).await;
    }
}

async fn perform_update() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let releases_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    
    let release: Release = client
        .get(&releases_url)
        .header("User-Agent", "rust-zlt-updater")
        .send()
        .await?
        .json()
        .await?;
    
    // Get the appropriate asset for the current platform
    let asset = get_platform_asset(&release.assets)?;
    
    // Create temp directory for download
    let temp_dir = tempfile::Builder::new().prefix("zlt-update").tempdir()?;
    let download_path = temp_dir.path().join(&asset.name);
    
    // Download the new version
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await?;
    
    let mut file = tokio::fs::File::create(&download_path).await?;
    let content = response.bytes().await?;
    tokio::io::AsyncWriteExt::write_all(&mut file, &content).await?;
    
    // Get the current executable path
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("old");
    
    // On Unix systems, we need to make the new binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&download_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&download_path, perms)?;
    }
    
    // Replace the current executable
    // First, rename the current executable to .old
    if let Err(e) = tokio::fs::rename(&current_exe, &backup_path).await {
        error!("Failed to create backup: {}", e);
        return Err(e.into());
    }
    
    // Then, move the new executable into place
    if let Err(e) = tokio::fs::rename(&download_path, &current_exe).await {
        // If this fails, try to restore the backup
        if let Err(restore_err) = tokio::fs::rename(&backup_path, &current_exe).await {
            error!("Failed to restore backup after failed update: {}", restore_err);
        }
        return Err(e.into());
    }
    
    info!("Update successful! The application will restart on next launch.");
    
    // Clean up
    if let Err(e) = tokio::fs::remove_file(&backup_path).await {
        error!("Failed to remove backup file: {}", e);
    }
    
    Ok(())
}

fn get_platform_asset(assets: &[Asset]) -> Result<&Asset, Box<dyn std::error::Error>> {
    let platform_suffix = {
        #[cfg(target_os = "windows")]
        { ".exe" }
        #[cfg(target_os = "linux")]
        { "-linux" }
        #[cfg(target_os = "macos")]
        { "-macos" }
    };
    
    assets
        .iter()
        .find(|a| a.name.contains(platform_suffix))
        .ok_or_else(|| "No compatible release found for this platform".into())
} 