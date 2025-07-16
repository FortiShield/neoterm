use warp::{Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_mode_eval::{AgentModeEvaluator, ai_client::ChatMessage};
use anyhow::anyhow;

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub full_conversation: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

pub fn ai_api(
    evaluator: Arc<AgentModeEvaluator>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let evaluator_filter = warp::any().map(move || evaluator.clone());

    warp::path!("api" / "ai" / "chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(evaluator_filter.clone())
        .and_then(handle_chat_request)
        .or(warp::path!("api" / "ai" / "history")
            .and(warp::get())
            .and(evaluator_filter.clone())
            .and_then(handle_history_request))
        .or(warp::path!("api" / "ai" / "reset")
            .and(warp::post())
            .and(evaluator_filter)
            .and_then(handle_reset_request))
}

async fn handle_chat_request(
    request: ChatRequest,
    evaluator: Arc<AgentModeEvaluator>,
) -> Result<impl Reply, Rejection> {
    log::info!("Received AI chat request: {:?}", request);
    match evaluator.handle_user_input(request.message).await {
        Ok(response_messages) => {
            let last_message_content = response_messages.last()
                .map(|msg| msg.content.clone())
                .unwrap_or_else(|| "No response content.".to_string());

            let full_conversation = evaluator.get_conversation_history().await;

            let response = ChatResponse {
                response: last_message_content,
                full_conversation,
            };
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            log::error!("Error handling AI chat request: {:?}", e);
            let error_response = ErrorResponse {
                message: format!("Failed to get AI response: {}", e),
            };
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_history_request(
    evaluator: Arc<AgentModeEvaluator>,
) -> Result<impl Reply, Rejection> {
    log::info!("Received AI history request");
    let history = evaluator.get_conversation_history().await;
    Ok(warp::reply::json(&history))
}

async fn handle_reset_request(
    evaluator: Arc<AgentModeEvaluator>,
) -> Result<impl Reply, Rejection> {
    log::info!("Received AI reset request");
    evaluator.reset_conversation().await;
    Ok(warp::reply::with_status(
        "Conversation reset successfully",
        warp::http::StatusCode::OK,
    ))
}
