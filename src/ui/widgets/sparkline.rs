//! Mini sparkline widget for inline metrics
#![allow(dead_code)]

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// A compact inline sparkline (single line)
pub struct MiniSparkline<'a> {
    data: &'a [u64],
    max: Option<u64>,
    style: Style,
    bar_chars: [char; 8],
}

impl<'a> MiniSparkline<'a> {
    pub fn new(data: &'a [u64]) -> Self {
        Self {
            data,
            max: None,
            style: Style::default().fg(Color::Cyan),
            bar_chars: ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'],
        }
    }

    pub fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for MiniSparkline<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.data.is_empty() {
            return;
        }

        let max = self.max.unwrap_or_else(|| *self.data.iter().max().unwrap_or(&1));
        let max = max.max(1); // Avoid division by zero

        // Take the last N values that fit in the area
        let data_len = self.data.len().min(area.width as usize);
        let data_start = self.data.len().saturating_sub(data_len);

        for (i, &value) in self.data[data_start..].iter().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }

            // Scale value to 0-7 range
            let scaled = if max > 0 {
                ((value as f64 / max as f64) * 7.0).round() as usize
            } else {
                0
            };
            let scaled = scaled.min(7);

            let ch = self.bar_chars[scaled];
            buf.get_mut(x, area.y).set_char(ch).set_style(self.style);
        }
    }
}

/// Format sparkline data as inline text (for status messages)
pub fn sparkline_text(data: &[u64], width: usize) -> String {
    if data.is_empty() {
        return String::new();
    }

    let bar_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = *data.iter().max().unwrap_or(&1);
    let max = max.max(1);

    let data_len = data.len().min(width);
    let data_start = data.len().saturating_sub(data_len);

    data[data_start..]
        .iter()
        .map(|&value| {
            let scaled = if max > 0 {
                ((value as f64 / max as f64) * 7.0).round() as usize
            } else {
                0
            };
            bar_chars[scaled.min(7)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_text() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let text = sparkline_text(&data, 8);
        assert_eq!(text.chars().count(), 8);
    }

    #[test]
    fn test_sparkline_text_empty() {
        let data: [u64; 0] = [];
        let text = sparkline_text(&data, 8);
        assert!(text.is_empty());
    }
}
