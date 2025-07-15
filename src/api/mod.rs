use warp::{Filter, Rejection, Reply};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent_mode_eval::AgentMode;

pub mod ai;

pub async fn start_api_server(agent_mode: Arc<RwLock<AgentMode>>) {
    let ai_route = ai::ai_routes(agent_mode);

    let routes = ai_route.with(warp::log("neoterm_api"));

    // Start the server in a separate Tokio task
    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    });
    log::info!("API server started on http://127.0.0.1:3030");
}
