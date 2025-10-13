use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    Json,
};
use serde_json::json;
use std::{collections::HashMap, fs, sync::Arc};

use crate::{auth::AuthState, cloud_folder::CloudFolder, server::ServerState};

// Login page
pub async fn login_page() -> Html<String> {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>CloudTUI Login</title>
    <style>
        body { 
            font-family: Arial, sans-serif; 
            margin: 0; 
            padding: 0; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .login-container {
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0,0,0,0.1);
            width: 100%;
            max-width: 400px;
        }
        .login-header {
            text-align: center;
            margin-bottom: 30px;
        }
        .login-header h1 {
            color: #333;
            margin: 0;
            font-size: 28px;
        }
        .login-header p {
            color: #666;
            margin: 10px 0 0 0;
        }
        .form-group {
            margin-bottom: 20px;
        }
        .form-group label {
            display: block;
            margin-bottom: 5px;
            color: #333;
            font-weight: bold;
        }
        .form-group input {
            width: 100%;
            padding: 12px;
            border: 2px solid #ddd;
            border-radius: 5px;
            font-size: 16px;
            box-sizing: border-box;
        }
        .form-group input:focus {
            outline: none;
            border-color: #667eea;
        }
        .login-button {
            width: 100%;
            padding: 12px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 5px;
            font-size: 16px;
            cursor: pointer;
            transition: transform 0.2s;
        }
        .login-button:hover {
            transform: translateY(-2px);
        }
        .error-message {
            color: #e74c3c;
            text-align: center;
            margin-top: 15px;
            padding: 10px;
            background: #fdf2f2;
            border-radius: 5px;
            display: none;
        }
        .success-message {
            color: #27ae60;
            text-align: center;
            margin-top: 15px;
            padding: 10px;
            background: #f0f9f0;
            border-radius: 5px;
            display: none;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="login-header">
            <h1>🌩️ CloudTUI</h1>
            <p>Enter your password to access your cloud storage</p>
        </div>
        <form id="loginForm">
            <div class="form-group">
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" required>
            </div>
            <button type="submit" class="login-button">Login</button>
        </form>
        <div id="errorMessage" class="error-message"></div>
        <div id="successMessage" class="success-message"></div>
    </div>

    <script>
        document.getElementById('loginForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const password = document.getElementById('password').value;
            const errorDiv = document.getElementById('errorMessage');
            const successDiv = document.getElementById('successMessage');
            
            // Hide previous messages
            errorDiv.style.display = 'none';
            successDiv.style.display = 'none';
            
            try {
                const response = await fetch('/api/login', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ password: password })
                });
                
                if (response.ok) {
                    const data = await response.json();
                    // Store token in cookie
                    document.cookie = `auth_token=${data.token}; path=/; max-age=86400`; // 24 hours
                    successDiv.textContent = 'Login successful! Redirecting...';
                    successDiv.style.display = 'block';
                    
                    // Redirect to home page
                    setTimeout(() => {
                        window.location.href = '/';
                    }, 1000);
                } else {
                    const error = await response.message.text();
                    errorDiv.textContent = error;
                    errorDiv.style.display = 'block';
                }
            } catch (error) {
                errorDiv.textContent = 'Login failed. Please try again.';
                errorDiv.style.display = 'block';
            }
        });
    </script>
</body>
</html>
    "#;
    Html(html.to_string())
}

// Login API endpoint
pub async fn login(
    State(server_state): State<ServerState>,
    Json(login_request): Json<crate::auth::LoginRequest>,
) -> Result<Json<crate::auth::LoginResponse>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Verify password
    let auth_state = get_authentication_state(&server_state);
    if auth_state.verify_password(&login_request.password) {
        // Generate JWT token
        if let Ok(token) = auth_state.generate_token() {
            return Ok(Json(crate::auth::LoginResponse { token }));
        }
    }

    // Return proper error response for incorrect password
    Err((
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({
            "error": "Invalid credentials",
            "message": "Incorrect password. Please try again."
        })),
    ))
}

// Helper function to acquire lock on cloudfolders collection
fn acquire_cloudfolders_lock(
    server_state: &ServerState,
) -> Result<std::sync::MutexGuard<'_, HashMap<String, CloudFolder>>, StatusCode> {
    server_state
        .cloudfolders
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// Helper function to get authentication state from server state
fn get_authentication_state(server_state: &ServerState) -> &Arc<AuthState> {
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
                if let Some(token) = cookie.strip_prefix("auth_token=") {
                    if auth_state.verify_token(token).is_ok() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

// Detects if request is from an API client (vs web browser)
fn is_api_client(headers: &HeaderMap) -> bool {
    // Check for JSON content type
    if let Some(content_type) = headers.get("Content-Type") {
        if let Ok(content_type_str) = content_type.to_str() {
            if content_type_str.contains("application/json") {
                return true;
            }
        }
    }

    // Check for JSON accept header
    if let Some(accept) = headers.get("Accept") {
        if let Ok(accept_str) = accept.to_str() {
            if accept_str.contains("application/json") {
                return true;
            }
        }
    }

    // Check for Authorization header (API clients typically use this)
    if headers.get("Authorization").is_some() {
        return true;
    }

    false
}

// Verifies authentication and returns appropriate error response if unauthorized
fn verify_authentication(
    headers: &HeaderMap,
    auth_state: &AuthState,
) -> Result<(), (StatusCode, axum::Json<serde_json::Value>)> {
    if has_valid_token(headers, auth_state) {
        return Ok(());
    }

    let error_response = if is_api_client(headers) {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": "Unauthorized",
                "message": "Authentication required. Please provide a valid JWT token.",
                "login_url": "/api/login"
            })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": "Unauthorized",
                "message": "Please login to access this resource.",
                "redirect_to": "/login"
            })),
        )
    };

    Err(error_response)
}

