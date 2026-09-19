use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Footer<'a> {
    theme: &'a Theme,
    is_paused: bool,
}

impl<'a> Footer<'a> {
    pub fn new(theme: &'a Theme, is_paused: bool) -> Self {
        Self { theme, is_paused }
    }
}

impl<'a> Widget for Footer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        let bg_style = if let Some(bg) = theme.bg {
            Style::default().bg(bg)
        } else {
            Style::default()
        };

        let pause_label = if self.is_paused {
            Span::styled(
                " [PAUSED] ",
                Style::default()
                    .bg(theme.status_offline)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        };

        let keys = vec![
            pause_label,
            Span::styled(" [Esc] ", Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled("Options/Themes  ", Style::default().fg(theme.fg_dim)),
            Span::styled("[Tab] ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("View  ", Style::default().fg(theme.fg_dim)),
            Span::styled("[↑/↓] ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("Select/Scroll  ", Style::default().fg(theme.fg_dim)),
            Span::styled("[p] ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("Pause  ", Style::default().fg(theme.fg_dim)),
            Span::styled("[1-5] ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("Direct  ", Style::default().fg(theme.fg_dim)),
            Span::styled("[q] ", Style::default().fg(theme.status_offline).add_modifier(Modifier::BOLD)),
            Span::styled("Quit", Style::default().fg(theme.fg_dim)),
        ];

        Paragraph::new(Line::from(keys))
            .style(bg_style)
            .render(area, buf);
    }
}
