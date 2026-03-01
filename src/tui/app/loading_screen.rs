use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*};

use crate::tui::widgets::BrailleSpinner;

/// Represents the progress of fetching a repository snapshot
#[derive(Debug, Clone)]
pub struct FetchProgress {
    /// Total number of endpoints to fetch
    pub total: usize,
    /// Number of completed endpoints
    pub completed: usize,
    /// Name of the endpoint currently being fetched (if any)
    pub current_endpoint: Option<String>,
    /// Whether the fetch is complete
    pub done: bool,
    /// Any error that occurred
    pub error: Option<String>,
    /// Total star count of the repository (for detecting large repos)
    pub star_count: Option<u64>,
    /// Current page being fetched (for paginated endpoints like stargazers)
    pub current_page: Option<u32>,
    /// Total estimated pages (for paginated endpoints)
    pub total_pages: Option<u32>,
}

impl FetchProgress {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            current_endpoint: None,
            done: false,
            error: None,
            star_count: None,
            current_page: None,
            total_pages: None,
        }
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.completed as f64 / self.total as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        self.done || self.completed >= self.total
    }

    /// Check if this is a large repo that will take a while
    pub fn is_large_repo(&self) -> bool {
        match self.star_count {
            Some(stars) => stars > 5_000,
            None => false,
        }
    }

    /// Get status text showing page progress for paginated endpoints
    pub fn get_status_text(&self) -> String {
        if let Some(ref endpoint) = self.current_endpoint {
            // Show page progress for Star History
            if endpoint == "Star History" {
                if let (Some(current), Some(total)) = (self.current_page, self.total_pages) {
                    if self.is_large_repo() {
                        return format!(
                            "Fetching {} (page {}/{} of {} stars)...",
                            endpoint,
                            current,
                            total,
                            format_stars(self.star_count.unwrap_or(0))
                        );
                    } else {
                        return format!("Fetching {} (page {}/{})...", endpoint, current, total);
                    }
                }
            }
            format!("Fetching {}...", endpoint)
        } else if self.is_complete() {
            "Complete!".to_string()
        } else if self.error.is_some() {
            "Error occurred".to_string()
        } else {
            "Waiting...".to_string()
        }
    }
}

/// A simple Pong game for the loading screen
#[derive(Debug)]
pub struct PongGame {
    /// Ball position (x, y) in character coordinates (0-100, 0-30)
    ball: (f32, f32),
    /// Ball velocity (dx, dy)
    ball_velocity: (f32, f32),
    /// Left paddle position (y center, height 4)
    left_paddle: f32,
    /// Right paddle position (y center, height 4) - AI controlled
    right_paddle: f32,
    /// Player score
    player_score: u32,
    /// AI score
    ai_score: u32,
    /// Game area width
    width: u16,
    /// Game area height
    height: u16,
    /// Last update time for game physics
    last_update: Instant,
    /// Physics update interval (60 fps)
    update_interval: Duration,
}

