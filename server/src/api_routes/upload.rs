use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use axum_extra::extract::Multipart;
use serde_json::json;
use std::path::Path as StdPath;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::cloud::CloudServerState;
use crate::utils::parse_target_path;

/// Generate a unique filename by appending (1), (2), etc. if the file already exists
/// This mimics Windows-style duplicate file handling
fn generate_unique_filename(base_path: &StdPath, filename: &str) -> String {
    if !base_path.exists() {
        return filename.to_string();
    }

    let path = base_path.join(filename);
    if !path.exists() {
        return filename.to_string();
    }

    // Split filename into name and extension
    let (name, extension) = if let Some(dot_pos) = filename.rfind('.') {
        let (name_part, ext_part) = filename.split_at(dot_pos);
        (name_part, ext_part)
    } else {
        (filename, "")
    };

    // Try filename(1), filename(2), etc.
    for i in 1..50 {
        let new_filename = if extension.is_empty() {
            format!("{}({})", name, i)
        } else {
            format!("{}({}){}", name, i, extension)
        };

        let new_path = base_path.join(&new_filename);
        if !new_path.exists() {
            return new_filename;
        }
    }

    // Fallback: use timestamp if we can't find a unique name
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    if extension.is_empty() {
        format!("{}_{}", name, timestamp)
    } else {
        format!("{}_{}{}", name, timestamp, extension)
    }
}

/// Shared function to save uploaded file to the filesystem with duplicate handling
async fn save_uploaded_file(
    server_state: &CloudServerState,
    cloud_folder_name: &str,
    subdirectory_path: &str,
    filename: &str,
    data: &[u8],
) -> Result<(String, String), (StatusCode, Json<serde_json::Value>)> {
    // Find the cloud folder
    let cloud_folder = server_state
        .cloud
        .cloud_folders
        .iter()
        .find(|folder| folder.name == *cloud_folder_name)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Cloud folder not found"
                })),
            )
        })?;

    // Create the directory path for the uploaded file
    let final_path = if subdirectory_path.is_empty() {
        cloud_folder.folder_path.clone()
    } else {
        cloud_folder.folder_path.join(subdirectory_path)
    };

    // Ensure the directory exists
    fs::create_dir_all(&final_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to create directory: {}", e)
            })),
        )
    })?;

    // Generate unique filename to handle duplicates
    let unique_filename = generate_unique_filename(&final_path, filename);
    let upload_path = final_path.join(&unique_filename);

    // Try to create the file with create_new (fails if exists)
    let mut file = match fs::File::create_new(&upload_path).await {
        Ok(file) => file,
        Err(_) => {
            // If create_new fails, fall back to regular create (overwrite)
            fs::File::create(&upload_path).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to create file: {}", e)
                    })),
                )
            })?
        }
    };

    file.write_all(data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("Failed to write file: {}", e)
            })),
        )
    })?;

    Ok((upload_path.to_string_lossy().to_string(), unique_filename))
}

/// Upload a file to a specific path
/// The path should be in format: "cloud_folder_name/subdirectory/path"
pub async fn api_upload_file(
    State(server_state): State<CloudServerState>,
    Path(target_path): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Parse the target path using shared utils
    let parsed_path = parse_target_path(&target_path)?;

    // Process the multipart form data
    if let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Failed to read multipart field: {}", e)
            })),
        )
    })? {
        let filename = field
            .file_name()
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "No filename provided"
                    })),
                )
            })?
            .to_string();

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!("Failed to read file data: {}", e)
                })),
            )
        })?;

        // Use the shared function to save the file
        let (upload_path, actual_filename) = save_uploaded_file(
            &server_state,
            &parsed_path.cloud_folder_name,
            &parsed_path.subdirectory_path,
            &filename,
            &data,
        )
        .await?;

        // Check if filename was changed due to duplicates
        let duplicate_info = if actual_filename != filename {
            json!({
                "original_filename": filename,
                "actual_filename": actual_filename,
                "duplicate_handled": true
            })
        } else {
            json!({
                "duplicate_handled": false
            })
        };

        return Ok(Json(json!({
            "success": true,
            "message": format!("File '{}' uploaded successfully", actual_filename),
            "path": upload_path,
            "filename": actual_filename,
            "duplicate_info": duplicate_info,
            "usage": {
                "path_based": "POST /api/upload/{cloud_folder_name}/{subdirectory_path}",
                "examples": [
                    "POST /api/upload/my_cloud",
                    "POST /api/upload/my_cloud/documents/projects"
                ]
            }
        })));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": "No file provided in the request"
        })),
    ))
}
