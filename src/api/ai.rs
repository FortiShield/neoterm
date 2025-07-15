use warp::{reply, Rejection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use futures_util::stream;
use crate::agent_mode_eval::{AgentMode, AgentMessage};

#[derive(Debug, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub context_block_id: Option<String>, // For future use to provide context
}

#[derive(Debug, Serialize)]
pub struct AiResponseChunk {
    pub content: String,
    pub is_done: bool,
}

pub async fn handle_ai_request(
    request: AiRequest,
    agent_mode: Arc<RwLock<AgentMode>>,
) -> Result<impl reply::Reply, Rejection> {
    let mut agent_mode_guard = agent_mode.write().await;

    // Ensure agent mode is enabled and conversation is started
    if !agent_mode_guard.is_enabled() {
        agent_mode_guard.toggle(); // Enable it if not already
        if let Err(e) = agent_mode_guard.start_conversation() {
            eprintln!("Failed to start agent conversation: {}", e);
            return Ok(reply::with_status(
                reply::json(&AiResponseChunk {
                    content: format!("Error: Failed to start agent conversation: {}", e),
                    is_done: true,
                }),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }

    let prompt = request.prompt;
    let agent_mode_clone = agent_mode_guard.clone(); // Clone for the async block

    let stream = async_stream::stream! {
        match agent_mode_clone.send_message(prompt).await {
            Ok(mut rx) => {
                while let Some(chunk) = rx.recv().await {
                    yield Ok(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                        content: chunk,
                        is_done: false,
                    }).unwrap()));
                }
                yield Ok(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                    content: "".to_string(),
                    is_done: true,
                }).unwrap()));
            }
            Err(e) => {
                eprintln!("Agent message error: {}", e);
                yield Ok(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                    content: format!("Error: {}", e),
                    is_done: true,
                }).unwrap()));
            }
        }
    };

    Ok(reply::with_header(
        reply::Response::new(warp::sse::reply(stream).into_response().into_body()),
        "Content-Type",
        "text/event-stream",
    ))
}
