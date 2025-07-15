use warp::{Filter, Rejection, Reply, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use futures_util::StreamExt;

use crate::agent_mode_eval::{AgentMode, AgentMessage};

#[derive(Debug, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub context_block_id: Option<String>, // Optional: ID of a block to provide context
}

#[derive(Debug, Serialize)]
pub struct AiResponseChunk {
    pub content: String,
    pub is_error: bool,
    pub is_done: bool,
}

pub fn ai_routes(
    agent_mode: Arc<RwLock<AgentMode>>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
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
    let agent_mode_read = agent_mode_arc.read().await;
    if !agent_mode_read.is_enabled() {
        return Ok(warp::reply::with_status(
            "AI mode is not enabled.",
            StatusCode::FORBIDDEN,
        ));
    }

    let prompt = request.prompt;
    let (tx, rx) = mpsc::channel::<Result<bytes::Bytes, warp::Error>>(10);

    tokio::spawn(async move {
        let agent_mode_write = agent_mode_arc.write().await;
        let result = agent_mode_write.send_message(prompt).await;
        drop(agent_mode_write); // Release the write lock

        match result {
            Ok(mut stream_receiver) => {
                while let Some(chunk) = stream_receiver.recv().await {
                    let response_chunk = AiResponseChunk {
                        content: chunk,
                        is_error: false,
                        is_done: false,
                    };
                    let json_chunk = serde_json::to_string(&response_chunk).unwrap();
                    if tx.send(Ok(bytes::Bytes::from(format!("data: {}\n\n", json_chunk)))).await.is_err() {
                        log::warn!("Failed to send AI response chunk to client.");
                        break;
                    }
                }
                let final_chunk = AiResponseChunk {
                    content: "".to_string(),
                    is_error: false,
                    is_done: true,
                };
                let json_final_chunk = serde_json::to_string(&final_chunk).unwrap();
                let _ = tx.send(Ok(bytes::Bytes::from(format!("data: {}\n\n", json_final_chunk)))).await;
            }
            Err(e) => {
                let error_chunk = AiResponseChunk {
                    content: format!("Error: {}", e),
                    is_error: true,
                    is_done: true,
                };
                let json_error_chunk = serde_json::to_string(&error_chunk).unwrap();
                let _ = tx.send(Ok(bytes::Bytes::from(format!("data: {}\n\n", json_error_chunk)))).await;
            }
        }
    });

    let stream = ReceiverStream::new(rx);
    Ok(warp::reply::with_header(
        warp::reply::Response::new(warp::hyper::Body::wrap_stream(stream)),
        "Content-Type",
        "text/event-stream",
    ))
}
