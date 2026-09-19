use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, Row, Sparkline, Table, Widget,
    },
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

        // Split: Left = CPU history wave graph; Right = Multi-column core grid
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner);

        // Left: CPU History Sparkline
        let cpu_hist: Vec<u64> = self
            .app
            .cpu_usage_history
            .as_vec()
            .iter()
            .map(|&v| if v.is_finite() && v >= 0.0 { v.min(100.0) as u64 } else { 0 })
            .collect();

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(2), Constraint::Length(1)])
            .split(cols[0]);

        let chart_title = Line::from(vec![
            Span::styled("CPU Total Load (History): ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{global_usage:.1}%"), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
        ]);
        buf.set_line(left_chunks[0].x, left_chunks[0].y, &chart_title, left_chunks[0].width);

        Sparkline::default()
            .data(&cpu_hist)
            .max(100)
            .style(Style::default().fg(theme.box_cpu))
            .render(left_chunks[1], buf);

        let load_str = Line::from(vec![
            Span::styled("Load avg: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{:.2} {:.2} {:.2}", load_avg[0], load_avg[1], load_avg[2]), Style::default().fg(theme.fg)),
        ]);
        buf.set_line(left_chunks[2].x, left_chunks[2].y, &load_str, left_chunks[2].width);

        // Right: Multi-column CPU Cores Table (like btop)
        if let Some(cpu) = cpu_opt {
            let num_cores = cpu.core_usages.len();
            if num_cores > 0 {
                let max_cols = (cols[1].width / 15).max(1) as usize;
                let num_cols = max_cols.min(num_cores).max(1);
                let num_rows = num_cores.div_ceil(num_cols);

                let col_width = cols[1].width / (num_cols as u16).max(1);

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

                    let x = cols[1].x + (c as u16 * col_width);
                    let y = cols[1].y + r as u16;
                    if y < cols[1].y + cols[1].height {
                        let bar_len = ((usage_clamped / 100.0) * 5.0).round() as usize;
                        let bar_str = "■".repeat(bar_len);
                        let empty_str = "·".repeat(5_usize.saturating_sub(bar_len));

                        let core_line = Line::from(vec![
                            Span::styled(format!("C{core_idx:<2}:"), Style::default().fg(theme.fg_dim)),
                            Span::styled(bar_str, Style::default().fg(bar_color)),
                            Span::styled(format!("{empty_str} "), Style::default().fg(theme.bar_track)),
                            Span::styled(format!("{usage_clamped:>3.0}% "), Style::default().fg(theme.fg)),
                        ]);
                        buf.set_line(x, y, &core_line, col_width);
                    }
                }
            }

                // Bottom line: Memory & Swap
                let mem_y = cols[1].y + num_rows as u16;
                if mem_y < cols[1].y + cols[1].height {
                    let ram_used = cpu.ram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    let ram_total = cpu.ram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    let ram_pct = if ram_total > 0.0 { (ram_used / ram_total * 100.0) as u16 } else { 0 };

                    let ram_line = Line::from(vec![
                        Span::styled("RAM: ", Style::default().fg(theme.fg_dim)),
                        Span::styled(format!("{ram_used:.1}/{ram_total:.1} GiB ({ram_pct}%)"), Style::default().fg(theme.border_active)),
                    ]);
                    buf.set_line(cols[1].x, mem_y, &ram_line, cols[1].width);
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // Panel 2: GPU HARDWARE & VRAM (btop Style)
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

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner);

        // Left: Dual Sparkline (Compute & VRAM Bus activity)
        let gpu_hist: Vec<u64> = self
            .app
            .gpu_compute_history
            .as_vec()
            .iter()
            .map(|&v| if v.is_finite() && v >= 0.0 { v.min(100.0) as u64 } else { 0 })
            .collect();

        let mem_hist: Vec<u64> = self
            .app
            .gpu_vram_history
            .as_vec()
            .iter()
            .map(|&v| if v.is_finite() && v >= 0.0 { v.min(100.0) as u64 } else { 0 })
            .collect();

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(cols[0]);

        let comp_title = Line::from(vec![
            Span::styled("GPU Compute Activity: ", Style::default().fg(theme.fg_dim)),
            Span::styled(format!("{compute:.1}%"), Style::default().fg(theme.spark_compute).add_modifier(Modifier::BOLD)),
        ]);
        buf.set_line(left_chunks[0].x, left_chunks[0].y, &comp_title, left_chunks[0].width);
        Sparkline::default()
            .data(&gpu_hist)
            .max(100)
            .style(Style::default().fg(theme.spark_compute))
            .render(left_chunks[1], buf);

        if left_chunks.len() >= 4 {
            let mem_bus = gpu.map(|g| g.mem_utilization_percent).unwrap_or(0.0);
            let bus_title = Line::from(vec![
                Span::styled("Memory Bus / Controller: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{mem_bus:.1}%"), Style::default().fg(theme.spark_mem).add_modifier(Modifier::BOLD)),
            ]);
            buf.set_line(left_chunks[2].x, left_chunks[2].y, &bus_title, left_chunks[2].width);
            Sparkline::default()
                .data(&mem_hist)
                .max(100)
                .style(Style::default().fg(theme.spark_mem))
                .render(left_chunks[3], buf);
        }

        // Right: VRAM Bar, Power, Temperatures
        if let Some(g) = gpu {
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // VRAM Gauge
                    Constraint::Length(1), // Power Gauge
                    Constraint::Length(1), // Temperatures
                    Constraint::Min(1),    // Clocks
                ])
                .split(cols[1]);

            let vram_used_gib = g.vram_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let vram_total_gib = g.vram_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let vram_pct = if vram_total_gib > 0.0 {
                ((vram_used_gib / vram_total_gib) * 100.0).clamp(0.0, 100.0) as u16
            } else {
                0
            };

            let vram_label = format!("{vram_used_gib:.1} / {vram_total_gib:.1} GiB ({vram_pct}%)");
            Gauge::default()
                .block(Block::default().title(Span::styled("VRAM Allocation:", Style::default().fg(theme.fg_dim))))
                .gauge_style(Style::default().fg(theme.border_active).bg(theme.bar_track))
                .percent(vram_pct)
                .label(vram_label)
                .render(right_chunks[0], buf);

            let pwr_cur = g.power_current_w.unwrap_or(0.0);
            let pwr_cap = g.power_cap_w.unwrap_or(0.0);
            let pwr_line = Line::from(vec![
                Span::styled("PWR: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{pwr_cur:.1} W"), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" / {pwr_cap:.1} W    "), Style::default().fg(theme.fg_dim)),
                Span::styled("Fan: ", Style::default().fg(theme.fg_dim)),
                Span::styled(g.fan_percent.map(|f| format!("{f:.0}%")).unwrap_or_else(|| "Auto".to_string()), Style::default().fg(theme.fg)),
            ]);
            buf.set_line(right_chunks[1].x, right_chunks[1].y, &pwr_line, right_chunks[1].width);

            let temp_edge = g.temp_edge_c.unwrap_or(0.0);
            let temp_hot = g.temp_hotspot_c.unwrap_or(0.0);
            let temp_mem = g.temp_mem_c.unwrap_or(0.0);
            let temp_line = Line::from(vec![
                Span::styled("TEMP: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{temp_edge:.0}°C"), Style::default().fg(theme.temp_cool)),
                Span::styled(" (Edge)  ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{temp_hot:.0}°C"), Style::default().fg(theme.temp_warm)),
                Span::styled(" (Hotspot)  ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{temp_mem:.0}°C"), Style::default().fg(theme.temp_cool)),
                Span::styled(" (VRAM)", Style::default().fg(theme.fg_dim)),
            ]);
            buf.set_line(right_chunks[2].x, right_chunks[2].y, &temp_line, right_chunks[2].width);
        }
    }

    // ------------------------------------------------------------------------
    // Panel 3: LLM INFERENCE ENGINE & KV CACHE
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
                Constraint::Length(2), // KV Cache Pool Gauge
                Constraint::Length(2), // Context Window Gauge
                Constraint::Min(2),    // Throughput Stats & Sparkline
            ])
            .split(inner);

        let engine = llm.map(|l| l.engine_name.as_str()).unwrap_or("llama.cpp");
        let cache_k = llm.map(|l| l.cache_type_k.as_str()).unwrap_or("f16");
        let cache_v = llm.map(|l| l.cache_type_v.as_str()).unwrap_or("f16");
        let l1 = Line::from(vec![
            Span::styled("Engine: ", Style::default().fg(theme.fg_dim)),
            Span::styled(engine, Style::default().fg(theme.border_active).add_modifier(Modifier::BOLD)),
            Span::styled("   Quant: ", Style::default().fg(theme.fg_dim)),
            Span::styled(quant, Style::default().fg(theme.fg_highlight)),
            Span::styled("   Cache KV: ", Style::default().fg(theme.fg_dim)),
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

        // KV Cache Pool Gauge
        let kv_pool_pct = llm.map(|l| l.kv_cache_pool_percent).unwrap_or(0.0);
        let gauge_label = format!("{kv_pool_pct:.1}%");
        Gauge::default()
            .block(Block::default().title(Span::styled("KV Cache Pool:", Style::default().fg(theme.fg_dim))))
            .gauge_style(Style::default().fg(theme.border_active).bg(theme.bar_track))
            .percent(kv_pool_pct.round() as u16)
            .label(gauge_label)
            .render(chunks[2], buf);

        // Context Window Gauge
        let ctx_used = llm.map(|l| l.context_tokens_used).unwrap_or(0);
        let ctx_max = llm.map(|l| l.context_window_max).unwrap_or(1).max(1);
        let ctx_pct = ((ctx_used as f64 / ctx_max as f64) * 100.0).clamp(0.0, 100.0) as u16;
        let ctx_label = format!("{ctx_used} / {ctx_max} tokens ({ctx_pct}%)");
        Gauge::default()
            .block(Block::default().title(Span::styled("Context Window:", Style::default().fg(theme.fg_dim))))
            .gauge_style(Style::default().fg(theme.box_gpu).bg(theme.bar_track))
            .percent(ctx_pct)
            .label(ctx_label)
            .render(chunks[3], buf);

        // Throughput & 60s Sparkline
        if chunks[4].height >= 2 {
            let spark_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(chunks[4]);

            let prefill_tps = llm.map(|l| l.current_prefill_tps).unwrap_or(0.0);
            let tps_line = Line::from(vec![
                Span::styled("Decode: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{dec_tps:.1} t/s  "), Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD)),
                Span::styled("Prefill: ", Style::default().fg(theme.fg_dim)),
                Span::styled(format!("{prefill_tps:.0} t/s  "), Style::default().fg(theme.temp_cool)),
                Span::styled(format!("Peak: {:.1} t/s", self.app.decode_tps_history.max()), Style::default().fg(theme.fg_dim)),
            ]);
            buf.set_line(spark_layout[0].x, spark_layout[0].y, &tps_line, spark_layout[0].width);

            let hist_u64: Vec<u64> = self
                .app
                .decode_tps_history
                .as_vec()
                .iter()
                .map(|&v| if v.is_finite() && v >= 0.0 { (v * 10.0).min(50_000.0) as u64 } else { 0 })
                .collect();
            let max_val = (self.app.decode_tps_history.max() * 10.0).clamp(100.0, 50_000.0) as u64;

            Sparkline::default()
                .data(&hist_u64)
                .max(max_val)
                .style(Style::default().fg(theme.spark_tps))
                .render(spark_layout[1], buf);
        }
    }

    // ------------------------------------------------------------------------
    // Panel 4: ACTIVE SLOTS & TASKS TABLE (Interactive with ↑/↓ keys)
    // ------------------------------------------------------------------------
    fn render_slots_panel(&self, area: Rect, buf: &mut Buffer) {
        let theme = &self.app.theme;
        let llm = self.app.llm.as_ref();

        let active_slots = llm.map(|l| l.active_slots).unwrap_or(0);
        let total_slots = llm.map(|l| l.total_slots).unwrap_or(1);

        let title = Line::from(vec![
            Span::styled("┌3slots", Style::default().fg(theme.box_queue).add_modifier(Modifier::BOLD)),
            Span::styled(format!("───Parallel Slots ({active_slots}/{total_slots} Active)────────"), Style::default().fg(theme.box_queue)),
            Span::styled("──[↑/↓:Select]────┐", Style::default().fg(theme.box_queue)),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.box_queue));

        let inner = block.inner(area);
        block.render(area, buf);

        let header = Row::new(vec!["  Slot", "Task ID", "Status", "Prompt", "Decoded", "Mode"])
            .style(Style::default().fg(theme.fg_highlight).add_modifier(Modifier::BOLD));

        let mut rows = Vec::new();
        if let Some(l) = llm {
            for (i, slot) in l.slots.iter().enumerate() {
                let is_selected = i == self.app.selected_slot_index;
                let cursor = if is_selected { "▶ " } else { "  " };

                let status_color = if slot.is_processing {
                    theme.status_online
                } else {
                    theme.fg_dim
                };
                let status_str = if slot.is_processing { "Generating" } else { "Idle" };
                let task_str = slot.id_task.map(|t| t.to_string()).unwrap_or_else(|| "—".to_string());
                let mode_str = if slot.speculative { "Draft MTP" } else { "Standard" };

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
                    Line::from(mode_str.to_string()),
                ]).style(row_style));
            }
        }

        if rows.is_empty() {
            rows.push(
                Row::new(vec![
                    Line::from("▶ #0"),
                    Line::from("—"),
                    Line::from(Span::styled("Idle", Style::default().fg(theme.fg_dim))),
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
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .column_spacing(1);

        table.render(inner, buf);
    }
}

impl<'a> Widget for DashboardView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Divide dashboard into 3 tiers matching btop:
        // Tier 1: CPU & System Panel (top ~33%)
        // Tier 2: GPU Panel (middle ~33%)
        // Tier 3: LLM Inference & Slots (bottom ~34%) split into 2 columns
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

        // 2. GPU Panel
        self.render_gpu_panel(rows[1], buf);

        // 3. LLM & Slots Row (2 columns)
        let tier3_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[2]);

        self.render_llm_panel(tier3_cols[0], buf);
        self.render_slots_panel(tier3_cols[1], buf);
    }
}
