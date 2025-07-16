use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct CollapsibleBlock {
    pub title: String,
    pub content: Vec<Line<'static>>,
    pub is_collapsed: bool,
    pub is_active: bool,
}

impl CollapsibleBlock {
    pub fn new(title: String, content: Vec<Line<'static>>) -> Self {
        Self {
            title,
            content,
            is_collapsed: false,
            is_active: false,
        }
    }

    pub fn toggle_collapse(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let title_text = if self.is_collapsed {
            format!("{} [▶]", self.title)
        } else {
            format!("{} [▼]", self.title)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title_text, Style::default().fg(Color::LightGreen)));

        if self.is_collapsed {
            frame.render_widget(block, area);
        } else {
            let inner_area = block.inner(area);
            frame.render_widget(block, area);

            let paragraph = Paragraph::new(self.content.clone())
                .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(paragraph, inner_area);
        }
    }
}
