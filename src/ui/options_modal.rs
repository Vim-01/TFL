use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

pub struct OptionsModal<'a> {
    app: &'a App,
}

impl<'a> OptionsModal<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for OptionsModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Center popup modal: 60 columns wide, 14 rows high
        let width = 64.min(area.width.saturating_sub(4));
        let height = 15.min(area.height.saturating_sub(2));

        let popup_x = area.x + (area.width.saturating_sub(width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(popup_x, popup_y, width, height);

        // Clear background behind modal
        Clear.render(popup_area, buf);

        let theme = &self.app.theme;
        let bg_color = theme.bg.unwrap_or(Color::Rgb(20, 22, 32));

        // Fill modal backdrop
        for y in popup_area.y..(popup_area.y + popup_area.height) {
            for x in popup_area.x..(popup_area.x + popup_area.width) {
                buf.set_string(x, y, " ", Style::default().bg(bg_color));
            }
        }

        let block = Block::default()
            .title(Span::styled(
                " ⚙ OPTIONS & THEME SETTINGS ",
                Style::default()
                    .fg(theme.fg_highlight)
                    .bg(bg_color)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.border_active).bg(bg_color));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Subtitle
                Constraint::Length(7), // Menu Items
                Constraint::Min(2),    // Help / Footer
            ])
            .split(inner);

        // Subtitle
        let sub = Line::from(vec![
            Span::styled(
                "Configure display themes, background opacity, and telemetry:",
                Style::default().fg(theme.fg_dim).bg(bg_color),
            ),
        ]);
        Paragraph::new(sub).render(chunks[0], buf);

        // Menu items
        let items = [
            ("Select Theme", self.app.theme.name.to_string()),
            (
                "Background Mode",
                if self.app.theme_id.is_solid() {
                    "Solid Fill (Theme Builtin)".to_string()
                } else if self.app.solid_background {
                    "Solid Opaque (Override)".to_string()
                } else {
                    "Transparent (Terminal Default)".to_string()
                },
            ),
            (
                "Poll Interval",
                format!("{} ms", self.app.poll_interval_ms()),
            ),
            (
                "Active View",
                format!("{:?}", self.app.active_tab),
            ),
        ];

        let mut lines = Vec::new();
        for (i, (label, val)) in items.iter().enumerate() {
            let is_selected = i == self.app.options_menu_index;
            let (cursor, style) = if is_selected {
                (
                    " ▶ ",
                    Style::default()
                        .fg(theme.selected_fg)
                        .bg(theme.selected_bg)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    "   ",
                    Style::default().fg(theme.fg).bg(bg_color),
                )
            };

            let line = Line::from(vec![
                Span::styled(cursor, style),
                Span::styled(format!("{label:<18}"), style),
                Span::styled(" : ", Style::default().fg(theme.fg_dim).bg(bg_color)),
                Span::styled(
                    format!("◀ {val} ▶"),
                    if is_selected {
                        Style::default()
                            .fg(theme.fg_highlight)
                            .bg(theme.selected_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg_dim).bg(bg_color)
                    },
                ),
            ]);
            lines.push(line);
        }

        Paragraph::new(lines).render(chunks[1], buf);

        // Footer instructions
        let help_text = vec![
            Line::from(vec![
                Span::styled(" [↑/↓] ", Style::default().fg(theme.fg_highlight).bg(bg_color).add_modifier(Modifier::BOLD)),
                Span::styled("Navigate   ", Style::default().fg(theme.fg_dim).bg(bg_color)),
                Span::styled("[←/→/Enter] ", Style::default().fg(theme.fg_highlight).bg(bg_color).add_modifier(Modifier::BOLD)),
                Span::styled("Change Value   ", Style::default().fg(theme.fg_dim).bg(bg_color)),
                Span::styled("[Esc] ", Style::default().fg(theme.fg_highlight).bg(bg_color).add_modifier(Modifier::BOLD)),
                Span::styled("Close", Style::default().fg(theme.fg_dim).bg(bg_color)),
            ]),
        ];
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(chunks[2], buf);
    }
}
