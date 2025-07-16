use anyhow::{Result, anyhow};
use unicode_segmentation::UnicodeSegmentation;

// This module provides utilities for converting between different string offset
// types (byte, character, grapheme cluster) for accurate text manipulation.

pub struct StringOffsetManager {}

impl StringOffsetManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&self) {
        log::info!("String offset manager initialized.");
    }

    /// Converts a character index to a byte index.
    /// Returns `None` if the character index is out of bounds.
    pub fn char_to_byte_idx(&self, s: &str, char_idx: usize) -> Option<usize> {
        s.char_indices().nth(char_idx).map(|(byte_idx, _)| byte_idx)
    }

    /// Converts a byte index to a character index.
    /// Returns `None` if the byte index is not at a character boundary or out of bounds.
    pub fn byte_to_char_idx(&self, s: &str, byte_idx: usize) -> Option<usize> {
        if !s.is_char_boundary(byte_idx) {
            return None;
        }
        s[..byte_idx].chars().count().into()
    }

    /// Converts a grapheme cluster index to a byte index.
    /// Returns `None` if the grapheme index is out of bounds.
    pub fn grapheme_to_byte_idx(&self, s: &str, grapheme_idx: usize) -> Option<usize> {
        s.grapheme_indices(true).nth(grapheme_idx).map(|(byte_idx, _)| byte_idx)
    }

    /// Converts a byte index to a grapheme cluster index.
    /// Returns `None` if the byte index is not at a grapheme cluster boundary or out of bounds.
    pub fn byte_to_grapheme_idx(&self, s: &str, byte_idx: usize) -> Option<usize> {
        if !s.is_char_boundary(byte_idx) {
            return None; // Grapheme boundaries are also char boundaries
        }
        s[..byte_idx].graphemes(true).count().into()
    }

    /// Converts a character index to a grapheme cluster index.
    /// This is less common but can be useful.
    pub fn char_to_grapheme_idx(&self, s: &str, char_idx: usize) -> Option<usize> {
        let byte_idx = self.char_to_byte_idx(s, char_idx)?;
        self.byte_to_grapheme_idx(s, byte_idx)
    }

    /// Converts a grapheme cluster index to a character index.
    pub fn grapheme_to_char_idx(&self, s: &str, grapheme_idx: usize) -> Option<usize> {
        let byte_idx = self.grapheme_to_byte_idx(s, grapheme_idx)?;
        self.byte_to_char_idx(s, byte_idx)
    }
}
