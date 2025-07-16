// fuzzy_match module stub

/// A simple fuzzy matching utility.
/// This module provides functions to perform fuzzy string matching,
/// useful for search, command palette, and autocomplete features.
pub struct FuzzyMatcher {
    // Configuration for fuzzy matching algorithm (e.g., scoring parameters)
    case_sensitive: bool,
}

impl FuzzyMatcher {
    /// Creates a new `FuzzyMatcher` instance.
    pub fn new(case_sensitive: bool) -> Self {
        Self { case_sensitive }
    }

    /// Performs a fuzzy match between a `pattern` and a `text`.
    /// Returns a score (higher is better) and optionally the matched indices.
    /// A score of 0 means no match.
    pub fn match_string(&self, pattern: &str, text: &str) -> (f64, Option<Vec<usize>>) {
        let (pattern_chars, text_chars) = if self.case_sensitive {
            (pattern.chars().collect::<Vec<_>>(), text.chars().collect::<Vec<_>>())
        } else {
            (pattern.to_lowercase().chars().collect::<Vec<_>>(), text.to_lowercase().chars().collect::<Vec<_>>())
        };

        if pattern_chars.is_empty() {
            return (1.0, Some((0..text_chars.len()).collect())); // Empty pattern matches everything
        }
        if text_chars.is_empty() {
            return (0.0, None); // Cannot match non-empty pattern in empty text
        }

        let mut score = 0.0;
        let mut matched_indices = Vec::new();
        let mut text_idx = 0;

        for p_char in pattern_chars {
            let mut found = false;
            while text_idx < text_chars.len() {
                if text_chars[text_idx] == p_char {
                    score += 1.0; // Basic score for a match
                    matched_indices.push(text_idx);
                    found = true;
                    text_idx += 1;
                    break;
                }
                text_idx += 1;
            }
            if !found {
                return (0.0, None); // Pattern character not found
            }
        }

        // Add bonus for consecutive matches, start of word matches, etc.
        // This is a very basic scoring. Real fuzzy matchers use more complex algorithms (e.g., Levenshtein distance, trigrams).
        let mut consecutive_bonus = 0.0;
        for i in 1..matched_indices.len() {
            if matched_indices[i] == matched_indices[i-1] + 1 {
                consecutive_bonus += 0.1;
            }
        }
        score += consecutive_bonus;

        // Normalize score (e.g., by pattern length or text length)
        score /= pattern.len() as f64;

        (score, Some(matched_indices))
    }

    /// Finds the best fuzzy match for a `pattern` within a list of `candidates`.
    /// Returns the best matching candidate and its score, if any.
    pub fn find_best_match<'a>(&self, pattern: &str, candidates: &'a [String]) -> Option<(&'a String, f64)> {
        let mut best_score = 0.0;
        let mut best_match = None;

        for candidate in candidates {
            let (score, _) = self.match_string(pattern, candidate);
            if score > best_score {
                best_score = score;
                best_match = Some(candidate);
            }
        }
        best_match.map(|m| (m, best_score))
    }
}

pub fn init() {
    println!("fuzzy_match module initialized: Provides fuzzy string matching.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_basic() {
        let matcher = FuzzyMatcher::new(false);
        let (score, indices) = matcher.match_string("abc", "axbyc");
        assert!(score > 0.0);
        assert_eq!(indices, Some(vec![0, 2, 4]));

        let (score, _) = matcher.match_string("abc", "xyz");
        assert_eq!(score, 0.0);

        let (score, _) = matcher.match_string("term", "neoterminal");
        assert!(score > 0.0);
    }

    #[test]
    fn test_fuzzy_match_case_sensitive() {
        let matcher = FuzzyMatcher::new(true);
        let (score, _) = matcher.match_string("abc", "AxByC");
        assert_eq!(score, 0.0);

        let (score, _) = matcher.match_string("abc", "abc");
        assert!(score > 0.0);
    }

    #[test]
    fn test_find_best_match() {
        let matcher = FuzzyMatcher::new(false);
        let candidates = vec![
            "apple".to_string(),
            "banana".to_string(),
            "apricot".to_string(),
            "grape".to_string(),
        ];

        let (best_match, score) = matcher.find_best_match("ap", &candidates).unwrap();
        assert_eq!(best_match, &"apple".to_string());
        assert!(score > 0.0);

        let (best_match, score) = matcher.find_best_match("ana", &candidates).unwrap();
        assert_eq!(best_match, &"banana".to_string());
        assert!(score > 0.0);

        assert!(matcher.find_best_match("xyz", &candidates).is_none());
    }
}