impl PongGame {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            ball: (width as f32 / 2.0, height as f32 / 2.0),
            ball_velocity: (0.6, 0.4), // Slower ball speed for better playability
            left_paddle: height as f32 / 2.0,
            right_paddle: height as f32 / 2.0,
            player_score: 0,
            ai_score: 0,
            width,
            height,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(16), // ~60 fps
        }
    }

    /// Move player paddle up
    pub fn move_up(&mut self) {
        self.left_paddle = (self.left_paddle - 1.5).max(1.5);
    }

    /// Move player paddle down
    pub fn move_down(&mut self) {
        self.left_paddle = (self.left_paddle + 1.5).min(self.height as f32 - 1.5);
    }

    /// Update game physics
    pub fn update(&mut self) -> bool {
        if self.last_update.elapsed() < self.update_interval {
            return false;
        }
        self.last_update = Instant::now();

        // Move ball
        self.ball.0 += self.ball_velocity.0;
        self.ball.1 += self.ball_velocity.1;

        // Ball collision with top/bottom walls
        if self.ball.1 <= 0.0 || self.ball.1 >= self.height as f32 - 1.0 {
            self.ball_velocity.1 = -self.ball_velocity.1;
            self.ball.1 = self.ball.1.clamp(0.5, self.height as f32 - 1.5);
        }

        // Paddle dimensions - paddle is a single character wide (█)
        let paddle_half_height = 2.0; // Paddle is 4 chars tall
        let paddle_width = 0.5; // Paddle is 1 char wide (0.5 to each side of center)

        // Left paddle X position (where the paddle is drawn at column 2)
        let left_paddle_x = 2.0;
        // Right paddle X position (drawn at width - 3)
        let right_paddle_x = self.width as f32 - 3.0;

        // Ball collision with left paddle (player)
        // Ball must be approaching from the right (velocity < 0) and close to paddle X
        let ball_approaching_left = self.ball_velocity.0 < 0.0;
        // Check X: ball should be very close to paddle surface (within 0.6 chars)
        let ball_at_left_paddle =
            self.ball.0 >= left_paddle_x - 0.6 && self.ball.0 <= left_paddle_x + 0.6;
        // Check Y: ball should be within paddle's vertical range
        let ball_in_left_paddle_y = self.ball.1 >= self.left_paddle - paddle_half_height - 0.5
            && self.ball.1 <= self.left_paddle + paddle_half_height + 0.5;

        if ball_approaching_left && ball_at_left_paddle && ball_in_left_paddle_y {
            // Reverse ball direction and add spin
            self.ball_velocity.0 = self.ball_velocity.0.abs() * 1.02;
            // Add spin based on where ball hit the paddle
            let hit_offset = (self.ball.1 - self.left_paddle) / paddle_half_height;
            self.ball_velocity.1 += hit_offset * 0.3;
            // Push ball just outside paddle surface
            self.ball.0 = left_paddle_x + paddle_width + 0.1;
        }

        // Ball collision with right paddle (AI)
        let ball_approaching_right = self.ball_velocity.0 > 0.0;
        let ball_at_right_paddle =
            self.ball.0 >= right_paddle_x - 0.6 && self.ball.0 <= right_paddle_x + 0.6;
        let ball_in_right_paddle_y = self.ball.1 >= self.right_paddle - paddle_half_height - 0.5
            && self.ball.1 <= self.right_paddle + paddle_half_height + 0.5;

        if ball_approaching_right && ball_at_right_paddle && ball_in_right_paddle_y {
            self.ball_velocity.0 = -self.ball_velocity.0.abs() * 1.02;
            let hit_offset = (self.ball.1 - self.right_paddle) / paddle_half_height;
            self.ball_velocity.1 += hit_offset * 0.3;
            // Push ball just outside paddle surface
            self.ball.0 = right_paddle_x - paddle_width - 0.1;
        }

        // AI paddle movement (follow ball with some delay/reaction time)
        let target_y = self.ball.1;
        let diff = target_y - self.right_paddle;
        if diff.abs() > 0.3 {
            // AI moves slightly slower than player for fairness
            let ai_speed = 0.6;
            self.right_paddle += diff.signum() * ai_speed;
            self.right_paddle = self
                .right_paddle
                .clamp(paddle_half_height, self.height as f32 - paddle_half_height);
        }

        // Scoring
        if self.ball.0 < 0.0 {
            // AI scores
            self.ai_score += 1;
            self.reset_ball();
        } else if self.ball.0 > self.width as f32 {
            // Player scores
            self.player_score += 1;
            self.reset_ball();
        }

        // Clamp ball velocity to prevent too fast movement
        self.ball_velocity.0 = self.ball_velocity.0.clamp(-1.5, 1.5);
        self.ball_velocity.1 = self.ball_velocity.1.clamp(-1.2, 1.2);

        true
    }

    fn reset_ball(&mut self) {
        self.ball = (self.width as f32 / 2.0, self.height as f32 / 2.0);
        // Randomize starting direction
        let direction = if rand::random::<bool>() { 1.0 } else { -1.0 };
        self.ball_velocity = (direction * 0.6, (rand::random::<f32>() - 0.5) * 0.6);
    }

    /// Render the game
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear game area
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        // Draw center line
        for y in area.y..area.y + area.height {
            if y % 2 == 0 {
                frame.buffer_mut()[(area.x + area.width / 2, y)]
                    .set_char('│')
                    .set_fg(Color::DarkGray);
            }
        }

        // Draw left paddle (player)
        let paddle_height = 4;
        let left_paddle_y = (self.left_paddle - paddle_height as f32 / 2.0).round() as u16;
        for i in 0..paddle_height {
            let y = area.y + left_paddle_y + i;
            if y < area.y + area.height {
                frame.buffer_mut()[(area.x + 2, y)]
                    .set_char('█')
                    .set_fg(Color::Green);
            }
        }

        // Draw right paddle (AI)
        let right_paddle_y = (self.right_paddle - paddle_height as f32 / 2.0).round() as u16;
        for i in 0..paddle_height {
            let y = area.y + right_paddle_y + i;
            if y < area.y + area.height {
                frame.buffer_mut()[(area.x + area.width - 3, y)]
                    .set_char('█')
                    .set_fg(Color::Red);
            }
        }

        // Draw ball
        let ball_x =
            (area.x as f32 + (self.ball.0 / self.width as f32) * area.width as f32).round() as u16;
        let ball_y = (area.y as f32 + (self.ball.1 / self.height as f32) * area.height as f32)
            .round() as u16;
        if ball_x >= area.x
            && ball_x < area.x + area.width
            && ball_y >= area.y
            && ball_y < area.y + area.height
        {
            frame.buffer_mut()[(ball_x, ball_y)]
                .set_char('●')
                .set_fg(Color::White);
        }

        // Draw scores
        let score_text = format!("{} - {}", self.player_score, self.ai_score);
        let score_x = area.x + (area.width - score_text.len() as u16) / 2;
        for (i, ch) in score_text.chars().enumerate() {
            let x = score_x + i as u16;
            if x >= area.x && x < area.x + area.width {
                frame.buffer_mut()[(x, area.y)]
                    .set_char(ch)
                    .set_fg(Color::Yellow);
            }
        }
    }
}

