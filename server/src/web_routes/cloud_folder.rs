use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, Json, Response},
};
use serde_json::json;
use std::{fs, sync::Arc};

use crate::{
    auth::AuthState,
    cloud::{Cloud, CloudServerState},
};

// Helper function to get authentication state from server state
fn get_authentication_state(server_state: &CloudServerState) -> &Arc<AuthState> {
    &server_state.auth_state
}

// Checks if request has valid authentication token
fn has_valid_token(headers: &HeaderMap, auth_state: &AuthState) -> bool {
    // Check Authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if auth_state.verify_token(token).is_ok() {
                    return true;
                }
            }
        }
    }

    // Check cookies
    if let Some(cookie_header) = headers.get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("auth_token_") {
                    if let Some(token) = cookie.split('=').nth(1) {
                        if auth_state.verify_token(token).is_ok() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

// Verifies authentication and returns HTML redirect if unauthorized
fn verify_authentication(headers: &HeaderMap, auth_state: &AuthState) -> Result<(), Html<String>> {
    if has_valid_token(headers, auth_state) {
        return Ok(());
    }

    // Return HTML redirect to login page
    let redirect_html = r#"
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=/login">
    <title>Redirecting to Login</title>
</head>
<body>
    <p>Redirecting to login page...</p>
    <script>window.location.href = '/login';</script>
</body>
</html>
    "#;

    Err(Html(redirect_html.to_string()))
}

pub async fn show_cloud_folder_info(
    State(server_state): State<CloudServerState>,
    headers: HeaderMap,
) -> Result<Html<String>, Html<String>> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(redirect_html) => return Err(redirect_html),
    }

    let cloud = &server_state.cloud;

    let html = format!(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>CloudTUI - {}</title>
        <style>
            body {{ font-family: Arial, sans-serif; margin: 40px; }}
            .container {{ max-width: 800px; margin: 0 auto; }}
            .header {{ text-align: center; margin-bottom: 30px; }}
            .cloud-folder-info {{ background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
            .actions {{ background: #f9f9f9; padding: 20px; border-radius: 5px; }}
            .btn {{ 
                display: inline-block; 
                background: #007bff; 
                color: white; 
                padding: 10px 20px; 
                text-decoration: none; 
                border-radius: 5px; 
                margin: 5px;
            }}
            .btn:hover {{ background: #0056b3; }}
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>🌩️ CloudTUI</h1>
            </div>
            <div class="cloud-folder-info">
                <h2>Cloud Folder Information</h2>
                <p><strong>Name:</strong> {}</p>
                <p><strong>Folders:</strong> {}</p>
            </div>
            <div class="actions">
                <h2>Actions</h2>
                <a href="/web/{}/files" class="btn">📁 Browse Files</a>
                <a href="/" class="btn">🏠 Back to All Cloud Folders</a>
            </div>
        </div>
    </body>
    </html>
    "#,
        cloud.name,
        cloud.name,
        cloud.cloud_folders.len(),
        cloud.name,
    );

    Ok(Html(html))
}

#[axum::debug_handler]
pub async fn list_cloud_folder_files(
    Path(cloud_folder_name): Path<String>,
    State(server_state): State<CloudServerState>,
    headers: HeaderMap,
) -> Result<Html<String>, Html<String>> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(redirect_html) => return Err(redirect_html),
    }
    let cloud = &server_state.cloud;

    browse_directory_internal(cloud.as_ref().clone(), cloud_folder_name, "".to_string())
        .await
        .map_err(|_| {
            Html(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>File Listing Error</title>
</head>
<body>
    <h1>Error</h1>
    <p>Failed to list files. Please try again later.</p>
    <a href="/">Back to Home</a>
</body>
</html>
            "#
                .to_string(),
            )
        })
}

async fn browse_directory_internal(
    cloud: Cloud,
    cloud_folder_name: String,
    requested_path: String,
) -> Result<Html<String>, StatusCode> {
    // Find the specific cloud folder
    let base_path = if let Some(cloud_folder) = cloud
        .cloud_folders
        .iter()
        .find(|cf| cf.name == cloud_folder_name)
    {
        &cloud_folder.folder_path
    } else {
        return Err(StatusCode::NOT_FOUND);
    };
    let full_path = base_path.join(&requested_path);

    // Security check: ensure the requested path is within the cloud folder directory
    if !full_path.starts_with(base_path) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // If it's a file, redirect to the static file service
    if full_path.is_file() {
        let redirect_url = format!("/static/{}", requested_path);
        return Ok(Html(format!(
            r#"<html><head><meta http-equiv="refresh" content="0; url={}"></head><body>Redirecting to file...</body></html>"#,
            redirect_url
        )));
    }

    // Read directory contents
    let entries = match fs::read_dir(&full_path) {
        Ok(entries) => entries,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path();
        let is_dir = file_path.is_dir();
        let size = if is_dir {
            "DIR".to_string()
        } else {
            match fs::metadata(&file_path) {
                Ok(metadata) => format!("{} bytes", metadata.len()),
                Err(_) => "Unknown".to_string(),
            }
        };

        let item_path = if requested_path.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", requested_path, file_name)
        };

        items.push(json!({
            "name": file_name,
            "is_directory": is_dir,
            "size": size,
            "path": item_path
        }));
    }

    // Sort: directories first, then files, both alphabetically
    items.sort_by(|a, b| {
        let a_is_dir = a["is_directory"].as_bool().unwrap_or(false);
        let b_is_dir = b["is_directory"].as_bool().unwrap_or(false);

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or("")),
        }
    });

    let html = format!(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>CloudTUI - {}</title>
        <style>
            body {{ font-family: Arial, sans-serif; margin: 40px; }}
            .container {{ max-width: 1200px; margin: 0 auto; }}
            .header {{ text-align: center; margin-bottom: 30px; }}
            .breadcrumb {{ margin-bottom: 20px; }}
            .breadcrumb a {{ color: #0066cc; text-decoration: none; }}
            .breadcrumb a:hover {{ text-decoration: underline; }}
            .file-list {{ background: #f9f9f9; padding: 20px; border-radius: 5px; }}
            .file-item {{ 
                display: flex; 
                align-items: center; 
                padding: 8px 0; 
                border-bottom: 1px solid #eee; 
            }}
            .file-item:last-child {{ border-bottom: none; }}
            .file-icon {{ margin-right: 10px; font-size: 18px; }}
            .file-name {{ flex: 1; }}
            .file-size {{ color: #666; margin-left: 10px; }}
            .file-actions {{ margin-left: 10px; }}
            .file-item a {{ color: #0066cc; text-decoration: none; }}
            .file-item a:hover {{ text-decoration: underline; }}
            .directory {{ background: #e8f4fd; }}
            .delete-btn {{ 
                background: #dc3545; 
                color: white; 
                border: none; 
                padding: 4px 8px; 
                border-radius: 3px; 
                cursor: pointer; 
                font-size: 12px;
            }}
            .delete-btn:hover {{ background: #c82333; }}
            .upload-section {{ 
                background: #f0f8ff; 
                padding: 20px; 
                border-radius: 5px; 
                margin-bottom: 20px; 
            }}
            .upload-section form {{ 
                display: flex; 
                align-items: center; 
                gap: 10px; 
            }}
            .upload-section input[type="file"] {{ 
                flex: 1; 
                padding: 8px; 
                border: 1px solid #ddd; 
                border-radius: 3px; 
            }}
            .upload-section button {{ 
                background: #28a745; 
                color: white; 
                border: none; 
                padding: 8px 16px; 
                border-radius: 3px; 
                cursor: pointer; 
            }}
            .upload-section button:hover {{ background: #218838; }}
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>🌩️ CloudTUI File Browser</h1>
                <p>Cloud Folder: {}</p>
            </div>
            
            <div class="breadcrumb">
                <a href="/">🏠 Cloud Folders</a> / <a href="/web/{}/files">📁 Root</a>
                /{}
            </div>
            
            <div class="upload-section">
                <h2>📤 Upload File</h2>
                <form id="uploadForm" enctype="multipart/form-data">
                    <input type="file" id="fileInput" name="file" required>
                    <button type="submit">Upload</button>
                </form>
            </div>
            
            <div class="file-list">
                <h2>📂 Directory Contents</h2>
                {}
            </div>
        </div>
        
        <script>
            document.getElementById('uploadForm').addEventListener('submit', async function(e) {{
                e.preventDefault();
                const fileInput = document.getElementById('fileInput');
                const file = fileInput.files[0];
                
                if (!file) {{
                    alert('Please select a file to upload');
                    return;
                }}
                
                // Construct the upload URL from the current page path
                const currentPath = window.location.pathname;
                const pathParts = currentPath.split('/');
                const cloudFolder = pathParts[2]; // /web/cloud_folder/files/...
                const subPath = pathParts.slice(4).join('/'); // everything after /files/
                const uploadUrl = `/api/upload/${{cloudFolder}}${{subPath ? '/' + subPath : ''}}`;
                
                const formData = new FormData();
                formData.append('file', file);
                
                try {{
                    const response = await fetch(uploadUrl, {{
                        method: 'POST',
                        body: formData
                    }});
                    const result = await response.json();
                    if (response.ok) {{
                        alert('File uploaded successfully!');
                        location.reload();
                    }} else {{
                        alert('Upload failed: ' + result.error);
                    }}
                }} catch (error) {{
                    alert('Upload failed: ' + error.message);
                }}
            }});

            // Delete file function
            async function deleteFile(cloudFolder, filePath) {{
                if (!confirm(`Are you sure you want to delete "${{filePath}}"?`)) {{
                    return;
                }}
                
                try {{
                    const response = await fetch(`/api/delete/${{cloudFolder}}/${{filePath}}`, {{
                        method: 'DELETE'
                    }});
                    
                    const result = await response.json();
                    
                    if (response.ok) {{
                        alert(`File deleted successfully!\\nPlatform: ${{result.platform}}\\n${{result.trash_info}}`);
                        location.reload();
                    }} else {{
                        alert('Delete failed: ' + result.error);
                    }}
                }} catch (error) {{
                    alert('Delete failed: ' + error.message);
                }}
            }}
        </script>
    </body>
    </html>
    "#,
        requested_path,
        cloud.name,
        cloud_folder_name,
        generate_breadcrumb(&requested_path, &cloud_folder_name),
        generate_file_list(&items, &cloud_folder_name)
    );

    Ok(Html(html))
}

pub async fn browse_file_or_directory(
    Path((cloud_folder_name, path)): Path<(String, String)>,
    State(server_state): State<CloudServerState>,
    headers: HeaderMap,
) -> Result<Html<String>, Html<String>> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(redirect_html) => return Err(redirect_html),
    }
    let cloud = &server_state.cloud;

    // Find the specific cloud folder
    let base_path = cloud
        .cloud_folders
        .iter()
        .find(|cf| cf.name == cloud_folder_name)
        .ok_or_else(|| {
            Html(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>Not Found</title>
</head>
<body>
    <h1>Cloud Folder Not Found</h1>
    <p>Cloud folder '{}' not found.</p>
    <a href="/">Back to Home</a>
</body>
</html>
            "#
                .replace("{}", &cloud_folder_name)
                .to_string(),
            )
        })?
        .folder_path
        .clone();

    let full_path = base_path.join(&path);

    if !full_path.exists() {
        return Err(Html(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Not Found</title>
</head>
<body>
    <h1>Resource Not Found</h1>
    <p>The requested resource was not found.</p>
    <a href="/">Back to Home</a>
</body>
</html>
        "#
            .to_string(),
        ));
    }

    if full_path.is_dir() {
        // It's a directory, show directory listing
        browse_directory_internal(cloud.as_ref().clone(), cloud_folder_name, path)
            .await
            .map_err(|_| {
                Html(
                    r#"
<!DOCTYPE html>
<html>
<head>
    <title>Directory Browsing Error</title>
</head>
<body>
    <h1>Error</h1>
    <p>Failed to browse directory. Please try again later.</p>
    <a href="/">Back to Home</a>
</body>
</html>
                "#
                    .to_string(),
                )
            })
    } else {
        // It's a file, show a download link instead of serving directly
        let file_name = full_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        let html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>File: {}</title>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 40px; }}
                    .container {{ max-width: 800px; margin: 0 auto; }}
                    .file-info {{ background: #f0f0f0; padding: 20px; border-radius: 5px; }}
                    .download-btn {{ 
                        display: inline-block; 
                        background: #007bff; 
                        color: white; 
                        padding: 10px 20px; 
                        text-decoration: none; 
                        border-radius: 5px; 
                        margin-top: 10px;
                    }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>📄 File: {}</h1>
                    <div class="file-info">
                        <p><strong>File:</strong> {}</p>
                        <p><strong>Cloud:</strong> {}</p>
                        <a href="/static/{}" class="download-btn">⬇️ Download File</a>
                    </div>
                </div>
            </body>
            </html>
            "#,
            file_name, file_name, file_name, cloud.name, path
        );

        Ok(Html(html))
    }
}

fn generate_breadcrumb(path: &str, cloud_folder_name: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut breadcrumb = String::new();
    let mut current_path = String::new();

    for (i, part) in parts.iter().enumerate() {
        current_path.push('/');
        current_path.push_str(part);

        if i > 0 {
            breadcrumb.push_str(" / ");
        }
        breadcrumb.push_str(&format!(
            "<a href=\"/web/{}/files{}\">📁 {}</a>",
            cloud_folder_name, current_path, part
        ));
    }

    breadcrumb
}

fn generate_file_list(items: &[serde_json::Value], cloud_folder_name: &str) -> String {
    if items.is_empty() {
        return "<p>📭 This directory is empty</p>".to_string();
    }

    let mut html = String::new();
    for item in items {
        let name = item["name"].as_str().unwrap_or("");
        let is_dir = item["is_directory"].as_bool().unwrap_or(false);
        let size = item["size"].as_str().unwrap_or("");
        let path = item["path"].as_str().unwrap_or("");

        let icon = if is_dir { "📁" } else { "📄" };
        let class = if is_dir { "directory" } else { "" };

        let link_url = if is_dir {
            format!("/web/{}/files/{}", cloud_folder_name, path)
        } else {
            format!("/api/{}/static/{}", cloud_folder_name, path)
        };

        // Add delete button for files only
        let delete_button = if !is_dir {
            format!(
                r#"<button class="delete-btn" onclick="deleteFile('{}', '{}')" title="Delete file">🗑️</button>"#,
                cloud_folder_name, path
            )
        } else {
            String::new()
        };

        html.push_str(&format!(
            r#"<div class="file-item {}">
                <span class="file-icon">{}</span>
                <span class="file-name"><a href="{}">{}</a></span>
                <span class="file-size">{}</span>
                <span class="file-actions">{}</span>
            </div>"#,
            class, icon, link_url, name, size, delete_button
        ));
    }

    html
}

pub async fn serve_static_file(
    State(server_state): State<CloudServerState>,
    Path((cloud_folder_name, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response<axum::body::Body>, Json<serde_json::Value>> {
    // Check authentication for web routes (API routes can skip this)
    let auth_state = get_authentication_state(&server_state);
    if !has_valid_token(&headers, auth_state) {
        return Err(Json(json!({
            "error": "Unauthorized",
            "message": "Authentication required"
        })));
    }

    let cloud = &server_state.cloud;

    // Find the specific cloud folder
    let base_path = cloud
        .cloud_folders
        .iter()
        .find(|f| f.name == cloud_folder_name)
        .ok_or_else(|| {
            Json(json!({
                "error": "Not Found",
                "message": "Cloud folder not found."
            }))
        })?
        .folder_path
        .clone();

    let full_path = base_path.join(&path);

    // Security check: ensure the requested path is within the cloud folder directory
    if !full_path.starts_with(&base_path) {
        return Err(Json(json!({
            "error": "Forbidden",
            "message": "You don't have permission to access this file."
        })));
    }

    if !full_path.exists() || full_path.is_dir() {
        return Err(Json(json!({
            "error": "Not Found",
            "message": "The requested file was not found."
        })));
    }

    // Read file content
    let file_content = match fs::read(&full_path) {
        Ok(content) => content,
        Err(_) => {
            return Err(Json(json!({
                "error": "Error",
                "message": "Failed to read the requested file."
            })));
        }
    };

    // Get file extension to determine MIME type
    let mime_type = get_mime_type(&full_path);

    // Create response with proper headers
    let mut response = Response::new(axum::body::Body::from(file_content));
    let headers = response.headers_mut();

    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_str(mime_type)
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );

    // Add filename for inline display
    if let Some(file_name) = full_path.file_name().and_then(|n| n.to_str()) {
        headers.insert(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_str(&format!("inline; filename=\"{}\"", file_name))
                .unwrap_or_else(|_| header::HeaderValue::from_static("inline")),
        );
    }

    Ok(response)
}

fn get_mime_type(path: &std::path::Path) -> &'static str {
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        match extension.to_lowercase().as_str() {
            // Images
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",

            // Text files
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "csv" => "text/csv",
            "md" | "markdown" => "text/markdown",

            // Documents
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

            // Archives
            "zip" => "application/zip",
            "rar" => "application/x-rar-compressed",
            "7z" => "application/x-7z-compressed",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",

            // Audio
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",

            // Video
            "mp4" => "video/mp4",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "wmv" => "video/x-ms-wmv",
            "webm" => "video/webm",

            // Code files
            "rs" => "text/plain",
            "py" => "text/plain",
            "java" => "text/plain",
            "cpp" | "cc" | "cxx" => "text/plain",
            "c" => "text/plain",
            "h" => "text/plain",
            "hpp" => "text/plain",
            "cs" => "text/plain",
            "php" => "text/plain",
            "rb" => "text/plain",
            "go" => "text/plain",
            "swift" => "text/plain",
            "kt" => "text/plain",
            "scala" => "text/plain",
            "sh" => "text/plain",
            "bat" => "text/plain",
            "ps1" => "text/plain",

            _ => "application/octet-stream",
        }
    } else {
        "application/octet-stream"
    }
}
