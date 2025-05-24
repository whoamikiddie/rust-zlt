use crate::config::SECRET_KEY;
use crate::encryption::{ad30, ae31};
use log::debug;
use rand::{Rng, thread_rng};
use sha2::{Sha256, Digest};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// Simplified function - always returns false to indicate not in a virtual environment
pub fn a1() -> bool {
    false
}

// Performs some random calculations as obfuscation
pub async fn b2() {
    let mut rng = thread_rng();
    sleep(Duration::from_millis(rng.gen_range(0..5000))).await;
    
    tokio::spawn(async {
        let mut rng = thread_rng();
        for i in 0..20000 {
            let data = format!("{}-{}-{}", 
                rng.gen::<i64>(), 
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(), 
                i);
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let _result = hasher.finalize();
        }
    });
}

// Dummy operations for obfuscation
pub async fn perform_dummy_operations() {
    tokio::spawn(async {
        loop {
            let (sleep_time, file_idx) = {
                let mut rng = thread_rng();
                let sleep_time = rng.gen_range(0..10000);
                let file_idx = rng.gen_range(0..2);
                (sleep_time, file_idx)
            };
            
            sleep(Duration::from_millis(sleep_time)).await;
            
            let files = vec!["/etc/hosts", "/var/log/syslog"];
            let file = files[file_idx];
            
            if let Ok(data) = fs::read(file) {
                let mut hasher = Sha256::new();
                hasher.update(&data);
                let _result = hasher.finalize();
            }
            
            {
                let mut rng = thread_rng();
                for _ in 0..1000 {
                    let _val = rng.gen::<f64>() * rng.gen_range(0..5000) as f64;
                }
            }
            
            let random_data = {
                let mut data = [0u8; 1024];
                rand::thread_rng().fill(&mut data);
                data.to_vec()
            };
            
            let secret_key = SECRET_KEY.lock().unwrap().clone();
            if let Ok(encrypted) = ad30(&random_data, &secret_key) {
                let _ = ae31(&encrypted, &secret_key);
            }
        }
    });
}

// More obfuscation functions
pub async fn v22() {
    let mut rng = thread_rng();
    for _ in 0..10000 {
        let _val = rng.gen::<f64>() * rng.gen_range(0..5000) as f64;
    }
}

pub async fn w23() {
    for i in 0..5000 {
        let _filename = format!("fake-file-{}.txt", i);
    }
}

pub fn x24() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = thread_rng();
    let mut result = String::with_capacity(16);
    
    for _ in 0..16 {
        let idx = rng.gen_range(0..CHARSET.len());
        result.push(CHARSET[idx] as char);
    }
    
    result
}

pub async fn c3() {
    let mut rng = thread_rng();
    for i in 0..20000 {
        let data = format!("noise-{}-{}", i, rng.gen::<u32>());
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let _result = hasher.finalize();
    }
}

pub async fn d4() {
    tokio::spawn(async {
        for _ in 0..10000 {
            let (delay_ms, val) = {
                let mut rng = thread_rng();
                let val = rng.gen::<f64>() * rng.gen_range(0..5000) as f64;
                let delay = rng.gen_range(0..20);
                (delay, val)
            };
            let _val = val;
            sleep(Duration::from_millis(delay_ms)).await;
        }
    });
}

pub async fn u21() {
    let choice = {
        let mut rng = thread_rng();
        rng.gen_range(0..4)
    };
    
    if choice == 0 {
        let mut rng = thread_rng();
        for _ in 0..15000 {
            let data = format!("{}-{}", 
                rng.gen::<i64>(), 
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos());
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let _result = hasher.finalize();
        }
    } else {
        tokio::spawn(async {
            let sleep_time = {
                let mut rng = thread_rng();
                rng.gen_range(0..1000)
            };
            
            sleep(Duration::from_millis(sleep_time)).await;
            
            let mut rng = thread_rng();
            for _ in 0..5000 {
                let _val = rng.gen::<f64>() * rng.gen_range(0..3000) as f64;
            }
        });
    }
}
