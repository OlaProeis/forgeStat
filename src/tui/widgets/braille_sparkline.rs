use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

/// A sparkline widget that uses Unicode Braille patterns for 2x vertical resolution.
///
/// Braille patterns provide 4 vertical dots per character half (left/right),
/// allowing each character to represent 2 data points with 4 height levels each.
/// This effectively doubles the vertical resolution compared to block characters.
///
/// Unicode Braille range: U+2800 (blank) to U+28FF (all 8 dots)
/// Dot pattern: 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80
/// Layout:     1 4
///             2 5
///             3 6
///             7 8
#[derive(Debug, Clone, PartialEq)]
pub struct BrailleSparkline<'a> {
    /// The data to display
    data: &'a [u64],
    /// The style for rendering
    style: Style,
    /// Maximum value for scaling (defaults to max of data)
    max: Option<u64>,
}

impl<'a> BrailleSparkline<'a> {
    /// Create a new BrailleSparkline with the given data
    pub fn new(data: &'a [u64]) -> Self {
        Self {
            data,
            style: Style::default(),
            max: None,
        }
    }

    /// Set the style for the sparkline
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the maximum value for scaling
    pub fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Calculate the maximum value for scaling
    fn max_value(&self) -> u64 {
        self.max
            .or_else(|| self.data.iter().copied().max())
            .unwrap_or(1)
            .max(1) // Avoid division by zero
    }

    /// Convert a value (0.0 to 1.0) to a 4-level Braille pattern (0-4)
    fn value_to_level(value: f64) -> u8 {
        if value <= 0.0 {
            0
        } else if value < 0.25 {
            1
        } else if value < 0.5 {
            2
        } else if value < 0.75 {
            3
        } else {
            4
        }
    }

    /// Get the left column pattern (dots 1, 2, 3, 7) for a given level (0-4)
    fn left_pattern(level: u8) -> u8 {
        match level {
            0 => 0,    // Empty
            1 => 0x04, // Dot 3 only (bottom)
            2 => 0x06, // Dots 2, 3 (lower half)
            3 => 0x07, // Dots 1, 2, 3 (upper half)
            4 => 0x47, // Dots 1, 2, 3, 7 (full)
            _ => 0,
        }
    }

    /// Get the right column pattern (dots 4, 5, 6, 8) for a given level (0-4)
    fn right_pattern(level: u8) -> u8 {
        match level {
            0 => 0,    // Empty
            1 => 0x20, // Dot 6 only (bottom)
            2 => 0x30, // Dots 5, 6 (lower half)
            3 => 0x38, // Dots 4, 5, 6 (upper half)
            4 => 0xB8, // Dots 4, 5, 6, 8 (full)
            _ => 0,
        }
    }

    /// Resample data to target width using linear interpolation
    fn resample_to_width(&self, target_width: usize) -> Vec<f64> {
        if self.data.is_empty() || target_width == 0 {
            return Vec::new();
        }

        let data_len = self.data.len();
        let max_val = self.max_value() as f64;

        if data_len >= target_width {
            // Downsample by averaging
            let bucket_width = data_len as f64 / target_width as f64;
            let mut result = Vec::with_capacity(target_width);

            for i in 0..target_width {
                let start_idx = (i as f64 * bucket_width) as usize;
                let end_idx = ((i + 1) as f64 * bucket_width) as usize;
                let end_idx = end_idx.min(data_len);

                let avg = if start_idx < end_idx {
                    self.data[start_idx..end_idx].iter().sum::<u64>() as f64
                        / (end_idx - start_idx) as f64
                } else {
                    self.data[start_idx.min(data_len - 1)] as f64
                };
                result.push(avg / max_val);
            }
            result
        } else {
            // Upsample by linear interpolation
            let mut result = Vec::with_capacity(target_width);
            let scale = (data_len - 1) as f64 / (target_width - 1).max(1) as f64;

            for i in 0..target_width {
                let exact_idx = i as f64 * scale;
                let idx_low = exact_idx.floor() as usize;
                let idx_high = exact_idx.ceil() as usize;
                let frac = exact_idx.fract();

                let low_val = self.data[idx_low.min(data_len - 1)] as f64;
                let high_val = self.data[idx_high.min(data_len - 1)] as f64;

                let interpolated = low_val + frac * (high_val - low_val);
                result.push(interpolated / max_val);
            }
            result
        }
    }

