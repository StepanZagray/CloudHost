use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use serde_json::json;
use std::fs;

use crate::profile::Profile;

pub async fn index() -> Html<&'static str> {
    Html(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>CloudTUI Server</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .container { max-width: 800px; margin: 0 auto; }
            .header { text-align: center; margin-bottom: 30px; }
            .status { background: #f0f0f0; padding: 20px; border-radius: 5px; }
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
                <p>✅ Server is running on port 3000</p>
                <p>📁 Files are being served from your profile folder</p>
                <p>🔗 Access your files at: <a href="/files">/files</a></p>
            </div>
        </div>
    </body>
    </html>
    "#,
    )
}

pub async fn status(
    State(profile): State<Profile>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let status = json!({
        "status": "running",
        "profile": {
            "id": profile.id,
            "name": profile.name,
            "folder_path": profile.folder_path
        },
        "timestamp": chrono::Utc::now()
    });

    Ok(axum::Json(status))
}

pub async fn list_root_directory(
    State(profile): State<Profile>,
) -> Result<Html<String>, StatusCode> {
    list_directory_internal(profile, "".to_string()).await
}

pub async fn list_directory(
    State(profile): State<Profile>,
    Path(requested_path): Path<String>,
) -> Result<Html<String>, StatusCode> {
    list_directory_internal(profile, requested_path).await
}

async fn list_directory_internal(
    profile: Profile,
    requested_path: String,
) -> Result<Html<String>, StatusCode> {
    let base_path = &profile.folder_path;
    let full_path = base_path.join(&requested_path);

    // Security check: ensure the requested path is within the profile directory
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
                <p>Profile: {}</p>
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
        profile.name,
        generate_breadcrumb(&requested_path),
        generate_file_list(&items)
    );

    Ok(Html(html))
}

pub async fn handle_file_or_directory(
    State(profile): State<Profile>,
    Path(path): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let full_path = profile.folder_path.join(&path);

    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    if full_path.is_dir() {
        // It's a directory, show directory listing
        list_directory(State(profile), Path(path)).await
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
                        <a href="/static/{}" class="download-btn">⬇️ Download File</a>
                    </div>
                </div>
            </body>
            </html>
            "#,
            file_name, file_name, file_name, path, path
        );

        Ok(Html(html))
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
