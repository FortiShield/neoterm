// This module defines the AI API endpoints for NeoTerm.
// It interacts with the AgentMode to process AI requests.

use warp::{Filter, Rejection, Reply, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use crate::agent_mode_eval::{AgentMode, AgentMessage};
use async_stream::stream;

#[derive(Debug, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub context_block_id: Option<String>, // For future context awareness
}

#[derive(Debug, Serialize)]
pub struct AiResponseChunk {
    pub content: String,
    pub is_done: bool,
}

pub fn ai_routes(
    agent_mode: Arc<RwLock<AgentMode>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "ai")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_agent_mode(agent_mode))
        .and_then(handle_ai_request)
}

fn with_agent_mode(
    agent_mode: Arc<RwLock<AgentMode>>,
) -> impl Filter<Extract = (Arc<RwLock<AgentMode>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || agent_mode.clone())
}

async fn handle_ai_request(
    request: AiRequest,
    agent_mode_arc: Arc<RwLock<AgentMode>>,
) -> Result<impl Reply, Rejection> {
    let mut agent_mode = agent_mode_arc.write().await;

    if !agent_mode.is_enabled() {
        return Err(warp::reject::custom(super::ApiError("Agent mode is not enabled.".to_string())));
    }

    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let prompt = request.prompt.clone();
    tokio::spawn(async move {
        let mut agent_mode = agent_mode_arc.write().await;
        let result = agent_mode.process_user_input(&prompt, tx.clone()).await;
        if let Err(e) = result {
            eprintln!("Error processing AI request: {}", e);
            let _ = tx.send(AgentMessage::Error(e.to_string())).await;
        }
    });

    let response_stream = stream! {
        let mut full_content = String::new();
        let mut receiver_stream = ReceiverStream::new(rx);
        while let Some(msg) = receiver_stream.next().await {
            match msg {
                AgentMessage::AgentResponseChunk(chunk) => {
                    full_content.push_str(&chunk);
                    yield Ok::<_, warp::Error>(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                        content: chunk,
                        is_done: false,
                    }).unwrap()));
                }
                AgentMessage::SystemMessage(msg) => {
                    // Optionally send system messages as well, or log them
                    eprintln!("Agent System Message: {}", msg);
                }
                AgentMessage::Error(e) => {
                    eprintln!("Agent Error: {}", e);
                    yield Ok::<_, warp::Error>(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                        content: format!("Error: {}", e),
                        is_done: true,
                    }).unwrap()));
                    break;
                }
                AgentMessage::Done => {
                    yield Ok::<_, warp::Error>(warp::sse::Event::default().data(serde_json::to_string(&AiResponseChunk {
                        content: "".to_string(),
                        is_done: true,
                    }).unwrap()));
                    break;
                }
                _ => {} // Ignore other message types for streaming to client
            }
        }
    };

    Ok(warp::sse::reply(response_stream).keep_alive(warp::sse::ServerSentEvent::default()))
}
