use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use chrono::{DateTime, Utc};
use crate::workflows::{Workflow, WorkflowStep, WorkflowExecutor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub id: String,
    pub workflow: Workflow,
    pub current_step: usize,
    pub execution_state: ExecutionState,
    pub breakpoints: Vec<usize>,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_history: Vec<StepExecution>,
    pub start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionState {
    NotStarted,
    Running,
    Paused,
    StepBreakpoint,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    pub step_index: usize,
    pub step_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: StepStatus,
    pub output: String,
    pub error: Option<String>,
    pub variables_snapshot: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
pub enum DebugCommand {
    Start,
    Pause,
    Resume,
    StepOver,
    StepInto,
    StepOut,
    Stop,
    SetBreakpoint(usize),
    RemoveBreakpoint(usize),
    SetVariable(String, serde_json::Value),
    Restart,
}

pub struct WorkflowDebugger {
    sessions: HashMap<String, DebugSession>,
    active_session: Option<String>,
    executor: WorkflowExecutor,
    command_sender: Option<mpsc::UnboundedSender<DebugCommand>>,
    event_receiver: Option<mpsc::UnboundedReceiver<DebugEvent>>,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    SessionStarted(String),
    StepStarted(usize),
    StepCompleted(usize, String),
    StepFailed(usize, String),
    BreakpointHit(usize),
    VariableChanged(String, serde_json::Value),
    ExecutionPaused,
    ExecutionResumed,
    ExecutionCompleted,
    ExecutionFailed(String),
}

impl WorkflowDebugger {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_session: None,
            executor: WorkflowExecutor::new(),
            command_sender: None,
            event_receiver: None,
        }
    }

    pub fn create_session(&mut self, workflow: Workflow) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = DebugSession {
            id: session_id.clone(),
            workflow,
            current_step: 0,
            execution_state: ExecutionState::NotStarted,
            breakpoints: Vec::new(),
            variables: HashMap::new(),
            step_history: Vec::new(),
            start_time: Utc::now(),
        };

        self.sessions.insert(session_id.clone(), session);
        self.active_session = Some(session_id.clone());
        session_id
    }

    pub fn start_debugging(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.execution_state = ExecutionState::Running;
            session.start_time = Utc::now();
            
            // Set up communication channels
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
            let (event_tx, event_rx) = mpsc::unbounded_channel();
            
            self.command_sender = Some(cmd_tx);
            self.event_receiver = Some(event_rx);
            
            // Start debug execution loop
            let session_clone = session.clone();
            let executor_clone = self.executor.clone();
            
            tokio::spawn(async move {
                Self::debug_execution_loop(session_clone, executor_clone, cmd_rx, event_tx).await;
            });
            
            Ok(())
        } else {
            Err("Session not found".into())
        }
    }

    async fn debug_execution_loop(
        mut session: DebugSession,
        executor: WorkflowExecutor,
        mut cmd_rx: mpsc::UnboundedReceiver<DebugCommand>,
        event_tx: mpsc::UnboundedSender<DebugEvent>,
    ) {
        let _ = event_tx.send(DebugEvent::SessionStarted(session.id.clone()));
        
        while session.current_step < session.workflow.steps.len() {
            // Check for breakpoints
            if session.breakpoints.contains(&session.current_step) {
                session.execution_state = ExecutionState::StepBreakpoint;
                let _ = event_tx.send(DebugEvent::BreakpointHit(session.current_step));
                
                // Wait for resume command
                loop {
                    if let Some(cmd) = cmd_rx.recv().await {
                        match cmd {
                            DebugCommand::Resume => {
                                session.execution_state = ExecutionState::Running;
                                let _ = event_tx.send(DebugEvent::ExecutionResumed);
                                break;
                            }
                            DebugCommand::StepOver => {
                                session.execution_state = ExecutionState::Running;
                                break;
                            }
                            DebugCommand::Stop => {
                                return;
                            }
                            _ => continue,
                        }
                    }
                }
            }

            // Execute current step
            let step = &session.workflow.steps[session.current_step];
            let step_execution = Self::execute_step_with_debug(
                step, 
                session.current_step, 
                &mut session.variables,
                &executor
            ).await;

            session.step_history.push(step_execution.clone());
            
            match step_execution.status {
                StepStatus::Completed => {
                    let _ = event_tx.send(DebugEvent::StepCompleted(
                        session.current_step, 
                        step_execution.output
                    ));
                }
                StepStatus::Failed => {
                    let error = step_execution.error.unwrap_or_default();
                    let _ = event_tx.send(DebugEvent::StepFailed(session.current_step, error.clone()));
                    session.execution_state = ExecutionState::Failed(error);
                    let _ = event_tx.send(DebugEvent::ExecutionFailed(error));
                    return;
                }
                _ => {}
            }

            session.current_step += 1;

            // Check for pause commands
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    DebugCommand::Pause => {
                        session.execution_state = ExecutionState::Paused;
                        let _ = event_tx.send(DebugEvent::ExecutionPaused);
                        
                        // Wait for resume
                        loop {
                            if let Some(resume_cmd) = cmd_rx.recv().await {
                                match resume_cmd {
                                    DebugCommand::Resume => {
                                        session.execution_state = ExecutionState::Running;
                                        let _ = event_tx.send(DebugEvent::ExecutionResumed);
                                        break;
                                    }
                                    DebugCommand::Stop => return,
                                    _ => continue,
                                }
                            }
                        }
                    }
                    DebugCommand::Stop => return,
                    _ => {}
                }
            }
        }

        session.execution_state = ExecutionState::Completed;
        let _ = event_tx.send(DebugEvent::ExecutionCompleted);
    }

    async fn execute_step_with_debug(
        step: &WorkflowStep,
        step_index: usize,
        variables: &mut HashMap<String, serde_json::Value>,
        executor: &WorkflowExecutor,
    ) -> StepExecution {
        let start_time = Utc::now();
        let variables_snapshot = variables.clone();

        let mut step_execution = StepExecution {
            step_index,
            step_name: step.name.clone(),
            start_time,
            end_time: None,
            status: StepStatus::Running,
            output: String::new(),
            error: None,
            variables_snapshot,
        };

        // Execute the step (simplified - would integrate with actual executor)
        match executor.execute_step(step, variables).await {
            Ok(output) => {
                step_execution.status = StepStatus::Completed;
                step_execution.output = output;
            }
            Err(e) => {
                step_execution.status = StepStatus::Failed;
                step_execution.error = Some(e.to_string());
            }
        }

        step_execution.end_time = Some(Utc::now());
        step_execution
    }

    pub fn send_command(&mut self, command: DebugCommand) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref sender) = self.command_sender {
            sender.send(command)?;
        }
        Ok(())
    }

    pub fn set_breakpoint(&mut self, session_id: &str, step_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            if !session.breakpoints.contains(&step_index) {
                session.breakpoints.push(step_index);
                session.breakpoints.sort();
            }
        }
        Ok(())
    }

    pub fn remove_breakpoint(&mut self, session_id: &str, step_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.breakpoints.retain(|&x| x != step_index);
        }
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Option<&DebugSession> {
        self.sessions.get(session_id)
    }

    pub fn get_active_session(&self) -> Option<&DebugSession> {
        self.active_session.as_ref()
            .and_then(|id| self.sessions.get(id))
    }

    pub fn get_step_history(&self, session_id: &str) -> Vec<StepExecution> {
        self.sessions.get(session_id)
            .map(|s| s.step_history.clone())
            .unwrap_or_default()
    }

    pub fn get_variables(&self, session_id: &str) -> HashMap<String, serde_json::Value> {
        self.sessions.get(session_id)
            .map(|s| s.variables.clone())
            .unwrap_or_default()
    }

    pub fn set_variable(&mut self, session_id: &str, name: String, value: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.variables.insert(name.clone(), value.clone());
            
            // Send command to update variable in running session
            if let Some(ref sender) = self.command_sender {
                sender.send(DebugCommand::SetVariable(name, value))?;
            }
        }
        Ok(())
    }

    pub fn restart_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.current_step = 0;
            session.execution_state = ExecutionState::NotStarted;
            session.step_history.clear();
            session.variables.clear();
            session.start_time = Utc::now();
            
            if let Some(ref sender) = self.command_sender {
                sender.send(DebugCommand::Restart)?;
            }
        }
        Ok(())
    }

    pub fn get_execution_summary(&self, session_id: &str) -> Option<String> {
        self.sessions.get(session_id).map(|session| {
            let total_steps = session.workflow.steps.len();
            let completed_steps = session.step_history.iter()
                .filter(|s| matches!(s.status, StepStatus::Completed))
                .count();
            let failed_steps = session.step_history.iter()
                .filter(|s| matches!(s.status, StepStatus::Failed))
                .count();
            
            format!(
                "Workflow: {} | Steps: {}/{} | Failed: {} | State: {:?}",
                session.workflow.name,
                completed_steps,
                total_steps,
                failed_steps,
                session.execution_state
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::{Workflow, WorkflowStep};

    #[test]
    fn test_debug_session_creation() {
        let mut debugger = WorkflowDebugger::new();
        let workflow = Workflow {
            name: "Test Workflow".to_string(),
            description: "Test".to_string(),
            steps: vec![
                WorkflowStep {
                    name: "Step 1".to_string(),
                    command: "echo hello".to_string(),
                    args: vec![],
                    working_directory: None,
                    environment: HashMap::new(),
                    timeout: None,
                    retry_count: 0,
                    condition: None,
                }
            ],
            environment: HashMap::new(),
            timeout: None,
        };

        let session_id = debugger.create_session(workflow);
        assert!(debugger.get_session(&session_id).is_some());
        assert_eq!(debugger.active_session, Some(session_id));
    }

    #[test]
    fn test_breakpoint_management() {
        let mut debugger = WorkflowDebugger::new();
        let workflow = Workflow {
            name: "Test".to_string(),
            description: "Test".to_string(),
            steps: vec![],
            environment: HashMap::new(),
            timeout: None,
        };

        let session_id = debugger.create_session(workflow);
        
        debugger.set_breakpoint(&session_id, 0).unwrap();
        debugger.set_breakpoint(&session_id, 2).unwrap();
        
        let session = debugger.get_session(&session_id).unwrap();
        assert_eq!(session.breakpoints, vec![0, 2]);
        
        debugger.remove_breakpoint(&session_id, 0).unwrap();
        let session = debugger.get_session(&session_id).unwrap();
        assert_eq!(session.breakpoints, vec![2]);
    }
}
