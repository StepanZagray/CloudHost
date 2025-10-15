use std::path::PathBuf;

/// Determines if we're running in development mode
/// Development mode is detected by the presence of a Cargo.toml file in the current directory
/// or any parent directory, indicating we're in a Rust project
pub fn is_dev_mode() -> bool {
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Check current directory and up to 3 parent directories for Cargo.toml
    for _ in 0..4 {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            return true;
        }

        if !current_dir.pop() {
            break;
        }
    }

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

/// Get the project root directory (where Cargo.toml is located)
fn get_project_root() -> PathBuf {
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Walk up directories to find Cargo.toml
    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            return current_dir;
        }

        if !current_dir.pop() {
            break;
        }
    }

    // Fallback to current directory if no Cargo.toml found
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Get the appdata directory for production mode
fn get_appdata_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| {
        // Fallback to current directory if appdata is not available
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
    path.push("CloudHost");
    path
}

/// Get the full path to the server config file
pub fn get_server_config_path() -> PathBuf {
    let mut path = get_config_dir();
    path.push("server-config.toml");
    path
}

/// Get the full path to the TUI config file
pub fn get_tui_config_path() -> PathBuf {
    let mut path = get_config_dir();
    path.push("tui-config.toml");
    path
}

/// Ensure the config directory exists
pub fn ensure_config_dir() -> std::io::Result<()> {
    let config_dir = get_config_dir();
    std::fs::create_dir_all(&config_dir)?;
    Ok(())
}
