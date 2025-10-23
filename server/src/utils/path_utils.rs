use axum::{http::StatusCode, response::Json};
use serde_json::json;
use std::path::PathBuf;

use crate::cloud::CloudServerState;

/// Parsed path components for API operations
#[derive(Debug)]
pub struct ParsedPath {
    pub cloud_folder_name: String,
    pub subdirectory_path: String,
    pub filename: Option<String>,
}

/// Parse a target path into components
/// Expected format: "cloud_folder_name/subdirectory/path" or "cloud_folder_name/subdirectory/path/filename"
pub fn parse_target_path(
    target_path: &str,
) -> Result<ParsedPath, (StatusCode, Json<serde_json::Value>)> {
    let path_parts: Vec<&str> = target_path.split('/').collect();

    if path_parts.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid path format. Expected: cloud_folder_name/subdirectory/path"
            })),
        ));
    }

    let cloud_folder_name = path_parts[0].to_string();
    let remaining_parts = &path_parts[1..];

    if remaining_parts.is_empty() {
        // Just cloud folder name, no subdirectory or filename
        return Ok(ParsedPath {
            cloud_folder_name,
            subdirectory_path: String::new(),
            filename: None,
        });
    }

    // Check if last part looks like a filename (has extension or is a single word)
    let last_part = remaining_parts.last().map_or("", |v| v);
    let has_extension = last_part.contains('.');
    let is_likely_filename = has_extension || remaining_parts.len() == 1;

    if is_likely_filename {
        // Last part is filename
        let filename = last_part.to_string();
        let subdirectory_path = if remaining_parts.len() > 1 {
            remaining_parts[..remaining_parts.len() - 1].join("/")
        } else {
            String::new()
        };

        Ok(ParsedPath {
            cloud_folder_name,
            subdirectory_path,
            filename: Some(filename),
        })
    } else {
        // No filename, just subdirectory path
        let subdirectory_path = remaining_parts.join("/");

        Ok(ParsedPath {
            cloud_folder_name,
            subdirectory_path,
            filename: None,
        })
    }
}

/// Find a cloud folder by name
pub fn find_cloud_folder<'a>(
    server_state: &'a CloudServerState,
    cloud_folder_name: &str,
) -> Result<&'a crate::cloud::CloudFolder, (StatusCode, Json<serde_json::Value>)> {
    server_state
        .cloud
        .cloud_folders
        .iter()
        .find(|folder| folder.name == cloud_folder_name)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Cloud folder not found"
                })),
            )
        })
}

/// Construct the full file path from parsed components
pub fn construct_file_path(
    cloud_folder: &crate::cloud::CloudFolder,
    subdirectory_path: &str,
    filename: &str,
) -> PathBuf {
    let base_path = cloud_folder.folder_path.clone();
    if subdirectory_path.is_empty() {
        base_path.join(filename)
    } else {
        base_path.join(subdirectory_path).join(filename)
    }
}

/// Construct the full directory path from parsed components
pub fn construct_directory_path(
    cloud_folder: &crate::cloud::CloudFolder,
    subdirectory_path: &str,
) -> PathBuf {
    let base_path = cloud_folder.folder_path.clone();
    if subdirectory_path.is_empty() {
        base_path
    } else {
        base_path.join(subdirectory_path)
    }
}

/// Validate that a file exists
pub async fn validate_file_exists(
    file_path: &std::path::Path,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !file_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "File not found"
            })),
        ));
    }
    Ok(())
}

/// Create directory if it doesn't exist
pub async fn ensure_directory_exists(
    dir_path: &std::path::Path,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = tokio::fs::create_dir_all(dir_path).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create directory: {}", e)
            })),
        ));
    }
    Ok(())
}
