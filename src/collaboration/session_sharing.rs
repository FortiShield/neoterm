use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSession {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub participants: Vec<Participant>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub settings: SessionSettings,
    pub state: SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub role: ParticipantRole,
    pub joined_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub cursor_position: Option<CursorPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub max_participants: usize,
    pub allow_anonymous: bool,
    pub require_approval: bool,
    pub auto_save_interval: u64, // seconds
    pub sync_cursors: bool,
    pub sync_selections: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub command_history: Vec<SharedCommand>,
    pub active_blocks: Vec<SharedBlock>,
    pub environment_variables: HashMap<String, String>,
    pub working_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCommand {
    pub id: String,
    pub command: String,
    pub executed_by: String,
    pub timestamp: DateTime<Utc>,
    pub output: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedBlock {
    pub id: String,
    pub content: String,
    pub block_type: String,
    pub created_by: String,
    pub timestamp: DateTime<Utc>,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    ParticipantJoined(Participant),
    ParticipantLeft(String),
    CommandExecuted(SharedCommand),
    BlockAdded(SharedBlock),
    BlockUpdated(SharedBlock),
    CursorMoved(String, CursorPosition),
    StateSync(SessionState),
    ChatMessage(ChatMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: ChatMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessageType {
    Text,
    System,
    Command,
    File,
}

pub struct SessionSharingManager {
    sessions: RwLock<HashMap<String, SharedSession>>,
    event_broadcasters: RwLock<HashMap<String, broadcast::Sender<SessionEvent>>>,
    participants: RwLock<HashMap<String, String>>, // participant_id -> session_id
}

impl SessionSharingManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_broadcasters: RwLock::new(HashMap::new()),
            participants: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_session(&self, name: String, owner: String) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4().to_string();
        let (tx, _) = broadcast::channel(1000);

        let session = SharedSession {
            id: session_id.clone(),
            name,
            owner: owner.clone(),
            participants: vec![Participant {
                id: owner.clone(),
                name: owner,
                role: ParticipantRole::Owner,
                joined_at: Utc::now(),
                last_seen: Utc::now(),
                cursor_position: None,
            }],
            created_at: Utc::now(),
            last_activity: Utc::now(),
            settings: SessionSettings {
                max_participants: 10,
                allow_anonymous: false,
                require_approval: false,
                auto_save_interval: 30,
                sync_cursors: true,
                sync_selections: true,
            },
            state: SessionState {
                command_history: Vec::new(),
                active_blocks: Vec::new(),
                environment_variables: HashMap::new(),
                working_directory: std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            },
        };

        let mut sessions = self.sessions.write().await;
        let mut broadcasters = self.event_broadcasters.write().await;
        
        sessions.insert(session_id.clone(), session);
        broadcasters.insert(session_id.clone(), tx);

        Ok(session_id)
    }

    pub async fn join_session(&self, session_id: &str, participant: Participant) -> Result<broadcast::Receiver<SessionEvent>, Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let broadcasters = self.event_broadcasters.read().await;
        let mut participants = self.participants.write().await;

        if let Some(session) = sessions.get_mut(session_id) {
            if session.participants.len() >= session.settings.max_participants {
                return Err("Session is full".into());
            }

            // Check if participant already exists
            if !session.participants.iter().any(|p| p.id == participant.id) {
                session.participants.push(participant.clone());
                session.last_activity = Utc::now();
                
                participants.insert(participant.id.clone(), session_id.to_string());

                // Broadcast join event
                if let Some(broadcaster) = broadcasters.get(session_id) {
                    let _ = broadcaster.send(SessionEvent::ParticipantJoined(participant));
                }
            }

            if let Some(broadcaster) = broadcasters.get(session_id) {
                Ok(broadcaster.subscribe())
            } else {
                Err("Session broadcaster not found".into())
            }
        } else {
            Err("Session not found".into())
        }
    }

    pub async fn leave_session(&self, session_id: &str, participant_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let broadcasters = self.event_broadcasters.read().await;
        let mut participants = self.participants.write().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.participants.retain(|p| p.id != participant_id);
            session.last_activity = Utc::now();
            
            participants.remove(participant_id);

            // Broadcast leave event
            if let Some(broadcaster) = broadcasters.get(session_id) {
                let _ = broadcaster.send(SessionEvent::ParticipantLeft(participant_id.to_string()));
            }

            // Remove session if no participants left
            if session.participants.is_empty() {
                drop(sessions);
                self.remove_session(session_id).await?;
            }
        }

        Ok(())
    }

    pub async fn execute_command(&self, session_id: &str, command: SharedCommand) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let broadcasters = self.event_broadcasters.read().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.state.command_history.push(command.clone());
            session.last_activity = Utc::now();

            // Broadcast command execution
            if let Some(broadcaster) = broadcasters.get(session_id) {
                let _ = broadcaster.send(SessionEvent::CommandExecuted(command));
            }
        }

        Ok(())
    }

    pub async fn add_block(&self, session_id: &str, block: SharedBlock) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let broadcasters = self.event_broadcasters.read().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.state.active_blocks.push(block.clone());
            session.last_activity = Utc::now();

            // Broadcast block addition
            if let Some(broadcaster) = broadcasters.get(session_id) {
                let _ = broadcaster.send(SessionEvent::BlockAdded(block));
            }
        }

        Ok(())
    }

    pub async fn update_cursor(&self, session_id: &str, participant_id: &str, position: CursorPosition) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let broadcasters = self.event_broadcasters.read().await;

        if let Some(session) = sessions.get_mut(session_id) {
            if let Some(participant) = session.participants.iter_mut().find(|p| p.id == participant_id) {
                participant.cursor_position = Some(position.clone());
                participant.last_seen = Utc::now();
                session.last_activity = Utc::now();

                // Broadcast cursor movement if enabled
                if session.settings.sync_cursors {
                    if let Some(broadcaster) = broadcasters.get(session_id) {
                        let _ = broadcaster.send(SessionEvent::CursorMoved(participant_id.to_string(), position));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn send_chat_message(&self, session_id: &str, message: ChatMessage) -> Result<(), Box<dyn std::error::Error>> {
        let sessions = self.sessions.read().await;
        let broadcasters = self.event_broadcasters.read().await;

        if sessions.contains_key(session_id) {
            if let Some(broadcaster) = broadcasters.get(session_id) {
                let _ = broadcaster.send(SessionEvent::ChatMessage(message));
            }
        }

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Option<SharedSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn list_sessions(&self) -> Vec<SharedSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    pub async fn sync_state(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sessions = self.sessions.read().await;
        let broadcasters = self.event_broadcasters.read().await;

        if let Some(session) = sessions.get(session_id) {
            if let Some(broadcaster) = broadcasters.get(session_id) {
                let _ = broadcaster.send(SessionEvent::StateSync(session.state.clone()));
            }
        }

        Ok(())
    }

    async fn remove_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let mut broadcasters = self.event_broadcasters.write().await;

        sessions.remove(session_id);
        broadcasters.remove(session_id);

        Ok(())
    }

    pub async fn cleanup_inactive_sessions(&self, max_idle_hours: u64) -> Result<usize, Box<dyn std::error::Error>> {
        let mut sessions = self.sessions.write().await;
        let mut broadcasters = self.event_broadcasters.write().await;
        
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_idle_hours as i64);
        let mut removed_count = 0;

        let inactive_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.last_activity < cutoff_time)
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in inactive_sessions {
            sessions.remove(&session_id);
            broadcasters.remove(&session_id);
            removed_count += 1;
        }

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionSharingManager::new();
        let session_id = manager.create_session("Test Session".to_string(), "user1".to_string()).await.unwrap();
        
        let session = manager.get_session(&session_id).await.unwrap();
        assert_eq!(session.name, "Test Session");
        assert_eq!(session.owner, "user1");
        assert_eq!(session.participants.len(), 1);
    }

    #[tokio::test]
    async fn test_participant_join_leave() {
        let manager = SessionSharingManager::new();
        let session_id = manager.create_session("Test".to_string(), "owner".to_string()).await.unwrap();
        
        let participant = Participant {
            id: "user2".to_string(),
            name: "User 2".to_string(),
            role: ParticipantRole::Editor,
            joined_at: Utc::now(),
            last_seen: Utc::now(),
            cursor_position: None,
        };

        let _receiver = manager.join_session(&session_id, participant).await.unwrap();
        
        let session = manager.get_session(&session_id).await.unwrap();
        assert_eq!(session.participants.len(), 2);

        manager.leave_session(&session_id, "user2").await.unwrap();
        
        let session = manager.get_session(&session_id).await.unwrap();
        assert_eq!(session.participants.len(), 1);
    }
}
