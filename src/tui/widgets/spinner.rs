use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

/// A rotating Braille spinner widget for showing loading/progress states.
///
/// Uses Unicode Braille patterns for smooth animation:
/// ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrailleSpinner {
    /// Current frame of the animation (0-9)
    frame: usize,
    /// Style for rendering
    style: Style,
}

impl BrailleSpinner {
    /// The sequence of Braille characters for the spinner animation
    const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    /// Create a new spinner starting at frame 0
    pub fn new() -> Self {
        Self {
            frame: 0,
            style: Style::default(),
        }
    }

    /// Create a spinner at a specific frame
    pub fn at_frame(frame: usize) -> Self {
        Self {
            frame: frame % Self::FRAMES.len(),
            style: Style::default(),
        }
    }

    /// Set the style for the spinner
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Advance to the next frame
    pub fn next_frame(&mut self) {
        self.frame = (self.frame + 1) % Self::FRAMES.len();
    }

    /// Set the frame directly
    pub fn set_frame(&mut self, frame: usize) {
        self.frame = frame % Self::FRAMES.len();
    }

    /// Get the current frame index
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Get the current character being displayed
    pub fn current_char(&self) -> char {
        Self::FRAMES[self.frame]
    }

    /// Get the total number of frames
    pub fn frame_count() -> usize {
        Self::FRAMES.len()
    }
}

impl Default for BrailleSpinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for BrailleSpinner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Center the spinner in the available area
        let x = area.x + area.width / 2;
        let y = area.y + area.height / 2;

        // Render the current Braille character
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(self.current_char());
            cell.set_style(self.style);
        }
    }
}

/// A widget that displays an animated counter with count-up effect.
/// Shows the current value with optional target value for animation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedCounter {
    /// Current displayed value
    current: u64,
    /// Target value (final value to count up to)
    target: u64,
    /// Style for rendering
    style: Style,
    /// Whether the animation is complete
    complete: bool,
}

impl AnimatedCounter {
    /// Create a new counter starting at 0, counting to target
    pub fn new(target: u64) -> Self {
        Self {
            current: 0,
            target,
            style: Style::default(),
            complete: target == 0,
        }
    }

    /// Create a counter with initial value
    pub fn with_current(target: u64, current: u64) -> Self {
        Self {
            current,
            target,
            style: Style::default(),
            complete: current >= target,
        }
    }

    /// Set the style for the counter
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Update the counter by one step
    /// Returns true if animation is still in progress
    pub fn step(&mut self) -> bool {
        if self.complete {
            return false;
        }

        // Calculate how much to increment based on remaining distance
        let remaining = self.target.saturating_sub(self.current);
        let increment = (remaining / 10).max(1).min(remaining);

        self.current += increment;

        if self.current >= self.target {
            self.current = self.target;
            self.complete = true;
        }

        !self.complete
    }

    /// Update the counter to a specific progress (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: f64) {
        let clamped = progress.clamp(0.0, 1.0);
        self.current = (self.target as f64 * clamped) as u64;
        self.complete = self.current >= self.target;
    }

    /// Get the current value
    pub fn current(&self) -> u64 {
        self.current
    }

    /// Get the target value
    pub fn target(&self) -> u64 {
        self.target
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Reset the counter to start from 0 again
    pub fn reset(&mut self) {
        self.current = 0;
        self.complete = self.target == 0;
    }

    /// Update the target value and reset animation
    pub fn set_target(&mut self, target: u64) {
        self.target = target;
        self.reset();
    }
}

impl Widget for AnimatedCounter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let text = format!("{}", self.current);
        let text_len = text.len() as u16;

        // Right-align the number in the area
        let x = area.x + area.width.saturating_sub(text_len).min(area.width - 1);
        let y = area.y;

        // Write each character
        for (i, ch) in text.chars().enumerate() {
            let col = x + i as u16;
            if col < area.x + area.width {
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_char(ch);
                    cell.set_style(self.style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_frames() {
        let spinner = BrailleSpinner::new();
        assert_eq!(spinner.frame(), 0);
        assert_eq!(spinner.current_char(), '⠋');
    }

    #[test]
    fn test_spinner_next_frame() {
        let mut spinner = BrailleSpinner::new();
        spinner.next_frame();
        assert_eq!(spinner.frame(), 1);
        assert_eq!(spinner.current_char(), '⠙');
    }

    #[test]
    fn test_spinner_wraps_around() {
        let mut spinner = BrailleSpinner::at_frame(9);
        assert_eq!(spinner.current_char(), '⠏');
        spinner.next_frame();
        assert_eq!(spinner.frame(), 0);
        assert_eq!(spinner.current_char(), '⠋');
    }

    #[test]
    fn test_spinner_set_frame() {
        let mut spinner = BrailleSpinner::new();
        spinner.set_frame(5);
        assert_eq!(spinner.current_char(), '⠴');
    }

    #[test]
    fn test_spinner_frame_count() {
        assert_eq!(BrailleSpinner::frame_count(), 10);
    }

    #[test]
    fn test_counter_new() {
        let counter = AnimatedCounter::new(100);
        assert_eq!(counter.current(), 0);
        assert_eq!(counter.target(), 100);
        assert!(!counter.is_complete());
    }

    #[test]
    fn test_counter_step() {
        let mut counter = AnimatedCounter::new(100);
        let still_running = counter.step();
        assert!(still_running);
        assert!(counter.current() > 0);
        assert!(counter.current() < 100);
    }

    #[test]
    fn test_counter_completes() {
        let mut counter = AnimatedCounter::new(10);
        // Run until complete
        let mut steps = 0;
        while counter.step() && steps < 100 {
            steps += 1;
        }
        assert!(counter.is_complete());
        assert_eq!(counter.current(), 10);
    }

    #[test]
    fn test_counter_set_progress() {
        let mut counter = AnimatedCounter::new(100);
        counter.set_progress(0.5);
        assert_eq!(counter.current(), 50);
        counter.set_progress(1.0);
        assert_eq!(counter.current(), 100);
        assert!(counter.is_complete());
    }

    #[test]
    fn test_counter_reset() {
        let mut counter = AnimatedCounter::with_current(100, 100);
        assert!(counter.is_complete());
        counter.reset();
        assert_eq!(counter.current(), 0);
        assert!(!counter.is_complete());
    }

    #[test]
    fn test_counter_set_target() {
        let mut counter = AnimatedCounter::with_current(100, 100);
        counter.set_target(200);
        assert_eq!(counter.target(), 200);
        assert_eq!(counter.current(), 0);
        assert!(!counter.is_complete());
    }
}
