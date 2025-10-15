use std::path::PathBuf;

// Constants for config file names
const APP_NAME: &str = "CloudHost";
const SERVER_CONFIG_FILE: &str = "server-config.toml";
const TUI_CONFIG_FILE: &str = "tui-config.toml";

/// Get the current directory with fallback
fn get_current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Determines if we're running in development mode
/// This function checks multiple indicators in order of priority:
/// 1. CLOUDHOST_DEV environment variable (explicit override)
/// 2. CARGO environment variable (indicates running via cargo)
/// 3. DEBUG environment variable (common in development)
/// 4. RUST_LOG environment variable (common in development)
pub fn is_dev_mode() -> bool {
    // 1. Explicit environment variable override
    if let Ok(dev_mode) = std::env::var("CLOUDHOST_DEV") {
        return dev_mode == "1" || dev_mode.to_lowercase() == "true";
    }

    // 2. Check if running via cargo (most reliable indicator)
    if std::env::var("CARGO").is_ok() {
        return true;
    }

    // 3. Check for DEBUG environment variable (common in development)
    if std::env::var("DEBUG").is_ok() {
        return true;
    }

    // 4. Check for RUST_LOG (common in development)
    if std::env::var("RUST_LOG").is_ok() {
        return true;
    }

    // Default to production mode if no dev indicators found
    false
}

/// Get the config directory path based on the current mode
pub fn get_config_dir() -> PathBuf {
    if is_dev_mode() {
        // Development mode: use project root
        get_project_root()
    } else {
        // Production mode: use appdata directory
        get_appdata_dir()
    }
}

/// Get the project root directory for development mode
/// In dev mode, we use the current working directory as the project root
fn get_project_root() -> PathBuf {
    get_current_dir()
}

/// Get the appdata directory for production mode
fn get_appdata_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| {
        // Fallback to current directory if appdata is not available
        get_current_dir()
    });
    path.push(APP_NAME);
    path
}

/// Get the full path to the server config file
pub fn get_server_config_path() -> PathBuf {
    let mut path = get_config_dir();
    path.push(SERVER_CONFIG_FILE);
    path
}

/// Get the full path to the TUI config file
pub fn get_tui_config_path() -> PathBuf {
    let mut path = get_config_dir();
    path.push(TUI_CONFIG_FILE);
    path
}

/// Ensure the config directory exists
pub fn ensure_config_dir() -> std::io::Result<()> {
    let config_dir = get_config_dir();
    std::fs::create_dir_all(&config_dir)?;
    Ok(())
}
