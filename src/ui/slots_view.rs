use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, Widget},
};

pub struct SlotsDetailsView<'a> {
    app: &'a App,
}

impl<'a> SlotsDetailsView<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for SlotsDetailsView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;

        let block = Block::default()
            .title(Span::styled(
                " INFERENCE SLOTS & MODEL RUNTIME INSPECTOR ",
                Style::default().fg(theme.box_llm).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_llm));

        let inner = block.inner(area);
        block.render(area, buf);

        let llm = match self.app.llm.as_ref() {
            Some(l) => l,
            None => {
                let p = Paragraph::new("No LLM backend connected. Start llama-server or check endpoint URL.")
                    .style(Style::default().fg(theme.fg_dim));
                p.render(inner, buf);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Engine & Model Metadata Header
                Constraint::Min(6),    // Detailed Slots Table
            ])
            .split(inner);

        // 1. Model & Engine Metadata
        let meta_lines = vec![
            Line::from(vec![
                Span::styled("Engine: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&llm.engine_name, Style::default().fg(theme.box_llm).add_modifier(Modifier::BOLD)),
                Span::styled("   URL: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&llm.endpoint_url, Style::default().fg(theme.fg)),
                Span::styled("   Slots: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{}/{}", llm.active_slots, llm.total_slots), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Model Alias: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&llm.model_alias, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled("   Quant: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&llm.model_ftype, Style::default().fg(theme.box_gpu)),
                Span::styled("   Cache KV: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("K: {} │ V: {}", llm.cache_type_k, llm.cache_type_v), Style::default().fg(theme.box_llm)),
            ]),
            Line::from(vec![
                Span::styled("Model Path: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&llm.model_path, Style::default().fg(theme.fg_dim)),
            ]),
            Line::from(vec![
                Span::styled("Speculative Draft: ", Style::default().fg(theme.fg_dim)),
                Span::styled(llm.speculative_draft_model.as_deref().unwrap_or("None"), Style::default().fg(theme.fg_highlight)),
                Span::styled("   Type: ", Style::default().fg(theme.fg_dim)),
                Span::styled(llm.speculative_type.as_deref().unwrap_or("None"), Style::default().fg(theme.box_llm)),
                Span::styled("   Draft Quant: ", Style::default().fg(theme.fg_dim)),
                Span::styled(llm.speculative_draft_quant.as_deref().unwrap_or("N/A"), Style::default().fg(theme.box_gpu)),
            ]),
        ];
        Paragraph::new(meta_lines).render(chunks[0], buf);

        // 2. Detailed Slots Table
        let header = Row::new(vec![
            "Slot ID",
            "Task ID",
            "State",
            "Context (n_ctx)",
            "Prompt Tokens",
            "Processed",
            "Cache Hit",
            "Decoded Tokens",
            "Speculative Mode",
        ])
        .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        for (i, slot) in llm.slots.iter().enumerate() {
            let is_selected = i == self.app.selected_slot_index;
            let (state_str, state_color) = if slot.is_processing {
                ("Generating", theme.status_online)
            } else {
                ("Idle", theme.fg_dim)
            };

            let row_style = if is_selected {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let cursor = if is_selected { "▶ " } else { "  " };

            let row = Row::new(vec![
                Line::from(format!("{cursor}#{}", slot.id)),
                Line::from(slot.id_task.map(|t| t.to_string()).unwrap_or_else(|| "—".to_string())),
                Line::from(Span::styled(state_str, Style::default().fg(if is_selected { theme.selected_fg } else { state_color }).add_modifier(Modifier::BOLD))),
                Line::from(format!("{}", slot.n_ctx)),
                Line::from(format!("{}", slot.n_prompt_tokens)),
                Line::from(format!("{}", slot.n_prompt_tokens_processed)),
                Line::from(format!("{}", slot.n_prompt_tokens_cache)),
                Line::from(format!("{}", slot.n_decoded)),
                Line::from(slot.speculative_type.as_deref().unwrap_or(if slot.speculative { "Draft MTP" } else { "None" })),
            ])
            .style(row_style);

            rows.push(row);
        }

        if rows.is_empty() {
            rows.push(
                Row::new(vec![
                    Line::from("▶ #0"),
                    Line::from("—"),
                    Line::from(Span::styled("Idle", Style::default().fg(theme.fg_dim))),
                    Line::from("0"),
                    Line::from("0"),
                    Line::from("0"),
                    Line::from("0"),
                    Line::from("0"),
                    Line::from("Standard"),
                ])
                .style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg)),
            );
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(16),
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(16),
                Constraint::Min(16),
            ],
        )
        .header(header)
        .column_spacing(1);

        table.render(chunks[1], buf);
    }
}
