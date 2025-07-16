use std::collections::HashMap;

/// A simple language detector.
/// This is a conceptual stub and would require a more sophisticated algorithm
/// or integration with a language detection library (e.g., `whatlang`).
pub struct LanguageDetector {
    // Could hold language profiles, n-gram models, etc.
    supported_languages: HashMap<String, String>, // e.g., "en" -> "English"
}

impl LanguageDetector {
    /// Creates a new `LanguageDetector` instance.
    pub fn new() -> Self {
        let mut supported_languages = HashMap::new();
        supported_languages.insert("en".to_string(), "English".to_string());
        supported_languages.insert("es".to_string(), "Spanish".to_string());
        supported_languages.insert("fr".to_string(), "French".to_string());
        supported_languages.insert("de".to_string(), "German".to_string());
        supported_languages.insert("zh".to_string(), "Chinese".to_string());
        supported_languages.insert("ja".to_string(), "Japanese".to_string());
        supported_languages.insert("ru".to_string(), "Russian".to_string());
        supported_languages.insert("ar".to_string(), "Arabic".to_string());
        Self { supported_languages }
    }

    /// Detects the language of the given text.
    /// Returns the ISO 639-1 code (e.g., "en", "es") and the confidence score (0.0-1.0).
    /// This is a very basic, rule-based or keyword-based simulation.
    pub fn detect(&self, text: &str) -> Option<(String, f32)> {
        let lower_text = text.to_lowercase();

        // Simple keyword-based detection
        if lower_text.contains("hello") || lower_text.contains("world") || lower_text.contains("rust") {
            return Some(("en".to_string(), 0.9));
        }
        if lower_text.contains("hola") || lower_text.contains("mundo") {
            return Some(("es".to_string(), 0.8));
        }
        if lower_text.contains("bonjour") || lower_text.contains("monde") {
            return Some(("fr".to_string(), 0.7));
        }
        if lower_text.contains("guten tag") || lower_text.contains("welt") {
            return Some(("de".to_string(), 0.75));
        }

        // Fallback to a default or "unknown" if no strong match
        if !text.is_empty() {
            Some(("und".to_string(), 0.1)) // "und" for undetermined
        } else {
            None
        }
    }

    /// Returns the full name of a language given its ISO 639-1 code.
    pub fn get_language_name(&self, iso_code: &str) -> Option<&String> {
        self.supported_languages.get(iso_code)
    }

    /// Returns a list of all supported language codes.
    pub fn get_supported_language_codes(&self) -> Vec<String> {
        self.supported_languages.keys().cloned().collect()
    }
}

pub fn init() {
    println!("natural_language_detection module initialized: Provides basic language detection.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        let detector = LanguageDetector::new();

        let (lang, confidence) = detector.detect("Hello, world!").unwrap();
        assert_eq!(lang, "en");
        assert!(confidence > 0.5);

        let (lang, confidence) = detector.detect("Hola mundo").unwrap();
        assert_eq!(lang, "es");
        assert!(confidence > 0.5);

        let (lang, confidence) = detector.detect("Bonjour").unwrap();
        assert_eq!(lang, "fr");
        assert!(confidence > 0.5);

        let (lang, confidence) = detector.detect("").unwrap();
        assert_eq!(lang, "und"); // Empty string might default to undetermined or None depending on implementation
        assert!(confidence < 0.5);

        assert!(detector.detect("Some random text that doesn't match keywords").is_some());
        assert_eq!(detector.detect("Some random text that doesn't match keywords").unwrap().0, "und");
    }

    #[test]
    fn test_get_language_name() {
        let detector = LanguageDetector::new();
        assert_eq!(detector.get_language_name("en"), Some(&"English".to_string()));
        assert_eq!(detector.get_language_name("es"), Some(&"Spanish".to_string()));
        assert_eq!(detector.get_language_name("xyz"), None);
    }
}
