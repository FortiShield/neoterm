/// This module provides utilities for working with string offsets,
/// particularly for converting between byte offsets, character offsets,
/// and line/column numbers in multi-byte character strings (like UTF-8).
/// This is crucial for text editors, terminal emulators, and any
/// application that needs precise cursor positioning or text selection.

/// Represents a position within a string, typically as (line, column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextPosition {
    pub line: usize,    // 0-indexed line number
    pub column: usize,  // 0-indexed column (character count)
}

impl Default for TextPosition {
    fn default() -> Self {
        Self { line: 0, column: 0 }
    }
}

/// A utility struct to efficiently convert between different string offset types.
/// It pre-calculates line start byte offsets for faster lookups.
pub struct StringOffsetConverter {
    text: String,
    line_start_byte_offsets: Vec<usize>, // Stores byte offset of the start of each line
}

impl StringOffsetConverter {
    /// Creates a new `StringOffsetConverter` for the given text.
    pub fn new(text: String) -> Self {
        let mut line_start_byte_offsets = vec![0]; // First line starts at byte 0
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_start_byte_offsets.push(i + c.len_utf8());
            }
        }
        Self {
            text,
            line_start_byte_offsets,
        }
    }

    /// Converts a byte offset to a `TextPosition` (line, column).
    pub fn byte_to_position(&self, byte_offset: usize) -> Option<TextPosition> {
        if byte_offset > self.text.len() {
            return None;
        }

        // Find the line number
        let line = match self.line_start_byte_offsets.binary_search(&byte_offset) {
            Ok(l) => l, // Exact match, it's the start of a line
            Err(l) => l.saturating_sub(1), // Not exact, it's in the previous line
        };

        let line_start_byte = *self.line_start_byte_offsets.get(line)?;
        let line_slice = &self.text[line_start_byte..byte_offset];

        // Calculate column by counting characters in the slice
        let column = line_slice.chars().count();

        Some(TextPosition { line, column })
    }

    /// Converts a `TextPosition` (line, column) to a byte offset.
    pub fn position_to_byte(&self, position: TextPosition) -> Option<usize> {
        let line_start_byte = *self.line_start_byte_offsets.get(position.line)?;
        let line_content = self.text.lines().nth(position.line)?;

        let mut current_column = 0;
        let mut byte_offset_in_line = 0;

        for (i, c) in line_content.char_indices() {
            if current_column == position.column {
                break;
            }
            current_column += 1;
            byte_offset_in_line = i + c.len_utf8();
        }

        if current_column < position.column {
            // Requested column is beyond the end of the line
            // Return the byte offset of the end of the line
            Some(line_start_byte + line_content.len())
        } else {
            Some(line_start_byte + byte_offset_in_line)
        }
    }

    /// Returns the content of a specific line.
    pub fn get_line_content(&self, line_index: usize) -> Option<&str> {
        self.text.lines().nth(line_index)
    }

    /// Returns the total number of lines in the text.
    pub fn total_lines(&self) -> usize {
        self.line_start_byte_offsets.len()
    }

    /// Returns the total length of the text in bytes.
    pub fn total_bytes(&self) -> usize {
        self.text.len()
    }
}

pub fn init() {
    println!("string_offset module initialized: Provides string offset conversion utilities.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_position() {
        let text = "Hello, world!\nRust is great.\n🦀🚀";
        let converter = StringOffsetConverter::new(text.to_string());

        // Test start of line
        assert_eq!(converter.byte_to_position(0), Some(TextPosition { line: 0, column: 0 }));
        assert_eq!(converter.byte_to_position(14), Some(TextPosition { line: 1, column: 0 })); // After '\n'
        assert_eq!(converter.byte_to_position(29), Some(TextPosition { line: 2, column: 0 })); // After '\n'

        // Test mid-line
        assert_eq!(converter.byte_to_position(7), Some(TextPosition { line: 0, column: 7 }));
        assert_eq!(converter.byte_to_position(20), Some(TextPosition { line: 1, column: 6 }));

        // Test multi-byte characters
        assert_eq!(converter.byte_to_position(30), Some(TextPosition { line: 2, column: 1 })); // 🦀 is 4 bytes
        assert_eq!(converter.byte_to_position(34), Some(TextPosition { line: 2, column: 2 })); // 🚀 is 4 bytes

        // Test end of string
        assert_eq!(converter.byte_to_position(text.len()), Some(TextPosition { line: 2, column: 3 })); // After 🚀
        assert_eq!(converter.byte_to_position(text.len() + 1), None); // Out of bounds
    }

    #[test]
    fn test_position_to_byte() {
        let text = "Hello, world!\nRust is great.\n🦀🚀";
        let converter = StringOffsetConverter::new(text.to_string());

        // Test start of line
        assert_eq!(converter.position_to_byte(TextPosition { line: 0, column: 0 }), Some(0));
        assert_eq!(converter.position_to_byte(TextPosition { line: 1, column: 0 }), Some(14));
        assert_eq!(converter.position_to_byte(TextPosition { line: 2, column: 0 }), Some(29));

        // Test mid-line
        assert_eq!(converter.position_to_byte(TextPosition { line: 0, column: 7 }), Some(7));
        assert_eq!(converter.position_to_byte(TextPosition { line: 1, column: 6 }), Some(20));

        // Test multi-byte characters
        assert_eq!(converter.position_to_byte(TextPosition { line: 2, column: 1 }), Some(33)); // After 🦀
        assert_eq!(converter.position_to_byte(TextPosition { line: 2, column: 2 }), Some(37)); // After 🚀

        // Test beyond end of line
        assert_eq!(converter.position_to_byte(TextPosition { line: 0, column: 100 }), Some(13)); // End of "Hello, world!"
        assert_eq!(converter.position_to_byte(TextPosition { line: 2, column: 100 }), Some(37)); // End of "🦀🚀"

        // Test out of bounds line
        assert_eq!(converter.position_to_byte(TextPosition { line: 100, column: 0 }), None);
    }

    #[test]
    fn test_empty_string() {
        let text = "";
        let converter = StringOffsetConverter::new(text.to_string());
        assert_eq!(converter.total_lines(), 1); // An empty string still has one line
        assert_eq!(converter.total_bytes(), 0);
        assert_eq!(converter.byte_to_position(0), Some(TextPosition { line: 0, column: 0 }));
        assert_eq!(converter.position_to_byte(TextPosition { line: 0, column: 0 }), Some(0));
        assert_eq!(converter.byte_to_position(1), None);
    }

    #[test]
    fn test_only_newline() {
        let text = "\n";
        let converter = StringOffsetConverter::new(text.to_string());
        assert_eq!(converter.total_lines(), 2);
        assert_eq!(converter.byte_to_position(0), Some(TextPosition { line: 0, column: 0 }));
        assert_eq!(converter.byte_to_position(1), Some(TextPosition { line: 1, column: 0 }));
        assert_eq!(converter.position_to_byte(TextPosition { line: 0, column: 0 }), Some(0));
        assert_eq!(converter.position_to_byte(TextPosition { line: 1, column: 0 }), Some(1));
    }
}
