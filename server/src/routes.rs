use axum::{extract::State, Json};
use serde_json::json;

use crate::{
    auth::{LoginRequest, LoginResponse},
    cloud::CloudServerState,
};

// Re-export web routes
pub use crate::web_routes::*;

// Re-export cloud routes
pub use crate::web_routes::cloud_folder::{browse_file_or_directory, serve_static_file};

// Re-export API routes
pub use crate::api_routes::cloud::{
    api_browse_file_or_directory, api_list_cloud_folder_files, get_cloud_folder_info,
};
pub use crate::api_routes::index::api_index;

// Wrapper for login function to work with CloudServerState
pub async fn login(
    State(server_state): State<CloudServerState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let auth_state = &server_state.auth_state;

    if auth_state.verify_password(&payload.password) {
        if let Ok(token) = auth_state.generate_token() {
            return Ok(Json(LoginResponse { token }));
        }
    }

    Err((
        axum::http::StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "Invalid credentials"
        })),
    ))
}
