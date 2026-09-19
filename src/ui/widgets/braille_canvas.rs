use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Continuous Green -> Yellow -> Orange -> Red color gradient based on percentage (0..100)
pub fn value_gradient_color(pct: f64) -> Color {
    let p = if pct.is_finite() { pct.clamp(0.0, 100.0) / 100.0 } else { 0.0 };
    if p < 0.35 {
        // Pure green: #32E678
        Color::Rgb(50, 230, 120)
    } else if p < 0.60 {
        // Green (#32E678) -> Yellow (#FFE15A)
        let t = ((p - 0.35) / 0.25) as f32;
        let r = (50.0 + (255.0 - 50.0) * t) as u8;
        let g = (230.0 + (225.0 - 230.0) * t) as u8;
        let b = (120.0 + (90.0 - 120.0) * t) as u8;
        Color::Rgb(r, g, b)
    } else if p < 0.80 {
        // Yellow (#FFE15A) -> Orange (#FF9632)
        let t = ((p - 0.60) / 0.20) as f32;
        let r = 255;
        let g = (225.0 + (150.0 - 225.0) * t) as u8;
        let b = (90.0 + (50.0 - 90.0) * t) as u8;
        Color::Rgb(r, g, b)
    } else {
        // Orange (#FF9632) -> Red (#FF3C3C)
        let t = ((p - 0.80) / 0.20) as f32;
        let r = 255;
        let g = (150.0 + (60.0 - 150.0) * t) as u8;
        let b = (50.0 + (60.0 - 50.0) * t) as u8;
        Color::Rgb(r, g, b)
    }
}

/// 2D Braille Canvas Widget
/// Renders a full-height, full-width high-resolution historical graph using Unicode Braille patterns (U+2800..U+28FF).
/// - 2x horizontal dot resolution (2 dot columns per terminal cell)
/// - 4x vertical dot resolution (4 dot rows per terminal cell)
/// - Stretches / resamples across 100% of the container width (zero blank space on the left)
/// - At 0% / idle: renders a dotted baseline `⣀⣀⣀` across the entire bottom row
/// - At 1..100%: fills vertically from bottom to top across the full height of the block
/// - Dynamic Green -> Yellow -> Orange -> Red gradient based on percentage value
/// - Preserves terminal transparency: cells above the graph are completely untouched
pub struct BrailleCanvas<'a> {
    data: &'a [f64],
    max_value: f64,
    use_gradient: bool,
    baseline_color: Color,
}

impl<'a> BrailleCanvas<'a> {
    pub fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            max_value: 100.0,
            use_gradient: true,
            baseline_color: Color::Rgb(45, 115, 75), // Subtle dimmed green baseline
        }
    }

    pub fn max(mut self, max: f64) -> Self {
        if max.is_finite() && max > 0.0 {
            self.max_value = max;
        }
        self
    }

    pub fn baseline_color(mut self, color: Color) -> Self {
        self.baseline_color = color;
        self
    }

    pub fn use_gradient(mut self, enabled: bool) -> Self {
        self.use_gradient = enabled;
        self
    }
}

fn braille_dot_bit(sub_col: usize, sub_row: usize) -> u8 {
    match (sub_col, sub_row) {
        (0, 0) => 0x01, // Dot 1 (top left)
        (0, 1) => 0x02, // Dot 2 (mid-top left)
        (0, 2) => 0x04, // Dot 3 (mid-bottom left)
        (1, 0) => 0x08, // Dot 4 (top right)
        (1, 1) => 0x10, // Dot 5 (mid-top right)
        (1, 2) => 0x20, // Dot 6 (mid-bottom right)
        (0, 3) => 0x40, // Dot 7 (bottom left)
        (1, 3) => 0x80, // Dot 8 (bottom right)
        _ => 0,
    }
}

