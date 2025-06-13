use std::net::{TcpListener, SocketAddr, IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use log::{info, warn, error};
use thiserror::Error;
use rand::Rng;

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("File system error: {0}")]
    FileSystemError(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Port range exhausted")]
    PortRangeExhausted,
}

/// Find an available port, starting from the preferred port and incrementing if not available
/// 
/// # Arguments
/// * `preferred_port` - The initial port to try
/// * `max_attempts` - Maximum number of ports to try after the preferred port
/// 
/// # Returns
/// * `Result<u16, UtilsError>` - The available port or an error
pub fn find_available_port(preferred_port: u16, max_attempts: Option<u16>) -> Result<u16, UtilsError> {
    let mut port = preferred_port;
    let max_attempts = max_attempts.unwrap_or(100);
    
    for attempt in 0..max_attempts {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
        match TcpListener::bind(addr) {
            Ok(_) => {
                if attempt > 0 {
                    info!("Port {} was busy, using port {} instead", preferred_port, port);
                }
                return Ok(port);
            },
            Err(e) => {
                warn!("Port {} is not available: {}", port, e);
                port = port.wrapping_add(1);
            }
        }
    }
    
    // If all attempts failed, try a random high port
    let random_port = generate_random_port(49152, 65535)?;
    info!("Could not find available port after {} attempts, using random port {}", max_attempts, random_port);
    Ok(random_port)
}

/// Generate a random port within a specified range
/// 
/// # Arguments
/// * `min` - Minimum port number (inclusive)
/// * `max` - Maximum port number (inclusive)
/// 
/// # Returns
/// * `Result<u16, UtilsError>` - A random port or an error
pub fn generate_random_port(min: u16, max: u16) -> Result<u16, UtilsError> {
    if min >= max {
        return Err(UtilsError::NetworkError("Invalid port range".to_string()));
    }
    
    let mut rng = rand::thread_rng();
    let port = rng.gen_range(min..=max);
    
    // Verify the port is available
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    match TcpListener::bind(addr) {
        Ok(_) => Ok(port),
        Err(e) => Err(UtilsError::NetworkError(format!("Random port {} is not available: {}", port, e))),
    }
}

/// Get the current timestamp in seconds since Unix epoch
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Create a directory if it doesn't exist
/// 
/// # Arguments
/// * `path` - Path to create
/// 
/// # Returns
/// * `Result<(), UtilsError>` - Success or error
pub fn ensure_directory_exists(path: &Path) -> Result<(), UtilsError> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| UtilsError::FileSystemError(format!("Failed to create directory: {}", e)))?;
    }
    Ok(())
}

/// Get the size of a file or directory in bytes
/// 
/// # Arguments
/// * `path` - Path to check
/// 
/// # Returns
/// * `Result<u64, UtilsError>` - Size in bytes or error
pub fn get_path_size(path: &Path) -> Result<u64, UtilsError> {
    if !path.exists() {
        return Err(UtilsError::InvalidPath(format!("Path does not exist: {:?}", path)));
    }

    if path.is_file() {
        fs::metadata(path)
            .map(|m| m.len())
            .map_err(|e| UtilsError::FileSystemError(format!("Failed to get file size: {}", e)))
    } else if path.is_dir() {
        let mut size = 0;
        for entry in fs::read_dir(path)
            .map_err(|e| UtilsError::FileSystemError(format!("Failed to read directory: {}", e)))? {
            let entry = entry
                .map_err(|e| UtilsError::FileSystemError(format!("Failed to read directory entry: {}", e)))?;
            size += get_path_size(&entry.path())?;
        }
        Ok(size)
    } else {
        Err(UtilsError::InvalidPath(format!("Path is neither file nor directory: {:?}", path)))
    }
}

/// Format bytes into a human-readable string
/// 
/// # Arguments
/// * `bytes` - Number of bytes
/// 
/// # Returns
/// * `String` - Formatted size string (e.g., "1.5 MB")
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Get the absolute path of a file or directory
/// 
/// # Arguments
/// * `path` - Path to resolve
/// 
/// # Returns
/// * `Result<PathBuf, UtilsError>` - Absolute path or error
pub fn get_absolute_path(path: &Path) -> Result<PathBuf, UtilsError> {
    path.canonicalize()
        .map_err(|e| UtilsError::FileSystemError(format!("Failed to get absolute path: {}", e)))
}

/// Check if a path is within a base directory (for security)
/// 
/// # Arguments
/// * `path` - Path to check
/// * `base` - Base directory
/// 
/// # Returns
/// * `Result<bool, UtilsError>` - True if path is within base directory
pub fn is_path_within_base(path: &Path, base: &Path) -> Result<bool, UtilsError> {
    let abs_path = get_absolute_path(path)?;
    let abs_base = get_absolute_path(base)?;
    
    Ok(abs_path.starts_with(&abs_base))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_find_available_port() {
        let port = find_available_port(8080, Some(10)).unwrap();
        assert!(port >= 8080 && port < 8090);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_path_operations() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        File::create(&test_file).unwrap();
        
        assert!(ensure_directory_exists(temp_dir.path()).is_ok());
        assert!(get_path_size(&test_file).unwrap() == 0);
        assert!(is_path_within_base(&test_file, temp_dir.path()).unwrap());
    }
}
