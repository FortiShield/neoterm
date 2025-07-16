// This module defines the API server for NeoTerm, primarily for AI integration.
// It uses the `warp` crate to create a web server.

use warp::{Filter, Rejection, Reply};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent_mode_eval::AgentMode;

pub mod ai; // Sub-module for AI API endpoints

/// Starts the API server on a given port.
pub async fn start_api_server(agent_mode: Arc<RwLock<AgentMode>>) {
    let ai_route = ai::ai_routes(agent_mode);

    let routes = ai_route.with(warp::cors().allow_any_origin());

    // You can adjust the port as needed
    let port = 3030;
    println!("API server starting on http://127.0.0.1:{}", port);
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}

// Custom error handling for API routes
#[derive(Debug)]
struct ApiError(String);

impl warp::reject::Reject for ApiError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        Ok(warp::reply::with_status("NOT_FOUND", warp::http::StatusCode::NOT_FOUND))
    } else if let Some(e) = err.find::<ApiError>() {
        eprintln!("API Error: {}", e.0);
        Ok(warp::reply::with_status(e.0.clone(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    } else {
        // Log other rejections for debugging
        eprintln!("Unhandled rejection: {:?}", err);
        Err(err)
    }
}
