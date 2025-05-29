use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sysinfo::{ComponentExt, CpuExt, DiskExt, NetworkExt, System, SystemExt, RefreshKind, CpuRefreshKind};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::alloc::GlobalAlloc;

// Only include libc on Unix-like systems (Linux, macOS)
#[cfg(unix)]
extern crate libc;

// For Windows-specific functionality
#[cfg(windows)]
extern crate winapi;

// Define consistent memory size constants
const MB: u64 = 1024 * 1024;
const DEFAULT_MIN_MEMORY: u64 = 50 * MB;
const DEFAULT_MAX_MEMORY: u64 = 200 * MB;

// Structure to hold system stats (minimal version to save memory)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemStats {
    timestamp: u64,
    cpu_usage: f32,
    memory_total: u64,
    memory_used: u64,
    memory_usage_percent: f32,
    disk_total: u64,
    disk_free: u64,
    disk_usage_percent: f32,
    network_received: u64,
    network_transmitted: u64,
    uptime: u64,
    system_load: Vec<f64>,
    cpu_temp: Option<f32>,
    processes_count: usize,
    hostname: String,
    // Flag to indicate if memory consumption is within bounds
    memory_within_bounds: bool,
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            cpu_usage: 0.0,
            memory_total: 0,
            memory_used: 0,
            memory_usage_percent: 0.0,
            disk_total: 0,
            disk_free: 0,
            disk_usage_percent: 0.0,
            network_received: 0,
            network_transmitted: 0,
            uptime: 0,
            system_load: vec![0.0, 0.0, 0.0],
            cpu_temp: None,
            processes_count: 0,
            hostname: String::new(),
            memory_within_bounds: true,
        }
    }
}

// Structure to wrap system monitoring and its stats
pub struct SystemMonitor {
    pub system: System,
    pub stats: SystemStats,
    pub active: AtomicBool,
    pub update_interval: Duration,
    pub last_update: Instant,
    pub cpu_history: VecDeque<f32>,
    pub memory_history: VecDeque<u64>,
    pub cycle_count: u64,
    pub memory_limit_min: u64,
    pub memory_limit_max: u64,
    pub resource_level: u8, // 0=minimal, 1=low, 2=medium, 3=high
    pub last_memory_cleanup: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        // Create system monitor with minimal refresh settings to reduce resource usage
        let mut system = System::new_with_specifics(RefreshKind::new()
            .with_memory()
            .with_cpu(CpuRefreshKind::new()));
        
        // Initial refresh to populate system data
        system.refresh_memory();
        system.refresh_cpu();
        
        // Configure history sizes based on performance/memory tradeoff
        // Smaller sizes = less memory usage, fewer data points for graphs
        let cpu_history_size = 60;    // 1 minute at 1 sec refresh
        let memory_history_size = 30; // 30 seconds at 1 sec refresh
        
        // Pre-allocate VecDeques with capacity to avoid reallocation
        let mut cpu_history = VecDeque::with_capacity(cpu_history_size);
        let mut memory_history = VecDeque::with_capacity(memory_history_size);
        
        // Initialize history with zeros (more efficient with extend)
        cpu_history.extend(std::iter::repeat(0.0).take(cpu_history_size));
        memory_history.extend(std::iter::repeat(0).take(memory_history_size));
        
