use crate::debug_stream::DebugStream;
use crate::{
    auth::AuthState,
    cloud::{Cloud, CloudFolder, CloudServer},
    clouds_config::CloudsConfig,
    error::{ServerError, ServerResult},
};
use std::collections::HashMap;
use std::sync::Arc;

const BASE_PORT: u16 = 3000;

/// Orchestrator - manages multiple clouds and their server lifecycle
pub struct Orchestrator {
    pub running_clouds: HashMap<String, CloudServer>, // cloud_name -> CloudServer (running)
    pub clouds_config: CloudsConfig,
    pub next_port: u16,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    pub fn new() -> Self {
        let clouds_config =
            CloudsConfig::load_from_file().unwrap_or_else(|_e| CloudsConfig::default());

        // Note: data directory check removed as it's not critical

        Self {
            running_clouds: HashMap::new(),
            clouds_config,
            next_port: BASE_PORT,
        }
    }

    /// Start a server for a specific cloud
    pub async fn start_cloud(&mut self, cloud_name: &str) -> ServerResult<u16> {
        // Check if already running
        if self.running_clouds.contains_key(cloud_name) {
            return Err(ServerError::ServerAlreadyRunning);
        }

        // Get the cloud from config
        let cloud = self
            .clouds_config
            .get_cloud(cloud_name)
            .ok_or_else(|| ServerError::Validation(format!("Cloud '{}' not found", cloud_name)))?;

        // Check if cloud has password
        if !cloud.has_password() {
            return Err(ServerError::Validation(format!(
                "Cloud '{}' has no password set. Set a password first.",
                cloud_name
            )));
        }

        // Verify cloud has cloud folders
        if cloud.cloud_folders.is_empty() {
            return Err(ServerError::Validation(format!(
                "Cloud '{}' has no cloud folders",
                cloud_name
            )));
        }

        // Create AuthState for this cloud
        let auth_state = Arc::new(AuthState::new(
            cloud.jwt_secret.clone(),
            cloud.password.clone(),
            cloud.password_changed_at,
        ));

        // Assign port
        let port = self.next_port;
        self.next_port += 1;

        // Create and start the cloud server
        let mut cloud_server = CloudServer::new(cloud.clone(), port);
        cloud_server
            .start_server(auth_state, Arc::new(DebugStream::new(100)))
            .await?;

        self.running_clouds
            .insert(cloud_name.to_string(), cloud_server);

        Ok(port)
    }

    /// Stop a specific cloud's server
    pub async fn stop_cloud(&mut self, cloud_name: &str) -> ServerResult<()> {
        if let Some(mut cloud_server) = self.running_clouds.remove(cloud_name) {
            cloud_server.stop_server().await?;
            Ok(())
        } else {
            Err(ServerError::ServerNotRunning)
        }
    }

    /// Stop all running servers
    pub async fn stop_all(&mut self) -> ServerResult<()> {
        let cloud_names: Vec<String> = self.running_clouds.keys().cloned().collect();

        for cloud_name in cloud_names {
            let _ = self.stop_cloud(&cloud_name).await;
        }

        self.running_clouds.clear();
        self.next_port = BASE_PORT;

        Ok(())
    }

    /// Check if a specific cloud is running
    pub fn is_cloud_running(&self, cloud_name: &str) -> bool {
        self.running_clouds.contains_key(cloud_name)
    }

    /// Check if any cloud is running
    pub fn is_any_running(&self) -> bool {
        !self.running_clouds.is_empty()
    }

    /// Get port for a specific cloud
    pub fn get_cloud_port(&self, cloud_name: &str) -> Option<u16> {
        self.running_clouds
            .get(cloud_name)
            .and_then(|cloud_server| cloud_server.get_server_port())
    }

    /// Get full server URL for a specific cloud
    pub fn get_cloud_server_url(&self, cloud_name: &str) -> Option<String> {
        self.running_clouds
            .get(cloud_name)
            .and_then(|cloud_server| cloud_server.get_server_url())
    }

    /// Get all running clouds with their ports
    pub fn get_running_clouds(&self) -> HashMap<String, u16> {
        self.running_clouds
            .iter()
            .filter_map(|(name, cloud_server)| {
                cloud_server
                    .get_server_port()
                    .map(|port| (name.clone(), port))
            })
            .collect()
    }

    // ========== Clouds Management ==========

    /// Get all clouds
    pub fn get_clouds(&self) -> Vec<Cloud> {
        self.clouds_config.get_clouds().to_vec()
    }

    /// Get a specific cloud
    pub fn get_cloud(&self, cloud_name: &str) -> Option<Cloud> {
        self.clouds_config.get_cloud(cloud_name).cloned()
    }

