use axum::{
    routing::{get, post},
    Router,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;
// Remove tracing import to avoid stdout output

use crate::{
    auth::AuthState,
    cloud_folder::CloudFolder,
    config::ServerConfig,
    error::{ServerError, ServerResult},
    routes,
};
use cloudhost_shared::debug_stream::{log_error, log_info};

// Combined state for the server
#[derive(Clone)]
pub struct ServerState {
    pub cloudfolders: Arc<Mutex<HashMap<String, CloudFolder>>>,
    pub auth_state: Arc<AuthState>,
}

pub struct CloudServer {
    pub cloudfolders: Arc<Mutex<HashMap<String, CloudFolder>>>, // cloudfolder_name -> CloudFolder
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub port: Option<u16>,
    pub config: ServerConfig,
    pub auth_state: Option<Arc<crate::auth::AuthState>>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Default for CloudServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudServer {
    pub fn new() -> Self {
        let config = ServerConfig::load_from_file().unwrap_or_else(|e| {
            // Log config loading issues on server side
            tokio::spawn(async move {
                log_info(
                    "Server",
                    &format!("Using default server config (file not found: {})", e),
                )
                .await;
            });
            ServerConfig::default()
        });
        let auth_state = Arc::new(crate::auth::AuthState::new(
            config.jwt_secret.clone(),
            config.password_hash.clone(),
            config.password_changed_at,
        ));

        // Log data directory status
        if dirs::data_dir().is_none() {
            tokio::spawn(async move {
                log_info(
                    "Server",
                    "Could not determine data directory, using current directory",
                )
                .await;
            });
        }
        // Load cloudfolders from config into HashMap for runtime access
        let mut cloudfolders_map = HashMap::new();
        for cloudfolder in &config.cloudfolders {
            cloudfolders_map.insert(cloudfolder.name.clone(), cloudfolder.clone());
        }

        Self {
            cloudfolders: Arc::new(Mutex::new(cloudfolders_map)),
            server_handle: None,
            port: None,
            config,
            auth_state: Some(auth_state),
            shutdown_tx: None,
        }
    }

    pub fn add_cloudfolder(&mut self, cloudfolder: CloudFolder) -> ServerResult<()> {
        // Add to runtime HashMap
        if let Ok(mut cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.insert(cloudfolder.name.clone(), cloudfolder.clone());
        }

        // Add to config and save
        self.config
            .add_cloudfolder(cloudfolder.name.clone(), cloudfolder.folder_path.clone());
        self.config.save_to_file()?;

        Ok(())
    }

    pub fn remove_cloudfolder(&mut self, cloudfolder_name: &str) -> ServerResult<()> {
        // Remove from runtime HashMap
        if let Ok(mut cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.remove(cloudfolder_name);
        }

        // Remove from config and save
        self.config.remove_cloudfolder(cloudfolder_name);
        self.config.save_to_file()?;

        Ok(())
    }

    pub fn get_cloudfolders(&self) -> Vec<CloudFolder> {
        if let Ok(cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_cloudfolder(&self, name: &str) -> Option<CloudFolder> {
        if let Ok(cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.get(name).cloned()
        } else {
            None
        }
    }

    pub fn start_server(&mut self, port: u16) -> ServerResult<()> {
        if self.server_handle.is_some() {
            tokio::spawn(async move {
                log_info("Server", "⚠️ Server is already running").await;
            });
            return Err(ServerError::internal("Server is already running"));
        }

        // Check if password is set
        if !self.config.has_password() {
            tokio::spawn(async move {
                log_error("Server", "❌ Cannot start server: No password set").await;
                log_info(
                    "Server",
                    "💡 Go to Settings tab and press 'p' to create a password",
                )
                .await;
            });
            return Err(ServerError::configuration(
                "Cannot start server: No password set. Please set a password in settings first.",
            ));
        }

        let cloudfolders = self.cloudfolders.clone();
        let auth_state = self
            .auth_state
            .as_ref()
            .ok_or_else(|| ServerError::internal("Auth state not initialized"))?
            .clone();

        // Get cloudfolders for logging before moving server_state
        let cloudfolders_for_logging = cloudfolders.lock().unwrap().clone();
        let cloudfolders_count = cloudfolders_for_logging.len();

        let server_state = ServerState {
            cloudfolders,
            auth_state,
        };

        let app = Router::new()
            .route("/", get(routes::index))
            .route("/login", get(routes::login_page))
            .route("/api/login", post(routes::login))
            .route("/api/status", get(routes::api_index))
            .route("/api/cloudfolders", get(routes::api_index))
            .route("/:cloudfolder_name", get(routes::show_cloudfolder_info))
            .route(
                "/:cloudfolder_name/files",
                get(routes::list_cloudfolder_files),
            )
            .route(
                "/:cloudfolder_name/files/*path",
                get(routes::browse_file_or_directory),
            )
            .route(
                "/:cloudfolder_name/static/*path",
                get(routes::serve_static_file),
            )
            .route(
                "/api/:cloudfolder_name/static/*path",
                get(routes::serve_static_file),
            )
            .layer(CorsLayer::permissive())
            .with_state(server_state);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let handle = tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    log_error("Server", &format!("Failed to bind to address: {}", e)).await;
                    return;
                }
            };

            // Use the shutdown signal for graceful shutdown
            let shutdown_signal = async {
                let _ = shutdown_rx.await;
            };

            if let Err(e) = axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal)
                .await
            {
                log_error("Server", &format!("Server error: {}", e)).await;
            }
        });

        self.server_handle = Some(handle);
        self.port = Some(port);

        // Log server startup with details
        tokio::spawn(async move {
            log_info("Server", "🚀 Starting CloudTUI server").await;
            log_info(
                "Server",
                &format!("🌐 Server started on http://127.0.0.1:{}", port),
            )
            .await;
            log_info(
                "Server",
                &format!("📊 Serving {} cloudfolders", cloudfolders_count),
            )
            .await;

            // Log each cloudfolder
            for cloudfolder in cloudfolders_for_logging.values() {
                log_info(
                    "Server",
                    &format!(
                        "📁 Cloud Folder '{}': http://127.0.0.1:{}/{}",
                        cloudfolder.name, port, cloudfolder.name
                    ),
                )
                .await;
            }

            log_info(
                "Server",
                &format!("🔗 Server status: http://127.0.0.1:{}/api/status", port),
            )
            .await;
        });

        Ok(())
    }

    pub async fn stop_server(&mut self) {
        // Send shutdown signal first for graceful shutdown
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            self.port = None;

            // Log server shutdown
            let port = self.port;
            tokio::spawn(async move {
                if let Some(port) = port {
                    log_info("Server", &format!("🛑 Stopped server on port {}", port)).await;
                } else {
                    log_info("Server", "🛑 Stopped server").await;
                }
                log_info("Server", "📴 All connections closed").await;
            });
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    pub fn get_port(&self) -> Option<u16> {
        self.port
    }

    pub fn set_password(&mut self, password: &str) -> ServerResult<()> {
        if password.len() < 8 {
            return Err(ServerError::validation(
                "Password must be at least 8 characters long",
            ));
        }

        // Check if password is actually changing
        let password_changing = !self.config.verify_password(password);

        self.config.set_password(password)?;
        self.config.save_to_file()?;

        // If password is changing, invalidate all existing tokens
        if password_changing {
            self.invalidate_all_tokens()?;
            tokio::spawn(async move {
                log_info("Server", "Password changed and all tokens invalidated").await;
            });
        } else {
            tokio::spawn(async move {
                log_info("Server", "Password set successfully").await;
            });
        }

        Ok(())
    }

    pub fn has_password(&self) -> bool {
        self.config.has_password()
    }

    pub fn get_password_changed_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.config.password_changed_at
    }

    /// Invalidates all existing tokens by generating a new JWT secret
    /// This should be called whenever the password is changed
    pub fn invalidate_all_tokens(&mut self) -> ServerResult<()> {
        // Generate a new JWT secret to invalidate all existing tokens
        self.config.jwt_secret = crate::config::generate_jwt_secret();
        self.config.password_changed_at = Some(chrono::Utc::now());
        self.config.save_to_file()?;

        // Recreate the AuthState with the new secret
        self.auth_state = Some(Arc::new(crate::auth::AuthState::new(
            self.config.jwt_secret.clone(),
            self.config.password_hash.clone(),
            self.config.password_changed_at,
        )));

        Ok(())
    }

    /// Creates an AuthState for token-based authentication
    /// This can be used when implementing full authentication middleware
    pub fn create_auth_state(&self) -> Option<Arc<crate::auth::AuthState>> {
        self.auth_state.as_ref().map(|auth| auth.clone())
    }
}
