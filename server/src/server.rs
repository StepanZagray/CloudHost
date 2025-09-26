use axum::{routing::get, Router};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, services::ServeDir};
// Remove tracing import to avoid stdout output

use crate::{profile::Profile, routes};

pub struct CloudServer {
    pub profiles: Arc<Vec<Profile>>,
    pub current_profile: Option<Profile>,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Default for CloudServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudServer {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(Vec::new()),
            current_profile: None,
            server_handle: None,
        }
    }

    pub fn add_profile(&mut self, profile: Profile) {
        let mut profiles = Vec::new();
        for p in self.profiles.iter() {
            profiles.push(p.clone());
        }
        profiles.push(profile);
        self.profiles = Arc::new(profiles);
    }

    pub fn start_server(
        &mut self,
        profile: Profile,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.server_handle.is_some() {
            return Err("Server is already running".into());
        }

        let profile_path = profile.folder_path.clone();
        if !profile_path.exists() {
            return Err("Profile folder does not exist".into());
        }

        let app = Router::new()
            .route("/", get(routes::index))
            .route("/api/status", get(routes::status))
            .route("/files", get(routes::list_root_directory))
            .route("/files/*path", get(routes::handle_file_or_directory))
            .nest_service("/static", ServeDir::new(&profile_path))
            .layer(CorsLayer::permissive())
            // Remove TraceLayer to avoid stdout output
            // .layer(TraceLayer::new_for_http())
            .with_state(profile.clone());

        let handle = tokio::spawn(async move {
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            // Remove info! macro to avoid stdout output
            // info!("Starting server on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        self.server_handle = Some(handle);
        self.current_profile = Some(profile);
        Ok(())
    }

    pub fn stop_server(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            self.current_profile = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }
}