impl<'a> Widget for BrailleCanvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let total_dot_cols = width * 2;
        let total_dot_height = height * 4;

        if total_dot_cols == 0 || total_dot_height == 0 {
            return;
        }

        // Resample / stretch data across 100% of total_dot_cols to eliminate empty gaps on the left
        let mut dot_heights = vec![0_usize; total_dot_cols];
        let data_len = self.data.len();

        for (i, slot) in dot_heights.iter_mut().enumerate() {
            let val = if data_len == 0 {
                0.0
            } else if data_len == 1 {
                self.data[0]
            } else {
                let data_pos = (i as f64 / (total_dot_cols - 1) as f64) * (data_len - 1) as f64;
                let idx0 = (data_pos.floor() as usize).min(data_len - 1);
                let idx1 = (idx0 + 1).min(data_len - 1);
                let t = data_pos - idx0 as f64;
                let v0 = self.data[idx0];
                let v1 = self.data[idx1];
                v0 + (v1 - v0) * t
            };

            if val.is_finite() && val > 0.0 {
                let frac = (val / self.max_value).clamp(0.0, 1.0);
                let filled = (frac * total_dot_height as f64).round() as usize;
                *slot = filled.min(total_dot_height);
            }
        }

        // Render each cell in area
        for cell_y in 0..height {
            let buf_y = area.y + cell_y as u16;

            for cell_x in 0..width {
                let buf_x = area.x + cell_x as u16;

                let mut mask = 0_u8;
                let mut is_pure_baseline = true;
                let mut max_cell_frac: f64 = 0.0;

                for sub_col in 0..2 {
                    let dot_col = cell_x * 2 + sub_col;
                    let filled_dots = dot_heights[dot_col];

                    for sub_row in 0..4 {
                        let dot_y_from_top = cell_y * 4 + sub_row;
                        let dot_y_from_bottom = (total_dot_height - 1).saturating_sub(dot_y_from_top);

                        if filled_dots == 0 {
                            // Baseline across the entire bottom row of the canvas (100% width)
                            if cell_y == height - 1 && sub_row == 3 {
                                mask |= braille_dot_bit(sub_col, sub_row);
                            }
                        } else if dot_y_from_bottom < filled_dots {
                            mask |= braille_dot_bit(sub_col, sub_row);
                            is_pure_baseline = false;
                            let frac = (filled_dots as f64) / (total_dot_height as f64);
                            if frac > max_cell_frac {
                                max_cell_frac = frac;
                            }
                        }
                    }
                }

                if mask == 0 {
                    // Transparent: leave cell untouched to preserve native terminal transparency
                    continue;
                }

                let ch = char::from_u32(0x2800 + mask as u32).unwrap_or(' ');
                let color = if is_pure_baseline {
                    self.baseline_color
                } else if self.use_gradient {
                    value_gradient_color(max_cell_frac * 100.0)
                } else {
                    Color::Rgb(50, 230, 120)
                };

                let cell = &mut buf[(buf_x, buf_y)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(color));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_braille_canvas_empty_renders_baseline_full_width() {
        let data = [0.0; 5];
        let canvas = BrailleCanvas::new(&data);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));

        canvas.render(Rect::new(0, 0, 10, 2), &mut buf);

        // Top row should be untouched (space)
        assert_eq!(buf[(0, 0)].symbol(), " ");
        // Bottom row should have baseline braille dots '⣀' across the entire width
        for x in 0..10 {
            assert_eq!(buf[(x, 1)].symbol(), "⣀", "Column {} must have baseline dots", x);
        }
    }

    #[test]
    fn test_braille_canvas_full_fill() {
        let data = [100.0; 10];
        let canvas = BrailleCanvas::new(&data);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));

        canvas.render(Rect::new(0, 0, 5, 2), &mut buf);

        // Top and bottom should both have full braille '⣿' (code 0x28FF)
        assert_eq!(buf[(0, 0)].symbol(), "⣿");
        assert_eq!(buf[(0, 1)].symbol(), "⣿");
    }

    #[test]
    fn test_value_gradient_color_interpolation() {
        // Green at 0%
        assert_eq!(value_gradient_color(0.0), Color::Rgb(50, 230, 120));
        // Red at 100%
        assert_eq!(value_gradient_color(100.0), Color::Rgb(255, 60, 60));
    }
}