        SystemMonitor {
            system,
            stats: SystemStats::default(),
            active: AtomicBool::new(true),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(1),
            memory_limit_min: DEFAULT_MIN_MEMORY,
            memory_limit_max: DEFAULT_MAX_MEMORY,
            last_memory_cleanup: Instant::now(),
            cpu_history,
            memory_history,
            resource_level: 2, // Default to medium resources
            cycle_count: 0,
        }
    }
    
    /// Platform-specific memory cleanup operation
    fn perform_memory_cleanup(&self) {
        // Unix-specific memory cleanup
        #[cfg(unix)]
        unsafe {
            // Call malloc_trim to release memory back to the OS
            libc::malloc_trim(0);
            
            // On Linux, we can also try to write to /proc/self/oom_score_adj
            // to adjust our OOM killer score (lower priority for killing)
            #[cfg(target_os = "linux")]
            if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open("/proc/self/oom_score_adj") {
                use std::io::Write;
                let _ = file.write_all(b"-500\n"); // Lower priority (-1000 to 1000 range)
            }
        }
        
        // Windows-specific memory cleanup
        #[cfg(windows)]
        {
            // Windows doesn't have malloc_trim, so use multiple approaches
            
            // 1. Force garbage collection by allocating and dropping memory
            drop(Vec::<u8>::with_capacity(4 * MB as usize));
            
            // 2. Request heap compaction
            unsafe {
                use winapi::um::heapapi::{GetProcessHeap, HeapCompact};
                let heap = GetProcessHeap();
                if !heap.is_null() {
                    HeapCompact(heap, 0);
                }
            }
            
            // 3. Allow time for Windows memory manager to reclaim
            std::thread::sleep(Duration::from_millis(15));
        }
        
        // Cross-platform memory cleanup techniques
        unsafe {
            let layout = std::alloc::Layout::new::<u8>();
            let alloc = std::alloc::System.alloc(layout);
            std::alloc::System.dealloc(alloc, layout);
        }
        
        // Use drop to encourage cleanup
        drop(String::with_capacity(1024));
    }
    
    pub fn refresh(&mut self) -> SystemStats {
        // Only refresh if monitoring is active
        if !self.active.load(Ordering::Relaxed) {
            return self.stats.clone();
        }
        
        // Only refresh if enough time has passed since last update
        let now = Instant::now();
        if now.duration_since(self.last_update) < self.update_interval {
            return self.stats.clone();
        }
        
        // Selectively refresh only what we need to save resources
        self.system.refresh_cpu();
        self.system.refresh_memory();
        
        // Only periodically refresh these (every 3 cycles) to save resources
        static mut REFRESH_COUNTER: u8 = 0;
        unsafe {
            REFRESH_COUNTER = (REFRESH_COUNTER + 1) % 3;
            if REFRESH_COUNTER == 0 {
                self.system.refresh_disks_list();
                self.system.refresh_disks();
                self.system.refresh_networks_list();
                self.system.refresh_networks();
                // Only refresh components if we actually use CPU temp
                self.system.refresh_components_list();
                self.system.refresh_components();
            }
        }
        
        // Only refresh CPU stats based on resource level
        let cpu_usage = if self.resource_level >= 1 {
            self.system.refresh_cpu();
            self.system.global_cpu_info().cpu_usage()
        } else {
            // In minimal mode, just use the last value
            self.stats.cpu_usage
        };
        
        // Update CPU history
        self.cpu_history.pop_front();
        self.cpu_history.push_back(cpu_usage);
        
        // Check current memory usage to see if we need to reduce resource usage
        self.system.refresh_memory();
        let memory_total = self.system.total_memory();
        let memory_used = self.system.used_memory();
        
        // Adjust resource level based on memory usage
        if memory_used > self.memory_limit_max {
            // Severe memory pressure - go to minimal mode
            self.resource_level = 0;
            println!("SEVERE: Memory usage too high ({} bytes), switching to minimal resources", memory_used);
        } else if memory_used > (self.memory_limit_max * 80 / 100) {
            // High memory pressure - go to low mode
            self.resource_level = 1;
            println!("WARNING: Memory usage high ({} bytes), switching to low resources", memory_used);
        } else if memory_used < self.memory_limit_min {
            // Memory usage too low - can use more resources
            self.resource_level = 2;
        } else {
            // Normal memory usage - use medium resources
            self.resource_level = 2;
        }
        
        // Enforce memory limits with periodic cleanup - more sophisticated approach
        let now = Instant::now();
        let cleanup_interval = if memory_used > self.memory_limit_max {
            // More frequent cleanup under memory pressure
            Duration::from_secs(10)
        } else {
            Duration::from_secs(30)
        };
        
        if now.duration_since(self.last_memory_cleanup) > cleanup_interval || memory_used > self.memory_limit_max {
            // Record cleanup time first to prevent double cleanup if this takes time
            self.last_memory_cleanup = now;
            let pre_cleanup_memory = self.system.used_memory();
            
            // Platform-specific memory management with improved techniques
            self.perform_memory_cleanup();
            
            // Recreate system monitor with appropriate refresh kind based on resource level
            let refresh_kind = match self.resource_level {
                0 => RefreshKind::new().with_memory(), // Minimal - memory only
                1 => RefreshKind::new().with_memory().with_cpu(CpuRefreshKind::new()), // Low
                _ => RefreshKind::new().with_memory().with_cpu(CpuRefreshKind::new()) // Medium and above
            };
            
            self.system = System::new_with_specifics(refresh_kind);
            
            // Allow time for memory to be reclaimed
            std::thread::sleep(Duration::from_millis(50));
            
            // Calculate memory savings
            let post_cleanup_memory = self.system.used_memory();
            let memory_saved = pre_cleanup_memory.saturating_sub(post_cleanup_memory);
            
            // Only log if significant memory was saved or we're over limits
            if memory_saved > MB || memory_used > self.memory_limit_max {
                println!("Memory cleanup performed: {} bytes freed, current usage: {} bytes", 
                          memory_saved, post_cleanup_memory);
            }
        }
        
        let memory_usage_percent = (memory_used as f32 / memory_total as f32) * 100.0;
        
        // Update memory history
        self.memory_history.pop_front();
        self.memory_history.push_back(memory_used);
        
        // Refresh disk stats based on resource level - less frequent in lower modes
        let (disk_total, disk_free, disk_usage_percent) = if 
            (self.resource_level >= 2 && self.cycle_count % 5 == 0) || // Medium+ mode: every 5 cycles
            (self.resource_level == 1 && self.cycle_count % 10 == 0) // Low mode: every 10 cycles
        {
            self.system.refresh_disks_list();
            let mut total = 0;
            let mut free = 0;
            
            // Only get the first few disks in low resource mode
            let disk_limit = if self.resource_level >= 2 { usize::MAX } else { 2 };
            
            for (i, disk) in self.system.disks().iter().enumerate() {
                if i >= disk_limit { break; }
                total += disk.total_space();
                free += disk.available_space();
            }
            
            let used = total.saturating_sub(free);
            let percent = if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 };
            
            (total, free, percent)
        } else {
            // Use cached values from last refresh
            (self.stats.disk_total, self.stats.disk_free, self.stats.disk_usage_percent)
        };
        
        // Network stats - very expensive, only refresh in higher resource modes
        let (network_received, network_transmitted) = if 
            (self.resource_level >= 3 && self.cycle_count % 3 == 0) || // High mode: every 3 cycles
            (self.resource_level == 2 && self.cycle_count % 8 == 0)   // Medium mode: every 8 cycles
        {
            self.system.refresh_networks_list();
            let mut received = 0;
            let mut transmitted = 0;
            
            // Only get first few networks to save memory
            let net_limit = if self.resource_level >= 3 { usize::MAX } else { 2 };
            
            let mut i = 0;
            for (_interface_name, network) in self.system.networks() {
                if i >= net_limit { break; }
                received += network.received();
                transmitted += network.transmitted();
                i += 1;
            }
            
            (received, transmitted)
        } else {
            // Use cached values
            (self.stats.network_received, self.stats.network_transmitted)
        };
        
        // System uptime - low cost so refresh every time
        let uptime = self.system.uptime();
        
        // Load average - low cost so refresh every time
        let system_load = {
            let load_avg = self.system.load_average();
            vec![load_avg.one, load_avg.five, load_avg.fifteen]
        };
        
        // CPU temperature - only get if we refreshed components this cycle
        // CPU temperature - only in higher resource modes
        let cpu_temp = if self.resource_level >= 2 && self.cycle_count % 10 == 0 {
            self.system.refresh_components();
            let mut max_temp: f32 = 0.0;
            
            for component in self.system.components() {
                if component.label().contains("CPU") {
                    max_temp = max_temp.max(component.temperature());
                    break; // Only get first CPU temp to save resources
                }
            }
            
            Some(max_temp)
        } else {
            // Use cached value
            self.stats.cpu_temp
        };
            
        // Process count - only refresh periodically to save resources
        let processes_count = unsafe {
            if REFRESH_COUNTER == 0 {
                self.system.refresh_processes();
                self.system.processes().len()
            } else {
                self.stats.processes_count
            }
        };
        
        // Hostname - only need to get this once since it rarely changes
        let hostname = if self.stats.hostname.is_empty() {
            self.system.host_name().unwrap_or_else(|| String::from("Unknown"))
        } else {
            self.stats.hostname.clone()
        };
        
        // Refresh stats
        self.stats = SystemStats {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            cpu_usage,
            memory_total,
            memory_used,
            memory_usage_percent,
            disk_total,
            disk_free,
            disk_usage_percent,
            network_received,
            network_transmitted,
            uptime,
            system_load,
            cpu_temp,
            processes_count,
            hostname,
            memory_within_bounds: memory_used >= self.memory_limit_min && memory_used <= self.memory_limit_max,
        };
        
        self.last_update = now;
        self.cycle_count += 1;
        self.stats.clone()
    }
    
    pub fn get_stats(&mut self) -> SystemStats {
        self.refresh()
    }

    pub fn get_detailed_memory_status(&self) -> String {
        let memory_used = self.system.used_memory();
        let memory_total = self.system.total_memory();
        let memory_percentage = (memory_used as f64 / memory_total as f64) * 100.0;
        
        format!(
            "Memory Status:\n\
             Used: {:.2} MB\n\
             Total: {:.2} MB\n\
             Usage: {:.1}%\n\
             Resource Level: {}\n\
             Within Bounds: {}\n\
             Min Limit: {:.2} MB\n\
             Max Limit: {:.2} MB",
            memory_used as f64 / MB as f64,
            memory_total as f64 / MB as f64,
            memory_percentage,
            self.resource_level,
            memory_used >= self.memory_limit_min && memory_used <= self.memory_limit_max,
            self.memory_limit_min as f64 / MB as f64,
            self.memory_limit_max as f64 / MB as f64
        )
    }
}

