use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::{collections::HashMap, fs, sync::Arc};

use crate::{auth::AuthState, cloud_folder::CloudFolder, server::ServerState};

// Security headers for API responses
fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // XSS Protection
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );

    response
}

// Input validation for cloudfolder names
fn validate_cloudfolder_name(
    name: &str,
) -> Result<(), (StatusCode, axum::Json<serde_json::Value>)> {
    // Check for empty or too long names
    if name.is_empty() || name.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            axum::Json(json!({
                "error": "Invalid Input",
                "message": "Cloud folder name must be between 1 and 100 characters"
            })),
        ));
    }

    // Check for dangerous characters
    if name.contains("..") || name.contains("/") || name.contains("\\") {
        return Err((
            StatusCode::BAD_REQUEST,
            axum::Json(json!({
                "error": "Invalid Input",
                "message": "Cloud folder name contains invalid characters"
            })),
        ));
    }

    // Check for only whitespace
    if name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            axum::Json(json!({
                "error": "Invalid Input",
                "message": "Cloud folder name cannot be only whitespace"
            })),
        ));
    }

    Ok(())
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

// Verifies authentication and returns JSON error if unauthorized
fn verify_authentication(
    headers: &HeaderMap,
    auth_state: &AuthState,
) -> Result<(), (StatusCode, axum::Json<serde_json::Value>)> {
    if has_valid_token(headers, auth_state) {
        return Ok(());
    }

    // Return JSON error for API clients
    Err((
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({
            "error": "Unauthorized",
            "message": "Authentication required. Please provide a valid JWT token.",
            "login_url": "/api/login"
        })),
    ))
}

// API endpoint for cloudfolder info
pub async fn get_cloudfolder_info(
    State(server_state): State<ServerState>,
    Path(cloudfolder_name): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    // Validate input
    if let Err(error_response) = validate_cloudfolder_name(&cloudfolder_name) {
        return Err(error_response);
    }

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

    let response = json!({
        "name": cloudfolder.name,
        "id": cloudfolder.id,
        "created_at": cloudfolder.created_at
    });

    let json_response = axum::Json(response);
    let response = axum::response::Response::builder()
        .header("Content-Type", "application/json")
        .body(json_response.into_response().into_body())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "error": "Internal Server Error",
                    "message": "Failed to build response"
                })),
            )
        })?;

    Ok(add_security_headers(response))
}

// API endpoint for listing cloudfolder files
pub async fn api_list_cloudfolder_files(
    State(server_state): State<ServerState>,
    Path(cloudfolder_name): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
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

// API endpoint for browsing files/directories
pub async fn api_browse_file_or_directory(
    State(server_state): State<ServerState>,
    Path((cloudfolder_name, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    // Validate inputs
    if let Err(error_response) = validate_cloudfolder_name(&cloudfolder_name) {
        return Err(error_response);
    }

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
                "message": "The requested resource was not found"
            })),
        ));
    }

    if full_path.is_dir() {
        // It's a directory, return directory listing as JSON
        let json_response = browse_directory_internal(cloudfolder, path)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(
                        json!({"error": "Directory browsing error", "message": e.to_string()}),
                    ),
                )
            })?;

        let response = axum::response::Response::builder()
            .header("Content-Type", "application/json")
            .body(json_response.into_response().into_body())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({
                        "error": "Internal Server Error",
                        "message": "Failed to build response"
                    })),
                )
            })?;

        Ok(add_security_headers(response))
    } else {
        // It's a file, return file info as JSON
        let file_name = full_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        let metadata = fs::metadata(&full_path).ok();
        let size = metadata.map(|m| m.len()).unwrap_or(0);

        let response = json!({
            "type": "file",
            "name": file_name,
            "path": path,
            "size": size,
            "download_url": format!("/api/{}/static/{}", cloudfolder_name, path)
        });

        let json_response = axum::Json(response);
        let response = axum::response::Response::builder()
            .header("Content-Type", "application/json")
            .body(json_response.into_response().into_body())
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({
                        "error": "Internal Server Error",
                        "message": "Failed to build response"
                    })),
                )
            })?;

        Ok(add_security_headers(response))
    }
}

// Internal function to browse directory and return JSON
async fn browse_directory_internal(
    cloudfolder: CloudFolder,
    requested_path: String,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let base_path = &cloudfolder.folder_path;
    let full_path = base_path.join(&requested_path);

    // Security check: ensure the requested path is within the cloudfolder directory
    if !full_path.starts_with(base_path) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // If it's a file, return file info
    if full_path.is_file() {
        let file_name = full_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        let metadata = fs::metadata(&full_path).ok();
        let size = metadata.map(|m| m.len()).unwrap_or(0);

        let response = json!({
            "type": "file",
            "name": file_name,
            "path": requested_path,
            "size": size,
            "download_url": format!("/api/{}/static/{}", cloudfolder.name, requested_path)
        });

        return Ok(axum::Json(response));
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
            0 // Directories don't have meaningful size
        } else {
            match fs::metadata(&file_path) {
                Ok(metadata) => metadata.len(),
                Err(_) => 0,
            }
        };

        let item_path = if requested_path.is_empty() {
            file_name.clone()
        } else {
            format!("{}/{}", requested_path, file_name)
        };

        items.push(json!({
            "name": file_name,
            "path": item_path,
            "type": if is_dir { "directory" } else { "file" },
            "size": size
        }));
    }

    // Sort: directories first, then files, both alphabetically
    items.sort_by(|a, b| {
        let a_is_dir = a["type"].as_str().unwrap_or("") == "directory";
        let b_is_dir = b["type"].as_str().unwrap_or("") == "directory";

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or("")),
        }
    });

    let response = json!({
        "type": "directory",
        "path": requested_path,
        "items": items
    });

    Ok(axum::Json(response))
}
