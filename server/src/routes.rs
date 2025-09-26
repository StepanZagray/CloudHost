use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
};
use serde_json::json;

use crate::profile::Profile;

pub async fn index() -> Html<&'static str> {
    Html(r#"
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
                <p>🔗 Access your files at: <a href="/files/">/files/</a></p>
            </div>
        </div>
    </body>
    </html>
    "#)
}

pub async fn status(State(profile): State<Profile>) -> Result<axum::Json<serde_json::Value>, StatusCode> {
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
