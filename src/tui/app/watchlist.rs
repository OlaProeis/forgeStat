use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::{
    backend::Backend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

use crate::core::health::{compute_health_score, HealthGrade, HealthScore};
use crate::core::models::RepoSnapshot;
use crate::core::theme::ThemeConfig;
use crate::tui::widgets::BrailleSpinner;

/// Actions that can be returned from the watchlist event loop
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchlistAction {
    Quit,
    Refresh,
    SelectRepo(String, String),
}

/// A row in the watchlist table
#[allow(dead_code)]
struct WatchlistRow {
    repo: String,
    owner: String,
    repo_name: String,
    snapshot: Option<RepoSnapshot>,
    health: Option<HealthScore>,
    error: Option<String>,
    stars: u64,
    stars_30d: u64,
    issues: u64,
    prs: u64,
    last_release_days: Option<u64>,
    security_alerts: u64,
}

/// Watchlist dashboard application state
#[allow(dead_code)]
pub struct WatchlistApp {
    repos: Vec<String>,
    snapshots: HashMap<String, RepoSnapshot>,
    selected_index: usize,
    is_fetching: bool,
    theme: ThemeConfig,
    last_refresh: Instant,
    table_state: TableState,
    spinner: BrailleSpinner,
    last_spinner_update: Instant,
    scroll_offset: usize,
    rows_visible: usize,
}

impl WatchlistApp {
    /// Create a new watchlist app with the given repos
    pub fn new(repos: Vec<String>, theme: ThemeConfig) -> Self {
        let mut table_state = TableState::default();
        if !repos.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            repos,
            snapshots: HashMap::new(),
            selected_index: 0,
            is_fetching: false,
            theme,
            last_refresh: Instant::now(),
            table_state,
            spinner: BrailleSpinner::new(),
            last_spinner_update: Instant::now(),
            scroll_offset: 0,
            rows_visible: 10,
        }
    }

    /// Add a fetched snapshot
    pub fn add_snapshot(&mut self, repo: String, snapshot: RepoSnapshot) {
        self.snapshots.insert(repo, snapshot);
    }

    /// Add an error for a repo
    pub fn add_error(&mut self, repo: String, error: String) {
        log::warn!("Watchlist error for {}: {}", repo, error);
        // Still insert an empty snapshot so the repo shows in the list
        // The error state will be handled during rendering
    }

    /// Clear all snapshots (for refresh)
    pub fn clear_snapshots(&mut self) {
        self.snapshots.clear();
    }

    /// Set fetching state
    pub fn set_fetching(&mut self, fetching: bool) {
        self.is_fetching = fetching;
    }

    /// Build watchlist rows from current snapshots
    fn build_rows(&self) -> Vec<WatchlistRow> {
        self.repos
            .iter()
            .map(|repo| {
                let parts: Vec<&str> = repo.split('/').collect();
                let owner = parts[0].to_string();
                let repo_name = parts[1].to_string();

                if let Some(snapshot) = self.snapshots.get(repo) {
                    let health = Some(compute_health_score(snapshot));
                    let stars = snapshot.stars.total_count;

                    // Calculate 30d stars from sparkline sum
                    let stars_30d: u64 =
                        snapshot.stars.sparkline_30d.iter().map(|&v| v as u64).sum();

                    let issues = snapshot.open_issues_count() as u64;
                    let prs = snapshot.open_prs_count() as u64;
                    let last_release_days = snapshot.days_since_last_release().map(|d| d as u64);
                    let security_alerts = snapshot
                        .security_alerts
                        .as_ref()
                        .map(|s| s.total_open)
                        .unwrap_or(0);

                    WatchlistRow {
                        repo: repo.clone(),
                        owner,
                        repo_name,
                        snapshot: Some(snapshot.clone()),
                        health,
                        error: None,
                        stars,
                        stars_30d,
                        issues,
                        prs,
                        last_release_days,
                        security_alerts,
                    }
                } else {
                    // No snapshot yet (loading or error)
                    WatchlistRow {
                        repo: repo.clone(),
                        owner,
                        repo_name,
                        snapshot: None,
                        health: None,
                        error: None,
                        stars: 0,
                        stars_30d: 0,
                        issues: 0,
                        prs: 0,
                        last_release_days: None,
                        security_alerts: 0,
                    }
                }
            })
            .collect()
    }

    /// Get color for a health grade
    fn health_color(&self, grade: &HealthGrade) -> Color {
        match grade {
            HealthGrade::Excellent => self.theme.indicator_success_color(),
            HealthGrade::Good => self.theme.text_highlight_color(),
            HealthGrade::Fair => self.theme.indicator_warning_color(),
            HealthGrade::NeedsAttention => self.theme.indicator_warning_color(),
            HealthGrade::Critical => self.theme.indicator_error_color(),
        }
    }

    /// Render the watchlist dashboard
    pub fn render<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()>
    where
        B::Error: std::fmt::Debug,
    {
        terminal
            .draw(|frame| {
                self.draw(frame);
            })
            .map_err(|e| anyhow::anyhow!("Terminal draw error: {:?}", e))?;
        Ok(())
    }

    /// Draw the watchlist UI
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: header, content, status bar
        let [header_area, content_area, status_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        self.draw_header(frame, header_area);
        self.draw_table(frame, content_area);
        self.draw_status_bar(frame, status_area);
    }

    /// Draw the header
    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" Watchlist Dashboard — {} repos ", self.repos.len());

        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.header_border_color()));

        let header_text = if self.is_fetching {
            format!(
                "{} Fetching repository data...",
                self.spinner.current_char()
            )
        } else {
            let loaded = self.snapshots.len();
            let total = self.repos.len();
            if loaded == total {
                format!("All {} repositories loaded — Press ? for help", total)
            } else {
                format!("Loaded {}/{} repositories", loaded, total)
            }
        };

        let paragraph = Paragraph::new(header_text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Draw the watchlist table
    fn draw_table(&mut self, frame: &mut Frame, area: Rect) {
        let rows = self.build_rows();
        self.rows_visible = (area.height as usize).saturating_sub(3); // Account for header/footer

        // Define column widths
        let header = Row::new(vec![
            Cell::from("Repository"),
            Cell::from("Health"),
            Cell::from("★ Stars"),
            Cell::from("30d"),
            Cell::from("Issues"),
            Cell::from("PRs"),
            Cell::from("Release"),
            Cell::from("Security"),
        ])
        .style(
            Style::default()
                .fg(self.theme.text_highlight_color())
                .add_modifier(Modifier::BOLD),
        );

        let table_rows: Vec<Row> = rows
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.rows_visible)
            .map(|(idx, row)| {
                let is_selected = idx == self.selected_index;

                // Determine row style based on health grade
                let base_style = if is_selected {
                    Style::default()
                        .bg(self.theme.text_highlight_color())
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else if let Some(ref health) = row.health {
                    let health_color = self.health_color(&health.grade);
                    Style::default().fg(health_color)
                } else {
                    Style::default().fg(self.theme.text_secondary_color())
                };

                // Format cells
                let repo_cell = Cell::from(format!("{}/{}", row.owner, row.repo_name));

                let health_cell = if let Some(ref health) = row.health {
                    Cell::from(format!("{} {}", health.total, health.grade.as_letter()))
                } else {
                    Cell::from("—")
                };

                let stars_cell = Cell::from(format_count(row.stars));
                let stars_30d_cell = Cell::from(format!("+{}", format_count(row.stars_30d)));
                let issues_cell = Cell::from(format_count(row.issues));
                let prs_cell = Cell::from(format_count(row.prs));

                let release_cell = if let Some(days) = row.last_release_days {
                    let text = if days == 0 {
                        "today".to_string()
                    } else if days < 30 {
                        format!("{}d", days)
                    } else if days < 365 {
                        format!("{}mo", days / 30)
                    } else {
                        format!("{}y", days / 365)
                    };
                    Cell::from(text)
                } else {
                    Cell::from("—")
                };

                let security_cell = if row.security_alerts > 0 {
                    Cell::from(format!("⚠ {}", row.security_alerts))
                } else {
                    Cell::from("✓")
                };

                Row::new(vec![
                    repo_cell,
                    health_cell,
                    stars_cell,
                    stars_30d_cell,
                    issues_cell,
                    prs_cell,
                    release_cell,
                    security_cell,
                ])
                .style(base_style)
            })
            .collect();

        let table = Table::new(
            table_rows,
            [
                Constraint::Length(25), // Repo
                Constraint::Length(10), // Health
                Constraint::Length(10), // Stars
                Constraint::Length(6),  // 30d
                Constraint::Length(8),  // Issues
                Constraint::Length(6),  // PRs
                Constraint::Length(10), // Release
                Constraint::Length(10), // Security
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.header_border_color())),
        )
        .row_highlight_style(
            Style::default()
                .bg(self.theme.text_highlight_color())
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

        // Update table state selection
        self.table_state
            .select(Some(self.selected_index.saturating_sub(self.scroll_offset)));

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    /// Draw the status bar
    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let help_text = if self.is_fetching {
            "Fetching...".to_string()
        } else {
            format!(
                "↑/↓:nav | Enter:select | r:refresh | q:quit | {} repos",
                self.repos.len()
            )
        };

        let paragraph = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(self.theme.text_secondary_color()));

        frame.render_widget(paragraph, area);
    }

    /// Run the event loop for the watchlist
    pub async fn run_event_loop<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<WatchlistAction> {
        let mut last_update = Instant::now();
        let spinner_interval = Duration::from_millis(80);

        loop {
            // Update spinner animation
            if self.is_fetching && last_update.elapsed() >= spinner_interval {
                self.spinner.next_frame();
                self.render(terminal)?;
                last_update = Instant::now();
            }

            // Check for events with timeout
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(WatchlistAction::Quit);
                            }
                            KeyCode::Char('r') => {
                                return Ok(WatchlistAction::Refresh);
                            }
                            KeyCode::Enter => {
                                // Get selected repo and switch to single view
                                if let Some(repo) = self.repos.get(self.selected_index) {
                                    let parts: Vec<&str> = repo.split('/').collect();
                                    if parts.len() == 2 {
                                        return Ok(WatchlistAction::SelectRepo(
                                            parts[0].to_string(),
                                            parts[1].to_string(),
                                        ));
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                    // Adjust scroll if needed
                                    if self.selected_index < self.scroll_offset {
                                        self.scroll_offset = self.selected_index;
                                    }
                                    self.render(terminal)?;
                                }
                            }
                            KeyCode::Down => {
                                if self.selected_index < self.repos.len().saturating_sub(1) {
                                    self.selected_index += 1;
                                    // Adjust scroll if needed
                                    if self.selected_index >= self.scroll_offset + self.rows_visible
                                    {
                                        self.scroll_offset = self
                                            .selected_index
                                            .saturating_sub(self.rows_visible - 1);
                                    }
                                    self.render(terminal)?;
                                }
                            }
                            KeyCode::PageUp => {
                                self.selected_index =
                                    self.selected_index.saturating_sub(self.rows_visible);
                                self.scroll_offset =
                                    self.scroll_offset.saturating_sub(self.rows_visible);
                                self.render(terminal)?;
                            }
                            KeyCode::PageDown => {
                                let max_idx = self.repos.len().saturating_sub(1);
                                self.selected_index =
                                    (self.selected_index + self.rows_visible).min(max_idx);
                                if self.selected_index >= self.scroll_offset + self.rows_visible {
                                    self.scroll_offset =
                                        self.selected_index.saturating_sub(self.rows_visible - 1);
                                }
                                self.render(terminal)?;
                            }
                            KeyCode::Home => {
                                self.selected_index = 0;
                                self.scroll_offset = 0;
                                self.render(terminal)?;
                            }
                            KeyCode::End => {
                                self.selected_index = self.repos.len().saturating_sub(1);
                                self.scroll_offset =
                                    self.repos.len().saturating_sub(self.rows_visible);
                                self.render(terminal)?;
                            }
                            _ => {}
                        }
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            if self.selected_index > 0 {
                                self.selected_index -= 1;
                                if self.selected_index < self.scroll_offset {
                                    self.scroll_offset = self.selected_index;
                                }
                                self.render(terminal)?;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if self.selected_index < self.repos.len().saturating_sub(1) {
                                self.selected_index += 1;
                                if self.selected_index >= self.scroll_offset + self.rows_visible {
                                    self.scroll_offset =
                                        self.selected_index.saturating_sub(self.rows_visible - 1);
                                }
                                self.render(terminal)?;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            // Periodic redraw for animations
            if self.is_fetching && last_update.elapsed() >= spinner_interval {
                self.render(terminal)?;
            }
        }
    }
}