/// A loading screen that displays progress while fetching repository data
pub struct LoadingScreen {
    spinner: BrailleSpinner,
    last_spinner_update: Instant,
    spinner_interval: Duration,
    owner: String,
    repo: String,
    /// Optional Pong game for large repos
    pong_game: Option<PongGame>,
    /// Whether to show the game
    show_game: bool,
    /// Animation time for visual effects (increments each tick)
    animation_time: f32,
    /// Background stars for twinkle effect
    bg_stars: Vec<BackgroundStar>,
}

/// A single twinkling star in the background
#[derive(Debug, Clone)]
struct BackgroundStar {
    x: u16,
    y: u16,
    phase: f32, // Animation phase offset (0-2π)
    speed: f32, // Twinkle speed
    char: char, // Character to display (░, ▒, ▓, or ·)
}

impl BackgroundStar {
    /// Calculate current brightness based on time
    fn brightness(&self, time: f32) -> u8 {
        let t = time * self.speed + self.phase;
        let sine = t.sin();
        // Map -1..1 to 20..80 (subtle, dark range)
        ((sine + 1.0) * 30.0 + 20.0) as u8
    }
}

/// Generate random background stars within an area
fn generate_stars(area: Rect, count: usize) -> Vec<BackgroundStar> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars = ['░', '▒', '▓', '·', ':', '.'];

    (0..count)
        .map(|_| BackgroundStar {
            x: rng.gen_range(area.x..area.x + area.width),
            y: rng.gen_range(area.y..area.y + area.height),
            phase: rng.gen_range(0.0..std::f32::consts::TAU),
            speed: rng.gen_range(0.5..2.0),
            char: chars[rng.gen_range(0..chars.len())],
        })
        .collect()
}

impl LoadingScreen {
    pub fn new(owner: String, repo: String) -> Self {
        Self {
            spinner: BrailleSpinner::new(),
            last_spinner_update: Instant::now(),
            spinner_interval: Duration::from_millis(80),
            owner,
            repo,
            pong_game: None,
            show_game: false,
            animation_time: 0.0,
            bg_stars: Vec::new(), // Will be initialized on first render when we know the area
        }
    }

    /// Update the spinner animation
    pub fn tick(&mut self) -> bool {
        // Update spinner frame
        if self.last_spinner_update.elapsed() >= self.spinner_interval {
            self.spinner.next_frame();
            self.last_spinner_update = Instant::now();
        }

        // Increment animation time (for visual effects)
        self.animation_time += 0.05;

        // Update game if active
        if let Some(ref mut game) = self.pong_game {
            game.update();
        }

        // Always return true because we have continuous animations
        // (twinkling stars and pulsing border glow)
        true
    }

