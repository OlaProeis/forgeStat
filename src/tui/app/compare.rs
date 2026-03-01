use ratatui::{prelude::*, widgets::*};

use super::utils::resample_to_width;
use super::App;
use crate::core::health::{compute_health_score, HealthScore};
use crate::core::models::RepoSnapshot;
use crate::tui::widgets::BrailleSparkline;

/// Which side is focused in compare mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareFocus {
    Left,
    Right,
}

impl CompareFocus {
    pub fn toggle(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl App {
    /// Render the compare mode overlay with side-by-side repository comparison
    pub(super) fn render_compare_overlay(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Clear, area);

        // Full-screen compare view
        let [header_area, content_area, status_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Render compare header
        self.render_compare_header(frame, header_area);

        // Render compare content (split screen)
        self.render_compare_content(frame, content_area);

        // Render compare status bar
        self.render_compare_status_bar(frame, status_area);
    }

    /// Render the compare mode header
    fn render_compare_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title(" Compare Mode ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.header_border_color()));

        let header_text = format!(
            "{} vs {} [Tab: switch focus  q: quit compare]",
            self.format_repo_name(),
            self.format_compare_repo_name()
        );

        let paragraph = Paragraph::new(header_text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Format the primary repo name
    fn format_repo_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// Format the compare repo name
    fn format_compare_repo_name(&self) -> String {
        self.compare_snapshot
            .as_ref()
            .map(|s| format!("{}/{}", s.repo.owner, s.repo.name))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Render the split-screen compare content
    fn render_compare_content(&self, frame: &mut Frame, area: Rect) {
        let [left_area, right_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(area);

        // Left side: Primary repository
        self.render_compare_side(
            frame,
            left_area,
            &self.format_repo_name(),
            self.snapshot.as_ref(),
            self.health_score,
            self.compare_focus == CompareFocus::Left,
        );

        // Right side: Compare repository
        self.render_compare_side(
            frame,
            right_area,
            &self.format_compare_repo_name(),
            self.compare_snapshot.as_ref(),
            self.compare_health_score,
            self.compare_focus == CompareFocus::Right,
        );
    }

    /// Render one side of the compare view
    fn render_compare_side(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        snapshot: Option<&RepoSnapshot>,
        health_score: Option<HealthScore>,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let Some(snap) = snapshot else {
            // No snapshot available
            let block = Block::bordered()
                .title(format!(" {} ", label))
                .border_style(Style::default().fg(border_color));

            let paragraph = Paragraph::new("No data available").block(block);
            frame.render_widget(paragraph, area);
            return;
        };

        // Split into sections: Health (prominent), Stars, Issues, PRs, Contributors, Releases, Security
        let [health_area, stars_area, issues_area, prs_area, contrib_area, releases_area, security_area] =
            Layout::vertical([
                Constraint::Length(5), // Health score (prominent)
                Constraint::Length(7), // Stars
                Constraint::Length(5), // Issues
                Constraint::Length(5), // PRs
                Constraint::Length(4), // Contributors
                Constraint::Length(4), // Releases
                Constraint::Fill(1),   // Security
            ])
            .areas(area);

        // Render health panel (prominent)
        self.render_compare_health(frame, health_area, snap, health_score, label, is_focused);

        // Render stars panel with comparison
        self.render_compare_stars(frame, stars_area, snap, label, is_focused);

        // Render issues panel with comparison
        self.render_compare_issues(frame, issues_area, snap, label, is_focused);

        // Render PRs panel with comparison
        self.render_compare_prs(frame, prs_area, snap, label, is_focused);

        // Render contributors panel with comparison
        self.render_compare_contributors(frame, contrib_area, snap, label, is_focused);

        // Render releases panel with comparison
        self.render_compare_releases(frame, releases_area, snap, label, is_focused);

        // Render security panel with comparison
        self.render_compare_security(frame, security_area, snap, label, is_focused);
    }

    /// Render health score panel (prominent)
    fn render_compare_health(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        health_score: Option<HealthScore>,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let block = Block::bordered()
            .title(format!(" Health — {} ", label))
            .border_style(Style::default().fg(border_color));

        let health = health_score.unwrap_or_else(|| compute_health_score(snap));

        // Get the other repo's health for comparison
        let other_health = match self.compare_focus {
            CompareFocus::Left => self.compare_health_score,
            CompareFocus::Right => self.health_score,
        };

        // Determine winner highlighting
        let is_winner = other_health
            .map(|oh| health.total > oh.total)
            .unwrap_or(false);
        let is_tied = other_health
            .map(|oh| health.total == oh.total)
            .unwrap_or(false);

        let (score_color, winner_indicator) = if is_winner {
            (self.theme.indicator_success_color(), " ★")
        } else if is_tied {
            (self.theme.text_highlight_color(), " =")
        } else {
            (self.theme.text_primary_color(), "")
        };

        // Grade color
        let grade_color = match health.grade {
            crate::core::health::HealthGrade::Excellent => self.theme.indicator_success_color(),
            crate::core::health::HealthGrade::Good => self.theme.text_highlight_color(),
            crate::core::health::HealthGrade::Fair => self.theme.indicator_warning_color(),
            crate::core::health::HealthGrade::NeedsAttention => {
                self.theme.indicator_warning_color()
            }
            crate::core::health::HealthGrade::Critical => self.theme.indicator_error_color(),
        };

        let text = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} {}/100", winner_indicator, health.total),
                    Style::default().fg(score_color).bold(),
                ),
                Span::styled(
                    format!(" (Grade {})", health.grade.as_letter()),
                    Style::default().fg(grade_color).bold(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Activity: {}/25  ", health.activity),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("Community: {}/25  ", health.community),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("Maintenance: {}/25  ", health.maintenance),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("Growth: {}/25", health.growth),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render stars panel with comparison
    fn render_compare_stars(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let star_count = snap.stars.total_count;

        // Get other repo's star count for comparison
        let other_stars = self.get_other_repo_stars();
        let delta = other_stars.map(|o| star_count as i64 - o as i64);

        let (count_text, count_style) = match delta {
            Some(d) if d > 0 => (
                format!("{} (+{} vs other)", star_count, d),
                self.theme.indicator_success_color(),
            ),
            Some(d) if d < 0 => (
                format!("{} ({} vs other)", star_count, d),
                self.theme.indicator_error_color(),
            ),
            _ => (star_count.to_string(), self.theme.text_primary_color()),
        };

        let block = Block::bordered()
            .title(format!(" ★ Stars — {} ", label))
            .border_style(Style::default().fg(border_color));

        // Split area for sparkline
        let [text_area, spark_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area.inner(
                ratatui::layout::Margin {
                    horizontal: 1,
                    vertical: 0,
                },
            ));

        let text = vec![Line::from(vec![
            Span::styled(
                "Total: ",
                Style::default().fg(self.theme.text_secondary_color()),
            ),
            Span::styled(count_text, Style::default().fg(count_style).bold()),
        ])];

        let paragraph = Paragraph::new(text);
        frame.render_widget(paragraph, text_area);
        frame.render_widget(block, area);

        // Render sparkline if we have data
        let sparkline_data: Vec<u64> = snap.stars.sparkline_30d.iter().map(|&x| x as u64).collect();
        if !sparkline_data.is_empty() {
            let width = spark_area.width as usize;
            let resampled = resample_to_width(&sparkline_data, width.max(10));

            if self.theme.braille_mode {
                let braille = BrailleSparkline::new(&resampled)
                    .style(Style::default().fg(self.theme.sparkline_color()));
                frame.render_widget(braille, spark_area);
            } else {
                let sparkline = Sparkline::default()
                    .data(&resampled)
                    .style(Style::default().fg(self.theme.sparkline_color()));
                frame.render_widget(sparkline, spark_area);
            }
        }
    }

    /// Get the other repository's star count for comparison
    fn get_other_repo_stars(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self.compare_snapshot.as_ref().map(|s| s.stars.total_count),
            CompareFocus::Right => self.snapshot.as_ref().map(|s| s.stars.total_count),
        }
    }

    /// Render issues panel with comparison
    fn render_compare_issues(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let issue_count = snap.issues.total_open;

        // Get other repo's issue count for comparison
        let other_issues = self.get_other_repo_issues();
        let delta = other_issues.map(|o| issue_count as i64 - o as i64);

        // For issues, fewer is better (lower is winning)
        let (count_text, count_style) = match delta {
            Some(d) if d < 0 => (
                format!("{} ({} vs other)", issue_count, d),
                self.theme.indicator_success_color(),
            ),
            Some(d) if d > 0 => (
                format!("{} (+{} vs other)", issue_count, d),
                self.theme.indicator_error_color(),
            ),
            _ => (issue_count.to_string(), self.theme.text_primary_color()),
        };

        let block = Block::bordered()
            .title(format!(" Issues — {} ", label))
            .border_style(Style::default().fg(border_color));

        let label_count = snap.issues.by_label.len();
        let unlabelled_count = snap.issues.unlabelled.len();

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Open: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(count_text, Style::default().fg(count_style).bold()),
            ]),
            Line::from(vec![Span::styled(
                format!(
                    "By label: {} | Unlabelled: {}",
                    label_count, unlabelled_count
                ),
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Get the other repository's issue count for comparison
    fn get_other_repo_issues(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self.compare_snapshot.as_ref().map(|s| s.issues.total_open),
            CompareFocus::Right => self.snapshot.as_ref().map(|s| s.issues.total_open),
        }
    }

    /// Render PRs panel with comparison
    fn render_compare_prs(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let pr_count = snap.pull_requests.open_count;

        // Get other repo's PR count for comparison
        let other_prs = self.get_other_repo_prs();
        let delta = other_prs.map(|o| pr_count as i64 - o as i64);

        // For PRs, fewer open is generally better (lower backlog)
        let (count_text, count_style) = match delta {
            Some(d) if d < 0 => (
                format!("{} ({} vs other)", pr_count, d),
                self.theme.indicator_success_color(),
            ),
            Some(d) if d > 0 => (
                format!("{} (+{} vs other)", pr_count, d),
                self.theme.indicator_error_color(),
            ),
            _ => (pr_count.to_string(), self.theme.text_primary_color()),
        };

        let block = Block::bordered()
            .title(format!(" Pull Requests — {} ", label))
            .border_style(Style::default().fg(border_color));

        let draft_count = snap.pull_requests.draft_count;
        let ready_count = snap.pull_requests.ready_count;
        let merged_30d = snap.pull_requests.merged_last_30d.len();

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Open: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(count_text, Style::default().fg(count_style).bold()),
            ]),
            Line::from(vec![Span::styled(
                format!("Draft: {} | Ready: {}", draft_count, ready_count),
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
            Line::from(vec![Span::styled(
                format!("Merged (30d): {}", merged_30d),
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Get the other repository's PR count for comparison
    fn get_other_repo_prs(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self
                .compare_snapshot
                .as_ref()
                .map(|s| s.pull_requests.open_count),
            CompareFocus::Right => self.snapshot.as_ref().map(|s| s.pull_requests.open_count),
        }
    }

    /// Render contributors panel with comparison
    fn render_compare_contributors(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let contrib_count = snap.contributors.total_unique;

        // Get other repo's contributor count for comparison
        let other_contributors = self.get_other_repo_contributors();
        let delta = other_contributors.map(|o| contrib_count as i64 - o as i64);

        let (count_text, count_style) = match delta {
            Some(d) if d > 0 => (
                format!("{} (+{} vs other)", contrib_count, d),
                self.theme.indicator_success_color(),
            ),
            Some(d) if d < 0 => (
                format!("{} ({} vs other)", contrib_count, d),
                self.theme.indicator_error_color(),
            ),
            _ => (contrib_count.to_string(), self.theme.text_primary_color()),
        };

        let block = Block::bordered()
            .title(format!(" Contributors — {} ", label))
            .border_style(Style::default().fg(border_color));

        let new_contributors = snap.contributors.new_contributors_last_30d.len();

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Total: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(count_text, Style::default().fg(count_style).bold()),
            ]),
            Line::from(vec![Span::styled(
                format!("New (30d): {}", new_contributors),
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Get the other repository's contributor count for comparison
    fn get_other_repo_contributors(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self
                .compare_snapshot
                .as_ref()
                .map(|s| s.contributors.total_unique),
            CompareFocus::Right => self.snapshot.as_ref().map(|s| s.contributors.total_unique),
        }
    }

    /// Render releases panel with comparison
    fn render_compare_releases(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let release_count = snap.releases.len() as u64;

        // Get other repo's release count for comparison
        let other_releases = self.get_other_repo_releases();
        let delta = other_releases.map(|o| release_count as i64 - o as i64);

        let (count_text, count_style) = match delta {
            Some(d) if d > 0 => (
                format!("{} (+{} vs other)", release_count, d),
                self.theme.indicator_success_color(),
            ),
            Some(d) if d < 0 => (
                format!("{} ({} vs other)", release_count, d),
                self.theme.indicator_error_color(),
            ),
            _ => (release_count.to_string(), self.theme.text_primary_color()),
        };

        let block = Block::bordered()
            .title(format!(" Releases — {} ", label))
            .border_style(Style::default().fg(border_color));

        let latest_text = snap
            .releases
            .first()
            .map(|r| {
                let days_text = r
                    .days_since
                    .map(|d| format!("{}d ago", d))
                    .unwrap_or_else(|| "unknown".to_string());
                format!("{} ({})", r.tag_name, days_text)
            })
            .unwrap_or_else(|| "No releases".to_string());

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Count: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(count_text, Style::default().fg(count_style).bold()),
            ]),
            Line::from(vec![
                Span::styled(
                    "Latest: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    latest_text,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Get the other repository's release count for comparison
    fn get_other_repo_releases(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self
                .compare_snapshot
                .as_ref()
                .map(|s| s.releases.len() as u64),
            CompareFocus::Right => self.snapshot.as_ref().map(|s| s.releases.len() as u64),
        }
    }

    /// Render security panel with comparison
    fn render_compare_security(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        label: &str,
        is_focused: bool,
    ) {
        let border_color = if is_focused {
            self.theme.border_selected_color()
        } else {
            self.theme.border_unselected_color()
        };

        let block = Block::bordered()
            .title(format!(" Security — {} ", label))
            .border_style(Style::default().fg(border_color));

        let security_text = if let Some(sec) = &snap.security_alerts {
            let total = sec.total_open;
            let critical = sec.critical_count;
            let high = sec.high_count;

            // Compare with other repo
            let other_security = self.get_other_repo_security();
            let delta = other_security.map(|o| total as i64 - o as i64);

            // For security alerts, fewer is better
            let (total_text, total_style) = match delta {
                Some(d) if d < 0 => (
                    format!("{} ({} vs other)", total, d),
                    self.theme.indicator_success_color(),
                ),
                Some(d) if d > 0 => (
                    format!("{} (+{} vs other)", total, d),
                    self.theme.indicator_error_color(),
                ),
                _ => (total.to_string(), self.theme.text_primary_color()),
            };

            vec![
                Line::from(vec![
                    Span::styled(
                        "Total: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(total_text, Style::default().fg(total_style).bold()),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Critical: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        critical.to_string(),
                        Style::default().fg(self.theme.indicator_error_color()),
                    ),
                    Span::styled(
                        " | High: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        high.to_string(),
                        Style::default().fg(self.theme.indicator_warning_color()),
                    ),
                ]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "No security data available",
                Style::default().fg(self.theme.text_secondary_color()),
            ))]
        };

        let paragraph = Paragraph::new(security_text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Get the other repository's security alert count for comparison
    fn get_other_repo_security(&self) -> Option<u64> {
        match self.compare_focus {
            CompareFocus::Left => self
                .compare_snapshot
                .as_ref()
                .and_then(|s| s.security_alerts.as_ref().map(|sec| sec.total_open)),
            CompareFocus::Right => self
                .snapshot
                .as_ref()
                .and_then(|s| s.security_alerts.as_ref().map(|sec| sec.total_open)),
        }
    }

    /// Render compare mode status bar
    fn render_compare_status_bar(&self, frame: &mut Frame, area: Rect) {
        let focus_text = match self.compare_focus {
            CompareFocus::Left => "Focus: Left (Primary)",
            CompareFocus::Right => "Focus: Right (Compare)",
        };

        let mut spans: Vec<Span> = vec![
            Span::styled(
                "Compare Mode",
                Style::default()
                    .fg(self.theme.text_highlight_color())
                    .bold(),
            ),
            Span::styled(
                " | ",
                Style::default().fg(self.theme.text_secondary_color()),
            ),
            Span::styled(
                focus_text,
                Style::default().fg(self.theme.text_primary_color()),
            ),
        ];

        // Add comparison summary
        if let (Some(primary), Some(compare)) = (&self.snapshot, &self.compare_snapshot) {
            let primary_health_score = self
                .health_score
                .as_ref()
                .map(|h| h.total)
                .unwrap_or_else(|| compute_health_score(primary).total);
            let compare_health_score = self
                .compare_health_score
                .as_ref()
                .map(|h| h.total)
                .unwrap_or_else(|| compute_health_score(compare).total);

            spans.push(Span::styled(
                " | ",
                Style::default().fg(self.theme.text_secondary_color()),
            ));

            if primary_health_score > compare_health_score {
                spans.push(Span::styled(
                    format!("Winner: {}/{}", primary.repo.owner, primary.repo.name),
                    Style::default()
                        .fg(self.theme.indicator_success_color())
                        .bold(),
                ));
            } else if compare_health_score > primary_health_score {
                spans.push(Span::styled(
                    format!("Winner: {}/{}", compare.repo.owner, compare.repo.name),
                    Style::default()
                        .fg(self.theme.indicator_success_color())
                        .bold(),
                ));
            } else {
                spans.push(Span::styled(
                    "Tied",
                    Style::default()
                        .fg(self.theme.text_highlight_color())
                        .bold(),
                ));
            }
        }

        spans.push(Span::styled(
            " | Tab: switch  q: quit",
            Style::default().fg(self.theme.text_secondary_color()),
        ));

        let status_line = Line::from(spans);
        let paragraph = Paragraph::new(status_line).alignment(Alignment::Left);
        frame.render_widget(paragraph, area);
    }
}
