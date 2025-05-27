use std::cmp::Ordering;
use std::net::{TcpListener, SocketAddr};
use log::info;

// Min function (equivalent to Go's min function)
pub fn min<T: Ord>(a: T, b: T) -> T {
    match a.cmp(&b) {
        Ordering::Less | Ordering::Equal => a,
        Ordering::Greater => b,
    }
}

// Function to check if a string is a valid URL
pub fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

// Platform-specific path handling
pub fn platform_path(path: &str) -> String {
    #[cfg(windows)]
    {
        path.replace("/", "\\")
    }
    
    #[cfg(not(windows))]
    {
        path.to_string()
    }
}

// Check if a path is within bounds (security check)
pub fn is_path_safe(base: &str, path: &str) -> bool {
    use path_absolutize::Absolutize;
    use std::path::Path;
    
    let base_path = Path::new(base);
    let target_path = Path::new(path);
    
    if let (Ok(abs_base), Ok(abs_target)) = (base_path.absolutize(), target_path.absolutize()) {
        abs_target.starts_with(abs_base)
    } else {
        false
    }
}

// Find an available port, starting from the preferred port and incrementing if not available
pub fn find_available_port(preferred_port: u16) -> u16 {
    let mut port = preferred_port;
    let max_attempts = 100; // Try up to 100 ports after the preferred one
    
    for attempt in 0..max_attempts {
        // Try to bind to the current port
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        match TcpListener::bind(addr) {
            Ok(_) => {
                // We found an available port
                if attempt > 0 {
                    info!("Port {} was busy, using port {} instead", preferred_port, port);
                }
                return port;
            },
            Err(_) => {
                // Port is not available, try the next one
                port = port + 1;
            }
        }
    }
    
    // If all attempts failed, return a random high port
    let random_port = 10000 + (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 50000) as u16;
    info!("Could not find available port after {} attempts, using random port {}", max_attempts, random_port);
    random_port
}
