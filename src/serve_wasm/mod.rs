use warp::{Filter, Reply};
use std::path::{Path, PathBuf};
use std::fs;
use mime_guess::from_path;

/// A simple HTTP server for serving WebAssembly (WASM) modules and associated assets.
/// This is useful for web-based frontends that consume WASM, or for serving WASM plugins
/// to a remote client.
pub struct WasmServer {
    port: u16,
    serve_dir: PathBuf,
}

impl WasmServer {
    /// Creates a new `WasmServer` instance.
    pub fn new(port: u16, serve_dir: PathBuf) -> Self {
        Self { port, serve_dir }
    }

    /// Starts the HTTP server to serve files from the `serve_dir`.
    /// This function will block unless spawned in a separate Tokio task.
    pub async fn start(&self) -> Result<(), String> {
        if !self.serve_dir.exists() || !self.serve_dir.is_dir() {
            return Err(format!("Serve directory does not exist or is not a directory: {}", self.serve_dir.display()));
        }

        let serve_dir_clone = self.serve_dir.clone();
        let routes = warp::fs::dir(serve_dir_clone)
            .or(warp::path::end().and(warp::fs::file(self.serve_dir.join("index.html")))) // Serve index.html for root
            .with(warp::log("wasm_server"));

        println!("WasmServer: Serving files from '{}' on port {}", self.serve_dir.display(), self.port);
        warp::serve(routes).run(([127, 0, 0, 1], self.port)).await;
        Ok(())
    }

    /// Helper to create a filter for serving a specific WASM file with the correct MIME type.
    pub fn serve_wasm_file(path: impl AsRef<Path>) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let path_buf = path.as_ref().to_path_buf();
        warp::get()
            .and(warp::path::end())
            .and_then(move || {
                let file_path = path_buf.clone();
                async move {
                    match fs::read(&file_path) {
                        Ok(bytes) => {
                            let mime_type = from_path(&file_path).first_or_octet_stream();
                            Ok(warp::reply::with_header(bytes, "Content-Type", mime_type.to_string()))
                        },
                        Err(_) => Err(warp::reject::not_found()),
                    }
                }
            })
    }
}

pub fn init() {
    println!("serve_wasm module initialized: Provides a simple HTTP server for WASM assets.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs as tokio_fs;

    #[tokio::test]
    async fn test_wasm_server_start() {
        // Create a temporary directory and some dummy files
        let temp_dir = PathBuf::from("test_serve_dir");
        tokio_fs::create_dir_all(&temp_dir).await.unwrap();
        tokio_fs::write(temp_dir.join("index.html"), "<html><body>Hello WASM!</body></html>").await.unwrap();
        tokio_fs::write(temp_dir.join("app.wasm"), b"\x00\x61\x73\x6d\x01\x00\x00\x00").await.unwrap(); // Minimal WASM binary

        let port = 8081;
        let server = WasmServer::new(port, temp_dir.clone());

        // Spawn the server in a background task
        let server_handle = tokio::spawn(async move {
            server.start().await.unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Test fetching index.html
        let resp = reqwest::get(format!("http://127.0.0.1:{}/", port)).await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.text().await.unwrap(), "<html><body>Hello WASM!</body></html>");

        // Test fetching app.wasm
        let resp = reqwest::get(format!("http://127.0.0.1:{}/app.wasm", port)).await.unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.headers().get("content-type").unwrap(), "application/wasm");
        assert_eq!(resp.bytes().await.unwrap().to_vec(), b"\x00\x61\x73\x6d\x01\x00\x00\x00");

        // Clean up
        server_handle.abort(); // Stop the server task
        tokio_fs::remove_dir_all(&temp_dir).await.unwrap();
    }
}
