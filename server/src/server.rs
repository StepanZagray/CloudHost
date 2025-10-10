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

use crate::{auth::AuthState, cloud_folder::CloudFolder, config::ServerConfig, routes};

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
        let config = ServerConfig::load_from_file().unwrap_or_default();
        let auth_state = Arc::new(crate::auth::AuthState::new(
            config.jwt_secret.clone(),
            config.password_hash.clone(),
            config.password_changed_at,
        ));
        Self {
            cloudfolders: Arc::new(Mutex::new(HashMap::new())),
            server_handle: None,
            port: None,
            config,
            auth_state: Some(auth_state),
            shutdown_tx: None,
        }
    }

    pub fn add_cloudfolder(&self, cloudfolder: CloudFolder) {
        if let Ok(mut cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.insert(cloudfolder.name.clone(), cloudfolder);
        }
    }

    pub fn remove_cloudfolder(&self, cloudfolder_name: &str) {
        if let Ok(mut cloudfolders) = self.cloudfolders.lock() {
            cloudfolders.remove(cloudfolder_name);
        }
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

    pub fn start_server(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        if self.server_handle.is_some() {
            return Err("Server is already running".into());
        }

        // Check if password is set
        if !self.config.has_password() {
            return Err(
                "Cannot start server: No password set. Please set a password in settings first."
                    .into(),
            );
        }

        let cloudfolders = self.cloudfolders.clone();
        let auth_state = self.auth_state.as_ref().unwrap().clone();
        let server_state = ServerState {
            cloudfolders,
            auth_state,
        };

        let app = Router::new()
            .route("/", get(routes::index))
            .route("/login", get(routes::login_page))
            .route("/api/login", post(routes::login))
            .route("/api/status", get(routes::status))
            .route("/api/cloudfolders", get(routes::get_cloudfolders_list))
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
                get(routes::download_file),
            )
            .layer(CorsLayer::permissive())
            .with_state(server_state);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let handle = tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

            // Use the shutdown signal for graceful shutdown
            let shutdown_signal = async {
                let _ = shutdown_rx.await;
            };

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal)
                .await
                .unwrap();
        });

        self.server_handle = Some(handle);
        self.port = Some(port);
        Ok(())
    }

    pub fn stop_server(&mut self) {
        // Send shutdown signal first for graceful shutdown
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            self.port = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    pub fn get_port(&self) -> Option<u16> {
        self.port
    }

    pub fn set_password(&mut self, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        if password.len() < 8 {
            return Err("Password must be at least 8 characters long".into());
        }

        // Check if password is actually changing
        let password_changing = !self.config.verify_password(password);

        self.config.set_password(password)?;
        self.config.save_to_file()?;

        // If password is changing, invalidate all existing tokens
        if password_changing {
            self.invalidate_all_tokens()?;
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
    pub fn invalidate_all_tokens(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    pub fn create_auth_state(&self) -> Arc<crate::auth::AuthState> {
        self.auth_state.as_ref().unwrap().clone()
    }
}
