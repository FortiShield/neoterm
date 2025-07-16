use anyhow::Result;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyMatchResult {
    pub text: String,
    pub score: i64,
    pub indices: Vec<usize>,
}

pub struct FuzzyMatchManager {
    matcher: SkimMatcherV2,
}

impl FuzzyMatchManager {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default().ignore_case(),
        }
    }

    pub fn init(&self) {
        log::info!("Fuzzy match manager initialized.");
    }

    /// Performs a fuzzy match against a list of candidates.
    pub fn fuzzy_match(&self, query: &str, candidates: &[String]) -> Vec<FuzzyMatchResult> {
        let mut results: Vec<FuzzyMatchResult> = candidates
            .iter()
            .filter_map(|candidate| {
                self.matcher.fuzzy_match_with_matches(candidate, query)
                    .map(|(score, indices)| FuzzyMatchResult {
                        text: candidate.clone(),
                        score,
                        indices,
                    })
            })
            .collect();

        // Sort by score in descending order
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Highlights the matched characters in a string.
    pub fn highlight_match(text: &str, indices: &[usize], highlight_char: char) -> String {
        let mut highlighted_text = String::new();
        let mut last_index = 0;

        for &idx in indices {
            if idx >= text.len() { continue; } // Should not happen with correct indices

            // Append text before the match
            highlighted_text.push_str(&text[last_index..idx]);
            // Append the matched character with highlight
            highlighted_text.push(highlight_char);
            highlighted_text.push(text.chars().nth(idx).unwrap()); // Get char at index
            highlighted_text.push(highlight_char);
            last_index = idx + text.chars().nth(idx).unwrap().len_utf8(); // Move past the char
        }
        // Append any remaining text
        highlighted_text.push_str(&text[last_index..]);
        highlighted_text
    }
}
