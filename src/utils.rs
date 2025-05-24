use std::cmp::Ordering;

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
