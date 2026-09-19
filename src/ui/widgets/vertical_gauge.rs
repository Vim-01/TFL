use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// Thermal color thresholds strictly matching user specification:
/// 0..=70°C: Green
/// 71..=80°C: Yellow
/// 81..=84°C: Orange
/// 85°C+: Red
pub fn thermal_color(celsius: f32) -> Color {
    let c = if celsius.is_finite() { celsius } else { 0.0 };
    if c <= 70.0 {
        Color::Rgb(50, 230, 120) // Green
    } else if c <= 80.0 {
        Color::Rgb(255, 225, 90) // Yellow
    } else if c <= 84.0 {
        Color::Rgb(255, 150, 50) // Orange
    } else {
        Color::Rgb(255, 60, 60) // Red
    }
}

/// Dynamic power color gradient: dots from green to red based on % power cap
pub fn power_gradient_color(pct: f32) -> Color {
    let clamped_pct = if pct.is_finite() { pct.clamp(0.0, 100.0) } else { 0.0 };
    let p = clamped_pct / 100.0;
    if p < 0.5 {
        // Green (0, 230, 120) -> Yellow (255, 225, 90)
        let t = p * 2.0;
        let r = (50.0 + (255.0 - 50.0) * t) as u8;
        let g = (230.0 + (225.0 - 230.0) * t) as u8;
        let b = (120.0 + (90.0 - 120.0) * t) as u8;
        Color::Rgb(r, g, b)
    } else {
        // Yellow (255, 225, 90) -> Red (255, 60, 60)
        let t = (p - 0.5) * 2.0;
        let r = 255;
        let g = (225.0 + (60.0 - 225.0) * t) as u8;
        let b = (90.0 + (60.0 - 90.0) * t) as u8;
        Color::Rgb(r, g, b)
    }
}

/// Vertical column gauge that fills vertically from bottom to top
/// Uses smooth sub-block Unicode characters: ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'.
pub struct VerticalGauge<'a> {
    label: &'a str,
    value_text: &'a str,
    percent: f32,
    fill_color: Color,
    dim_color: Color,
}

impl<'a> VerticalGauge<'a> {
    pub fn new(label: &'a str, value_text: &'a str, percent: f32, fill_color: Color) -> Self {
        let percent = if percent.is_finite() { percent.clamp(0.0, 100.0) } else { 0.0 };
        Self {
            label,
            value_text,
            percent,
            fill_color,
            dim_color: Color::Rgb(100, 115, 140),
        }
    }

    pub fn dim_color(mut self, color: Color) -> Self {
        self.dim_color = color;
        self
    }
}

const VERTICAL_BLOCKS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

impl<'a> Widget for VerticalGauge<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Layout:
        // Top line: Label (if height >= 3)
        // Middle rows: Vertical column of blocks
        // Bottom line: Value text (e.g. "26%" or "40°")
        let show_label = area.height >= 3;
        let show_value = area.height >= 2;

        let col_start_y = if show_label { area.y + 1 } else { area.y };
        let col_end_y = if show_value {
            area.y + area.height - 1
        } else {
            area.y + area.height
        };
        let col_height = col_end_y.saturating_sub(col_start_y) as usize;

        if show_label {
            let label_x = area.x + (area.width.saturating_sub(self.label.len() as u16) / 2);
            buf.set_string(
                label_x,
                area.y,
                self.label,
                Style::default().fg(self.dim_color).add_modifier(Modifier::BOLD),
            );
        }

        if col_height > 0 {
            let total_steps = col_height * 8;
            let filled_steps = ((self.percent / 100.0) * total_steps as f32).round() as usize;

            for r in 0..col_height {
                // r = 0 is top of column, r = col_height - 1 is bottom of column
                let row_from_bottom = col_height - 1 - r;
                let step_floor = row_from_bottom * 8;

                let (ch, style) = if filled_steps >= step_floor + 8 {
                    // Fully filled row
                    ('█', Style::default().fg(self.fill_color))
                } else if filled_steps > step_floor {
                    // Fractional sub-block row
                    let fraction_idx = (filled_steps - step_floor).min(7);
                    (VERTICAL_BLOCKS[fraction_idx], Style::default().fg(self.fill_color))
                } else {
                    // Empty: subtle track dot or space (transparent)
                    ('·', Style::default().fg(self.dim_color))
                };

                let y = col_start_y + r as u16;
                // Center column horizontally in area
                let center_x = area.x + (area.width / 2);
                buf[(center_x, y)].set_char(ch).set_style(style);
            }
        }

        if show_value {
            let val_y = area.y + area.height - 1;
            let val_x = area.x + (area.width.saturating_sub(self.value_text.len() as u16) / 2);
            buf.set_string(
                val_x,
                val_y,
                self.value_text,
                Style::default().fg(self.fill_color).add_modifier(Modifier::BOLD),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_thermal_color_thresholds() {
        assert_eq!(thermal_color(50.0), Color::Rgb(50, 230, 120)); // Green
        assert_eq!(thermal_color(70.0), Color::Rgb(50, 230, 120)); // Green
        assert_eq!(thermal_color(75.0), Color::Rgb(255, 225, 90)); // Yellow
        assert_eq!(thermal_color(80.0), Color::Rgb(255, 225, 90)); // Yellow
        assert_eq!(thermal_color(82.0), Color::Rgb(255, 150, 50)); // Orange
        assert_eq!(thermal_color(84.0), Color::Rgb(255, 150, 50)); // Orange
        assert_eq!(thermal_color(85.0), Color::Rgb(255, 60, 60));  // Red
        assert_eq!(thermal_color(95.0), Color::Rgb(255, 60, 60));  // Red
    }

    #[test]
    fn test_vertical_gauge_render() {
        let gauge = VerticalGauge::new("FAN", "50%", 50.0, Color::Cyan);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 4));
        gauge.render(Rect::new(0, 0, 5, 4), &mut buf);

        // Should not panic, label and value should be written
        assert!(buf[(1, 0)].symbol() == "F" || buf[(0, 0)].symbol() == "F");
    }
}