    /// Add a cloud
    pub fn add_cloud(&mut self, cloud: Cloud) -> ServerResult<()> {
        self.clouds_config.add_cloud(cloud)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    /// Remove a cloud
    pub fn remove_cloud(&mut self, cloud_name: &str) -> ServerResult<()> {
        self.clouds_config.remove_cloud(cloud_name)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    /// Update a cloud
    pub fn update_cloud(&mut self, old_name: &str, new_cloud: Cloud) -> ServerResult<()> {
        self.clouds_config.update_cloud(old_name, new_cloud)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    // ========== Cloud Folders Management ==========

    /// Get all cloud folders
    pub fn get_cloud_folders(&self) -> Vec<CloudFolder> {
        self.clouds_config.get_cloud_folders().to_vec()
    }

    /// Add a cloud folder
    pub fn add_cloud_folder(&mut self, cloud_folder: CloudFolder) -> ServerResult<()> {
        self.clouds_config.add_cloud_folder(cloud_folder)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    /// Remove a cloud folder
    pub fn remove_cloud_folder(&mut self, cloud_folder_name: &str) -> ServerResult<()> {
        self.clouds_config.remove_cloud_folder(cloud_folder_name)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    /// Update a cloud folder
    pub fn update_cloud_folder(
        &mut self,
        old_name: &str,
        new_cloud_folder: CloudFolder,
    ) -> ServerResult<()> {
        self.clouds_config
            .update_cloud_folder(old_name, new_cloud_folder)?;
        self.clouds_config.save_to_file()?;
        Ok(())
    }

    // ========== Cloud Password Management ==========

    /// Set password for a specific cloud
    pub fn set_cloud_password(&mut self, cloud_name: &str, password: &str) -> ServerResult<()> {
        // Get the cloud from config
        let mut cloud = self
            .clouds_config
            .get_cloud(cloud_name)
            .ok_or_else(|| ServerError::Validation(format!("Cloud '{}' not found", cloud_name)))?
            .clone();

        // Set the password
        cloud
            .set_password(password)
            .map_err(|e| ServerError::Internal(format!("Failed to set password: {}", e)))?;

        // Update the cloud in config
        self.clouds_config.update_cloud(cloud_name, cloud)?;
        self.clouds_config.save_to_file()?;

        Ok(())
    }

    /// Check if a cloud has a password
    pub fn cloud_has_password(&self, cloud_name: &str) -> bool {
        self.clouds_config
            .get_cloud(cloud_name)
            .map(|c| c.has_password())
            .unwrap_or(false)
    }

    /// Verify password for a cloud
    pub fn verify_cloud_password(&self, cloud_name: &str, password: &str) -> bool {
        self.clouds_config
            .get_cloud(cloud_name)
            .map(|c| c.verify_password(password))
            .unwrap_or(false)
    }

    /// Get debug logs for a specific cloud
    pub async fn get_cloud_debug_logs(
        &self,
        cloud_name: &str,
    ) -> Vec<crate::debug_stream::DebugMessage> {
        if let Some(cloud_server) = self.running_clouds.get(cloud_name) {
            if let Some(debug_stream) = &cloud_server.debug_stream {
                debug_stream.get_history().await
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
    // ========== Cloud config Management ==========

    /// Reload the clouds config and restart affected clouds
    pub async fn reload_config(&mut self) -> ServerResult<()> {
        let running_clouds: Vec<String> = self.running_clouds.keys().cloned().collect();
        self.clouds_config =
            CloudsConfig::load_from_file().unwrap_or_else(|_e| CloudsConfig::default()); // Check which clouds are still valid after reload
        let valid_clouds: std::collections::HashSet<String> = self
            .clouds_config
            .clouds
            .iter()
            .map(|c| c.name.clone())
            .collect();

        // Stop clouds that are no longer in config
        for cloud_name in &running_clouds {
            if !valid_clouds.contains(cloud_name) {
                if let Some(mut cloud_server) = self.running_clouds.remove(cloud_name) {
                    if let Err(e) = cloud_server.stop_server().await {
                        eprintln!(
                            "Failed to stop cloud '{}' during config reload: {}",
                            cloud_name, e
                        );
                    }
                }
            }
        }

        // Restart clouds that were running and are still valid
        for cloud_name in &running_clouds {
            if valid_clouds.contains(cloud_name) {
                // Stop the existing server
                if let Some(mut cloud_server) = self.running_clouds.remove(cloud_name) {
                    if let Err(e) = cloud_server.stop_server().await {
                        eprintln!("Failed to stop cloud '{}' for restart: {}", cloud_name, e);
                    }
                }

                // Start the server with new config
                if let Err(e) = self.start_cloud(cloud_name).await {
                    eprintln!(
                        "Failed to restart cloud '{}' after config reload: {}",
                        cloud_name, e
                    );
                }
            }
        }

        Ok(())
    }
}
