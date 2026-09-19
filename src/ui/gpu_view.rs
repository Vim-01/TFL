use crate::app::App;
use crate::ui::widgets::SegmentedBar;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Sparkline, Table, Widget},
};

pub struct GpuDetailsView<'a> {
    app: &'a App,
}

impl<'a> GpuDetailsView<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for GpuDetailsView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;

        let block = Block::default()
            .title(Span::styled(
                " GPU HARDWARE & MEMORY ARCHITECTURE ",
                Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_gpu));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.app.gpus.is_empty() {
            let p = Paragraph::new("No dedicated GPU detected or supported driver not loaded.")
                .style(Style::default().fg(theme.fg_dim));
            p.render(inner, buf);
            return;
        }

        let gpu_idx = self.app.selected_gpu_index.min(self.app.gpus.len().saturating_sub(1));
        let gpu = &self.app.gpus[gpu_idx];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // Core GPU Info & Sensors Table
                Constraint::Length(3),  // VRAM Segmented Bar
                Constraint::Length(5),  // GTT & Clocks
                Constraint::Min(6),     // Compute & VRAM Charts
            ])
            .split(inner);

        // 1. Info Table
        let rows = vec![
            Row::new(vec![
                Span::styled("Device Name:", Style::default().fg(theme.fg_dim)),
                Span::styled(&gpu.name, Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                Span::styled("Vendor / Driver:", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{} ({})", gpu.vendor.as_str(), gpu.driver_version), Style::default().fg(theme.border_active)),
            ]),
            Row::new(vec![
                Span::styled("PCI Bus ID:", Style::default().fg(theme.fg_dim)),
                Span::styled(&gpu.pci_bus_id, Style::default().fg(theme.fg_highlight)),
                Span::styled("Memory Vendor:", Style::default().fg(theme.fg_dim)),
                Span::styled(gpu.mem_vendor.as_deref().unwrap_or("Unknown"), Style::default().fg(theme.box_gpu)),
            ]),
            Row::new(vec![
                Span::styled("Current Power:", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{:.1} W / {:.1} W", gpu.power_current_w.unwrap_or(0.0), gpu.power_cap_w.unwrap_or(0.0)), Style::default().fg(theme.fg_highlight)),
                Span::styled("Fan Speed:", Style::default().fg(theme.fg_dim)),
                Span::styled(gpu.fan_percent.map(|f| format!("{f:.1}%")).unwrap_or_else(|| "Auto".to_string()), Style::default().fg(theme.status_online)),
            ]),
            Row::new(vec![
                Span::styled("Temperatures:", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("Edge: {:.1}°C │ Hotspot: {:.1}°C │ VRAM: {:.1}°C",
                    gpu.temp_edge_c.unwrap_or(0.0),
                    gpu.temp_hotspot_c.unwrap_or(0.0),
                    gpu.temp_mem_c.unwrap_or(0.0)
                ), Style::default().fg(theme.temp_warm)),
                Span::styled("Compute Load:", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{:.1}%", gpu.compute_percent), Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)),
            ]),
        ];

        let table = Table::new(
            rows,
            [
                Constraint::Length(16),
                Constraint::Percentage(35),
                Constraint::Length(18),
                Constraint::Percentage(35),
            ],
        );
        table.render(chunks[0], buf);

        // 2. VRAM Segmented Bar
        let total_gb = gpu.vram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let total_str = format!("{total_gb:.1} GB");
        let seg_bar = SegmentedBar::new("Dedicated VRAM:", &total_str, gpu.vram_total_bytes)
            .segment("Used", gpu.vram_used_bytes, theme.box_gpu)
            .segment("Free", gpu.vram_breakdown.free_bytes, theme.bar_track);
        seg_bar.render(chunks[1], buf);

        // 3. Shared GTT & Clocks
        let gtt_used_gib = gpu.gtt_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let gtt_total_gib = gpu.gtt_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let sclk_str = gpu.sclk_mhz.map(|s| format!("{s} MHz")).unwrap_or_else(|| "N/A".to_string());
        let mclk_str = gpu.mclk_mhz.map(|m| format!("{m} MHz")).unwrap_or_else(|| "N/A".to_string());

        let gtt_lines = vec![
            Line::from(vec![
                Span::styled("GTT System Memory: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{gtt_used_gib:.2} GB / {gtt_total_gib:.2} GB"), Style::default().fg(theme.spark_mem)),
                Span::styled("   Core Clock (SCLK): ", Style::default().fg(theme.fg_dim)),
                Span::styled(sclk_str, Style::default().fg(theme.fg_highlight)),
                Span::styled("   Memory Clock (MCLK): ", Style::default().fg(theme.fg_dim)),
                Span::styled(mclk_str, Style::default().fg(theme.fg_highlight)),
            ]),
        ];
        Paragraph::new(gtt_lines).render(chunks[2], buf);

        // 4. Sparkline Graphs
        if chunks[3].height >= 4 {
            let spark_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(2),
                    Constraint::Length(1),
                    Constraint::Min(2),
                ])
                .split(chunks[3]);

            let vram_hist: Vec<u64> = self
                .app
                .gpu_vram_history
                .as_vec()
                .iter()
                .map(|&v| if v.is_finite() && v >= 0.0 { v.min(100.0) as u64 } else { 0 })
                .collect();
            let vram_label = Line::from(vec![
                Span::styled("VRAM Controller History (60s): ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{:.1}%", self.app.gpu_vram_history.latest().unwrap_or(0.0)), Style::default().fg(theme.spark_mem)),
            ]);
            buf.set_line(spark_layout[0].x, spark_layout[0].y, &vram_label, spark_layout[0].width);
            Sparkline::default()
                .data(&vram_hist)
                .max(100)
                .style(Style::default().fg(theme.spark_mem))
                .render(spark_layout[1], buf);

            let comp_hist: Vec<u64> = self
                .app
                .gpu_compute_history
                .as_vec()
                .iter()
                .map(|&v| if v.is_finite() && v >= 0.0 { v.min(100.0) as u64 } else { 0 })
                .collect();
            let comp_label = Line::from(vec![
                Span::styled("GPU Compute History (60s): ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{:.1}%", self.app.gpu_compute_history.latest().unwrap_or(0.0)), Style::default().fg(theme.spark_compute)),
            ]);
            buf.set_line(spark_layout[2].x, spark_layout[2].y, &comp_label, spark_layout[2].width);
            Sparkline::default()
                .data(&comp_hist)
                .max(100)
                .style(Style::default().fg(theme.spark_compute))
                .render(spark_layout[3], buf);
        }
    }
}