/// Format a count number with K/M suffixes
fn format_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1000), "1.0K");
        assert_eq!(format_count(1500), "1.5K");
        assert_eq!(format_count(999500), "999.5K");
        assert_eq!(format_count(1000000), "1.0M");
        assert_eq!(format_count(2500000), "2.5M");
    }

    #[test]
    fn test_watchlist_app_new() {
        let repos = vec!["owner/repo1".to_string(), "owner/repo2".to_string()];
        let theme = ThemeConfig::default();
        let app = WatchlistApp::new(repos.clone(), theme);

        assert_eq!(app.repos, repos);
        assert!(app.snapshots.is_empty());
        assert_eq!(app.selected_index, 0);
        assert!(!app.is_fetching);
    }

    #[test]
    fn test_watchlist_add_snapshot() {
        let repos = vec!["owner/repo1".to_string()];
        let theme = ThemeConfig::default();
        let mut app = WatchlistApp::new(repos, theme);

        // Create a minimal snapshot using actual struct fields
        use chrono::Utc;
        use std::collections::HashMap;

        let snapshot = RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: uuid::Uuid::new_v4(),
            repo: crate::core::models::RepoMeta {
                name: "repo1".to_string(),
                owner: "owner".to_string(),
                description: None,
                language: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: crate::core::models::StarHistory {
                total_count: 100,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: crate::core::models::IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: crate::core::models::PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: crate::core::models::ContributorStats {
                total_unique: 0,
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
            },
            releases: vec![],
            velocity: crate::core::models::VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        };

        app.add_snapshot("owner/repo1".to_string(), snapshot);
        assert_eq!(app.snapshots.len(), 1);
    }

    #[test]
    fn test_health_color_mapping() {
        let repos = vec![];
        let theme = ThemeConfig::default();
        let app = WatchlistApp::new(repos, theme);

        // Just verify it doesn't panic
        let _ = app.health_color(&HealthGrade::Excellent);
        let _ = app.health_color(&HealthGrade::Critical);
    }
}