    /// Render the Braille sparkline to the buffer
    fn render_braille(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let max_val = self.max_value();
        if max_val == 0 || self.data.is_empty() {
            return;
        }

        // Each Braille character represents 2 data points horizontally
        // So we can show 2*width data points across the available width
        let target_data_points = (area.width as usize) * 2;
        let resampled = self.resample_to_width(target_data_points);

        if resampled.is_empty() {
            return;
        }

        // Process data in pairs (each Braille char = 2 data points)
        for (col, chunk) in resampled.chunks(2).enumerate() {
            if col >= area.width as usize {
                break;
            }

            let left_level = Self::value_to_level(chunk[0]);
            let right_level = if chunk.len() > 1 {
                Self::value_to_level(chunk[1])
            } else {
                0
            };

            let pattern = Self::left_pattern(left_level) | Self::right_pattern(right_level);
            let braille_char = char::from_u32(0x2800 + pattern as u32).unwrap_or(' ');

            // Draw vertically centered in the available height
            let row = area.y + (area.height / 2);
            if row < area.y + area.height {
                let x = area.x + col as u16;
                if let Some(cell) = buf.cell_mut((x, row)) {
                    cell.set_char(braille_char);
                    cell.set_style(self.style);
                }
            }
        }
    }
}

impl<'a> Widget for BrailleSparkline<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_braille(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_to_level() {
        assert_eq!(BrailleSparkline::value_to_level(0.0), 0);
        assert_eq!(BrailleSparkline::value_to_level(0.1), 1);
        assert_eq!(BrailleSparkline::value_to_level(0.24), 1);
        assert_eq!(BrailleSparkline::value_to_level(0.25), 2);
        assert_eq!(BrailleSparkline::value_to_level(0.49), 2);
        assert_eq!(BrailleSparkline::value_to_level(0.5), 3);
        assert_eq!(BrailleSparkline::value_to_level(0.74), 3);
        assert_eq!(BrailleSparkline::value_to_level(0.75), 4);
        assert_eq!(BrailleSparkline::value_to_level(1.0), 4);
    }

    #[test]
    fn test_left_pattern() {
        assert_eq!(BrailleSparkline::left_pattern(0), 0);
        assert_eq!(BrailleSparkline::left_pattern(1), 0x04); // dot 3
        assert_eq!(BrailleSparkline::left_pattern(2), 0x06); // dots 2, 3
        assert_eq!(BrailleSparkline::left_pattern(3), 0x07); // dots 1, 2, 3
        assert_eq!(BrailleSparkline::left_pattern(4), 0x47); // dots 1, 2, 3, 7
    }

    #[test]
    fn test_right_pattern() {
        assert_eq!(BrailleSparkline::right_pattern(0), 0);
        assert_eq!(BrailleSparkline::right_pattern(1), 0x20); // dot 6
        assert_eq!(BrailleSparkline::right_pattern(2), 0x30); // dots 5, 6
        assert_eq!(BrailleSparkline::right_pattern(3), 0x38); // dots 4, 5, 6
        assert_eq!(BrailleSparkline::right_pattern(4), 0xB8); // dots 4, 5, 6, 8
    }

    #[test]
    fn test_resample_empty() {
        let sparkline = BrailleSparkline::new(&[]);
        let result = sparkline.resample_to_width(10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resample_upsample() {
        let data = vec![0u64, 50, 100];
        let sparkline = BrailleSparkline::new(&data);
        let result = sparkline.resample_to_width(5);
        assert_eq!(result.len(), 5);
        // First and last should match original
        assert!((result[0] - 0.0).abs() < 0.01);
        assert!((result[4] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resample_downsample() {
        let data: Vec<u64> = (0..100).collect();
        let sparkline = BrailleSparkline::new(&data);
        let result = sparkline.resample_to_width(10);
        assert_eq!(result.len(), 10);
        // Values should be in ascending order
        for i in 1..result.len() {
            assert!(result[i] >= result[i - 1] || (result[i] - result[i - 1]).abs() < 0.1);
        }
    }

    #[test]
    fn test_braille_unicode_range() {
        // Verify we can create valid Braille characters
        for pattern in [0u8, 0x04, 0x06, 0x07, 0x47, 0x20, 0x30, 0x38, 0xB8] {
            let char_code = 0x2800 + pattern as u32;
            let ch = char::from_u32(char_code);
            assert!(
                ch.is_some(),
                "Failed to create Braille char for pattern {}",
                pattern
            );
        }
    }
}
