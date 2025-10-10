use std::fmt;

/// TUI error types
#[derive(Debug, Clone)]
pub enum TuiError {
    Configuration(String),
    FileSystem(String),
    Network(String),
    Internal(String),
    Validation(String),
    ServerCommunication(String),
    UserInput(String),
}

impl fmt::Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            TuiError::FileSystem(msg) => write!(f, "File system error: {}", msg),
            TuiError::Network(msg) => write!(f, "Network error: {}", msg),
            TuiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            TuiError::Validation(msg) => write!(f, "Validation error: {}", msg),
            TuiError::ServerCommunication(msg) => write!(f, "Server communication error: {}", msg),
            TuiError::UserInput(msg) => write!(f, "User input error: {}", msg),
        }
    }
}

impl std::error::Error for TuiError {}

impl TuiError {
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

    pub fn server_communication(msg: impl Into<String>) -> Self {
        Self::ServerCommunication(msg.into())
    }

    pub fn user_input(msg: impl Into<String>) -> Self {
        Self::UserInput(msg.into())
    }
}

// Auto-convert common error types
impl From<std::io::Error> for TuiError {
    fn from(err: std::io::Error) -> Self {
        Self::FileSystem(err.to_string())
    }
}

impl From<toml::de::Error> for TuiError {
    fn from(err: toml::de::Error) -> Self {
        Self::Configuration(format!("TOML parsing error: {}", err))
    }
}

impl From<toml::ser::Error> for TuiError {
    fn from(err: toml::ser::Error) -> Self {
        Self::Configuration(format!("TOML serialization error: {}", err))
    }
}

impl From<serde_json::Error> for TuiError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(format!("JSON error: {}", err))
    }
}

// Note: reqwest is not used in TUI, so no conversion needed

pub type TuiResult<T> = Result<T, TuiError>;
