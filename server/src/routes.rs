use axum::{extract::State, Json};
use serde_json::json;

use crate::{
    auth::{LoginRequest, LoginResponse},
    server::ServerState,
};

// Re-export web routes
pub use crate::web_routes::*;

// Re-export static file handler
pub use crate::web_routes::cloudfolder::serve_static_file;

// Re-export specific API routes to avoid naming conflicts
pub use crate::api_routes::cloudfolders::{
    api_browse_file_or_directory, api_list_cloudfolder_files, get_cloudfolder_info,
};
pub use crate::api_routes::index::api_index;

// Wrapper for login function to work with ServerState
pub async fn login(
    State(server_state): State<ServerState>,
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
