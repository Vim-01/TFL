use crate::app::{ActiveTab, App};
use chrono::Local;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct Header<'a> {
    app: &'a App,
}

impl<'a> Header<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for Header<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let bg_style = if let Some(bg) = theme.bg {
            Style::default().bg(bg)
        } else {
            Style::default()
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(25), // Title & Logo
                Constraint::Min(45),    // Navigation Tabs + Options Menu
                Constraint::Length(26), // Backend Status & Clock
            ])
            .split(area);

        // 1. Logo & App name
        let title_spans = vec![
            Span::styled("⚡ ", Style::default().fg(theme.fg_highlight)),
            Span::styled(
                "TFL",
                Style::default()
                    .fg(theme.border_active)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [Top For LLM] ", Style::default().fg(theme.fg_dim)),
            Span::styled(
                "v0.1",
                Style::default().fg(theme.fg_dim),
            ),
        ];
        Paragraph::new(Line::from(title_spans))
            .style(bg_style)
            .render(chunks[0], buf);

        // 2. Navigation Tabs & Menu
        let tabs = [
            (ActiveTab::Dashboard, "1:Dashboard"),
            (ActiveTab::GpuDetails, "2:GPU"),
            (ActiveTab::SlotsDetails, "3:Slots"),
            (ActiveTab::GpuTop, "4:GPU Top"),
            (ActiveTab::Help, "5:Help"),
        ];

        let mut tab_spans = Vec::new();
        for (tab, label) in tabs {
            let is_active = self.app.active_tab == tab;
            let style = if is_active {
                Style::default()
                    .fg(theme.bg.unwrap_or(ratatui::style::Color::Black))
                    .bg(theme.border_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg_dim)
            };

            tab_spans.push(Span::raw(" "));
            tab_spans.push(Span::styled(format!(" {label} "), style));
        }

        // Btop-style menu button
        let menu_style = if self.app.options_menu_open {
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(theme.fg_highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.fg_highlight)
                .add_modifier(Modifier::BOLD)
        };
        tab_spans.push(Span::raw("  "));
        tab_spans.push(Span::styled(" [Esc:Options/Theme] ", menu_style));

        Paragraph::new(Line::from(tab_spans))
            .style(bg_style)
            .render(chunks[1], buf);

        // 3. Status & Clock
        let (conn_status, conn_color) = if let Some(ref llm) = self.app.llm {
            if llm.is_connected {
                ("● ONLINE", theme.status_online)
            } else {
                ("○ OFFLINE", theme.status_offline)
            }
        } else {
            ("○ CONNECTING", theme.fg_highlight)
        };

        let now_str = Local::now().format("%H:%M:%S").to_string();
        let status_spans = vec![
            Span::styled(
                format!("{conn_status} "),
                Style::default()
                    .fg(conn_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(theme.fg_dim)),
            Span::styled(now_str, Style::default().fg(theme.fg)),
            Span::raw(" "),
        ];

        Paragraph::new(Line::from(status_spans))
            .style(bg_style)
            .alignment(Alignment::Right)
            .render(chunks[2], buf);
    }
}
