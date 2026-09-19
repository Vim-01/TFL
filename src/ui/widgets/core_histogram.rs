use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// Widget rendering per-core CPU load as a vertical histogram with Y-axis markers and gridlines.
pub struct CoreHistogram<'a> {
    core_usages: &'a [f32],
    avg_freq_ghz: f32,
    global_usage: f32,
}

impl<'a> CoreHistogram<'a> {
    pub fn new(core_usages: &'a [f32], avg_freq_ghz: f32, global_usage: f32) -> Self {
        Self {
            core_usages,
            avg_freq_ghz,
            global_usage,
        }
    }
}

impl<'a> Widget for CoreHistogram<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 15 || area.height < 6 {
            return;
        }

        // Top info line: e.g. "Total: 14% | Avg Clock: 3.80 GHz | Cores: 12"
        let title_line = format!(
            "Total: {:.0}%  |  Avg: {:.2} GHz  |  Cores: {}",
            self.global_usage,
            self.avg_freq_ghz,
            self.core_usages.len()
        );
        buf.set_string(
            area.x + 1,
            area.y,
            &title_line,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );

        // Define chart boundaries
        let chart_top = area.y + 1;
        let chart_bottom = area.y + area.height - 2; // reserve 1 line for core indices
        let chart_height = chart_bottom.saturating_sub(chart_top);
        if chart_height < 3 {
            return;
        }

        let y_axis_width: u16 = 6;
        let y_axis_x = area.x;
        let plot_start_x = y_axis_x + y_axis_width;
        let plot_width = area.width.saturating_sub(y_axis_width + 1);

        // Draw Y-axis labels and subtle horizontal guide lines
        let steps = [
            (100.0, "100%"),
            (75.0, " 75%"),
            (50.0, " 50%"),
            (25.0, " 25%"),
            (0.0, "  0%"),
        ];

        for &(val, label) in &steps {
            let row_offset = (((100.0 - val) / 100.0) * (chart_height - 1) as f32).round() as u16;
            let row_y = chart_top + row_offset;
            if row_y <= chart_bottom {
                buf.set_string(
                    y_axis_x,
                    row_y,
                    label,
                    Style::default().fg(Color::DarkGray),
                );
                // Horizontal guideline
                for gx in plot_start_x..(plot_start_x + plot_width) {
                    buf.set_string(gx, row_y, "─", Style::default().fg(Color::Rgb(40, 44, 52)));
                }
            }
        }

        // Draw vertical columns for each core
        let num_cores = self.core_usages.len();
        if num_cores == 0 {
            return;
        }

        // Calculate column width and spacing
        let total_avail = plot_width as usize;
        let col_width = (total_avail / num_cores).max(1);
        let bar_width = if col_width >= 3 { col_width - 1 } else { col_width };

        for (idx, &usage) in self.core_usages.iter().enumerate() {
            let col_x = plot_start_x as usize + idx * col_width;
            if col_x + bar_width > (plot_start_x + plot_width) as usize {
                break;
            }

            let usage_clamped = if usage.is_finite() { usage.clamp(0.0, 100.0) } else { 0.0 };
            let bar_height_exact = (usage_clamped / 100.0) * (chart_height as f32);
            let full_blocks = bar_height_exact.floor() as u16;
            let partial_frac = bar_height_exact - bar_height_exact.floor();

            // Color gradient depending on load
            let color = if usage_clamped < 40.0 {
                Color::Rgb(70, 130, 180) // Steel blue / teal
            } else if usage_clamped < 75.0 {
                Color::Rgb(220, 170, 60) // Amber / yellow
            } else {
                Color::Rgb(230, 70, 90) // Coral / red
            };

            // Draw vertical bar from bottom upwards
            for h in 0..chart_height {
                let y = chart_bottom - h;
                if h < full_blocks {
                    for bx in 0..bar_width {
                        buf.set_string(
                            col_x as u16 + bx as u16,
                            y,
                            "█",
                            Style::default().fg(color),
                        );
                    }
                } else if h == full_blocks && partial_frac > 0.15 {
                    let symbol = if partial_frac < 0.4 {
                        "▄"
                    } else if partial_frac < 0.7 {
                        "▅"
                    } else {
                        "▇"
                    };
                    for bx in 0..bar_width {
                        buf.set_string(
                            col_x as u16 + bx as u16,
                            y,
                            symbol,
                            Style::default().fg(color),
                        );
                    }
                }
            }

            // Draw core index number underneath
            let label_y = chart_bottom + 1;
            if label_y < area.y + area.height {
                let idx_str = format!("{idx}");
                let text_x = col_x as u16 + (bar_width as u16 / 2);
                buf.set_string(
                    text_x,
                    label_y,
                    &idx_str,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }
}