    /// Handle keyboard input for the game
    pub fn handle_input(&mut self) {
        if let Ok(true) = event::poll(Duration::from_millis(0)) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Up => {
                            if let Some(ref mut game) = self.pong_game {
                                game.move_up();
                            }
                        }
                        KeyCode::Down => {
                            if let Some(ref mut game) = self.pong_game {
                                game.move_down();
                            }
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => {
                            // Toggle game on/off
                            self.show_game = !self.show_game;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Render the loading screen
    pub fn render(&mut self, frame: &mut Frame, progress: &FetchProgress) {
        let area = frame.area();

        // Initialize background stars on first render if not already done
        if self.bg_stars.is_empty() && area.width > 0 && area.height > 0 {
            // Generate stars only in the outer margins (not over the main content)
            self.bg_stars = generate_stars(area, 40);
        }

        // Clear the screen with a dark background
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        // Draw background twinkling stars
        for star in &self.bg_stars {
            let brightness = star.brightness(self.animation_time);
            // Only draw if within bounds and not too dark
            if brightness > 25
                && star.x >= area.x
                && star.x < area.x + area.width
                && star.y >= area.y
                && star.y < area.y + area.height
            {
                frame.buffer_mut()[(star.x, star.y)]
                    .set_char(star.char)
                    .set_fg(Color::Rgb(brightness, brightness, brightness));
            }
        }

        // Determine if we should show the game
        let is_large_repo = progress.is_large_repo();
        let show_game = self.show_game && is_large_repo;

        // Calculate layout
        let main_content_height = if show_game { 12 } else { 14 };
        let game_height = if show_game { 12 } else { 0 };

        // Main content area (centered)
        let main_area = if show_game {
            // Split vertically - loading info on top, game below
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(main_content_height),
                    Constraint::Length(game_height),
                ])
                .margin(2)
                .split(area);
            chunks[0]
        } else {
            // Center the loading content
            let content_height = main_content_height;
            let content_width = 60;
            Rect {
                x: area.x + (area.width.saturating_sub(content_width)) / 2,
                y: area.y + (area.height.saturating_sub(content_height)) / 2,
                width: content_width.min(area.width),
                height: content_height.min(area.height),
            }
        };

        // Calculate animated border glow (subtle cyan pulse)
        let glow_intensity = (self.animation_time.sin() + 1.0) / 2.0; // 0.0 to 1.0
        let border_r = (0.0 + glow_intensity * 50.0) as u8; // 0-50
        let border_g = (150.0 + glow_intensity * 105.0) as u8; // 150-255
        let border_b = (200.0 + glow_intensity * 55.0) as u8; // 200-255
        let border_color = Color::Rgb(border_r, border_g, border_b);

        // Create the main block with animated glowing border
        let block = Block::default()
            .title(format!(" {} ", self.spinner.current_char()))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(main_area);
        frame.render_widget(block, main_area);

        // Split inner area for content
        let constraints = if is_large_repo {
            vec![
                Constraint::Length(2), // Title
                Constraint::Length(2), // Repo name
                Constraint::Length(1), // Warning for large repos
                Constraint::Length(2), // Progress bar
                Constraint::Length(2), // Status text
                Constraint::Length(1), // Game hint
                Constraint::Min(0),    // Error message (if any)
            ]
        } else {
            vec![
                Constraint::Length(2), // Title
                Constraint::Length(2), // Repo name
                Constraint::Length(2), // Progress bar
                Constraint::Length(2), // Status text
                Constraint::Length(1), // Spacing
                Constraint::Min(0),    // Error message (if any)
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(constraints)
            .split(inner);

        // Title
        let title = Paragraph::new("Loading Repository Data")
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);

        // Repository name
        let repo_text = Paragraph::new(format!("{}/{}", self.owner, self.repo))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        frame.render_widget(repo_text, chunks[1]);

        let progress_idx = if is_large_repo { 3 } else { 2 };
        let status_idx = if is_large_repo { 4 } else { 3 };

        // Warning for large repos
        if is_large_repo {
            let warning_text = if let Some(stars) = progress.star_count {
                format!(
                    "⚠ This repo has {} stars - loading may take a while!",
                    format_stars(stars)
                )
            } else {
                "⚠ Large repository - loading may take a while!".to_string()
            };
            let warning = Paragraph::new(warning_text)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            frame.render_widget(warning, chunks[2]);

            // Game hint
            let game_hint = if self.show_game {
                "↑/↓ to play Pong!  Space/Enter to hide game"
            } else {
                "Press Space or Enter to play Pong while waiting!"
            };
            let hint = Paragraph::new(game_hint)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(hint, chunks[5]);
        }

        // Progress bar - show page progress during star history, endpoint progress otherwise
        let (progress_pct_val, progress_lbl) = if progress.current_endpoint.as_deref()
            == Some("Star History")
        {
            if let (Some(current), Some(total)) = (progress.current_page, progress.total_pages) {
                let pct = (current as f64 / total as f64) * 100.0;
                (
                    pct as u16,
                    format!("page {}/{} ({:.0}%)", current, total, pct),
                )
            } else {
                (0_u16, "Starting...".to_string())
            }
        } else {
            let pct = progress.progress_percent() as u16;
            (
                pct,
                format!(
                    "{}/{} endpoints ({:.0}%)",
                    progress.completed, progress.total, pct
                ),
            )
        };
        let progress_bar = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .percent(progress_pct_val)
            .label(progress_lbl);
        frame.render_widget(progress_bar, chunks[progress_idx]);

        // Current status text with page progress
        let status_text = progress.get_status_text();

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(status, chunks[status_idx]);

        // Error message (if any)
        if let Some(ref error) = progress.error {
            let error_idx = if is_large_repo { 6 } else { 5 };
            let error_text = Paragraph::new(format!("Error: {}", error))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(error_text, chunks[error_idx]);
        }

        // Render game if active
        if show_game {
            let game_area = if show_game {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(main_content_height),
                        Constraint::Length(game_height),
                    ])
                    .margin(2)
                    .split(area);
                chunks[1]
            } else {
                area
            };

            if let Some(ref game) = self.pong_game {
                game.render(frame, game_area);
            }
        }
    }

