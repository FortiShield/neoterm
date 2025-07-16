use warp::{Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use crate::agent_mode_eval::{AgentMode, AgentMessage};
use crate::block::Block as UIBlock; // Alias to avoid conflict

#[derive(Debug, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub context_block_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiResponseChunk {
    pub content: String,
    pub is_done: bool,
    pub tool_call: Option<serde_json::Value>, // For tool calls
    pub tool_result: Option<serde_json::Value>, // For tool results
    pub error: Option<String>,
}

pub fn ai_routes(
    agent_mode: Arc<RwLock<AgentMode>>,
    blocks: Arc<RwLock<Vec<UIBlock>>>, // Pass blocks to access context
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("api" / "ai")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_agent_mode(agent_mode))
        .and(with_blocks(blocks))
        .and_then(ai_handler)
}

fn with_agent_mode(
    agent_mode: Arc<RwLock<AgentMode>>,
) -> impl Filter<Extract = (Arc<RwLock<AgentMode>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || agent_mode.clone())
}

fn with_blocks(
    blocks: Arc<RwLock<Vec<UIBlock>>>,
) -> impl Filter<Extract = (Arc<RwLock<Vec<UIBlock>>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || blocks.clone())
}

async fn ai_handler(
    request: AiRequest,
    agent_mode_arc: Arc<RwLock<AgentMode>>,
    blocks_arc: Arc<RwLock<Vec<UIBlock>>>,
) -> Result<impl Reply, Rejection> {
    let mut agent_mode = agent_mode_arc.write().await;
    
    let mut context_blocks = Vec::new();
    if let Some(block_id) = request.context_block_id {
        let blocks_guard = blocks_arc.read().await;
        if let Some(block) = blocks_guard.iter().find(|b| b.id == block_id) {
            context_blocks.push(block.clone());
        }
    }

    match agent_mode.send_message(request.prompt, context_blocks).await {
        Ok(mut rx) => {
            let stream = ReceiverStream::new(rx)
                .map(|agent_msg| {
                    let chunk = match agent_msg {
                        AgentMessage::AgentResponse(content) => AiResponseChunk {
                            content,
                            is_done: false,
                            tool_call: None,
                            tool_result: None,
                            error: None,
                        },
                        AgentMessage::ToolCall(tc) => AiResponseChunk {
                            content: format!("AI requested tool: {}", tc.name),
                            is_done: false,
                            tool_call: Some(serde_json::to_value(tc).unwrap_or_default()),
                            tool_result: None,
                            error: None,
                        },
                        AgentMessage::ToolResult(result) => AiResponseChunk {
                            content: format!("Tool result: {}", result),
                            is_done: false,
                            tool_call: None,
                            tool_result: Some(serde_json::to_value(result).unwrap_or_default()),
                            error: None,
                        },
                        AgentMessage::SystemMessage(content) => AiResponseChunk {
                            content: format!("System: {}", content),
                            is_done: false,
                            tool_call: None,
                            tool_result: None,
                            error: None,
                        },
                        AgentMessage::Error(err) => AiResponseChunk {
                            content: format!("Error: {}", err),
                            is_done: false,
                            tool_call: None,
                            tool_result: None,
                            error: Some(err),
                        },
                        AgentMessage::Done => AiResponseChunk {
                            content: "[DONE]".to_string(),
                            is_done: true,
                            tool_call: None,
                            tool_result: None,
                            error: None,
                        },
                        AgentMessage::UserMessage(_) => { // Should not be streamed back to API client
                            AiResponseChunk {
                                content: "".to_string(),
                                is_done: false,
                                tool_call: None,
                                tool_result: None,
                                error: None,
                            }
                        }
                    };
                    Ok::<_, warp::Error>(warp::sse::Event::json(&chunk).unwrap())
                });

            Ok(warp::sse::reply(stream))
        }
        Err(e) => {
            eprintln!("Error sending message to agent: {}", e);
            let error_chunk = AiResponseChunk {
                content: format!("Failed to process AI request: {}", e),
                is_done: true,
                tool_call: None,
                tool_result: None,
                error: Some(e.to_string()),
            };
            Ok(warp::sse::reply(warp::sse::Event::json(&error_chunk).unwrap()))
        }
    }
}

pub async fn start_api_server(agent_mode: Arc<RwLock<AgentMode>>) {
    let blocks: Arc<RwLock<Vec<UIBlock>>> = Arc::new(RwLock::new(Vec::new())); // Dummy blocks for API for now
    // In a real application, you'd pass the actual blocks from the main app state.
    // For this example, the API server is separate and doesn't directly share the UI's blocks.
    // The `main.rs` will handle the API calls to its own agent_mode.

    let routes = ai_routes(agent_mode, blocks);

    let addr = ([127, 0, 0, 1], 3030);
    eprintln!("API server running on http://{}:{}", addr.0.join("."), addr.1);
    warp::serve(routes).run(addr).await;
}
