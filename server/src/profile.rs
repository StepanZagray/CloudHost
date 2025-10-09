use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFolder {
    pub id: Uuid,
    pub name: String,
    pub folder_path: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CloudFolder {
    pub fn new(name: String, folder_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            folder_path,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn get_appdata_path() -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("CloudTUI");
        path.push("cloudfolders");
        path
    }

    pub fn get_cloudfolder_path(&self) -> PathBuf {
        let mut path = Self::get_appdata_path();
        path.push(self.id.to_string());
        path
    }

    pub fn create_folder(&self) -> Result<(), std::io::Error> {
        let cloudfolder_path = self.get_cloudfolder_path();
        std::fs::create_dir_all(&cloudfolder_path)?;
        Ok(())
    }
}
