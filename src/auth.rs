use actix_web::{
    dev::ServiceRequest,
    error::ErrorUnauthorized,
    Error,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    username: String,
    password_hash: String,
}

#[derive(Debug, Clone)]
pub struct AuthState {
    users: Arc<RwLock<Vec<User>>>,
}

impl AuthState {
    pub fn new() -> Self {
        let mut users = Vec::new();
        // Add default admin user
        users.push(User {
            username: "admin".to_string(),
            password_hash: hash_password("admin"),
        });
        
        AuthState {
            users: Arc::new(RwLock::new(users)),
        }
    }

    pub async fn validate_user(&self, username: &str, password: &str) -> bool {
        let users = self.users.read().await;
        let password_hash = hash_password(password);
        
        users.iter().any(|user| {
            user.username == username && user.password_hash == password_hash
        })
    }
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    STANDARD.encode(hasher.finalize())
}

pub async fn check_auth(req: ServiceRequest) -> Result<ServiceRequest, Error> {
    // Skip auth for login page and login POST
    if req.path() == "/login" {
        return Ok(req);
    }

    // Check session cookie
    if let Some(cookie) = req.cookie("zlt_session") {
        if cookie.value() == "authenticated" {
            return Ok(req);
        }
    }

    // If no valid session, return unauthorized
    Err(ErrorUnauthorized("Unauthorized"))
} 