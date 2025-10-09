use axum::{routing::get, Router};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;
// Remove tracing import to avoid stdout output

use crate::{profile::CloudFolder, routes};

pub struct CloudServer {
    pub cloudfolders: Arc<Mutex<HashMap<String, CloudFolder>>>, // cloudfolder_name -> CloudFolder
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub port: Option<u16>,
}

impl Default for CloudServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudServer {
    pub fn new() -> Self {
        Self {
            cloudfolders: Arc::new(Mutex::new(HashMap::new())),
            server_handle: None,
            port: None,
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

        let cloudfolders = self.cloudfolders.clone();

        let app = Router::new()
            .route("/", get(routes::index))
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
            .with_state(cloudfolders);

        let handle = tokio::spawn(async move {
            let addr = SocketAddr::from(([127, 0, 0, 1], port));

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        self.server_handle = Some(handle);
        self.port = Some(port);
        Ok(())
    }

    pub fn stop_server(&mut self) {
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
}
