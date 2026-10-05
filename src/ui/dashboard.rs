use crate::app::App;
use crate::ui::widgets::{
    power_gradient_color, thermal_color, BrailleCanvas, VerticalGauge,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Row, Table, Widget},
};

pub struct DashboardView<'a> {
    app: &'a App,
}

impl<'a> DashboardView<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }

    // ------------------------------------------------------------------------
    // Panel 1: CPU & SYSTEM (btop Style)
    // ------------------------------------------------------------------------
    fn render_cpu_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let cpu_opt = self.app.cpu.as_ref();

        let global_usage = cpu_opt.map(|c| c.global_usage_percent).unwrap_or(0.0);
        let avg_freq = cpu_opt.map(|c| c.avg_frequency_mhz as f32 / 1000.0).unwrap_or(0.0);
        let load_avg = cpu_opt.map(|c| c.load_average).unwrap_or([0.0, 0.0, 0.0]);
        let cpu_pwr = cpu_opt.and_then(|c| c.cpu_power_w).unwrap_or(35.0);
        let cpu_temp = cpu_opt.and_then(|c| c.cpu_temp_c).unwrap_or(48.0);

        let cpu_brand = cpu_opt
            .and_then(|c| c.brand.as_deref())
            .unwrap_or("Host CPU");

        // Top-level block title
        let title = Line::from(vec![
            Span::styled("┌1cpu", Style::default().fg(theme.box_cpu).add_modifier(Modifier::BOLD)),
            Span::styled(format!("───{cpu_brand}───────────────────"), Style::default().fg(theme.box_cpu)),
            Span::styled(format!("{global_usage:.0}% "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{avg_freq:.2} GHz"), Style::default().fg(theme.fg)),
            Span::styled("────────────────────┐", Style::default().fg(theme.box_cpu)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_cpu));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        // Split: Left = Full-height 2D Braille history graph (Area 1); Right = Compact btop-style cores sub-box (Area 3)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(inner);

        // --- Left: Full-height 2D Braille Canvas (Area 1) ---
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(2)])
            .split(cols[0]);

        let avg_1m = self.app.cpu_avg_1m();
        let avg_15m = self.app.cpu_avg_15m();
        let avg_all = self.app.cpu_avg_all_time();

        let chart_title = Line::from(vec![
            Span::styled("CPU Total Load: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{global_usage:.1}%  "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled(format!("avg 1m: {avg_1m:.1}% / 15m: {avg_15m:.1}% / all: {avg_all:.1}%  "), Style::default().fg(theme.fg_dim)),
            Span::styled(format!("| {avg_freq:.2} GHz | {cpu_pwr:.1}W {cpu_temp:.0}°C"), Style::default().fg(theme.fg_dim)),
        ]);
        buf.set_line(left_chunks[0].x, left_chunks[0].y, &chart_title, left_chunks[0].width);

        // 2D Braille Canvas: fills the entire left height from top to bottom with Green->Red gradient
        let cpu_hist = self.app.cpu_usage_history.as_vec();
        BrailleCanvas::new(&cpu_hist)
            .max(100.0)
            .use_gradient(true)
            .render(left_chunks[1], buf);

        // --- Right: Compact btop-style Cores & System Sub-box (Area 3) ---
        let cores_load_color = if global_usage < 40.0 {
            theme.temp_cool
        } else if global_usage < 75.0 {
            theme.temp_warm
        } else {
            theme.temp_hot
        };

        let cores_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.fg_dim))
            .title(Line::from(vec![
                Span::styled(" Cores & Package ", Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                Span::styled(format!("[Load: {global_usage:.0}%] "), Style::default().fg(cores_load_color).add_modifier(Modifier::BOLD)),
            ]));

        let cores_inner = cores_block.inner(cols[1]);
        cores_block.render(cols[1], buf);

        if let Some(cpu) = cpu_opt {
            let num_cores = cpu.core_usages.len();
            if num_cores > 0 && cores_inner.height >= 4 {
                let sub_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Header info (Model, Clock, Watts, Load %)
                        Constraint::Length(1), // Horizontal CPU temperature gauge spanning full width
                        Constraint::Min(2),    // Cores Grid (4 cols x 3 rows)
                        Constraint::Length(1), // Memory / Load avg / Uptime
                    ])
                    .split(cores_inner);

                // Sub-header with explicit Load % indicator
                let ram_used = cpu.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_total = cpu.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_pct = if ram_total > 0.0 { (ram_used / ram_total * 100.0) as u16 } else { 0 };

                let sub_title = Line::from(vec![
                    Span::styled(format!("{cpu_brand} "), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)),
                    Span::styled("Load: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format!("{global_usage:>3.0}%"), Style::default().fg(cores_load_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" ({} threads)", num_cores), Style::default().fg(theme.fg_dim)),
                ]);
                buf.set_line(sub_chunks[0].x, sub_chunks[0].y, &sub_title, sub_chunks[0].width);

                // Horizontal CPU temperature gauge + Live Clock, Power & Uptime filling the space
                let cpu_temp_color = thermal_color(cpu_temp);
                let w = sub_chunks[1].width;
                let cpu_bar_len = if w >= 70 { 12 } else if w >= 55 { 8 } else { 6 };
                let (cpu_bar, cpu_empty) = rectangular_bar(cpu_temp, 100.0, cpu_bar_len);

                let mut temp_spans = vec![
                    Span::styled("CPU Temp: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(cpu_bar, Style::default().fg(cpu_temp_color)),
                    Span::styled(cpu_empty, Style::default().fg(theme.bar_track)),
                    Span::styled(format!(" {cpu_temp:.0}°C"), Style::default().fg(cpu_temp_color).add_modifier(Modifier::BOLD)),
                ];

                if w >= 70 {
                    temp_spans.push(Span::styled("  |  Clock: ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{avg_freq:.2} GHz"), Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)));
                    temp_spans.push(Span::styled("  |  PWR: ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{cpu_pwr:.1}W"), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)));
                    temp_spans.push(Span::styled("  |  Up: ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format_uptime(cpu.uptime_seconds), Style::default().fg(theme.fg)));
                } else if w >= 54 {
                    temp_spans.push(Span::styled(" | ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{avg_freq:.2}GHz"), Style::default().fg(theme.fg)));
                    temp_spans.push(Span::styled(" | ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{cpu_pwr:.0}W"), Style::default().fg(theme.fg_highlight)));
                    temp_spans.push(Span::styled(" | Up: ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format_uptime(cpu.uptime_seconds), Style::default().fg(theme.fg)));
                } else if w >= 40 {
                    temp_spans.push(Span::styled(" | ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{avg_freq:.2}GHz"), Style::default().fg(theme.fg)));
                    temp_spans.push(Span::styled(" ", Style::default().fg(theme.fg_dim)));
                    temp_spans.push(Span::styled(format!("{cpu_pwr:.0}W"), Style::default().fg(theme.fg_highlight)));
                }

                let temp_line = Line::from(temp_spans);
                buf.set_line(sub_chunks[1].x, sub_chunks[1].y, &temp_line, sub_chunks[1].width);

                // Multi-column Core Grid with smooth rectangular solid bar filling matching cpu temp
                let num_cols = if sub_chunks[2].width >= 48 { 4 } else { 3 };
                let num_rows = num_cores.div_ceil(num_cols);
                let col_width = sub_chunks[2].width / num_cols as u16;

                for c in 0..num_cols {
                    for r in 0..num_rows {
                        let core_idx = c * num_rows + r;
                        if core_idx >= num_cores {
                            break;
                        }
                        let usage = cpu.core_usages[core_idx];
                        let usage_clamped = if usage.is_finite() { usage.clamp(0.0, 100.0) } else { 0.0 };

                        let bar_color = if usage_clamped < 40.0 {
                            theme.temp_cool
                        } else if usage_clamped < 75.0 {
                            theme.temp_warm
                        } else {
                            theme.temp_hot
                        };

                        let x = sub_chunks[2].x + (c as u16 * col_width);
                        let y = sub_chunks[2].y + r as u16;

                        if y < sub_chunks[2].y + sub_chunks[2].height {
                            // Dynamic bar length matching rectangular_bar style (█ and ░)
                            let core_bar_len = (col_width as usize).saturating_sub(9).clamp(3, 10);
                            let (bar_filled, bar_empty) = rectangular_bar(usage_clamped, 100.0, core_bar_len);

                            let core_line = Line::from(vec![
                                Span::styled(format!("C{core_idx:<2}:"), Style::default().fg(theme.fg_dim)),
                                Span::styled(bar_filled, Style::default().fg(bar_color)),
                                Span::styled(bar_empty, Style::default().fg(theme.bar_track)),
                                Span::styled(format!(" {usage_clamped:>3.0}%"), Style::default().fg(theme.fg)),
                            ]);
                            buf.set_line(x, y, &core_line, col_width);
                        }
                    }
                }

                // Bottom Line (RAM with bar, Load avg, Uptime)
                let ram_bar_len = ((sub_chunks[3].width as usize).saturating_sub(44)).clamp(4, 16);
                let (ram_bar, ram_empty) = rectangular_bar(ram_pct as f32, 100.0, ram_bar_len);

                let bot_line = Line::from(vec![
                    Span::styled("RAM: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(ram_bar, Style::default().fg(theme.border_active)),
                    Span::styled(ram_empty, Style::default().fg(theme.bar_track)),
                    Span::styled(format!(" {ram_used:.1}/{ram_total:.1}G ({ram_pct}%)  "), Style::default().fg(theme.border_active)),
                    Span::styled("|  Load: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format!("{:.2} {:.2} {:.2}  ", load_avg[0], load_avg[1], load_avg[2]), Style::default().fg(theme.fg)),
                    Span::styled("|  Up: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format_uptime(cpu.uptime_seconds), Style::default().fg(theme.fg)),
                ]);
                buf.set_line(sub_chunks[3].x, sub_chunks[3].y, &bot_line, sub_chunks[3].width);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Panel 2: GPU HARDWARE & VRAM (3-Way Split with Full Space Utilization)
    // ------------------------------------------------------------------------
    fn render_gpu_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let gpu = self
            .app
            .gpus
            .get(self.app.selected_gpu_index)
            .or_else(|| self.app.gpus.first());

        let gpu_name = gpu.map(|g| g.name.as_str()).unwrap_or("No GPU Detected");
        let compute = gpu.map(|g| g.compute_percent).unwrap_or(0.0);
        let sclk = gpu.and_then(|g| g.sclk_mhz).unwrap_or(0);
        let mclk = gpu.and_then(|g| g.mclk_mhz).unwrap_or(0);
        let mem_bus = gpu.map(|g| g.mem_utilization_percent).unwrap_or(0.0);

        let title = Line::from(vec![
            Span::styled("┌2gpu0", Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD)),
            Span::styled(format!("───{gpu_name}───────────────────"), Style::default().fg(theme.box_gpu)),
            Span::styled(format!("{compute:.0}% "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{sclk} MHz"), Style::default().fg(theme.fg)),
            Span::styled("────────────────────┐", Style::default().fg(theme.box_gpu)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_gpu));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 || inner.width < 24 {
            return;
        }

        // 3-Way Balanced Division:
        // Col 1 (6): GPU Compute Core Activity (2D Braille Canvas, Green->Red gradient)
        // Col 2 (5): GPU Memory Bus / IO Activity (2D Braille Canvas, Green->Red gradient)
        // Col 3 (2): VRAM, Full-Width Thermals, PWR Gradient, Clocks, PCIe, Vertical Fan/Hotspot
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
            .split(inner);

        // --- Sub-block 1 (Col 1 / 6): GPU Compute Activity ---
        let comp_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.fg_dim))
            .title(Line::from(vec![
                Span::styled(" GPU Compute ", Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{compute:.1}% | {sclk}MHz "), Style::default().fg(theme.fg_dim)),
            ]));
        let comp_inner = comp_block.inner(cols[0]);
        comp_block.render(cols[0], buf);

        if comp_inner.height >= 2 {
            let comp_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(comp_inner);

            let comp_line = Line::from(vec![
                Span::styled("Compute Load: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{compute:.1}%"), Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)),
            ]);
            buf.set_line(comp_chunks[0].x, comp_chunks[0].y, &comp_line, comp_chunks[0].width);

            let comp_hist = self.app.gpu_compute_history.as_vec();
            BrailleCanvas::new(&comp_hist)
                .max(100.0)
                .use_gradient(true)
                .render(comp_chunks[1], buf);
        }

        // --- Sub-block 2 (Col 2 / 5): Memory Bus / Controller IO ---
        let mem_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.fg_dim))
            .title(Line::from(vec![
                Span::styled(" Memory Bus / IO ", Style::default().fg(theme.spark_mem).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{mem_bus:.1}% | {mclk}MHz "), Style::default().fg(theme.fg_dim)),
            ]));
        let mem_inner = mem_block.inner(cols[1]);
        mem_block.render(cols[1], buf);

        if mem_inner.height >= 2 {
            let mem_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(mem_inner);

            let bus_line = Line::from(vec![
                Span::styled("Bus / Controller: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{mem_bus:.1}%"), Style::default().fg(theme.spark_mem).add_modifier(Modifier::BOLD)),
            ]);
            buf.set_line(mem_chunks[0].x, mem_chunks[0].y, &bus_line, mem_chunks[0].width);

            let mem_hist = self.app.gpu_mem_controller_history.as_vec();
            BrailleCanvas::new(&mem_hist)
                .max(100.0)
                .use_gradient(true)
                .render(mem_chunks[1], buf);
        }

        // --- Sub-block 3 (Col 3 / 2): Full Space Utilization (User Request #2) ---
        let vram_used_gib = gpu.map(|g| g.vram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0);
        let vram_total_gib = gpu.map(|g| g.vram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(24.0);
        let vram_pct = if vram_total_gib > 0.0 {
            ((vram_used_gib / vram_total_gib) * 100.0).clamp(0.0, 100.0) as u16
        } else {
            0
        };

        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.fg_dim))
            .title(Line::from(vec![
                Span::styled(" VRAM & Thermals ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{vram_used_gib:.1}/{vram_total_gib:.1} GiB "), Style::default().fg(theme.fg_dim)),
            ]));
        let right_inner = right_block.inner(cols[2]);
        right_block.render(cols[2], buf);

        if let Some(g) = gpu {
            if right_inner.height >= 4 {
                // Adapt layout dynamically based on available vertical rows, with a clean spacer between VRAM and Thermals
                let (right_chunks, middle_idx, clock_idx, pcie_idx) = if right_inner.height >= 8 {
                    (
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1), // [0] VRAM Header line
                                Constraint::Length(1), // [1] Full-width VRAM bar
                                Constraint::Length(1), // [2] Spacer between VRAM and Thermals
                                Constraint::Min(3),    // [3] Middle: Thermals & Gauges
                                Constraint::Length(1), // [4] Bottom: Clocks & Voltage
                                Constraint::Length(1), // [5] Bottom: PCIe & Driver info
                            ])
                            .split(right_inner),
                        3,
                        Some(4),
                        Some(5),
                    )
                } else if right_inner.height >= 7 {
                    (
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1), // [0] VRAM Header line
                                Constraint::Length(1), // [1] Full-width VRAM bar
                                Constraint::Length(1), // [2] Spacer between VRAM and Thermals
                                Constraint::Min(3),    // [3] Middle: Thermals & Gauges
                                Constraint::Length(1), // [4] Bottom: Clocks & Voltage
                            ])
                            .split(right_inner),
                        3,
                        Some(4),
                        None,
                    )
                } else if right_inner.height >= 5 {
                    (
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1), // [0] VRAM Header line
                                Constraint::Length(1), // [1] Full-width VRAM bar
                                Constraint::Min(2),    // [2] Middle: Thermals & Gauges
                                Constraint::Length(1), // [3] Bottom: Clocks & Voltage
                            ])
                            .split(right_inner),
                        2,
                        Some(3),
                        None,
                    )
                } else {
                    (
                        Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1), // [0] VRAM Header line
                                Constraint::Min(2),    // [1] Middle
                            ])
                            .split(right_inner),
                        1,
                        None,
                        None,
                    )
                };

                // 1. VRAM Header
                let vram_title = Line::from(vec![
                    Span::styled("VRAM Allocation: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format!("{vram_used_gib:.1} / {vram_total_gib:.1} GiB ({vram_pct}%)"), Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                ]);
                buf.set_line(right_chunks[0].x, right_chunks[0].y, &vram_title, right_chunks[0].width);

                // 2. Full-Width VRAM Rectangular Bar (stretches 100% horizontally)
                let vram_bar_len = right_chunks[1].width as usize;
                let (vram_bar_str, vram_empty_str) = rectangular_bar(vram_used_gib as f32, vram_total_gib as f32, vram_bar_len);
                let vram_bar_line = Line::from(vec![
                    Span::styled(vram_bar_str, Style::default().fg(theme.border_active)),
                    Span::styled(vram_empty_str, Style::default().fg(theme.bar_track)),
                ]);
                buf.set_line(right_chunks[1].x, right_chunks[1].y, &vram_bar_line, right_chunks[1].width);

                // 3. Middle Section: Split into Left (Horizontal Bars) and Right (Vertical Gauges)
                let lower_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(16),    // Horizontal Thermals and PWR
                        Constraint::Length(10), // Vertical FAN and HOTSPOT
                    ])
                    .split(right_chunks[middle_idx]);

                let temp_edge = g.temp_edge_c.unwrap_or(0.0);
                let temp_hot = g.temp_hotspot_c.unwrap_or(0.0);
                let temp_mem = g.temp_mem_c.unwrap_or(0.0);

                let pwr_cur = g.power_current_w.unwrap_or(0.0);
                let pwr_cap = g.power_cap_w.unwrap_or(402.0).max(1.0);
                let pwr_pct = (pwr_cur / pwr_cap * 100.0).clamp(0.0, 100.0);

                // Calculate dynamic length so every bar fills the ENTIRE horizontal space
                let avail_bar_width = (lower_cols[0].width.saturating_sub(15)) as usize;
                let bar_len = avail_bar_width.clamp(6, 45);

                let diag_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Edge temp
                        Constraint::Length(1), // Hotspot temp
                        Constraint::Length(1), // VRAM temp
                        Constraint::Length(1), // PWR dot gradient
                    ])
                    .split(lower_cols[0]);

                // Edge Temp (0-70 Green, 70-80 Yellow, 80-84 Orange, 85+ Red)
                if !diag_chunks.is_empty() && diag_chunks[0].height > 0 {
                    let edge_color = thermal_color(temp_edge);
                    let (edge_bar, edge_empty) = rectangular_bar(temp_edge, 100.0, bar_len);
                    let edge_line = Line::from(vec![
                        Span::styled("Edge: ", Style::default().fg(theme.fg_dim)),
                        Span::styled(edge_bar, Style::default().fg(edge_color)),
                        Span::styled(edge_empty, Style::default().fg(theme.bar_track)),
                        Span::styled(format!(" {temp_edge:.0}°C"), Style::default().fg(edge_color).add_modifier(Modifier::BOLD)),
                    ]);
                    buf.set_line(diag_chunks[0].x, diag_chunks[0].y, &edge_line, diag_chunks[0].width);
                }

                // Hotspot Temp
                if diag_chunks.len() >= 2 && diag_chunks[1].height > 0 {
                    let hot_color = thermal_color(temp_hot);
                    let (hot_bar, hot_empty) = rectangular_bar(temp_hot, 110.0, bar_len);
                    let hot_line = Line::from(vec![
                        Span::styled("Junc: ", Style::default().fg(theme.fg_dim)),
                        Span::styled(hot_bar, Style::default().fg(hot_color)),
                        Span::styled(hot_empty, Style::default().fg(theme.bar_track)),
                        Span::styled(format!(" {temp_hot:.0}°C"), Style::default().fg(hot_color).add_modifier(Modifier::BOLD)),
                    ]);
                    buf.set_line(diag_chunks[1].x, diag_chunks[1].y, &hot_line, diag_chunks[1].width);
                }

                // VRAM Temp
                if diag_chunks.len() >= 3 && diag_chunks[2].height > 0 {
                    let mem_color = thermal_color(temp_mem);
                    let (mem_bar, mem_empty) = rectangular_bar(temp_mem, 100.0, bar_len);
                    let mem_line = Line::from(vec![
                        Span::styled("VRAM: ", Style::default().fg(theme.fg_dim)),
                        Span::styled(mem_bar, Style::default().fg(mem_color)),
                        Span::styled(mem_empty, Style::default().fg(theme.bar_track)),
                        Span::styled(format!(" {temp_mem:.0}°C"), Style::default().fg(mem_color).add_modifier(Modifier::BOLD)),
                    ]);
                    buf.set_line(diag_chunks[2].x, diag_chunks[2].y, &mem_line, diag_chunks[2].width);
                }

                // PWR Dot Gradient: dots filled with green to red gradient based on % consumption
                if diag_chunks.len() >= 4 && diag_chunks[3].height > 0 {
                    let pwr_color = power_gradient_color(pwr_pct);
                    let (pwr_dots_filled, pwr_dots_empty) = dot_gradient_bar(pwr_pct, bar_len);
                    let pwr_line = Line::from(vec![
                        Span::styled("PWR:  ", Style::default().fg(theme.fg_dim)),
                        Span::styled(pwr_dots_filled, Style::default().fg(pwr_color)),
                        Span::styled(pwr_dots_empty, Style::default().fg(theme.bar_track)),
                        Span::styled(format!(" {pwr_cur:.0}W"), Style::default().fg(pwr_color).add_modifier(Modifier::BOLD)),
                    ]);
                    buf.set_line(diag_chunks[3].x, diag_chunks[3].y, &pwr_line, diag_chunks[3].width);
                }

                // Vertical Column Gauges for FAN and HOTSPOT
                let vert_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(lower_cols[1]);

                let fan_pct = g.fan_percent.unwrap_or(0.0);
                let fan_label = format!("{fan_pct:.0}%");
                let fan_color = if fan_pct < 40.0 { theme.temp_cool } else if fan_pct < 75.0 { theme.temp_warm } else { theme.temp_hot };
                VerticalGauge::new("FAN", &fan_label, fan_pct, fan_color)
                    .dim_color(theme.bar_track)
                    .render(vert_cols[0], buf);

                let hot_pct = (temp_hot / 110.0 * 100.0).clamp(0.0, 100.0);
                let hot_label = format!("{temp_hot:.0}°");
                let hot_color = thermal_color(temp_hot);
                VerticalGauge::new("HOT", &hot_label, hot_pct, hot_color)
                    .dim_color(theme.bar_track)
                    .render(vert_cols[1], buf);

                // 4. Clocks & Voltage Line (Guarded by clock_idx)
                if let Some(c_idx) = clock_idx {
                    if c_idx < right_chunks.len() && right_chunks[c_idx].height > 0 {
                        let mut clk_spans = vec![
                            Span::styled("Clocks: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("SCLK {sclk}MHz "), Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("| MCLK {mclk}MHz "), Style::default().fg(theme.spark_mem).add_modifier(Modifier::BOLD)),
                        ];
                        if let Some(mv) = g.voltage_mv {
                            clk_spans.push(Span::styled(format!("| VDD: {mv}mV"), Style::default().fg(theme.fg_dim)));
                        }
                        let clk_line = Line::from(clk_spans);
                        buf.set_line(right_chunks[c_idx].x, right_chunks[c_idx].y, &clk_line, right_chunks[c_idx].width);
                    }
                }

                // 5. PCIe, Fan & Driver Telemetry Line (Guarded by pcie_idx)
                if let Some(p_idx) = pcie_idx {
                    if p_idx < right_chunks.len() && right_chunks[p_idx].height > 0 {
                        let mut pcie_spans = Vec::new();
                        if let Some(ref pcie) = g.pcie_link {
                            pcie_spans.push(Span::styled("PCIe: ", Style::default().fg(theme.fg_dim)));
                            pcie_spans.push(Span::styled(format!("{pcie}  "), Style::default().fg(theme.fg)));
                        } else {
                            pcie_spans.push(Span::styled("PCI: ", Style::default().fg(theme.fg_dim)));
                            pcie_spans.push(Span::styled(format!("{}  ", g.pci_bus_id), Style::default().fg(theme.fg)));
                        }

                        pcie_spans.push(Span::styled("Fan: ", Style::default().fg(theme.fg_dim)));
                        if let Some(rpm) = g.fan_rpm {
                            pcie_spans.push(Span::styled(format!("{fan_pct:.0}% ({rpm} RPM)  "), Style::default().fg(theme.fg)));
                        } else {
                            pcie_spans.push(Span::styled(format!("{fan_pct:.0}%  "), Style::default().fg(theme.fg)));
                        }

                        let driver_label = if g.driver_version.is_empty() {
                            g.vendor.as_str().to_string()
                        } else {
                            format!("{}/{}", g.vendor.as_str(), g.driver_version)
                        };
                        pcie_spans.push(Span::styled("Driver: ", Style::default().fg(theme.fg_dim)));
                        pcie_spans.push(Span::styled(driver_label, Style::default().fg(theme.fg)));

                        let pcie_line = Line::from(pcie_spans);
                        buf.set_line(right_chunks[p_idx].x, right_chunks[p_idx].y, &pcie_line, right_chunks[p_idx].width);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // Panel 3: LLM INFERENCE ENGINE, MTP VALIDATION & KV CACHE
    // ------------------------------------------------------------------------
    fn render_llm_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let llm = self.app.llm.as_ref();

        let model = llm.map(|l| l.model_alias.as_str()).unwrap_or("No LLM Active");
        let quant = llm.map(|l| l.model_ftype.as_str()).unwrap_or("N/A");
        let dec_tps = llm.map(|l| l.current_decode_tps).unwrap_or(0.0);

        let title = Line::from(vec![
            Span::styled("┌3llm", Style::default().fg(theme.box_llm).add_modifier(Modifier::BOLD)),
            Span::styled(format!("───{model}───────────────────"), Style::default().fg(theme.box_llm)),
            Span::styled(format!("{dec_tps:.1} t/s "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled(quant.to_string(), Style::default().fg(theme.fg)),
            Span::styled("────────────────────┐", Style::default().fg(theme.box_llm)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_llm));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        let dec_cur = dec_tps;
        let dec_1m = self.app.llm_decode_avg_1m();
        let dec_15m = self.app.llm_decode_avg_15m();
        let dec_all = self.app.llm_decode_avg_all();
        let dec_pk = self.app.llm_decode_peak();

        let prf_cur = llm.map(|l| l.current_prefill_tps).unwrap_or(0.0) as f64;
        let prf_1m = self.app.llm_prefill_avg_1m();
        let prf_15m = self.app.llm_prefill_avg_15m();
        let prf_all = self.app.llm_prefill_avg_all();
        let prf_pk = self.app.llm_prefill_peak();

        let mtp_rate_opt = llm.and_then(|l| l.speculative_acceptance_rate);
        let mtp_cur = mtp_rate_opt.map(|r| r as f64);
        let mtp_1m = self.app.llm_mtp_avg_1m();
        let mtp_15m = self.app.llm_mtp_avg_15m();
        let mtp_all = self.app.llm_mtp_avg_all();
        let mtp_pk = self.app.llm_mtp_peak();

        // Divide LLM panel: Left 2/3 (Model Info, Gauges & Braille Graph); Right 1/3 (Throughput Averages & Stats)
        let is_wide = inner.width >= 50;
        let (left_area, right_area_opt) = if is_wide {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .spacing(1)
                .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
                .split(inner);
            (cols[0], Some(cols[1]))
        } else {
            (inner, None)
        };

        // --- Left (2/3): Engine Specs, MTP/KV Gauges & Braille History Waveform ---
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Engine, Quant, Cache KV
                Constraint::Length(1), // Speculative Draft Model
                Constraint::Length(2), // MTP Acceptance Validation Progress Bar
                Constraint::Length(2), // KV Cache Pool Gauge
                Constraint::Min(2),    // Mini Braille Graph of Throughput History
            ])
            .split(left_area);

        let engine = llm.map(|l| l.engine_name.as_str()).unwrap_or("llama.cpp");
        let cache_k = llm.map(|l| l.cache_type_k.as_str()).unwrap_or("f16");
        let cache_v = llm.map(|l| l.cache_type_v.as_str()).unwrap_or("f16");
        let l1 = Line::from(vec![
            Span::styled("Engine: ", Style::default().fg(theme.fg_dim)),
            Span::styled(engine, Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("  Quant: ", Style::default().fg(theme.fg_dim)),
            Span::styled(quant, Style::default().fg(theme.fg_highlight)),
            Span::styled("  KV: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("K:{cache_k} V:{cache_v}"), Style::default().fg(theme.fg)),
        ]);
        buf.set_line(left_chunks[0].x, left_chunks[0].y, &l1, left_chunks[0].width);

        let draft_model = llm.and_then(|l| l.speculative_draft_model.as_deref()).unwrap_or("None");
        let draft_type = llm.and_then(|l| l.speculative_type.as_deref()).unwrap_or("None");
        let l2 = Line::from(vec![
            Span::styled("Speculative: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{draft_type} "), Style::default().fg(theme.fg_highlight)),
            Span::styled(format!("({draft_model})"), Style::default().fg(theme.fg_dim)),
        ]);
        buf.set_line(left_chunks[1].x, left_chunks[1].y, &l2, left_chunks[1].width);

        // MTP Acceptance Validation Section
        let mtp_acc = llm.map(|l| l.mtp_draft_accepted).unwrap_or(0);
        let mtp_gen = llm.map(|l| l.mtp_draft_generated).unwrap_or(0);
        let mtp_mean = llm.and_then(|l| l.mtp_mean_len).unwrap_or(0.0);

        let (mtp_pct, mtp_color, mtp_title) = if let Some(rate) = mtp_rate_opt {
            let color = if rate >= 60.0 {
                theme.temp_cool // High acceptance
            } else if rate >= 40.0 {
                theme.temp_warm // Moderate
            } else {
                theme.temp_hot  // Low acceptance
            };
            let title = format!("MTP Validation: {rate:.1}% ({mtp_acc}/{mtp_gen} accepted, mean len: {mtp_mean:.2})");
            (rate.clamp(0.0, 100.0) as u16, color, title)
        } else {
            (0, theme.bar_track, "MTP Validation: Awaiting inference token telemetry...".to_string())
        };

        Gauge::default()
            .block(Block::default().title(Span::styled(mtp_title, Style::default().fg(theme.fg_dim))))
            .gauge_style(Style::default().fg(mtp_color).bg(theme.bar_track))
            .percent(mtp_pct)
            .label(format!("{mtp_pct}%"))
            .render(left_chunks[2], buf);

        // KV Cache Pool Gauge
        let kv_pool_pct = llm.map(|l| l.kv_cache_pool_percent).unwrap_or(0.0);
        let ctx_used = llm.map(|l| l.context_tokens_used).unwrap_or(0);
        let ctx_max = llm.map(|l| l.context_window_max).unwrap_or(1).max(1);
        let ctx_pct = ((ctx_used as f64 / ctx_max as f64) * 100.0).clamp(0.0, 100.0) as u16;

        let gauge_label = format!("KV: {kv_pool_pct:.1}% | Ctx: {ctx_used}/{ctx_max} ({ctx_pct}%)");
        Gauge::default()
            .block(Block::default().title(Span::styled("KV Cache Pool & Context:", Style::default().fg(theme.fg_dim))))
            .gauge_style(Style::default().fg(theme.border_active).bg(theme.bar_track))
            .percent(kv_pool_pct.round() as u16)
            .label(gauge_label)
            .render(left_chunks[3], buf);

        // Live Token Generation Waveform (2D Braille Canvas)
        if left_chunks[4].height >= 1 {
            let tps_hist = self.app.decode_tps_history.as_vec();
            let max_val = self.app.decode_tps_history.max().max(50.0);
            BrailleCanvas::new(&tps_hist)
                .max(max_val)
                .single_color(theme.spark_tps)
                .baseline_color(theme.bar_track)
                .render(left_chunks[4], buf);
        }

        // --- Right (1/3): Dedicated Speed & Telemetry Card (Current, 1m, 15m, all-time, Peak for Decode, Prefill, MTP) ---
        if let Some(right_area) = right_area_opt {
            let speed_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(theme.fg_dim))
                .title(Line::from(vec![
                    Span::styled(" Speed & MTP ", Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                ]));

            let speed_inner = speed_block.inner(right_area);
            speed_block.render(right_area, buf);

            if speed_inner.height >= 4 && speed_inner.width >= 10 {
                let width = speed_inner.width;
                let height = speed_inner.height;

                let mut lines: Vec<Line> = Vec::new();

                let mtp_color = match mtp_cur {
                    Some(r) if r >= 60.0 => theme.temp_cool,
                    Some(r) if r >= 40.0 => theme.temp_warm,
                    Some(_) => theme.temp_hot,
                    None => theme.fg_dim,
                };

                let mtp_cur_str = mtp_cur.map(|r| format!("{r:.1}%")).unwrap_or_else(|| "--%".to_string());
                let mtp_pk_str = if mtp_pk > 0.0 { format!("{mtp_pk:.1}%") } else { "--%".to_string() };
                let mtp_1m_str = if mtp_1m > 0.0 { format!("{mtp_1m:.1}%") } else { "--%".to_string() };
                let mtp_15m_str = if mtp_15m > 0.0 { format!("{mtp_15m:.1}%") } else { "--%".to_string() };
                let mtp_all_str = if mtp_all > 0.0 { format!("{mtp_all:.1}%") } else { "--%".to_string() };

                let prf_cur_str = format!("{prf_cur:.0}");
                let prf_pk_str = if prf_pk > 0.0 { format!("{prf_pk:.0}") } else { "--".to_string() };
                let prf_1m_str = if prf_1m > 0.0 { format!("{prf_1m:.0}") } else { "--".to_string() };
                let prf_15m_str = if prf_15m > 0.0 { format!("{prf_15m:.0}") } else { "--".to_string() };
                let prf_all_str = if prf_all > 0.0 { format!("{prf_all:.0}") } else { "--".to_string() };

                if height >= 9 {
                    let show_spacers = height >= 11;
                    // --- 1. Decode ---
                    if width >= 28 {
                        lines.push(Line::from(vec![
                            Span::styled("Decode:  ", Style::default().fg(theme.spark_tps).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{dec_cur:>5.1} "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                            Span::styled("t/s  ", Style::default().fg(theme.fg_dim)),
                            Span::styled("[Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_pk:>4.1}"), Style::default().fg(theme.temp_warm).add_modifier(Modifier::BOLD)),
                            Span::styled("]", Style::default().fg(theme.fg_dim)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  Avg 1m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_1m:>4.1} "), Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                            Span::styled(" 15m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_15m:>4.1}"), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  All-time: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_all:>4.1} "), Style::default().fg(theme.fg)),
                            Span::styled("t/s", Style::default().fg(theme.fg_dim)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Dec: ", Style::default().fg(theme.spark_tps).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{dec_cur:.1} "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                            Span::styled("Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_pk:.1}"), Style::default().fg(theme.temp_warm)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_1m:.1} "), Style::default().fg(theme.border_active)),
                            Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_15m:.1}"), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" All:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{dec_all:.1} t/s"), Style::default().fg(theme.fg)),
                        ]));
                    }

                    if show_spacers {
                        lines.push(Line::from(""));
                    }

                    // --- 2. Prefill ---
                    if width >= 28 {
                        lines.push(Line::from(vec![
                            Span::styled("Prefill: ", Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{prf_cur_str:>5} "), Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                            Span::styled("t/s  ", Style::default().fg(theme.fg_dim)),
                            Span::styled("[Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_pk_str:>4}"), Style::default().fg(theme.temp_warm).add_modifier(Modifier::BOLD)),
                            Span::styled("]", Style::default().fg(theme.fg_dim)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  Avg 1m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_1m_str:>4} "), Style::default().fg(theme.temp_cool)),
                            Span::styled(" 15m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_15m_str:>4}"), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  All-time: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_all_str:>4} "), Style::default().fg(theme.fg)),
                            Span::styled("t/s", Style::default().fg(theme.fg_dim)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Prf: ", Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{prf_cur_str} "), Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                            Span::styled("Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(prf_pk_str.clone(), Style::default().fg(theme.temp_warm)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_1m_str} "), Style::default().fg(theme.temp_cool)),
                            Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(prf_15m_str.clone(), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" All:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{prf_all_str} t/s"), Style::default().fg(theme.fg)),
                        ]));
                    }

                    if show_spacers {
                        lines.push(Line::from(""));
                    }

                    // --- 3. MTP Acceptance ---
                    if width >= 28 {
                        lines.push(Line::from(vec![
                            Span::styled("MTP Acc: ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{mtp_cur_str:>6} "), Style::default().fg(mtp_color).add_modifier(Modifier::BOLD)),
                            Span::styled(" [Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{mtp_pk_str:>5}"), Style::default().fg(theme.temp_warm).add_modifier(Modifier::BOLD)),
                            Span::styled("]", Style::default().fg(theme.fg_dim)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  Avg 1m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{mtp_1m_str:>5} "), Style::default().fg(theme.border_active)),
                            Span::styled(" 15m: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{mtp_15m_str:>5}"), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("  All-time: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{mtp_all_str:>5}"), Style::default().fg(theme.fg)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("MTP: ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{mtp_cur_str} "), Style::default().fg(mtp_color).add_modifier(Modifier::BOLD)),
                            Span::styled("Peak: ", Style::default().fg(theme.fg_dim)),
                            Span::styled(mtp_pk_str.clone(), Style::default().fg(theme.temp_warm)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(format!("{mtp_1m_str} "), Style::default().fg(theme.border_active)),
                            Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                            Span::styled(mtp_15m_str.clone(), Style::default().fg(theme.fg)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled(" All:", Style::default().fg(theme.fg_dim)),
                            Span::styled(mtp_all_str, Style::default().fg(theme.fg)),
                        ]));
                    }
                } else {
                    // Compact 2-line per metric layout (fits in 6 rows)
                    lines.push(Line::from(vec![
                        Span::styled("Decode:  ", Style::default().fg(theme.spark_tps).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{dec_cur:>4.1} "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                        Span::styled("t/s ", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("[Peak: {dec_pk:.1}]"), Style::default().fg(theme.temp_warm)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{dec_1m:>4.1} "), Style::default().fg(theme.border_active)),
                        Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{dec_15m:>4.1} "), Style::default().fg(theme.fg)),
                        Span::styled("All:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{dec_all:>4.1}"), Style::default().fg(theme.fg)),
                    ]));

                    lines.push(Line::from(vec![
                        Span::styled("Prefill: ", Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{prf_cur_str:>4} "), Style::default().fg(theme.temp_cool).add_modifier(Modifier::BOLD)),
                        Span::styled("t/s ", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("[Peak: {prf_pk_str}]"), Style::default().fg(theme.temp_warm)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{prf_1m_str:>4} "), Style::default().fg(theme.temp_cool)),
                        Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{prf_15m_str:>4} "), Style::default().fg(theme.fg)),
                        Span::styled("All:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{prf_all_str:>4}"), Style::default().fg(theme.fg)),
                    ]));

                    lines.push(Line::from(vec![
                        Span::styled("MTP Acc: ", Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{mtp_cur_str} "), Style::default().fg(mtp_color).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("[Peak: {mtp_pk_str}]"), Style::default().fg(theme.temp_warm)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(" 1m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{mtp_1m_str} "), Style::default().fg(theme.border_active)),
                        Span::styled("15m:", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{mtp_15m_str} "), Style::default().fg(theme.fg)),
                        Span::styled("All:", Style::default().fg(theme.fg_dim)),
                        Span::styled(mtp_all_str, Style::default().fg(theme.fg)),
                    ]));
                }

                for (idx, line) in lines.iter().enumerate() {
                    let y = speed_inner.y + idx as u16;
                    if y < speed_inner.y + speed_inner.height {
                        buf.set_line(speed_inner.x, y, line, speed_inner.width);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // Panel 4: ACTIVE SLOTS TABLE (Compressed with MTP Validation Column)
    // ------------------------------------------------------------------------
    fn render_slots_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let llm = self.app.llm.as_ref();

        let active_slots = llm.map(|l| l.active_slots).unwrap_or(0);
        let total_slots = llm.map(|l| l.total_slots).unwrap_or(1);

        let title = Line::from(vec![
            Span::styled("┌3slots", Style::default().fg(theme.box_queue).add_modifier(Modifier::BOLD)),
            Span::styled(format!("───Slots ({active_slots}/{total_slots})──"), Style::default().fg(theme.box_queue)),
            Span::styled("──[↑/↓]──┐", Style::default().fg(theme.box_queue)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_queue));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width < 10 {
            return;
        }

        let header = Row::new(vec!["Slot", "Task", "Status", "Prompt", "Dec", "MTP%", "Mode"])
            .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        if let Some(l) = llm {
            let inner_height = inner.height.saturating_sub(1) as usize;
            let selected = self.app.selected_slot_index.min(l.slots.len().saturating_sub(1));
            let scroll_offset = if selected >= inner_height && inner_height > 0 {
                selected.saturating_sub(inner_height.saturating_sub(1))
            } else {
                0
            };

            for (i, slot) in l.slots.iter().enumerate().skip(scroll_offset).take(inner_height) {
                let is_selected = i == self.app.selected_slot_index;
                let cursor = if is_selected { "▶" } else { " " };

                let status_color = if slot.is_processing {
                    theme.status_online
                } else {
                    theme.fg_dim
                };
                let status_str = if slot.is_processing { "Active" } else { "Idle" };
                let task_str = slot.id_task.map(|t| t.to_string()).unwrap_or_else(|| "—".to_string());
                let mode_str = if slot.speculative { "Draft" } else { "Base" };
                let mtp_str = slot
                    .draft_acceptance_rate
                    .map(|r| format!("{r:.0}%"))
                    .unwrap_or_else(|| "—".to_string());

                let row_style = if is_selected {
                    Style::default().fg(theme.selected_fg).bg(theme.selected_bg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                let status_style = if is_selected {
                    Style::default().fg(theme.selected_fg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD)
                };

                rows.push(Row::new(vec![
                    Line::from(format!("{cursor}#{}", slot.id)),
                    Line::from(task_str),
                    Line::from(Span::styled(status_str, status_style)),
                    Line::from(slot.n_prompt_tokens.to_string()),
                    Line::from(slot.n_decoded.to_string()),
                    Line::from(mtp_str),
                    Line::from(mode_str.to_string()),
                ]).style(row_style));
            }
        }

        if rows.is_empty() {
            rows.push(
                Row::new(vec![
                    Line::from("▶#0"),
                    Line::from("—"),
                    Line::from(Span::styled("Idle", Style::default().fg(theme.fg_dim))),
                    Line::from("0"),
                    Line::from("0"),
                    Line::from("—"),
                    Line::from("Draft"),
                ])
                .style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg)),
            );
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Min(6),
            ],
        )
        .header(header)
        .column_spacing(1);

        table.render(inner, buf);
    }

    // ------------------------------------------------------------------------
    // Panel 5: GPU TOP (Real-time DRM / GPU Process Client Monitor)
    // ------------------------------------------------------------------------
    fn render_gpu_top_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let gpu = self.app.gpus.first();
        let processes = gpu.map(|g| &g.processes[..]).unwrap_or(&[]);

        let title = Line::from(vec![
            Span::styled("┌4gputop", Style::default().fg(theme.box_gpu).add_modifier(Modifier::BOLD)),
            Span::styled("──GPU Top ", Style::default().fg(theme.box_gpu)),
            Span::styled(format!("({} procs)", processes.len()), Style::default().fg(theme.fg_highlight)),
            Span::styled("──", Style::default().fg(theme.box_gpu)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_gpu));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 2 || inner.width < 15 {
            return;
        }

        let header = Row::new(vec!["    PID", "Process", "    VRAM", "    GTT", "%VRAM"])
            .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        let max_procs = (inner.height.saturating_sub(1)) as usize;
        for p in processes.iter().take(max_procs) {
            let is_llama = p.name.to_lowercase().contains("llama");
            let row_style = if is_llama {
                Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)
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

            rows.push(Row::new(vec![
                Line::from(format!("{:>7}", p.pid)),
                Line::from(p.name.clone()),
                Line::from(format!("{:>8}", vram_str)),
                Line::from(format!("{:>7}", gtt_str)),
                Line::from(format!("{:>5}", format!("{:.0}%", p.vram_percent))),
            ]).style(row_style));
        }

        if rows.is_empty() {
            rows.push(Row::new(vec![
                Line::from("      —"),
                Line::from("No GPU clients"),
                Line::from("   0 MiB"),
                Line::from("  0 MiB"),
                Line::from("   0%"),
            ]).style(Style::default().fg(theme.fg_dim)));
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(7),
                Constraint::Min(10),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(5),
            ],
        )
        .header(header)
        .column_spacing(1);

        table.render(inner, buf);
    }
}

// ----------------------------------------------------------------------------
// Helper formatting functions
// ----------------------------------------------------------------------------


fn rectangular_bar(val: f32, max_val: f32, length: usize) -> (String, String) {
    let frac = if max_val > 0.0 { (val / max_val).clamp(0.0, 1.0) } else { 0.0 };
    let filled_len = (frac * length as f32).round() as usize;
    let filled_str = "█".repeat(filled_len);
    let empty_str = "░".repeat(length.saturating_sub(filled_len));
    (filled_str, empty_str)
}

fn dot_gradient_bar(pct: f32, length: usize) -> (String, String) {
    let frac = (pct / 100.0).clamp(0.0, 1.0);
    let filled_len = (frac * length as f32).round() as usize;
    let filled_str = "●".repeat(filled_len);
    let empty_str = "·".repeat(length.saturating_sub(filled_len));
    (filled_str, empty_str)
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

impl<'a> Widget for DashboardView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Divide dashboard into 3 tiers with vertical spacing so blocks do not fuse together
        let rows = if area.height >= 20 {
            Layout::default()
                .direction(Direction::Vertical)
                .spacing(1)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(area)
        };

        // 1. CPU Panel
        self.render_cpu_panel(rows[0], buf);

        // 2. GPU Panel (3-way split inside)
        self.render_gpu_panel(rows[1], buf);

        // 3. LLM & Compressed Slots / GPU Top Row with horizontal spacing
        let tier3_cols = Layout::default()
            .direction(Direction::Horizontal)
            .spacing(1)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(rows[2]);

        self.render_llm_panel(tier3_cols[0], buf);

        if tier3_cols[1].height >= 6 {
            let num_slots = self.app.llm.as_ref().map(|l| l.slots.len().max(1)).unwrap_or(1) as u16;
            // Slots box needs: 1 top border + 1 header + N slots + 1 bottom border = N + 3
            // Reserve at least 4 rows for GPU Top so its header and top process remain visible
            let max_slots_h = tier3_cols[1].height.saturating_sub(4).max(3);
            let slots_h = (num_slots + 3).min(max_slots_h).max(4);

            let right_sub = Layout::default()
                .direction(Direction::Vertical)
                .spacing(1)
                .constraints([
                    Constraint::Length(slots_h), // Scales dynamically with number of slots!
                    Constraint::Min(3),          // GPU Top table
                ])
                .split(tier3_cols[1]);

            self.render_slots_panel(right_sub[0], buf);
            self.render_gpu_top_panel(right_sub[1], buf);
        } else {
            self.render_slots_panel(tier3_cols[1], buf);
        }
    }
}
