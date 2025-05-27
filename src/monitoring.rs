use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sysinfo::{ComponentExt, CpuExt, DiskExt, NetworkExt, System, SystemExt, RefreshKind, CpuRefreshKind};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;

// Only include libc on Unix-like systems (Linux, macOS)
#[cfg(unix)]
extern crate libc;

// For Windows-specific functionality
#[cfg(windows)]
extern crate winapi;

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
    pub throttling_enabled: bool,
    pub memory_limit_min: u64,
    pub memory_limit_max: u64,
    pub resource_level: u8, // 0=minimal, 1=low, 2=medium, 3=high
    pub last_memory_cleanup: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        // Create a new system with absolute minimal resource usage
        let mut system = System::new_with_specifics(RefreshKind::new()
            .with_memory() // Only refresh memory
            .with_cpu(CpuRefreshKind::new())); // Only refresh CPU
        
        // Set initial stats
        let stats = SystemStats::default();
        
        // Set memory limits (50MB and 200MB in bytes)
        // These are hard limits for the overall application memory usage
        let memory_limit_min = 50 * 1024 * 1024;
        let memory_limit_max = 200 * 1024 * 1024;
        
        // Initialize history queues with minimal capacity to save memory
        let mut cpu_history = VecDeque::with_capacity(15); // Reduced to 15 data points
        let mut memory_history = VecDeque::with_capacity(15); // Reduced to 15 data points
        
        // Pre-fill with zeros for aesthetics, using minimal points
        for _ in 0..10 { // Only pre-fill 10 points
            cpu_history.push_back(0.0);
            memory_history.push_back(0);
        }
        
        SystemMonitor {
            system,
            stats,
            active: AtomicBool::new(true),
            update_interval: Duration::from_secs(5),
            last_update: Instant::now(),
            cpu_history,
            memory_history,
            cycle_count: 0,
            throttling_enabled: true,
            memory_limit_min,
            memory_limit_max,
            resource_level: 1, // Start with low resource mode
            last_memory_cleanup: Instant::now(),
        }
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
        
        // Enforce memory limits with periodic cleanup
        let now = Instant::now();
        if now.duration_since(self.last_memory_cleanup) > Duration::from_secs(30) || memory_used > self.memory_limit_max {
            // Clear caches and request memory release
            self.last_memory_cleanup = now;
            
            // Force OS to reclaim memory - platform-specific implementation
            #[cfg(unix)]
            unsafe { libc::malloc_trim(0); }
            
            // On Windows, use alternative memory cleanup
            #[cfg(windows)]
            {
                // Windows doesn't have malloc_trim, so we'll use a garbage collection approach
                drop(Vec::<u8>::with_capacity(1024 * 1024)); // Allocate and drop a large vector
                std::thread::sleep(Duration::from_millis(10)); // Brief pause to allow reclamation
            }
            
            // Completely recreate system instance with minimal features
            // Instead of dropping, create a new minimal instance
            self.system = System::new_with_specifics(RefreshKind::new()
                .with_memory() // Only refresh memory
                .with_cpu(CpuRefreshKind::new())); // Only refresh CPU
            std::thread::sleep(Duration::from_millis(50)); // Short pause to allow memory reclaim
            
            println!("Memory cleanup performed, current usage: {} bytes", self.system.used_memory());
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
            for (interface_name, network) in self.system.networks() {
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
}

// Data structure to hold monitoring thread state
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

// HTML endpoint for the monitoring dashboard
#[get("/dashboard")]
pub async fn dashboard() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../templates/dashboard.html"))
}
