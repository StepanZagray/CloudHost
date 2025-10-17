use crate::auth::AuthState;
use crate::debug_stream::DebugStream;
use crate::error::{ServerError, ServerResult};
use crate::routes;
use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;

/// Represents a single cloud folder with a name and path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudFolder {
    pub name: String,
    pub folder_path: PathBuf,
}

impl CloudFolder {
    pub fn new(name: String, folder_path: PathBuf) -> Self {
        Self { name, folder_path }
    }
}

/// Represents a cloud containing multiple cloud folders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cloud {
    pub name: String,
    pub cloud_folders: Vec<CloudFolder>,
    pub password_hash: Option<String>,
    pub password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub jwt_secret: String,
}

/// Runtime server state for a cloud (not serialized)
pub struct CloudServer {
    pub cloud: Cloud,
    pub port: u16,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub auth_state: Option<Arc<AuthState>>,
    pub debug_stream: Option<Arc<DebugStream>>,
}

/// State for an individual cloud server (used in routes)
#[derive(Clone)]
pub struct CloudServerState {
    pub cloud: Arc<Cloud>,
    pub auth_state: Arc<AuthState>,
}

impl Cloud {
    pub fn new(name: String, cloud_folders: Vec<CloudFolder>) -> Self {
        Self {
            name: name.clone(),
            cloud_folders,
            password_hash: None,
            password_changed_at: None,
            jwt_secret: Self::generate_jwt_secret(&name),
        }
    }

    /// Generate a unique JWT secret for this cloud
    fn generate_jwt_secret(cloud_name: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut hasher = DefaultHasher::new();
        cloud_name.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        format!("cloud-{}-secret-{:x}", cloud_name, hasher.finish())
    }

    /// Set password for this cloud
    pub fn set_password(&mut self, password: &str) -> Result<(), bcrypt::BcryptError> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        self.password_hash = Some(hash);
        self.password_changed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Check if password is set
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(ref hash) = self.password_hash {
            bcrypt::verify(password, hash).unwrap_or(false)
        } else {
            false
        }
    }

    /// Check if this cloud contains a cloud folder with the given name
    pub fn has_cloud_folder(&self, cloud_folder_name: &str) -> bool {
        self.cloud_folders
            .iter()
            .any(|f| f.name == cloud_folder_name)
    }

    /// Get a cloud folder by name
    pub fn get_cloud_folder(&self, cloud_folder_name: &str) -> Option<&CloudFolder> {
        self.cloud_folders
            .iter()
            .find(|f| f.name == cloud_folder_name)
    }

    /// Add a cloud folder to this cloud
    pub fn add_cloud_folder(&mut self, cloud_folder: CloudFolder) {
        if !self.has_cloud_folder(&cloud_folder.name) {
            self.cloud_folders.push(cloud_folder);
        }
    }

    /// Remove a cloud folder from this cloud
    pub fn remove_cloud_folder(&mut self, cloud_folder_name: &str) -> bool {
        if let Some(pos) = self
            .cloud_folders
            .iter()
            .position(|f| f.name == cloud_folder_name)
        {
            self.cloud_folders.remove(pos);
            true
        } else {
            false
        }
    }
}

impl CloudServer {
    pub fn new(cloud: Cloud, port: u16) -> Self {
        Self {
            cloud,
            port,
            server_handle: None,
            shutdown_tx: None,
            auth_state: None,
            debug_stream: None,
        }
    }

    // ========== Server Management ==========

    /// Start the cloud server
    pub async fn start_server(
        &mut self,
        auth_state: Arc<AuthState>,
        debug_stream: Arc<DebugStream>,
    ) -> ServerResult<()> {
        if self.server_handle.is_some() {
            return Err(ServerError::ServerAlreadyRunning);
        }

        // Verify all cloud folder paths exist
        for cloud_folder in &self.cloud.cloud_folders {
            if !cloud_folder.folder_path.exists() {
                return Err(ServerError::InvalidPath(format!(
                    "Cloud folder path does not exist: {}",
                    cloud_folder.folder_path.display()
                )));
            }
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let state = CloudServerState {
            cloud: Arc::new(self.cloud.clone()),
            auth_state: auth_state.clone(),
        };

        let mut app = Router::new()
            .route("/", get(routes::index))
            .route("/login", get(routes::login_page))
            .route("/api/login", post(routes::login))
            .route("/api", get(routes::api_index))
            .route(
                "/api/:cloud_folder_name",
                get(routes::get_cloud_folder_info),
            )
            .route(
                "/api/:cloud_folder_name/files",
                get(routes::api_list_cloud_folder_files),
            )
            .route(
                "/api/:cloud_folder_name/files/*path",
                get(routes::api_browse_file_or_directory),
            )
            .route(
                "/api/:cloud_folder_name/static/*path",
                get(routes::serve_static_file),
            );

        // Add dynamic routes for cloud folders
        app = app
            .route(
                "/web/:cloud_folder_name/files",
                get(routes::list_cloud_folder_files),
            )
            .route(
                "/web/:cloud_folder_name/files/*path",
                get(routes::browse_file_or_directory),
            );

        let app = app.layer(CorsLayer::permissive()).with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let cloud_name_clone = self.cloud.name.clone();

        debug_stream
            .info(
                "Cloud",
                &format!(
                    "Starting cloud '{}' on port {}",
                    cloud_name_clone, self.port
                ),
            )
            .await;

        let debug_stream_clone = debug_stream.clone();
        let server_handle = tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    debug_stream_clone
                        .error("Cloud", &format!("Failed to bind to {}: {}", addr, e))
                        .await;
                    return;
                }
            };

            debug_stream_clone
                .info(
                    "Cloud",
                    &format!("Cloud '{}' listening on {}", cloud_name_clone, addr),
                )
                .await;

            let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                shutdown_rx.await.ok();
            });

            if let Err(e) = server.await {
                debug_stream_clone
                    .error(
                        "Cloud",
                        &format!("Cloud server error for '{}': {}", cloud_name_clone, e),
                    )
                    .await;
            }
        });

        self.server_handle = Some(server_handle);
        self.shutdown_tx = Some(shutdown_tx);
        self.auth_state = Some(auth_state);
        self.debug_stream = Some(debug_stream);

        Ok(())
    }

    /// Stop the cloud server
    pub async fn stop_server(&mut self) -> ServerResult<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());

            if let Some(handle) = self.server_handle.take() {
                handle.await.map_err(|e| {
                    ServerError::ServerError(format!(
                        "Failed to wait for cloud server shutdown: {}",
                        e
                    ))
                })?;
            }

            if let Some(debug_stream) = &self.debug_stream {
                debug_stream
                    .info(
                        "Cloud",
                        &format!("Cloud '{}' stopped on port {}", self.cloud.name, self.port),
                    )
                    .await;
            }
            Ok(())
        } else {
            Err(ServerError::ServerNotRunning)
        }
    }

    /// Check if the cloud server is running
    pub fn is_server_running(&self) -> bool {
        self.server_handle.is_some()
    }

    /// Get the cloud server port
    pub fn get_server_port(&self) -> Option<u16> {
        if self.is_server_running() {
            Some(self.port)
        } else {
            None
        }
    }

    /// Get the full server URL for this cloud
    pub fn get_server_url(&self) -> Option<String> {
        if self.is_server_running() {
            Some(format!("http://localhost:{}", self.port))
        } else {
            None
        }
    }
}
