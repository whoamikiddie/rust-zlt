use std::net::{TcpListener, SocketAddr};
use log::info;

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
