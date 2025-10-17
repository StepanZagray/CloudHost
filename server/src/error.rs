use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// Server error types
#[derive(Debug, Clone)]
pub enum ServerError {
    Authentication(String),
    Configuration(String),
    FileSystem(String),
    Network(String),
    Internal(String),
    Validation(String),
    CloudFolder(String),
    ServerAlreadyRunning,
    ServerNotRunning,
    ServerError(String),
    InvalidPath(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ServerError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            ServerError::FileSystem(msg) => write!(f, "File system error: {}", msg),
            ServerError::Network(msg) => write!(f, "Network error: {}", msg),
            ServerError::Internal(msg) => write!(f, "Internal server error: {}", msg),
            ServerError::Validation(msg) => write!(f, "Validation error: {}", msg),
            ServerError::CloudFolder(msg) => write!(f, "Cloud folder error: {}", msg),
            ServerError::ServerAlreadyRunning => write!(f, "Server already running"),
            ServerError::ServerNotRunning => write!(f, "Server not running"),
            ServerError::ServerError(msg) => write!(f, "Server error: {}", msg),
            ServerError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
        }
    }
}

impl std::error::Error for ServerError {}

/// Convert to HTTP response
impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ServerError::Authentication(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            ServerError::Configuration(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR", msg)
            }
            ServerError::FileSystem(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "FILE_ERROR", msg),
            ServerError::Network(msg) => (StatusCode::BAD_GATEWAY, "NETWORK_ERROR", msg),
            ServerError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
            ServerError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            ServerError::CloudFolder(msg) => (StatusCode::NOT_FOUND, "CLOUD_FOLDER_ERROR", msg),
            ServerError::ServerAlreadyRunning => (
                StatusCode::CONFLICT,
                "SERVER_RUNNING",
                "Server already running".to_string(),
            ),
            ServerError::ServerNotRunning => (
                StatusCode::BAD_REQUEST,
                "SERVER_NOT_RUNNING",
                "Server not running".to_string(),
            ),
            ServerError::ServerError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "SERVER_ERROR", msg)
            }
            ServerError::InvalidPath(msg) => (StatusCode::BAD_REQUEST, "INVALID_PATH", msg),
        };

        let body = Json(json!({
            "error": error_code,
            "message": message,
            "timestamp": chrono::Utc::now()
        }));

        (status, body).into_response()
    }
}

impl ServerError {
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    pub fn file_system(msg: impl Into<String>) -> Self {
        Self::FileSystem(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn cloud_folder(msg: impl Into<String>) -> Self {
        Self::CloudFolder(msg.into())
    }
}

// Auto-convert common error types
impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        Self::FileSystem(err.to_string())
    }
}

impl From<toml::de::Error> for ServerError {
    fn from(err: toml::de::Error) -> Self {
        Self::Configuration(format!("TOML parsing error: {}", err))
    }
}

impl From<toml::ser::Error> for ServerError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Configuration(format!("TOML serialization error: {}", err))
    }
}

impl From<bcrypt::BcryptError> for ServerError {
    fn from(err: bcrypt::BcryptError) -> Self {
        Self::Internal(format!("Password hashing error: {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for ServerError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Authentication(format!("JWT error: {}", err))
    }
}

impl From<serde_json::Error> for ServerError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON error: {}", err))
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
