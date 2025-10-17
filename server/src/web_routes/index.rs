use axum::{extract::State, http::HeaderMap, response::Html};
use std::sync::Arc;

use crate::{auth::AuthState, cloud::CloudServerState};

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

pub async fn index(
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

    let cloud_folders_html = if cloud.cloud_folders.is_empty() {
        "<p>No cloud folders configured for this cloud.</p>".to_string()
    } else {
        cloud
            .cloud_folders
            .iter()
            .map(|folder| {
                format!(
                    r#"<div class="cloud-folder-item">
                        <div class="cloud-folder-name">📁 {}</div>
                        <a href="/web/{}/files" class="browse-btn">Browse Files</a>
                    </div>"#,
                    folder.name, folder.name
                )
            })
            .collect::<Vec<_>>()
            .join("")
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
            .cloud-folders {{ background: #f9f9f9; padding: 20px; border-radius: 5px; }}
            .cloud-folder-item {{ 
                background: white; 
                padding: 15px; 
                margin: 10px 0; 
                border-radius: 5px; 
                border-left: 4px solid #007bff;
            }}
            .cloud-folder-item a {{ color: #007bff; text-decoration: none; }}
            .cloud-folder-item a:hover {{ text-decoration: underline; }}
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
                <p>📊 Cloud: {}</p>
            </div>
            <div class="cloud-folders">
                <h2>📂 Cloud Folders</h2>
                {}
            </div>
        </div>
    </body>
    </html>
    "#,
        cloud.name, cloud_folders_html
    );

    Ok(Html(html))
}
