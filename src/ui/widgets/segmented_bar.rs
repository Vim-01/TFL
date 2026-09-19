use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// Widget that displays a multi-segment horizontal bar (e.g. Weights, KV Cache, Context, Free)
pub struct SegmentedBar<'a> {
    label: &'a str,
    total_label: &'a str,
    segments: Vec<(&'a str, u64, Color)>,
    total_bytes: u64,
}

impl<'a> SegmentedBar<'a> {
    pub fn new(label: &'a str, total_label: &'a str, total_bytes: u64) -> Self {
        Self {
            label,
            total_label,
            segments: Vec::new(),
            total_bytes,
        }
    }

    pub fn segment(mut self, name: &'a str, bytes: u64, color: Color) -> Self {
        self.segments.push((name, bytes, color));
        self
    }
}

impl<'a> Widget for SegmentedBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 2 {
            return;
        }

        // Line 1: Header and total
        let header_str = format!("{} ", self.label);
        buf.set_string(
            area.x,
            area.y,
            &header_str,
            Style::default().add_modifier(Modifier::BOLD),
        );

        let total_str = format!(" {}", self.total_label);
        let total_x = area.x + area.width.saturating_sub(total_str.len() as u16);
        buf.set_string(
            total_x,
            area.y,
            &total_str,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );

        // Bar calculation
        let bar_start_x = area.x + header_str.len() as u16;
        let bar_width = total_x.saturating_sub(bar_start_x);
        if bar_width == 0 || self.total_bytes == 0 {
            return;
        }

        let mut current_x = bar_start_x;
        for &(_, bytes, color) in &self.segments {
            let seg_width = ((bytes as f64 / self.total_bytes as f64) * bar_width as f64).round() as u16;
            let end_x = (current_x + seg_width).min(total_x);
            for x in current_x..end_x {
                buf.set_string(x, area.y, "█", Style::default().fg(color));
            }
            current_x = end_x;
        }
        // Fill remaining with dark track
        for x in current_x..total_x {
            buf.set_string(x, area.y, "░", Style::default().fg(Color::DarkGray));
        }

        // Line 2: Legend
        if area.height >= 2 {
            let mut legend_x = area.x;
            for &(name, bytes, color) in &self.segments {
                let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let item_str = format!("■ {}: {:.1}G  ", name, gib);
                if legend_x + (item_str.len() as u16) <= area.x + area.width {
                    buf.set_string(legend_x, area.y + 1, "■ ", Style::default().fg(color));
                    buf.set_string(
                        legend_x + 2,
                        area.y + 1,
                        format!("{}: {:.1}G  ", name, gib),
                        Style::default().fg(Color::Gray),
                    );
                    legend_x += item_str.len() as u16;
                }
            }
        }
    }
}
