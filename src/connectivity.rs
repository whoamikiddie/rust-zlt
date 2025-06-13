use std::time::Duration;
use tokio::time::sleep;
use log::{info, error, warn};
use reqwest::Client;
use crate::notification::{NotificationSystem, TelegramNotifier};
use crate::ngrok::setup_ngrok_tunnel;

const CHECK_INTERVAL: u64 = 10; // Check every 10 seconds
const CONNECTIVITY_TEST_TIMEOUT: u64 = 6; // 5 seconds timeout for connectivity test
const MAX_RETRIES: u32 = 10; // Maximum number of retries for each reconnection attempt

pub struct ConnectivityMonitor {
    port: u16,
    client: Client,
    current_url: Option<String>,
}

impl ConnectivityMonitor {
    pub fn new(port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(CONNECTIVITY_TEST_TIMEOUT))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            port,
            client,
            current_url: None,
        }
    }

    /// Check if internet connection is available
    async fn check_internet_connection(&self) -> bool {
        let test_urls = [
            "https://1.1.1.1",
            "https://8.8.8.8",
            "https://google.com",
        ];

        for url in test_urls.iter() {
            match self.client.get(*url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return true;
                    }
                }
                Err(_) => continue,
            }
        }
        false
    }

    /// Attempt to reconnect ngrok tunnel
    async fn reconnect_ngrok(&mut self) -> Result<String, String> {
        info!("Attempting to reconnect ngrok tunnel...");
        
        for retry in 1..=MAX_RETRIES {
            match setup_ngrok_tunnel(self.port).await {
                url if !url.is_empty() => {
                    info!("Successfully reconnected ngrok tunnel on attempt {}", retry);
                    self.current_url = Some(url.clone());
                    return Ok(url);
                }
                _ => {
                    warn!("Reconnection attempt {} failed, retrying...", retry);
                    sleep(Duration::from_secs(retry as u64 * 2)).await;
                }
            }
        }
        
        Err("Failed to reconnect ngrok tunnel after maximum retries".to_string())
    }

    /// Send reconnection notification
    async fn send_reconnection_notification(&self, new_url: &str) {
        let notifier = TelegramNotifier;
        let message = format!("🔄 Connection Restored\n🌐 New URL: {}", new_url);
        
        if let Err(e) = notifier.send_notification(&message).await {
            error!("Failed to send reconnection notification: {}", e);
        }
    }

    /// Start monitoring connectivity and handle reconnection
    pub async fn start_monitoring(&mut self) {
        info!("Starting connectivity monitoring...");
        let mut was_disconnected = false;

        loop {
            let is_connected = self.check_internet_connection().await;

            match (is_connected, was_disconnected) {
                (true, true) => {
                    // Connection restored after disconnection
                    info!("Internet connection restored, attempting to reconnect ngrok...");
                    match self.reconnect_ngrok().await {
                        Ok(new_url) => {
                            self.send_reconnection_notification(&new_url).await;
                            was_disconnected = false;
                        }
                        Err(e) => {
                            error!("Failed to reconnect: {}", e);
                        }
                    }
                }
                (false, false) => {
                    // Just lost connection
                    warn!("Internet connection lost, waiting for restoration...");
                    was_disconnected = true;
                }
                (true, false) => {
                    // Still connected, verify ngrok tunnel
                    if let Some(url) = &self.current_url {
                        if let Err(_) = self.client.get(url).send().await {
                            warn!("Ngrok tunnel appears to be down, attempting to reconnect...");
                            if let Ok(new_url) = self.reconnect_ngrok().await {
                                self.send_reconnection_notification(&new_url).await;
                            }
                        }
                    }
                }
                (false, true) => {
                    // Still disconnected
                    warn!("Internet connection still unavailable...");
                }
            }

            sleep(Duration::from_secs(CHECK_INTERVAL)).await;
        }
    }
} 