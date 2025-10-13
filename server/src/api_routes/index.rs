use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

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

// API endpoint for server status and cloudfolders list
pub async fn api_index(
    State(server_state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
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

    let response = json!({
        "status": "running",
        "cloudfolders": cloudfolder_list.len(),
        "cloudfolder_list": cloudfolder_list.iter().map(|cf| json!({
            "name": cf.name,
            "id": cf.id,
            "created_at": cf.created_at
        })).collect::<Vec<_>>(),
        "timestamp": chrono::Utc::now()
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