// Data structure to hold monitoring thread state
#[allow(dead_code)]
pub struct MonitoringData {
    pub monitor: Arc<Mutex<SystemMonitor>>,
    pub active: Arc<AtomicBool>,
    pub handle: Option<thread::JoinHandle<()>>,
}

// Initialize system monitoring
pub fn init_monitoring() -> MonitoringData {
    let monitor = Arc::new(Mutex::new(SystemMonitor::new()));
    let monitor_clone = Arc::clone(&monitor);
    let active = Arc::new(AtomicBool::new(true));
    let active_clone = Arc::clone(&active);
    
    // Start a background thread for monitoring
    let handle = thread::spawn(move || {
        let mut sleep_duration = Duration::from_secs(5);
        
        // Smaller initial delay to start faster
        thread::sleep(Duration::from_millis(500));
        
        // Keep monitoring until stopped
        while active_clone.load(Ordering::Relaxed) {
            // Sleep first to allow the system to start up
            thread::sleep(sleep_duration);
            
            let refresh_success = if let Ok(mut lock) = monitor_clone.lock() {
                // Track memory usage before refresh
                let _memory_before = lock.stats.memory_used;
                
                // Refresh stats
                let stats = lock.refresh();
                
                // Extremely aggressive memory control
                if stats.memory_used > lock.memory_limit_max {
                    // Immediate action if we're above 200MB
                    println!("CRITICAL: Memory usage ({} bytes) exceeds limit - taking emergency action", stats.memory_used);
                    
                    // 1. Force immediate memory release - extremely aggressive
                    // Replace system with minimal instance rather than dropping
                    lock.system = System::new_with_specifics(RefreshKind::new()
                        .with_memory() // Only refresh memory
                        .with_cpu(CpuRefreshKind::new())); // Only refresh CPU
                    std::thread::sleep(Duration::from_millis(100)); // Give OS time to reclaim memory
                    // Platform-specific memory management
                    #[cfg(unix)]
                    unsafe { libc::malloc_trim(0); } // Force OS-level memory trim on Unix
                    
                    #[cfg(windows)]
                    {
                        // Alternative memory cleanup on Windows
                        drop(Vec::<u8>::with_capacity(1024 * 1024)); // Allocate and drop a large vector
                        std::thread::sleep(Duration::from_millis(10)); // Brief pause to allow reclamation
                    }
                    
                    // 2. Set very conservative refresh policy
                    sleep_duration = Duration::from_secs(20); // Very infrequent updates
                    lock.resource_level = 0; // Minimal resources mode
                    
                    println!("Target memory: {}MB to {}MB", 
                             lock.memory_limit_min / (1024 * 1024),
                             lock.memory_limit_max / (1024 * 1024));
                } 
                else if stats.memory_used > (lock.memory_limit_max * 90 / 100) {
                    // Memory approaching limit - proactive action
                    println!("WARNING: Memory usage approaching limit: {} bytes", stats.memory_used);
                    sleep_duration = Duration::from_secs(10);
                    lock.resource_level = 1; // Low resources mode
                    
                    // Release memory proactively
                    unsafe { libc::malloc_trim(0); }
                }
                else if stats.memory_used < lock.memory_limit_min {
                    // Memory too low - can use more resources
                    sleep_duration = Duration::from_secs(3);
                    lock.resource_level = 2; // Medium resources
                }
                else if stats.memory_used < (lock.memory_limit_min + (lock.memory_limit_max - lock.memory_limit_min) / 2) {
                    // In the lower half of our target range
                    sleep_duration = Duration::from_secs(5);
                    lock.resource_level = 2; // Medium resources
                }
                else {
                    // In the upper half of our target range
                    sleep_duration = Duration::from_secs(8);
                    lock.resource_level = 1; // Low resources
                }
                
                // Log if we're in range now
                if stats.memory_within_bounds && !lock.stats.memory_within_bounds {
                    println!("SUCCESS: Memory now within target range ({} bytes)", stats.memory_used);
                }
                
                true
            } else {
                false
            };
            
            // If we failed to get the lock, use a shorter sleep
            if !refresh_success {
                sleep_duration = Duration::from_secs(1);
            }
            
            // Sleep for the determined duration
            thread::sleep(sleep_duration);
        }
    });
    
    MonitoringData {
        monitor,
        active,
        handle: Some(handle),
    }
}