    /// Run the loading screen until the fetch is complete
    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        mut progress_rx: tokio::sync::mpsc::Receiver<FetchProgress>,
    ) -> anyhow::Result<FetchProgress>
    where
        <B as Backend>::Error: Send + Sync + 'static,
    {
        let mut last_progress = FetchProgress::new(10);
        let mut initialized_game = false;

        loop {
            // Check for progress updates
            while let Ok(progress) = progress_rx.try_recv() {
                // Initialize game when we detect a large repo
                if progress.is_large_repo() && !initialized_game {
                    let area = terminal.size()?;
                    // Create game in bottom half of screen
                    let game_width = (area.width - 4).min(60);
                    let game_height = 12;
                    self.pong_game = Some(PongGame::new(game_width, game_height));
                    initialized_game = true;
                }
                last_progress = progress;
            }

            // Handle input for game
            self.handle_input();

            // Update spinner and game
            let needs_redraw = self.tick() || last_progress.is_complete();

            if needs_redraw {
                terminal.draw(|frame| self.render(frame, &last_progress))?;
            }

            // Check if complete
            if last_progress.is_complete() {
                // Small delay to show 100% completion
                tokio::time::sleep(Duration::from_millis(200)).await;
                return Ok(last_progress);
            }

            // Small sleep to prevent busy-waiting
            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    }
}

/// Format star count with K/M suffix
fn format_stars(stars: u64) -> String {
    if stars >= 1_000_000 {
        format!("{:.1}M", stars as f64 / 1_000_000.0)
    } else if stars >= 1_000 {
        format!("{:.1}k", stars as f64 / 1_000.0)
    } else {
        stars.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_progress_new() {
        let progress = FetchProgress::new(10);
        assert_eq!(progress.total, 10);
        assert_eq!(progress.completed, 0);
        assert!(progress.current_endpoint.is_none());
        assert!(!progress.done);
        assert!(progress.error.is_none());
    }

    #[test]
    fn test_fetch_progress_percent() {
        let mut progress = FetchProgress::new(10);
        assert_eq!(progress.progress_percent(), 0.0);

        progress.completed = 5;
        assert_eq!(progress.progress_percent(), 50.0);

        progress.completed = 10;
        assert_eq!(progress.progress_percent(), 100.0);
    }

    #[test]
    fn test_fetch_progress_is_complete() {
        let mut progress = FetchProgress::new(10);
        assert!(!progress.is_complete());

        progress.completed = 10;
        assert!(progress.is_complete());

        progress.completed = 5;
        progress.done = true;
        assert!(progress.is_complete());
    }

    #[test]
    fn test_large_repo_detection() {
        let mut progress = FetchProgress::new(10);
        assert!(!progress.is_large_repo());

        progress.star_count = Some(5_000);
        assert!(!progress.is_large_repo());

        progress.star_count = Some(15_000);
        assert!(progress.is_large_repo());

        progress.star_count = Some(100_000);
        assert!(progress.is_large_repo());
    }

    #[test]
    fn test_pong_game_new() {
        let game = PongGame::new(50, 20);
        assert_eq!(game.width, 50);
        assert_eq!(game.height, 20);
        assert_eq!(game.player_score, 0);
        assert_eq!(game.ai_score, 0);
    }

    #[test]
    fn test_format_stars() {
        assert_eq!(format_stars(500), "500");
        assert_eq!(format_stars(1500), "1.5k");
        assert_eq!(format_stars(1000000), "1.0M");
        assert_eq!(format_stars(2500000), "2.5M");
    }
}
