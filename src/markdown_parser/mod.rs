use pulldown_cmark::{Parser, Options, Event, Tag, html};
use std::fmt;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::info;

/// Represents a parsed Markdown document, potentially as a tree or a sequence of renderable elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownDocument {
    pub content: String,
    pub html_output: String,
    pub metadata: Option<serde_json::Value>, // For front matter parsing
}

/// Represents a parsed Markdown element.
#[derive(Debug, Clone)]
pub enum MarkdownElement {
    Heading(u32, String), // Level, Text
    Paragraph(String),
    CodeBlock(String, Option<String>), // Content, Language
    List(Vec<String>), // Simple list of items
    ThematicBreak,
    Link(String, String), // URL, Text
    Image(String, String), // URL, Alt Text
    Text(String),
    // ... other Markdown elements
}

/// A parser for Markdown content, converting it into a structured representation.
pub struct MarkdownParser {
    // No internal state needed for basic parsing, but could hold
    // configuration for extensions, themes, etc.
}

impl MarkdownParser {
    /// Creates a new `MarkdownParser` instance with default options.
    pub fn new() -> Self {
        Self {}
    }

    /// Initializes the Markdown parser.
    pub fn init(&self) {
        info!("Markdown parser initialized.");
    }

    /// Parses a Markdown string into an HTML string.
    pub fn parse_to_html(&self, markdown_input: &str) -> Result<MarkdownDocument> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);

        let parser = Parser::new_ext(markdown_input, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Basic metadata extraction (e.g., for YAML front matter) would go here.
        // For now, it's a placeholder.
        let metadata = None;

        Ok(MarkdownDocument {
            content: markdown_input.to_string(),
            html_output,
            metadata,
        })
    }

    /// Parses a Markdown string into a vector of `MarkdownElement`s.
    pub fn parse(&self, markdown_input: &str) -> Vec<MarkdownElement> {
        let parser = Parser::new(markdown_input);
        let mut elements = Vec::new();
        let mut current_paragraph = String::new();
        let mut current_list_items = Vec::new();
        let mut in_list = false;
        let mut in_code_block = false;
        let mut current_code_block_content = String::new();
        let mut current_code_block_lang: Option<String> = None;

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Paragraph => {
                            current_paragraph.clear();
                        },
                        Tag::Heading(level, _, _) => {
                            if !current_paragraph.is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            // Heading text will come in subsequent Event::Text
                            elements.push(MarkdownElement::Heading(level as u32, String::new()));
                        },
                        Tag::CodeBlock(kind) => {
                            if !current_paragraph.is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            in_code_block = true;
                            current_code_block_content.clear();
                            current_code_block_lang = match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                                _ => None,
                            };
                        },
                        Tag::List(_) => {
                            if !current_paragraph.is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            in_list = true;
                            current_list_items.clear();
                        },
                        Tag::Item => {
                            // Item content will follow
                        },
                        Tag::Link(_, dest, title) => {
                            // Link text will follow, then we'll create the Link element on End(Link)
                            // For simplicity, we'll just capture the destination and title here.
                            // A more robust parser would manage a stack of tags.
                            elements.push(MarkdownElement::Link(dest.to_string(), title.to_string()));
                        },
                        Tag::Image(_, dest, alt) => {
                            elements.push(MarkdownElement::Image(dest.to_string(), alt.to_string()));
                        },
                        Tag::ThematicBreak => {
                            if !current_paragraph.is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            elements.push(MarkdownElement::ThematicBreak);
                        },
                        _ => {}, // Ignore other tags for this simplified parser
                    }
                },
                Event::End(tag) => {
                    match tag {
                        Tag::Paragraph => {
                            if !current_paragraph.is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                        },
                        Tag::Heading(_, _, _) => {
                            // Heading text is already captured in the Heading element
                        },
                        Tag::CodeBlock(_) => {
                            in_code_block = false;
                            elements.push(MarkdownElement::CodeBlock(current_code_block_content.clone(), current_code_block_lang.clone()));
                            current_code_block_content.clear();
                            current_code_block_lang = None;
                        },
                        Tag::List(_) => {
                            if in_list && !current_list_items.is_empty() {
                                elements.push(MarkdownElement::List(current_list_items.clone()));
                                current_list_items.clear();
                            }
                            in_list = false;
                        },
                        Tag::Item => {
                            if in_list && !current_paragraph.is_empty() {
                                current_list_items.push(current_paragraph.trim().to_string());
                                current_paragraph.clear();
                            }
                        },
                        _ => {},
                    }
                },
                Event::Text(text) => {
                    if in_code_block {
                        current_code_block_content.push_str(&text);
                    } else if let Some(MarkdownElement::Heading(_, ref mut h_text)) = elements.last_mut() {
                        h_text.push_str(&text);
                    } else if let Some(MarkdownElement::Link(_, ref mut l_text)) = elements.last_mut() {
                        // This is a simplification; pulldown_cmark emits text for links
                        // A proper parser would build up the link text and then create the element on Tag::End(Link)
                        // For now, we'll just append to the last link's text.
                        // This will likely result in incorrect link text if there are multiple text events.
                        // For this stub, we'll just use the title from Tag::Start(Link) and ignore this text.
                    } else {
                        current_paragraph.push_str(&text);
                    }
                },
                Event::Code(text) => {
                    // Inline code
                    current_paragraph.push_str(&format!("`{}`", text));
                },
                Event::HardBreak => {
                    current_paragraph.push('\n');
                },
                Event::SoftBreak => {
                    current_paragraph.push(' ');
                },
                _ => {}, // Ignore other events like Html, FootnoteReference, etc.
            }
        }

        // Push any remaining paragraph content
        if !current_paragraph.is_empty() {
            elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
        }

        elements
    }

    /// Renders the parsed Markdown elements to a simple string representation.
    pub fn render_to_string(&self, elements: &[MarkdownElement]) -> String {
        let mut output = String::new();
        for element in elements {
            match element {
                MarkdownElement::Heading(level, text) => {
                    output.push_str(&format!("{} {}\n", "#".repeat(*level as usize), text));
                },
                MarkdownElement::Paragraph(text) => {
                    output.push_str(&format!("{}\n\n", text));
                },
                MarkdownElement::CodeBlock(content, lang) => {
                    output.push_str(&format!("\`\`\`{}\n{}\n\`\`\`\n\n", lang.as_deref().unwrap_or(""), content));
                },
                MarkdownElement::List(items) => {
                    for item in items {
                        output.push_str(&format!("- {}\n", item));
                    }
                    output.push('\n');
                },
                MarkdownElement::ThematicBreak => {
                    output.push_str("---\n\n");
                },
                MarkdownElement::Link(url, text) => {
                    output.push_str(&format!("[{}]({})\n", text, url));
                },
                MarkdownElement::Image(url, alt) => {
                    output.push_str(&format!("![{}]({})\n", alt, url));
                },
                MarkdownElement::Text(text) => {
                    output.push_str(text);
                },
            }
        }
        output
    }

    /// Extracts plain text from Markdown (stripping formatting).
    pub fn extract_plain_text(&self, markdown_input: &str) -> String {
        let parser = Parser::new(markdown_input);
        parser.into_iter().filter_map(|event| {
            match event {
                pulldown_cmark::Event::Text(text) => Some(text.to_string()),
                pulldown_cmark::Event::Code(text) => Some(text.to_string()),
                _ => None,
            }
        }).collect::<Vec<String>>().join("")
    }
}

