use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;

use crate::cloud::CloudServerState;
use crate::error::ServerError;
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

    // Platform-specific deletion handling
    let (deletion_info, recovery, platform) = delete_file(&file_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": e.to_string()
            })),
        )
    })?;

    Ok(Json(json!({
        "success": true,
        "message": format!("File '{}' deleted successfully", filename),
        "original_path": file_path.to_string_lossy(),
        "deletion_info": deletion_info,
        "recovery": recovery,
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

/// Delete a file using platform-appropriate method
async fn delete_file(file_path: &std::path::Path) -> Result<(String, String, String), ServerError> {
    #[cfg(feature = "desktop")]
    {
        delete_file_desktop(file_path).await
    }

    #[cfg(not(feature = "desktop"))]
    {
        delete_file_mobile(file_path).await
    }
}

#[cfg(feature = "desktop")]
async fn delete_file_desktop(
    file_path: &std::path::Path,
) -> Result<(String, String, String), ServerError> {
    use trash;

    trash::delete(file_path)
        .map_err(|e| ServerError::file_system(format!("Failed to move file to OS trash: {}", e)))?;

    Ok((
        "File moved to operating system trash/recycle bin".to_string(),
        "File can be restored from OS trash/recycle bin".to_string(),
        "desktop".to_string(),
    ))
}

#[cfg(not(feature = "desktop"))]
async fn delete_file_mobile(
    file_path: &std::path::Path,
) -> Result<(String, String, String), ServerError> {
    tokio::fs::remove_file(file_path)
        .await
        .map_err(|e| ServerError::file_system(format!("Failed to delete file: {}", e)))?;

    Ok((
        "File permanently deleted".to_string(),
        "File cannot be restored".to_string(),
        "mobile".to_string(),
    ))
}
