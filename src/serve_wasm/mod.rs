use anyhow::Result;
use warp::{Filter, Rejection, Reply};
use std::path::PathBuf;
use tokio::fs;

// This module provides a simple HTTP server to serve WASM modules and their
// associated assets, primarily for web-based clients or debugging.

pub struct WasmServer {
    serve_dir: PathBuf,
}

impl WasmServer {
    pub fn new() -> Self {
        let serve_dir = PathBuf::from("./wasm_dist"); // Directory to serve WASM files from
        Self { serve_dir }
    }

    pub async fn init(&self) -> Result<()> {
        log::info!("WASM server initialized. Serving from: {:?}", self.serve_dir);
        fs::create_dir_all(&self.serve_dir).await?;
        // Optionally, copy some dummy WASM files for testing
        Ok(())
    }

    pub async fn start_server(&self, port: u16) {
        let serve_dir_clone = self.serve_dir.clone();
        let routes = warp::fs::dir(serve_dir_clone);

        log::info!("Starting WASM file server on 127.0.0.1:{}", port);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;
    }

    /// Provides a URL for a given WASM module.
    pub fn get_wasm_url(&self, module_name: &str) -> String {
        format!("http://127.0.0.1:8080/{}.wasm", module_name) // Assuming default port 8080
    }
}

pub fn init() {
    log::info!("Serve WASM module initialized.");
}
