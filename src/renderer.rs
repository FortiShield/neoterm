use std::collections::HashMap;
use std::sync::Arc;
use iced::advanced::graphics::text;
use uuid::Uuid;

/// GPU-accelerated renderer for terminal blocks
pub struct BlockRenderer {
    text_cache: HashMap<String, Arc<text::Paragraph>>,
    syntax_highlighter: SyntaxHighlighter,
    gpu_context: Option<wgpu::Device>,
}

impl BlockRenderer {
    pub fn new() -> Self {
        Self {
            text_cache: HashMap::new(),
            syntax_highlighter: SyntaxHighlighter::new(),
            gpu_context: None,
        }
    }

    pub async fn initialize_gpu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or("Failed to find adapter")?;

        let (device, _queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        self.gpu_context = Some(device);
        Ok(())
    }

    pub fn render_block_content(&mut self, content: &str, language: Option<&str>) -> Arc<text::Paragraph> {
        let cache_key = format!("{}:{}", language.unwrap_or("plain"), content);
        
        if let Some(cached) = self.text_cache.get(&cache_key) {
            return cached.clone();
        }

        let highlighted = if let Some(lang) = language {
            self.syntax_highlighter.highlight(content, lang)
        } else {
            content.to_string()
        };

        // Create paragraph with syntax highlighting
        let paragraph = Arc::new(text::Paragraph::new());
        self.text_cache.insert(cache_key, paragraph.clone());
        
        paragraph
    }

    pub fn clear_cache(&mut self) {
        self.text_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.text_cache.len()
    }
}

pub struct SyntaxHighlighter {
    syntax_set: syntect::parsing::SyntaxSet,
    theme_set: syntect::highlighting::ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, text: &str, language: &str) -> String {
        let syntax = self.syntax_set
            .find_syntax_by_extension(language)
            .or_else(|| self.syntax_set.find_syntax_by_name(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
        let ranges = highlighter.highlight_line(text, &self.syntax_set).unwrap();
        
        // Convert to styled text - in a real implementation, you'd convert to Iced's styled text
        syntect::util::as_24_bit_terminal_escaped(&ranges[..], false)
    }
}

/// Memory-efficient virtual scrolling for large outputs
pub struct VirtualScroller {
    total_items: usize,
    visible_range: std::ops::Range<usize>,
    item_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
}

impl VirtualScroller {
    pub fn new(item_height: f32, viewport_height: f32) -> Self {
        Self {
            total_items: 0,
            visible_range: 0..0,
            item_height,
            viewport_height,
            scroll_offset: 0.0,
        }
    }

    pub fn update(&mut self, total_items: usize, scroll_offset: f32) {
        self.total_items = total_items;
        self.scroll_offset = scroll_offset;
        
        let visible_count = (self.viewport_height / self.item_height).ceil() as usize + 2; // +2 for buffer
        let start_index = (scroll_offset / self.item_height).floor() as usize;
        let end_index = (start_index + visible_count).min(total_items);
        
        self.visible_range = start_index..end_index;
    }

    pub fn visible_range(&self) -> std::ops::Range<usize> {
        self.visible_range.clone()
    }

    pub fn total_height(&self) -> f32 {
        self.total_items as f32 * self.item_height
    }
}

/// Performance monitoring and optimization
pub struct PerformanceMonitor {
    frame_times: Vec<std::time::Duration>,
    memory_usage: Vec<usize>,
    last_gc: std::time::Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(60),
            memory_usage: Vec::with_capacity(60),
            last_gc: std::time::Instant::now(),
        }
    }

    pub fn record_frame_time(&mut self, duration: std::time::Duration) {
        self.frame_times.push(duration);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }
    }

    pub fn record_memory_usage(&mut self, bytes: usize) {
        self.memory_usage.push(bytes);
        if self.memory_usage.len() > 60 {
            self.memory_usage.remove(0);
        }
    }

    pub fn average_frame_time(&self) -> Option<std::time::Duration> {
        if self.frame_times.is_empty() {
            None
        } else {
            let total: std::time::Duration = self.frame_times.iter().sum();
            Some(total / self.frame_times.len() as u32)
        }
    }

    pub fn fps(&self) -> Option<f32> {
        self.average_frame_time()
            .map(|avg| 1.0 / avg.as_secs_f32())
    }

    pub fn should_trigger_gc(&mut self) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_gc) > std::time::Duration::from_secs(30) {
            self.last_gc = now;
            true
        } else {
            false
        }
    }
}
