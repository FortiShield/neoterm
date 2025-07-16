use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionShareEvent {
    SessionStarted(String), // Session ID
    SessionEnded(String),
    UserJoined(String, String), // (Session ID, User ID)
    UserLeft(String, String),
    BlockAdded(String, String, String), // (Session ID, Block ID, Block Content)
    BlockUpdated(String, String, String),
    BlockDeleted(String, String),
    CommandExecuted(String, String), // (Session ID, Command)
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSession {
    pub id: Uuid,
    pub host_user_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub active_users: Vec<String>,
    // In a real system, this would contain a snapshot or stream of terminal state
    // For simplicity, we'll just track some metadata.
    pub last_activity: DateTime<Utc>,
}

pub struct SessionSharingManager {
    active_sessions: HashMap<Uuid, SharedSession>,
    // In a real system, this would involve WebSocket connections or a real-time backend
    // For now, we'll simulate events.
    event_sender: mpsc::UnboundedSender<SessionShareEvent>,
}

impl SessionSharingManager {
    pub fn new(event_sender: mpsc::UnboundedSender<SessionShareEvent>) -> Self {
        Self {
            active_sessions: HashMap::new(),
            event_sender,
        }
    }

    pub fn create_session(&mut self, host_user_id: String, title: String) -> Result<Uuid, String> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let session = SharedSession {
            id: session_id,
            host_user_id: host_user_id.clone(),
            title,
            created_at: now,
            active_users: vec![host_user_id],
            last_activity: now,
        };
        self.active_sessions.insert(session_id, session);
        let _ = self.event_sender.send(SessionShareEvent::SessionStarted(session_id.to_string()));
        Ok(session_id)
    }

    pub fn join_session(&mut self, session_id: Uuid, user_id: String) -> Result<(), String> {
        if let Some(session) = self.active_sessions.get_mut(&session_id) {
            if !session.active_users.contains(&user_id) {
                session.active_users.push(user_id.clone());
                session.last_activity = Utc::now();
                let _ = self.event_sender.send(SessionShareEvent::UserJoined(session_id.to_string(), user_id));
                Ok(())
            } else {
                Err("User already in session.".to_string())
            }
        } else {
            Err("Session not found.".to_string())
        }
    }

    pub fn leave_session(&mut self, session_id: Uuid, user_id: String) -> Result<(), String> {
        if let Some(session) = self.active_sessions.get_mut(&session_id) {
            session.active_users.retain(|u| u != &user_id);
            session.last_activity = Utc::now();
            let _ = self.event_sender.send(SessionShareEvent::UserLeft(session_id.to_string(), user_id));
            if session.active_users.is_empty() {
                self.end_session(session_id)?;
            }
            Ok(())
        } else {
            Err("Session not found.".to_string())
        }
    }

    pub fn end_session(&mut self, session_id: Uuid) -> Result<(), String> {
        if self.active_sessions.remove(&session_id).is_some() {
            let _ = self.event_sender.send(SessionShareEvent::SessionEnded(session_id.to_string()));
            Ok(())
        } else {
            Err("Session not found.".to_string())
        }
    }

    pub fn notify_block_added(&self, session_id: Uuid, block_id: String, content: String) {
        if self.active_sessions.contains_key(&session_id) {
            let _ = self.event_sender.send(SessionShareEvent::BlockAdded(session_id.to_string(), block_id, content));
        }
    }

    pub fn notify_command_executed(&self, session_id: Uuid, command: String) {
        if self.active_sessions.contains_key(&session_id) {
            let _ = self.event_sender.send(SessionShareEvent::CommandExecuted(session_id.to_string(), command));
        }
    }

    pub fn get_session_info(&self, session_id: Uuid) -> Option<&SharedSession> {
        self.active_sessions.get(&session_id)
    }
}

pub fn init() {
    println!("collaboration/session_sharing module loaded");
}
