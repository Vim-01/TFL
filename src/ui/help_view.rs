use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct HelpView<'a> {
    theme: &'a Theme,
}

impl<'a> HelpView<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }
}

impl<'a> Widget for HelpView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;

        let block = Block::default()
            .title(Span::styled(
                " TFL HELP & LLM INFERENCE MONITORING GUIDE ",
                Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_active));

        let inner = block.inner(area);
        block.render(area, buf);

        let text = vec![
            Line::from(vec![
                Span::styled("⚡ Keyboard Shortcuts & Navigation:", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  [Esc] / [o]     ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Open Options & Theme Settings modal (Themes, Opacity, Polling)", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [↑] / [↓]       ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Navigate through slots, GPU devices, or menu options", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [←] / [→]       ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Cycle views / tabs (or modify values inside Options menu)", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [Tab] / [S-Tab] ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Cycle forward / backward through view tabs", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [1] - [5]       ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Direct switch to Dashboard, GPU, Slots, GPU Top, or Help view", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [p]             ", Style::default().fg(theme.fg_highlight)),
                Span::styled("Toggle Pause / Resume live metrics streaming", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  [q] / [Ctrl+C]  ", Style::default().fg(theme.status_offline)),
                Span::styled("Quit TFL gracefully (or close active modal)", Style::default().fg(theme.fg)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("📊 Key LLM Inference Metrics Explained:", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  • Prefill Speed (t/s): ", Style::default().fg(theme.status_online).add_modifier(Modifier::BOLD)),
                Span::styled("Tokens/sec during prompt evaluation (compute-bound, peak GPU utilization).", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  • Decode Speed (t/s):  ", Style::default().fg(theme.spark_tps).add_modifier(Modifier::BOLD)),
                Span::styled("Tokens/sec during token generation (memory-bandwidth bound).", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  • TTFT:                ", Style::default().fg(theme.box_llm).add_modifier(Modifier::BOLD)),
                Span::styled("Time To First Token — latency from request submission to start of generation.", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  • ITL:                 ", Style::default().fg(theme.box_llm).add_modifier(Modifier::BOLD)),
                Span::styled("Inter-Token Latency — elapsed time between consecutive output tokens.", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  • KV Cache Pool:       ", Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD)),
                Span::styled("Memory allocated to past keys/values. Exceeding 100% causes eviction or OOM.", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("  • Speculative / MTP:   ", Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                Span::styled("Draft model token prediction acceptance rate. Higher rate = higher effective throughput.", Style::default().fg(theme.fg)),
            ]),
        ];

        Paragraph::new(text).render(inner, buf);
    }
}
