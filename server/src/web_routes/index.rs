use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Html,
};
use std::{collections::HashMap, sync::Arc};

use crate::{auth::AuthState, cloud_folder::CloudFolder, server::ServerState};

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

pub async fn index(
    State(server_state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Html<String>, Html<String>> {
    // Check authentication
    let auth_state = get_authentication_state(&server_state);
    match verify_authentication(&headers, auth_state) {
        Ok(()) => (),
        Err(redirect_html) => return Err(redirect_html),
    }

    let cloudfolders_guard = acquire_cloudfolders_lock(&server_state).map_err(|_| {
        Html(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Server Error</title>
</head>
<body>
    <h1>Server Error</h1>
    <p>Failed to access cloud folders. Please try again later.</p>
</body>
</html>
        "#
            .to_string(),
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
                        <p><a href="/{}/files">Browse Files</a></p>
                    </div>"#,
                    cloudfolder.name, cloudfolder.name, cloudfolder.name
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