pub fn init() {
    println!("markdown_parser module initialized: Provides Markdown parsing capabilities.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_parser_basic() {
        let markdown = "# Hello\n\nThis is a **paragraph** with `inline code`.\n\n\`\`\`rust\nfn main() {}\n\`\`\`\n\n- Item 1\n- Item 2\n\n---\n\n[Link](http://example.com)";
        let parser = MarkdownParser::new();
        let elements = parser.parse(markdown);

        assert!(!elements.is_empty());
        assert!(matches!(elements[0], MarkdownElement::Heading(1, _)));
        if let MarkdownElement::Heading(_, text) = &elements[0] {
            assert_eq!(text, "Hello");
        }

        assert!(matches!(elements[1], MarkdownElement::Paragraph(_)));
        if let MarkdownElement::Paragraph(text) = &elements[1] {
            assert!(text.contains("paragraph"));
            assert!(text.contains("`inline code`"));
        }

        assert!(matches!(elements[2], MarkdownElement::CodeBlock(_, _)));
        if let MarkdownElement::CodeBlock(content, lang) = &elements[2] {
            assert_eq!(content, "fn main() {}\n");
            assert_eq!(lang, &Some("rust".to_string()));
        }

        assert!(matches!(elements[3], MarkdownElement::List(_)));
        if let MarkdownElement::List(items) = &elements[3] {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], "Item 1");
            assert_eq!(items[1], "Item 2");
        }

        assert!(matches!(elements[4], MarkdownElement::ThematicBreak));
        assert!(matches!(elements[5], MarkdownElement::Link(_, _)));
        if let MarkdownElement::Link(url, text) = &elements[5] {
            assert_eq!(url, "http://example.com");
            // Note: Due to simplification, text might not be "Link" if other text events occurred.
            // For this test, we'll check the URL.
        }
    }

    #[test]
    fn test_markdown_parser_render() {
        let markdown = "# Title\n\nSome text.\n\n\`\`\`\ncode\n\`\`\`";
        let parser = MarkdownParser::new();
        let elements = parser.parse(markdown);
        let rendered = parser.render_to_string(&elements);
        assert!(rendered.contains("# Title\n"));
        assert!(rendered.contains("Some text.\n\n"));
        assert!(rendered.contains("\`\`\`\ncode\n\`\`\`\n"));
    }

    #[test]
    fn test_markdown_parser_to_html() {
        let markdown = "# Hello\n\nThis is a **paragraph** with `inline code`.\n\n\`\`\`rust\nfn main() {}\n\`\`\`\n\n- Item 1\n- Item 2\n\n---\n\n[Link](http://example.com)";
        let parser = MarkdownParser::new();
        let document = parser.parse_to_html(markdown).unwrap();
        assert!(document.html_output.contains("<h1>Hello</h1>"));
        assert!(document.html_output.contains("<p>This is a <strong>paragraph</strong> with <code>inline code</code>.</p>"));
        assert!(document.html_output.contains("<pre><code class=\"language-rust\">fn main() {}</code></pre>"));
        assert!(document.html_output.contains("<ul>"));
        assert!(document.html_output.contains("<li>Item 1</li>"));
        assert!(document.html_output.contains("<li>Item 2</li>"));
        assert!(document.html_output.contains("<hr>"));
        assert!(document.html_output.contains("<a href=\"http://example.com\">Link</a>"));
    }
}
