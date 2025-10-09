use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};

use crate::profile::CloudFolder;

// Helper function to reduce redundant error handling
fn get_cloudfolders_guard(
    cloudfolders: &Arc<Mutex<HashMap<String, CloudFolder>>>,
) -> Result<std::sync::MutexGuard<HashMap<String, CloudFolder>>, StatusCode> {
    cloudfolders
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn index(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
) -> Result<Html<String>, StatusCode> {
    let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
    let cloudfolder_list: Vec<&CloudFolder> = cloudfolders_guard.values().collect();

    let cloudfolders_html = if cloudfolder_list.is_empty() {
        "<p>📭 No Cloud Folders available</p>".to_string()
    } else {
        cloudfolder_list
            .iter()
            .map(|cloudfolder| {
                format!(
                    r#"<div class="cloudfolder-item">
                        <h3>📁 <a href="/{}">{}</a></h3>
                        <p>Path: {}</p>
                        <p><a href="/{}/files">Browse Files</a></p>
                    </div>"#,
                    cloudfolder.name,
                    cloudfolder.name,
                    cloudfolder.folder_path.display(),
                    cloudfolder.name
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let html = format!(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>CloudTUI Server</title>
        <style>
            body {{ font-family: Arial, sans-serif; margin: 40px; }}
            .container {{ max-width: 800px; margin: 0 auto; }}
            .header {{ text-align: center; margin-bottom: 30px; }}
            .status {{ background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
            .cloudfolders {{ background: #f9f9f9; padding: 20px; border-radius: 5px; }}
            .cloudfolder-item {{ 
                background: white; 
                padding: 15px; 
                margin: 10px 0; 
                border-radius: 5px; 
                border-left: 4px solid #007bff;
            }}
            .cloudfolder-item a {{ color: #007bff; text-decoration: none; }}
            .cloudfolder-item a:hover {{ text-decoration: underline; }}
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>🌩️ CloudTUI Server</h1>
                <p>Your personal cloud storage server</p>
            </div>
            <div class="status">
                <h2>Server Status</h2>
                <p>✅ Server is running</p>
                <p>📊 {} Cloud Folders available</p>
            </div>
            <div class="cloudfolders">
                <h2>📂 Available Cloud Folders</h2>
                {}
            </div>
        </div>
    </body>
    </html>
    "#,
        cloudfolder_list.len(),
        cloudfolders_html
    );

    Ok(Html(html))
}

pub async fn status(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
    let cloudfolder_list: Vec<&CloudFolder> = cloudfolders_guard.values().collect();

    let status = json!({
        "status": "running",
        "cloudfolders": cloudfolder_list.len(),
        "cloudfolder_list": cloudfolder_list.iter().map(|cf| json!({
            "name": cf.name,
            "id": cf.id,
            "folder_path": cf.folder_path
        })).collect::<Vec<_>>(),
        "timestamp": chrono::Utc::now()
    });

    Ok(axum::Json(status))
}

pub async fn get_cloudfolders_list(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
    let cloudfolder_list: Vec<&CloudFolder> = cloudfolders_guard.values().collect();

    let cloudfolders_json = cloudfolder_list
        .iter()
        .map(|cf| {
            json!({
                "name": cf.name,
                "id": cf.id,
                "folder_path": cf.folder_path,
                "created_at": cf.created_at
            })
        })
        .collect::<Vec<_>>();

    Ok(axum::Json(json!({
        "cloudfolders": cloudfolders_json
    })))
}

pub async fn show_cloudfolder_info(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
    Path(cloudfolder_name): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;

    let cloudfolder = cloudfolders_guard
        .get(&cloudfolder_name)
        .ok_or(StatusCode::NOT_FOUND)?;

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
            .cloudfolder-info {{ background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
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
                <h1>🌩️ CloudTUI - {}</h1>
                <p>Cloud Folder: {}</p>
            </div>
            <div class="cloudfolder-info">
                <h2>Cloud Folder Information</h2>
                <p><strong>Name:</strong> {}</p>
                <p><strong>Path:</strong> {}</p>
                <p><strong>Created:</strong> {}</p>
            </div>
            <div class="actions">
                <h2>Actions</h2>
                <a href="/{}/files" class="btn">📁 Browse Files</a>
                <a href="/" class="btn">🏠 Back to All Cloud Folders</a>
            </div>
        </div>
    </body>
    </html>
    "#,
        cloudfolder.name,
        cloudfolder.name,
        cloudfolder.name,
        cloudfolder.folder_path.display(),
        cloudfolder.created_at.format("%Y-%m-%d %H:%M:%S"),
        cloudfolder.name,
        cloudfolder.name
    );

    Ok(Html(html))
}
#[axum::debug_handler]
pub async fn list_cloudfolder_files(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
    Path(cloudfolder_name): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let cloudfolder = {
        let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };

    browse_directory_internal(cloudfolder, "".to_string()).await
}

async fn browse_directory_internal(
    cloudfolder: CloudFolder,
    requested_path: String,
) -> Result<Html<String>, StatusCode> {
    let base_path = &cloudfolder.folder_path;
    let full_path = base_path.join(&requested_path);

    // Security check: ensure the requested path is within the cloudfolder directory
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

        items.push(json!({
            "name": file_name,
            "is_directory": is_dir,
            "size": size,
            "path": requested_path.clone() + "/" + &file_name
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
            .file-item a {{ color: #0066cc; text-decoration: none; }}
            .file-item a:hover {{ text-decoration: underline; }}
            .directory {{ background: #e8f4fd; }}
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <h1>🌩️ CloudTUI File Browser</h1>
                <p>Cloud Folder: {}</p>
            </div>
            
            <div class="breadcrumb">
                <a href="/files">📁 Root</a>
                {}
            </div>
            
            <div class="file-list">
                <h2>📂 Directory Contents</h2>
                {}
            </div>
        </div>
    </body>
    </html>
    "#,
        requested_path,
        cloudfolder.name,
        generate_breadcrumb(&requested_path),
        generate_file_list(&items)
    );

    Ok(Html(html))
}

pub async fn browse_file_or_directory(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
    Path((cloudfolder_name, path)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    let cloudfolder = {
        let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };

    let full_path = cloudfolder.folder_path.join(&path);

    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    if full_path.is_dir() {
        // It's a directory, show directory listing
        browse_directory_internal(cloudfolder, path).await
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
                        <p><strong>Path:</strong> {}</p>
                        <p><strong>Cloud Folder:</strong> {}</p>
                        <a href="/{}/static/{}" class="download-btn">⬇️ Download File</a>
                    </div>
                </div>
            </body>
            </html>
            "#,
            file_name,
            file_name,
            file_name,
            cloudfolder_name,
            cloudfolder_name,
            path,
            cloudfolder_name
        );

        Ok(Html(html))
    }
}

pub async fn download_file(
    State(cloudfolders): State<Arc<Mutex<HashMap<String, CloudFolder>>>>,
    Path((cloudfolder_name, file_path)): Path<(String, String)>,
) -> Result<axum::response::Response, StatusCode> {
    let cloudfolder = {
        let cloudfolders_guard = get_cloudfolders_guard(&cloudfolders)?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or(StatusCode::NOT_FOUND)?
            .clone()
    };

    let full_path = cloudfolder.folder_path.join(&file_path);

    // Security check: ensure the requested path is within the cloudfolder directory
    if !full_path.starts_with(&cloudfolder.folder_path) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !full_path.exists() || !full_path.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Read the file and serve it
    match tokio::fs::read(&full_path).await {
        Ok(content) => {
            let mime_type = mime_guess::from_path(&full_path)
                .first_or_text_plain()
                .to_string();

            Ok(axum::response::Response::builder()
                .header("Content-Type", mime_type)
                .header(
                    "Content-Disposition",
                    format!(
                        "inline; filename=\"{}\"",
                        full_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("file")
                    ),
                )
                .body(content.into())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn generate_breadcrumb(path: &str) -> String {
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
            "<a href=\"/files{}\">📁 {}</a>",
            current_path, part
        ));
    }

    breadcrumb
}

fn generate_file_list(items: &[serde_json::Value]) -> String {
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
            format!("/files/{}", path)
        } else {
            format!("/static/{}", path)
        };

        html.push_str(&format!(
            r#"<div class="file-item {}">
                <span class="file-icon">{}</span>
                <span class="file-name"><a href="{}">{}</a></span>
                <span class="file-size">{}</span>
            </div>"#,
            class, icon, link_url, name, size
        ));
    }

    html
}
