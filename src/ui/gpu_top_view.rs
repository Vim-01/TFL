use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, Widget},
};

pub struct GpuTopView<'a> {
    app: &'a App,
}

impl<'a> GpuTopView<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for GpuTopView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;

        let block = Block::default()
            .title(Span::styled(
                " GPU TOP ── PROCESS & VRAM RESOURCE MONITOR (DRM fdinfo) ",
                Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_gpu));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        let gpu = match self.app.gpus.first() {
            Some(g) => g,
            None => {
                let p = Paragraph::new("No dedicated GPU detected or supported DRM driver not loaded.")
                    .style(Style::default().fg(theme.fg_dim));
                p.render(inner, buf);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // GPU Hardware & Memory Overview Card
                Constraint::Min(6),    // Full Process Table
            ])
            .split(inner);

        // 1. GPU Hardware & Memory Overview Card
        let total_vram_gb = gpu.vram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_vram_gb = gpu.vram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let vram_pct = if total_vram_gb > 0.0 {
            (used_vram_gb / total_vram_gb * 100.0) as f32
        } else {
            0.0
        };

        let total_proc_vram_bytes: u64 = gpu.processes.iter().map(|p| p.vram_bytes).sum();
        let total_proc_gtt_bytes: u64 = gpu.processes.iter().map(|p| p.gtt_bytes).sum();
        let total_proc_vram_gb = total_proc_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let total_proc_gtt_gb = total_proc_gtt_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        let overview_lines = vec![
            Line::from(vec![
                Span::styled("Card: ", Style::default().fg(theme.fg_dim)),
                Span::styled(&gpu.name, Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD)),
                Span::styled("   Driver: ", Style::default().fg(theme.fg_dim)),
                Span::styled("amdgpu (Linux DRM)", Style::default().fg(theme.fg)),
                Span::styled("   Clients: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{} active processes", gpu.processes.len()), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("VRAM Total: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{used_vram_gb:.1} / {total_vram_gb:.1} GiB ({vram_pct:.0}%)"), Style::default().fg(theme.fg_highlight)),
                Span::styled("   Proc Allocated: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{total_proc_vram_gb:.1} GiB VRAM"), Style::default().fg(theme.spark_compute)),
                Span::styled(" │ ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{total_proc_gtt_gb:.1} GiB GTT"), Style::default().fg(theme.fg_highlight)),
                Span::styled("   Sensors: ", Style::default().fg(theme.fg_dim)),
                Span::styled(
                    format!(
                        "{} │ {}",
                        gpu.temp_edge_c.map(|t| format!("{:.0}°C Edge", t)).unwrap_or_else(|| "N/A".into()),
                        gpu.power_current_w.map(|p| format!("{:.0}W", p)).unwrap_or_else(|| "N/A".into()),
                    ),
                    Style::default().fg(theme.fg),
                ),
            ]),
        ];

        let overview_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme.fg_dim));
        let overview_inner = overview_block.inner(chunks[0]);
        overview_block.render(chunks[0], buf);
        Paragraph::new(overview_lines).render(overview_inner, buf);

        // 2. Full Process Table with Selection and Scrolling
        let procs = &gpu.processes;
        if procs.is_empty() {
            let p = Paragraph::new("No active GPU client processes detected via /proc/*/fdinfo.")
                .style(Style::default().fg(theme.fg_dim));
            p.render(chunks[1], buf);
            return;
        }

        let table_inner_height = chunks[1].height.saturating_sub(1) as usize; // minus header
        let selected = self.app.selected_process_index.min(procs.len().saturating_sub(1));

        // Compute scroll offset to keep selected row visible
        let scroll_offset = if selected >= table_inner_height {
            selected.saturating_sub(table_inner_height.saturating_sub(1))
        } else {
            0
        };

        let header = Row::new(vec![
            " ",
            "    PID",
            "Process Name",
            "    VRAM",
            "%VRAM",
            "VRAM Allocation Bar",
            "    GTT",
            "Classification",
        ])
        .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        for (i, p) in procs.iter().enumerate().skip(scroll_offset).take(table_inner_height) {
            let is_selected = i == selected;
            let is_llama = p.name.to_lowercase().contains("llama");

            let marker = if is_selected { "▶" } else { " " };

            let row_style = if is_selected {
                Style::default()
                    .fg(theme.bg.unwrap_or(ratatui::style::Color::Black))
                    .bg(theme.border_active)
                    .add_modifier(Modifier::BOLD)
            } else if is_llama {
                Style::default()
                    .fg(theme.spark_compute)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let vram_str = if p.vram_bytes >= 1024 * 1024 * 1024 {
                format!("{:.1} GiB", p.vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            } else {
                format!("{:.0} MiB", p.vram_bytes as f64 / (1024.0 * 1024.0))
            };

            let gtt_str = if p.gtt_bytes >= 1024 * 1024 * 1024 {
                format!("{:.1} GiB", p.gtt_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            } else {
                format!("{:.0} MiB", p.gtt_bytes as f64 / (1024.0 * 1024.0))
            };

            // Visual bar of VRAM percentage
            let bar_len: usize = 16;
            let filled = ((p.vram_percent / 100.0).clamp(0.0, 1.0) * bar_len as f32).round() as usize;
            let bar_str = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len.saturating_sub(filled)));

            let role = classify_process(&p.name);

            rows.push(Row::new(vec![
                Line::from(marker),
                Line::from(format!("{:>7}", p.pid)),
                Line::from(p.name.clone()),
                Line::from(format!("{:>8}", vram_str)),
                Line::from(format!("{:>4.0}%", p.vram_percent)),
                Line::from(bar_str),
                Line::from(format!("{:>7}", gtt_str)),
                Line::from(role),
            ]).style(row_style));
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(1),  // Selection cursor ▶
                Constraint::Length(7),  // PID
                Constraint::Length(18), // Process Name
                Constraint::Length(8),  // VRAM
                Constraint::Length(5),  // %VRAM
                Constraint::Length(18), // VRAM Allocation Bar
                Constraint::Length(8),  // GTT
                Constraint::Min(15),    // Classification
            ],
        )
        .header(header)
        .column_spacing(2);

        table.render(chunks[1], buf);
    }
}

fn classify_process(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("llama") || lower.contains("vllm") || lower.contains("ollama") {
        "LLM Inference Engine"
    } else if lower.contains("xwayland") || lower.contains("wayland") || lower.contains("xorg") {
        "Display Server (Wayland/X11)"
    } else if lower.contains("plasma") || lower.contains("kwin") || lower.contains("gnome") {
        "Desktop Compositor / Shell"
    } else if lower.contains("steam") || lower.contains("game") {
        "Gaming / Steam Client"
    } else if lower.contains("discord") {
        "Chat / Communication"
    } else if lower.contains("chrome") || lower.contains("firefox") || lower.contains("brave") {
        "Web Browser"
    } else if lower.contains("antigravity") {
        "AI Coding Agent"
    } else {
        "GPU Client"
    }
}
