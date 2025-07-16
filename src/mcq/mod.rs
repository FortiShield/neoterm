use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a single Multiple Choice Question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqQuestion {
    pub id: Uuid,
    pub text: String,
    pub options: Vec<String>,
    pub correct_answer_index: usize, // Index into the options vector
    pub explanation: Option<String>,
}

/// Represents a quiz or a set of MCQs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqQuiz {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub questions: Vec<McqQuestion>,
}

/// Manages and presents Multiple Choice Questions for interactive learning or onboarding.
pub struct McqHandler {
    quizzes: HashMap<Uuid, McqQuiz>,
    active_quiz_session: Option<McqQuizSession>,
}

#[derive(Debug, Clone)]
pub struct McqQuizSession {
    pub quiz_id: Uuid,
    pub current_question_index: usize,
    pub score: u32,
    pub total_questions: u32,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub completed: bool,
    pub answers: HashMap<Uuid, Option<usize>>, // Question ID -> User's selected option index
}

impl McqHandler {
    pub fn new() -> Self {
        Self {
            quizzes: HashMap::new(),
            active_quiz_session: None,
        }
    }

    /// Adds a quiz to the handler.
    pub fn add_quiz(&mut self, quiz: McqQuiz) {
        self.quizzes.insert(quiz.id, quiz);
    }

    /// Starts a new quiz session.
    pub fn start_quiz(&mut self, quiz_id: Uuid) -> Result<&McqQuestion, String> {
        if let Some(quiz) = self.quizzes.get(&quiz_id) {
            if quiz.questions.is_empty() {
                return Err("Quiz has no questions.".to_string());
            }
            self.active_quiz_session = Some(McqQuizSession {
                quiz_id,
                current_question_index: 0,
                score: 0,
                total_questions: quiz.questions.len() as u32,
                start_time: chrono::Utc::now(),
                end_time: None,
                completed: false,
                answers: HashMap::new(),
            });
            Ok(&quiz.questions[0])
        } else {
            Err("Quiz not found.".to_string())
        }
    }

    /// Submits an answer for the current question in the active session.
    /// Returns true if the answer was correct, false otherwise.
    pub fn submit_answer(&mut self, answer_index: usize) -> Result<bool, String> {
        if let Some(session) = self.active_quiz_session.as_mut() {
            if session.completed {
                return Err("Quiz session already completed.".to_string());
            }
            if let Some(quiz) = self.quizzes.get(&session.quiz_id) {
                let current_question = &quiz.questions[session.current_question_index];
                session.answers.insert(current_question.id, Some(answer_index));

                let is_correct = answer_index == current_question.correct_answer_index;
                if is_correct {
                    session.score += 1;
                }
                Ok(is_correct)
            } else {
                Err("Active quiz not found in registry.".to_string())
            }
        } else {
            Err("No active quiz session.".to_string())
        }
    }

    /// Moves to the next question in the active session.
    /// Returns the next question, or None if the quiz is completed.
    pub fn next_question(&mut self) -> Option<&McqQuestion> {
        if let Some(session) = self.active_quiz_session.as_mut() {
            if session.completed {
                return None;
            }
            session.current_question_index += 1;
            if let Some(quiz) = self.quizzes.get(&session.quiz_id) {
                if session.current_question_index < quiz.questions.len() {
                    Some(&quiz.questions[session.current_question_index])
                } else {
                    session.completed = true;
                    session.end_time = Some(chrono::Utc::now());
                    None
                }
            } else {
                None // Should not happen if session.quiz_id is valid
            }
        } else {
            None
        }
    }

    /// Gets the current question in the active session.
    pub fn get_current_question(&self) -> Option<&McqQuestion> {
        self.active_quiz_session.as_ref().and_then(|session| {
            self.quizzes.get(&session.quiz_id).and_then(|quiz| {
                quiz.questions.get(session.current_question_index)
            })
        })
    }

    /// Gets the active quiz session details.
    pub fn get_active_session(&self) -> Option<&McqQuizSession> {
        self.active_quiz_session.as_ref()
    }

    /// Ends the current quiz session.
    pub fn end_session(&mut self) {
        if let Some(session) = self.active_quiz_session.as_mut() {
            session.completed = true;
            session.end_time = Some(chrono::Utc::now());
        }
        self.active_quiz_session = None;
    }
}

pub fn init() {
    println!("mcq module initialized: Provides Multiple Choice Question handling.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_quiz() -> McqQuiz {
        McqQuiz {
            id: Uuid::new_v4(),
            title: "Rust Basics".to_string(),
            description: Some("A short quiz on Rust fundamentals.".to_string()),
            questions: vec![
                McqQuestion {
                    id: Uuid::new_v4(),
                    text: "What is the Rust keyword for defining a function?".to_string(),
                    options: vec!["func".to_string(), "fn".to_string(), "function".to_string(), "def".to_string()],
                    correct_answer_index: 1,
                    explanation: Some("In Rust, functions are defined using the `fn` keyword.".to_string()),
                },
                McqQuestion {
                    id: Uuid::new_v4(),
                    text: "Which of the following is Rust's package manager?".to_string(),
                    options: vec!["npm".to_string(), "pip".to_string(), "cargo".to_string(), "gem".to_string()],
                    correct_answer_index: 2,
                    explanation: Some("Cargo is Rust's build system and package manager.".to_string()),
                },
            ],
        }
    }

    #[test]
    fn test_mcq_flow() {
        let mut handler = McqHandler::new();
        let quiz = create_sample_quiz();
        let quiz_id = quiz.id;
        handler.add_quiz(quiz);

        // Start quiz
        let first_q = handler.start_quiz(quiz_id).unwrap();
        assert_eq!(first_q.text, "What is the Rust keyword for defining a function?");
        assert_eq!(handler.get_active_session().unwrap().current_question_index, 0);

        // Submit correct answer for first question
        let is_correct = handler.submit_answer(1).unwrap();
        assert!(is_correct);
        assert_eq!(handler.get_active_session().unwrap().score, 1);

        // Move to next question
        let second_q = handler.next_question().unwrap();
        assert_eq!(second_q.text, "Which of the following is Rust's package manager?");
        assert_eq!(handler.get_active_session().unwrap().current_question_index, 1);

        // Submit incorrect answer for second question
        let is_correct = handler.submit_answer(0).unwrap();
        assert!(!is_correct);
        assert_eq!(handler.get_active_session().unwrap().score, 1); // Score remains 1

        // Move to next question (should be end of quiz)
        assert!(handler.next_question().is_none());
        let session = handler.get_active_session().unwrap();
        assert!(session.completed);
        assert!(session.end_time.is_some());
        assert_eq!(session.score, 1);
        assert_eq!(session.total_questions, 2);

        // Try to submit answer after completion
        let err = handler.submit_answer(0);
        assert!(err.is_err());
        assert_eq!(err.unwrap_err(), "Quiz session already completed.".to_string());
    }
}
