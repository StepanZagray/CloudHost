use crate::cloud::{Cloud, CloudFolder};
use crate::config_paths;
use crate::error::{ServerError, ServerResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CLOUDS_CONFIG_FILE: &str = "clouds-config.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudsConfig {
    pub cloud_folders: Vec<CloudFolder>,
    pub clouds: Vec<Cloud>,
}

impl CloudsConfig {
    /// Load clouds config from file
    pub fn load_from_file() -> ServerResult<Self> {
        let config_path = Self::get_config_path();

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path)?;
        let config: CloudsConfig = toml::from_str(&config_str)?;
        Ok(config)
    }

    /// Save clouds config to file
    pub fn save_to_file(&self) -> ServerResult<()> {
        let config_path = Self::get_config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config_str = toml::to_string_pretty(self)?;
        fs::write(&config_path, config_str)?;
        Ok(())
    }

    /// Get the path to the clouds config file
    fn get_config_path() -> PathBuf {
        config_paths::get_config_dir().join(CLOUDS_CONFIG_FILE)
    }

    /// Add a cloud folder
    pub fn add_cloud_folder(&mut self, cloud_folder: CloudFolder) -> ServerResult<()> {
        if self
            .cloud_folders
            .iter()
            .any(|f| f.name == cloud_folder.name)
        {
            return Err(ServerError::Validation(format!(
                "Cloud folder '{}' already exists",
                cloud_folder.name
            )));
        }
        self.cloud_folders.push(cloud_folder);
        Ok(())
    }

    /// Remove a cloud folder
    pub fn remove_cloud_folder(&mut self, cloud_folder_name: &str) -> ServerResult<()> {
        if let Some(pos) = self
            .cloud_folders
            .iter()
            .position(|f| f.name == cloud_folder_name)
        {
            self.cloud_folders.remove(pos);
            Ok(())
        } else {
            Err(ServerError::Validation(format!(
                "Cloud folder '{}' not found",
                cloud_folder_name
            )))
        }
    }

    /// Update a cloud folder
    pub fn update_cloud_folder(
        &mut self,
        old_name: &str,
        new_cloud_folder: CloudFolder,
    ) -> ServerResult<()> {
        // Check if new name conflicts with another cloud folder
        if old_name != new_cloud_folder.name
            && self
                .cloud_folders
                .iter()
                .any(|f| f.name == new_cloud_folder.name)
        {
            return Err(ServerError::Validation(format!(
                "Cloud folder '{}' already exists",
                new_cloud_folder.name
            )));
        }

        if let Some(cloud_folder) = self.cloud_folders.iter_mut().find(|f| f.name == old_name) {
            *cloud_folder = new_cloud_folder;
            Ok(())
        } else {
            Err(ServerError::Validation(format!(
                "Cloud folder '{}' not found",
                old_name
            )))
        }
    }

    /// Get all cloud folders
    pub fn get_cloud_folders(&self) -> &[CloudFolder] {
        &self.cloud_folders
    }

    /// Add a cloud
    pub fn add_cloud(&mut self, cloud: Cloud) -> ServerResult<()> {
        if self.clouds.iter().any(|c| c.name == cloud.name) {
            return Err(ServerError::Validation(format!(
                "Cloud '{}' already exists",
                cloud.name
            )));
        }
        if cloud.cloud_folders.is_empty() {
            return Err(ServerError::Validation(
                "Cloud must contain at least one cloud folder".to_string(),
            ));
        }
        self.clouds.push(cloud);
        Ok(())
    }

    /// Remove a cloud
    pub fn remove_cloud(&mut self, cloud_name: &str) -> ServerResult<()> {
        if let Some(pos) = self.clouds.iter().position(|c| c.name == cloud_name) {
            self.clouds.remove(pos);
            Ok(())
        } else {
            Err(ServerError::Validation(format!(
                "Cloud '{}' not found",
                cloud_name
            )))
        }
    }

    /// Update a cloud
    pub fn update_cloud(&mut self, old_name: &str, new_cloud: Cloud) -> ServerResult<()> {
        // Check if new name conflicts with another cloud
        if old_name != new_cloud.name && self.clouds.iter().any(|c| c.name == new_cloud.name) {
            return Err(ServerError::Validation(format!(
                "Cloud '{}' already exists",
                new_cloud.name
            )));
        }

        if new_cloud.cloud_folders.is_empty() {
            return Err(ServerError::Validation(
                "Cloud must contain at least one folder".to_string(),
            ));
        }

        if let Some(cloud) = self.clouds.iter_mut().find(|c| c.name == old_name) {
            *cloud = new_cloud;
            Ok(())
        } else {
            Err(ServerError::Validation(format!(
                "Cloud '{}' not found",
                old_name
            )))
        }
    }

    /// Get all clouds
    pub fn get_clouds(&self) -> &[Cloud] {
        &self.clouds
    }

    /// Get a specific cloud
    pub fn get_cloud(&self, cloud_name: &str) -> Option<&Cloud> {
        self.clouds.iter().find(|c| c.name == cloud_name)
    }
}
