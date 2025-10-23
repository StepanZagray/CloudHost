use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;

#[cfg(feature = "desktop")]
use trash;

use crate::cloud::CloudServerState;
use crate::utils::{
    construct_file_path, find_cloud_folder, parse_target_path, validate_file_exists,
};

/// Delete a file by moving it to trash
/// The path should be in format: "cloud_folder_name/subdirectory/path/filename"
pub async fn api_delete_file(
    State(server_state): State<CloudServerState>,
    Path(target_path): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Parse the target path using shared utils
    let parsed_path = parse_target_path(&target_path)?;

    // Ensure we have a filename for deletion
    let filename = parsed_path.filename.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Filename required for deletion. Expected: cloud_folder_name/subdirectory/path/filename"
            })),
        )
    })?;

    // Find the cloud folder using shared utils
    let cloud_folder = find_cloud_folder(&server_state, &parsed_path.cloud_folder_name)?;

    // Construct the full file path using shared utils
    let file_path = construct_file_path(cloud_folder, &parsed_path.subdirectory_path, &filename);

    // Validate file exists using shared utils
    validate_file_exists(&file_path).await?;

    // Platform-specific trash handling
    let (trash_info, recovery_info, platform) =
        move_to_trash(&file_path, cloud_folder, &filename).await?;

    Ok(Json(json!({
        "success": true,
        "message": format!("File '{}' moved to trash successfully", filename),
        "original_path": file_path.to_string_lossy(),
        "trash_info": trash_info,
        "recovery": recovery_info,
        "platform": platform,
        "usage": {
            "delete": "DELETE /api/delete/{cloud_folder_name}/{subdirectory_path}/{filename}",
            "examples": [
                "DELETE /api/delete/my_cloud/document.pdf",
                "DELETE /api/delete/my_cloud/documents/projects/report.pdf"
            ]
        }
    })))
}

/// Platform-specific trash handling
async fn move_to_trash(
    file_path: &std::path::Path,
    _cloud_folder: &crate::cloud::CloudFolder,
    _filename: &str,
) -> Result<(String, String, String), (StatusCode, Json<serde_json::Value>)> {
    if cfg!(feature = "desktop") {
        // Desktop platforms (Windows, macOS, Linux) - use OS trash
        #[cfg(feature = "desktop")]
        {
            if let Err(e) = trash::delete(file_path) {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to move file to OS trash: {}", e)
                    })),
                ));
            }
        }

        Ok((
            "File moved to operating system trash/recycle bin".to_string(),
            "File can be restored from OS trash/recycle bin".to_string(),
            "desktop".to_string(),
        ))
    } else {
        // Mobile platforms (Android, iOS) - permanently delete file
        if let Err(e) = tokio::fs::remove_file(file_path).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to delete file: {}", e)
                })),
            ));
        }

        Ok((
            "File permanently deleted".to_string(),
            "File cannot be restored".to_string(),
            "mobile".to_string(),
        ))
    }
}