pub async fn index(
    State(server_state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Html<String>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }

    let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({
                "error": "Internal Server Error",
                "message": "Failed to access cloud folders"
            })),
        )
    })?;
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
    State(server_state): State<ServerState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }

    let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({
                "error": "Internal Server Error",
                "message": "Failed to access cloud folders"
            })),
        )
    })?;
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
    State(server_state): State<ServerState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);

    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }
    let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({
                "error": "Internal Server Error",
                "message": "Failed to access cloud folders"
            })),
        )
    })?;
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
    State(server_state): State<ServerState>,
    Path(cloudfolder_name): Path<String>,
    headers: HeaderMap,
) -> Result<Html<String>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }

    let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({
                "error": "Internal Server Error",
                "message": "Failed to access cloud folders"
            })),
        )
    })?;

    let cloudfolder = cloudfolders_guard.get(&cloudfolder_name).ok_or((
        StatusCode::NOT_FOUND,
        axum::Json(json!({
            "error": "Not Found",
            "message": "Cloud folder not found"
        })),
    ))?;

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
                <h1>🌩️ CloudTUI</h1>
            </div>
            <div class="cloudfolder-info">
                <h2>Cloud Folder Information</h2>
                <p><strong>Name:</strong> {}</p>
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
        cloudfolder.created_at.format("%Y-%m-%d %H:%M:%S"),
        cloudfolder.name,
    );

    Ok(Html(html))
}
#[axum::debug_handler]
pub async fn list_cloudfolder_files(
    State(server_state): State<ServerState>,
    Path(cloudfolder_name): Path<String>,
    headers: HeaderMap,
) -> Result<Html<String>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }
    let cloudfolder = {
        let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "error": "Internal Server Error",
                    "message": "Failed to access cloud folders"
                })),
            )
        })?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or((
                StatusCode::NOT_FOUND,
                axum::Json(json!({
                    "error": "Not Found",
                    "message": "Cloud folder not found"
                })),
            ))?
            .clone()
    };

    browse_directory_internal(cloudfolder, "".to_string())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": "File listing error", "message": e.to_string()})),
            )
        })
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
        let redirect_url = format!("/{}/static/{}", cloudfolder.name, requested_path);
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
                <a href="/{}/files">📁 Root</a>
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
        cloudfolder.name,
        generate_breadcrumb(&requested_path, &cloudfolder.name),
        generate_file_list(&items, &cloudfolder.name)
    );

    Ok(Html(html))
}

pub async fn browse_file_or_directory(
    State(server_state): State<ServerState>,
    Path((cloudfolder_name, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Html<String>, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }
    let cloudfolder = {
        let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "error": "Internal Server Error",
                    "message": "Failed to access cloud folders"
                })),
            )
        })?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or((
                StatusCode::NOT_FOUND,
                axum::Json(json!({
                    "error": "Not Found",
                    "message": "Cloud folder not found"
                })),
            ))?
            .clone()
    };

    let full_path = cloudfolder.folder_path.join(&path);

    if !full_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            axum::Json(json!({
                "error": "Not Found",
                "message": format!("Path not found: {}", path)
            })),
        ));
    }

    if full_path.is_dir() {
        // It's a directory, show directory listing
        browse_directory_internal(cloudfolder, path)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(
                        json!({"error": "Directory browsing error", "message": e.to_string()}),
                    ),
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
    State(server_state): State<ServerState>,
    Path((cloudfolder_name, file_path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<axum::response::Response, (StatusCode, axum::Json<serde_json::Value>)> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(error_response) => return Err(error_response),
    }
    let cloudfolder = {
        let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "error": "Internal Server Error",
                    "message": "Failed to access cloud folders"
                })),
            )
        })?;
        cloudfolders_guard
            .get(&cloudfolder_name)
            .ok_or((
                StatusCode::NOT_FOUND,
                axum::Json(json!({
                    "error": "Not Found",
                    "message": "Cloud folder not found"
                })),
            ))?
            .clone()
    };

    let full_path = cloudfolder.folder_path.join(&file_path);

    // Security check: ensure the requested path is within the cloudfolder directory
    if !full_path.starts_with(&cloudfolder.folder_path) {
        return Err((
            StatusCode::FORBIDDEN,
            axum::Json(json!({
                "error": "Forbidden",
                "message": "Access denied to this path"
            })),
        ));
    }

    if !full_path.exists() || !full_path.is_file() {
        return Err((
            StatusCode::NOT_FOUND,
            axum::Json(json!({
                "error": "Not Found",
                "message": format!("File not found: {}", file_path)
            })),
        ));
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
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(json!({"error": "Response build error", "message": "Failed to build file response"}))))?)
        }
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({
                "error": "File read error",
                "message": "Failed to read file content"
            })),
        )),
    }
}

fn generate_breadcrumb(path: &str, cloudfolder_name: &str) -> String {
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
            "<a href=\"/{}/files{}\">📁 {}</a>",
            cloudfolder_name, current_path, part
        ));
    }

    breadcrumb
}

fn generate_file_list(items: &[serde_json::Value], cloudfolder_name: &str) -> String {
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
            format!("/{}/files/{}", cloudfolder_name, path)
        } else {
            format!("/{}/static/{}", cloudfolder_name, path)
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
