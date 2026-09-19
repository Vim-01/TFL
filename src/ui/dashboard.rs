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

        let cpu_brand = cpu_opt
            .and_then(|c| c.brand.as_deref())
            .unwrap_or("Host CPU");

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
            .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(inner);

        // --- Left: Full-height 2D Braille Canvas (Area 1) ---
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(2)])
            .split(cols[0]);

        let chart_title = Line::from(vec![
            Span::styled("CPU Total Load: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{global_usage:.1}% "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            Span::styled(format!("| Avg: {avg_freq:.2} GHz | Load: {:.2} {:.2} {:.2}", load_avg[0], load_avg[1], load_avg[2]), Style::default().fg(theme.fg_dim)),
        ]);
        buf.set_line(left_chunks[0].x, left_chunks[0].y, &chart_title, left_chunks[0].width);

        // 2D Braille Canvas: fills the entire left height from top to bottom
        let cpu_hist = self.app.cpu_usage_history.as_vec();
        BrailleCanvas::new(&cpu_hist)
            .max(100.0)
            .colors(theme.box_cpu, theme.fg_highlight, theme.temp_hot, theme.bar_track)
            .render(left_chunks[1], buf);

        // --- Right: Compact btop-style Cores & System Sub-box (Area 3) ---
        let cores_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(theme.fg_dim))
            .title(Line::from(vec![
                Span::styled(" Cores & Memory ", Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
            ]));

        let cores_inner = cores_block.inner(cols[1]);
        cores_block.render(cols[1], buf);

        if let Some(cpu) = cpu_opt {
            let num_cores = cpu.core_usages.len();
            if num_cores > 0 && cores_inner.height >= 3 {
                let sub_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Header
                        Constraint::Min(2),    // Cores Grid
                        Constraint::Length(1), // Memory / Load
                    ])
                    .split(cores_inner);

                // Sub-header
                let ram_used = cpu.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_total = cpu.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_pct = if ram_total > 0.0 { (ram_used / ram_total * 100.0) as u16 } else { 0 };

                let sub_title = Line::from(vec![
                    Span::styled(format!("{cpu_brand} "), Style::default().fg(theme.fg)),
                    Span::styled(format!("[{avg_freq:.2} GHz]  "), Style::default().fg(theme.fg_dim)),
                    Span::styled(format!("RAM: {ram_used:.1}/{ram_total:.1} GiB ({ram_pct}%)"), Style::default().fg(theme.border_active)),
                ]);
                buf.set_line(sub_chunks[0].x, sub_chunks[0].y, &sub_title, sub_chunks[0].width);

                // Multi-column Core Grid with Braille Dot Bars
                let num_cols = if cores_inner.width >= 48 { 4 } else { 3 };
                let num_rows = num_cores.div_ceil(num_cols);
                let col_width = sub_chunks[1].width / num_cols as u16;

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

                        let x = sub_chunks[1].x + (c as u16 * col_width);
                        let y = sub_chunks[1].y + r as u16;

                        if y < sub_chunks[1].y + sub_chunks[1].height {
                            // Braille dot bar: dots '⣀' when 0%, filling with '⡇' and '⣿'
                            let (bar_filled, bar_empty) = core_dot_bar(usage_clamped, 3);

                            let core_line = Line::from(vec![
                                Span::styled(format!("C{core_idx:<2}:"), Style::default().fg(theme.fg_dim)),
                                Span::styled(bar_filled, Style::default().fg(bar_color)),
                                Span::styled(bar_empty, Style::default().fg(theme.bar_track)),
                                Span::styled(format!("{usage_clamped:>3.0}% "), Style::default().fg(theme.fg)),
                            ]);
                            buf.set_line(x, y, &core_line, col_width);
                        }
                    }
                }

                // Bottom Line
                let bot_line = Line::from(vec![
                    Span::styled("Load avg: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format!("{:.2} {:.2} {:.2}  ", load_avg[0], load_avg[1], load_avg[2]), Style::default().fg(theme.fg)),
                    Span::styled("Uptime: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(format_uptime(cpu.uptime_seconds), Style::default().fg(theme.fg)),
                ]);
                buf.set_line(sub_chunks[2].x, sub_chunks[2].y, &bot_line, sub_chunks[2].width);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Panel 2: GPU HARDWARE & VRAM (3-Way Split)
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

        // 3-Way Division matching user instructions:
        // Col 1 (6): GPU Compute Core Activity (2D Braille Canvas)
        // Col 2 (5): GPU Memory Bus / IO Activity (2D Braille Canvas)
        // Col 3 (2): VRAM Allocation, Thermals with exact colors, PWR gradient, Vertical Fan/Hotspot
        let cols = Layout::default()
            .direction(Direction::Horizontal)
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
                .colors(theme.spark_compute, theme.fg_highlight, theme.temp_hot, theme.bar_track)
                .render(comp_chunks[1], buf);
        }

        // --- Sub-block 2 (Col 2 / 5): Memory Bus / Controller IO ---
        let mem_bus = gpu.map(|g| g.mem_utilization_percent).unwrap_or(0.0);
        let mclk = gpu.and_then(|g| g.mclk_mhz).unwrap_or(0);

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
                .colors(theme.spark_mem, theme.fg_highlight, theme.temp_hot, theme.bar_track)
                .render(mem_chunks[1], buf);
        }

        // --- Sub-block 3 (Col 3 / 2): VRAM, Thermals, Power & Vertical Gauges ---
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
            if right_inner.height >= 3 {
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(2), // VRAM rectangular gauge
                        Constraint::Min(2),    // Thermals, PWR, and Vertical Gauges
                    ])
                    .split(right_inner);

                // 1. VRAM Allocation rectangular gauge
                let vram_label = format!("{vram_used_gib:.1} / {vram_total_gib:.1} GiB ({vram_pct}%)");
                Gauge::default()
                    .block(Block::default().title(Span::styled("VRAM Allocation:", Style::default().fg(theme.fg_dim))))
                    .gauge_style(Style::default().fg(theme.border_active).bg(theme.bar_track))
                    .percent(vram_pct)
                    .label(vram_label)
                    .render(right_chunks[0], buf);

                // 2. Split lower section into: Left (Thermals & PWR) and Right (Vertical FAN & Hotspot)
                let lower_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(16),   // Thermals and PWR
                        Constraint::Length(10), // Vertical FAN and HOTSPOT
                    ])
                    .split(right_chunks[1]);

                // Lower Left: Thermal Scales with explicit colors and PWR dot gradient
                let temp_edge = g.temp_edge_c.unwrap_or(0.0);
                let temp_hot = g.temp_hotspot_c.unwrap_or(0.0);
                let temp_mem = g.temp_mem_c.unwrap_or(0.0);

                let pwr_cur = g.power_current_w.unwrap_or(0.0);
                let pwr_cap = g.power_cap_w.unwrap_or(402.0).max(1.0);
                let pwr_pct = (pwr_cur / pwr_cap * 100.0).clamp(0.0, 100.0);

                let diag_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // Edge temp
                        Constraint::Length(1), // VRAM temp
                        Constraint::Length(1), // PWR dot gradient
                    ])
                    .split(lower_cols[0]);

                // Edge Temp with user color rule: 0-70 Green, 70-80 Yellow, 80-84 Orange, 85+ Red
                let edge_color = thermal_color(temp_edge);
                let (edge_bar, edge_empty) = rectangular_bar(temp_edge, 100.0, 6);
                let edge_line = Line::from(vec![
                    Span::styled("Edge: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(edge_bar, Style::default().fg(edge_color)),
                    Span::styled(edge_empty, Style::default().fg(theme.bar_track)),
                    Span::styled(format!(" {temp_edge:.0}°C"), Style::default().fg(edge_color).add_modifier(Modifier::BOLD)),
                ]);
                buf.set_line(diag_chunks[0].x, diag_chunks[0].y, &edge_line, diag_chunks[0].width);

                // VRAM Temp with user color rule
                let mem_color = thermal_color(temp_mem);
                let (mem_bar, mem_empty) = rectangular_bar(temp_mem, 100.0, 6);
                let mem_line = Line::from(vec![
                    Span::styled("VRAM: ", Style::default().fg(theme.fg_dim)),
                    Span::styled(mem_bar, Style::default().fg(mem_color)),
                    Span::styled(mem_empty, Style::default().fg(theme.bar_track)),
                    Span::styled(format!(" {temp_mem:.0}°C"), Style::default().fg(mem_color).add_modifier(Modifier::BOLD)),
                ]);
                buf.set_line(diag_chunks[1].x, diag_chunks[1].y, &mem_line, diag_chunks[1].width);

                // PWR Dot Gradient: dots filled with green to red gradient based on % consumption
                let pwr_color = power_gradient_color(pwr_pct);
                let (pwr_dots_filled, pwr_dots_empty) = dot_gradient_bar(pwr_pct, 6);
                let pwr_line = Line::from(vec![
                    Span::styled("PWR:  ", Style::default().fg(theme.fg_dim)),
                    Span::styled(pwr_dots_filled, Style::default().fg(pwr_color)),
                    Span::styled(pwr_dots_empty, Style::default().fg(theme.bar_track)),
                    Span::styled(format!(" {pwr_cur:.0}W"), Style::default().fg(pwr_color).add_modifier(Modifier::BOLD)),
                ]);
                buf.set_line(diag_chunks[2].x, diag_chunks[2].y, &pwr_line, diag_chunks[2].width);

                // Lower Right: Vertical Column Gauges for FAN and HOTSPOT
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Engine, Quant, Cache KV
                Constraint::Length(1), // Speculative Draft Model
                Constraint::Length(2), // MTP Acceptance Validation Progress Bar
                Constraint::Length(2), // KV Cache Pool Gauge
                Constraint::Min(2),    // Throughput Stats & Mini Braille Graph
            ])
            .split(inner);

        let engine = llm.map(|l| l.engine_name.as_str()).unwrap_or("llama.cpp");
        let cache_k = llm.map(|l| l.cache_type_k.as_str()).unwrap_or("f16");
        let cache_v = llm.map(|l| l.cache_type_v.as_str()).unwrap_or("f16");
        let l1 = Line::from(vec![
            Span::styled("Engine: ", Style::default().fg(theme.fg_dim)),
            Span::styled(engine, Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("  Quant: ", Style::default().fg(theme.fg_dim)),
            Span::styled(quant, Style::default().fg(theme.fg_highlight)),
            Span::styled("  Cache KV: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("K:{cache_k} V:{cache_v}"), Style::default().fg(theme.fg)),
        ]);
        buf.set_line(chunks[0].x, chunks[0].y, &l1, chunks[0].width);

        let draft_model = llm.and_then(|l| l.speculative_draft_model.as_deref()).unwrap_or("None");
        let draft_type = llm.and_then(|l| l.speculative_type.as_deref()).unwrap_or("None");
        let l2 = Line::from(vec![
            Span::styled("Speculative: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{draft_type} "), Style::default().fg(theme.fg_highlight)),
            Span::styled(format!("({draft_model})"), Style::default().fg(theme.fg_dim)),
        ]);
        buf.set_line(chunks[1].x, chunks[1].y, &l2, chunks[1].width);

        // MTP Acceptance Validation Section
        let mtp_rate_opt = llm.and_then(|l| l.speculative_acceptance_rate);
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
            .render(chunks[2], buf);

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
            .render(chunks[3], buf);

        // Throughput & Live Token Generation Mini-Graph
        if chunks[4].height >= 2 {
            let spark_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(chunks[4]);

            let prefill_tps = llm.map(|l| l.current_prefill_tps).unwrap_or(0.0);
            let peak_tps = llm.map(|l| l.peak_decode_tps).unwrap_or(0.0).max(self.app.decode_tps_history.max() as f32);

            let tps_line = Line::from(vec![
                Span::styled("Decode: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{dec_tps:.1} t/s  "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                Span::styled("Prefill: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{prefill_tps:.0} t/s  "), Style::default().fg(theme.temp_cool)),
                Span::styled("Peak: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{peak_tps:.1} t/s"), Style::default().fg(theme.fg_highlight)),
            ]);
            buf.set_line(spark_layout[0].x, spark_layout[0].y, &tps_line, spark_layout[0].width);

            // Mini 2D Braille Canvas for token generation throughput history
            let tps_hist = self.app.decode_tps_history.as_vec();
            let max_val = self.app.decode_tps_history.max().max(50.0);
            BrailleCanvas::new(&tps_hist)
                .max(max_val)
                .colors(theme.spark_tps, theme.fg_highlight, theme.temp_hot, theme.bar_track)
                .render(spark_layout[1], buf);
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

        let header = Row::new(vec!["Slot", "Task", "Status", "Prompt", "Dec", "MTP%", "Mode"])
            .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        if let Some(l) = llm {
            for (i, slot) in l.slots.iter().enumerate() {
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
}

// ----------------------------------------------------------------------------
// Helper formatting functions
// ----------------------------------------------------------------------------

fn core_dot_bar(usage: f32, length: usize) -> (String, String) {
    let frac = (usage / 100.0).clamp(0.0, 1.0);
    let total_dots = length * 2;
    let filled_dots = (frac * total_dots as f32).round() as usize;
    let full_chars = filled_dots / 2;
    let has_half = !filled_dots.is_multiple_of(2);
    let mut filled_str = "⣿".repeat(full_chars);
    if has_half {
        filled_str.push('⡇');
    }
    let empty_len = length.saturating_sub(filled_str.chars().count());
    let empty_str = "⣀".repeat(empty_len);
    (filled_str, empty_str)
}

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
        // Divide dashboard into 3 tiers:
        // Tier 1: CPU Panel (~33%)
        // Tier 2: GPU Panel (~33%)
        // Tier 3: LLM & Slots Row (~34%) with 62/38 horizontal split
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        // 1. CPU Panel
        self.render_cpu_panel(rows[0], buf);

        // 2. GPU Panel (3-way split inside)
        self.render_gpu_panel(rows[1], buf);

        // 3. LLM & Compressed Slots Row
        let tier3_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(rows[2]);

        self.render_llm_panel(tier3_cols[0], buf);
        self.render_slots_panel(tier3_cols[1], buf);
    }
}