// API endpoint to get system stats
#[get("/api/system-stats")]
pub async fn get_system_stats(monitor: web::Data<MonitoringData>) -> impl Responder {
    let stats = if let Ok(mut monitor_guard) = monitor.monitor.lock() {
        monitor_guard.get_stats()
    } else {
        SystemStats::default()
    };
    
    HttpResponse::Ok().json(stats)
}

// Dashboard state cache to improve performance
static DASHBOARD_LAST_MODIFIED: Mutex<SystemTime> = Mutex::new(SystemTime::UNIX_EPOCH);

/// HTML endpoint for the monitoring dashboard with improved performance
#[get("/dashboard")]
pub async fn dashboard() -> impl Responder {
    // Update the last modified time for caching
    {
        let mut last_modified = DASHBOARD_LAST_MODIFIED.lock().unwrap();
        *last_modified = SystemTime::now();
    }
    
    // The correct way to build a response in actix-web
    let mut builder = HttpResponse::Ok();
    builder.content_type("text/html; charset=utf-8");
    builder.insert_header(("Cache-Control", "max-age=10"));
    builder.insert_header(("X-Content-Type-Options", "nosniff"));
    
    // Set platform-specific optimization hints
    #[cfg(windows)]
    {
        builder.insert_header(("X-UA-Compatible", "IE=edge"));
    }
    
    // Serve the dashboard HTML
    builder.body(include_str!("../templates/dashboard.html"))
}

// Add a new endpoint for memory status
#[get("/api/memory-status")]
pub async fn get_memory_status(monitor: web::Data<MonitoringData>) -> impl Responder {
    if let Ok(monitor_guard) = monitor.monitor.lock() {
        let status = monitor_guard.get_detailed_memory_status();
        HttpResponse::Ok().content_type("text/plain").body(status)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}
