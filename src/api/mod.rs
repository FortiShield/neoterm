use warp::Filter;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent_mode_eval::AgentMode;

pub mod ai;

pub async fn start_api_server(agent_mode: Arc<RwLock<AgentMode>>) {
    let ai_route = warp::path("api")
        .and(warp::path("ai"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_agent_mode(agent_mode))
        .and_then(ai::handle_ai_request);

    let routes = ai_route;

    println!("API server running on http://127.0.0.1:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

fn with_agent_mode(
    agent_mode: Arc<RwLock<AgentMode>>,
) -> impl Filter<Extract = (Arc<RwLock<AgentMode>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || agent_mode.clone())
}
