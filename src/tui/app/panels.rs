use ratatui::{prelude::*, widgets::*};

use super::utils::{format_age, format_count, resample_to_width, trim_leading_zeros, truncate};
use super::{App, Panel, StarTimeframe};
use crate::core::models::Issue;
use crate::tui::widgets::BrailleSparkline;

impl App {
    pub(super) fn panel_block(&self, panel: Panel, title: String) -> Block<'_> {
        let is_selected = self.selected_panel == panel;

        // Calculate flash intensity for animation
        let flash_intensity = self.get_flash_intensity(panel);

        let border_style = if flash_intensity > 0.5 {
            // Strong flash - use highlight color for maximum visibility
            Style::default()
                .fg(self.theme.text_highlight_color())
                .bold()
        } else if flash_intensity > 0.0 {
            // Fading flash - use selected border color with modifier
            Style::default()
                .fg(self.theme.border_selected_color())
                .bold()
        } else if is_selected {
            Style::default()
                .fg(self.theme.border_selected_color())
                .bold()
        } else {
            Style::default().fg(self.theme.border_unselected_color())
        };

        Block::bordered().title(title).border_style(border_style)
    }

    pub(super) fn render_stars(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Stars, " ★ Stars ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Use animated counter for star count if animation is in progress
        let display_count = if self.animation_config.is_count_up_enabled() {
            let animated = self.get_counter_value("stars", snap.stars.total_count);
            // Format with commas for readability
            format_count(animated)
        } else {
            format_count(snap.stars.total_count)
        };

        let block = self.panel_block(Panel::Stars, format!(" ★ Stars — {} ", display_count));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [label_area, spark_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        // Select the appropriate sparkline data based on the selected timeframe
        let data: Vec<u64> = match self.star_timeframe {
            StarTimeframe::Days30 => snap.stars.sparkline_30d.iter().map(|&v| v as u64).collect(),
            StarTimeframe::Days90 => snap.stars.sparkline_90d.iter().map(|&v| v as u64).collect(),
            StarTimeframe::Year1 => snap
                .stars
                .sparkline_365d
                .iter()
                .map(|&v| v as u64)
                .collect(),
        };

        let data_len = data.len();

        // For Year1 view, preserve leading zeros to show full history from 0 stars
        let trimmed = if self.star_timeframe == StarTimeframe::Year1 {
            data
        } else {
            trim_leading_zeros(&data)
        };

        // For 1-year view with limited data, show appropriate time unit
        let effective_label = if self.star_timeframe == StarTimeframe::Year1 && data_len <= 13 {
            format!("{} weeks", data_len)
        } else {
            self.star_timeframe.label().to_string()
        };

        // Update label if needed
        let label_text = if self.selected_panel == Panel::Stars {
            format!("{} [+/- to change]", effective_label)
        } else {
            effective_label
        };
        let label = Paragraph::new(label_text)
            .style(Style::default().fg(self.theme.text_secondary_color()));
        frame.render_widget(label, label_area);

        // Resample to fill the actual available width (account for Braille mode which uses 2 chars per point)
        let target_width = if self.theme.braille_mode {
            (spark_area.width as usize * 2).min(trimmed.len()).max(1)
        } else {
            spark_area.width as usize
        };
        let resampled = resample_to_width(&trimmed, target_width);

        if !resampled.is_empty() {
            if self.theme.braille_mode {
                // Use Braille sparkline for 2x resolution
                let braille = BrailleSparkline::new(&resampled)
                    .style(Style::default().fg(self.theme.sparkline_color()));
                frame.render_widget(braille, spark_area);
            } else {
                // Use classic bar sparkline
                let sparkline = Sparkline::default()
                    .data(&resampled)
                    .style(Style::default().fg(self.theme.sparkline_color()));
                frame.render_widget(sparkline, spark_area);
            }
        }
    }

    pub(super) fn render_issues(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Issues, " Issues ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered issues
        let filtered_issues = self.get_filtered_issues();
        let total_count = snap.issues.total_open;
        let filtered_count = filtered_issues.len() as u64;

        // Use animated counter for issue count if animation is in progress
        let display_total = if self.animation_config.is_count_up_enabled() {
            self.get_counter_value("issues", total_count)
        } else {
            total_count
        };

        // Build title with page indicator and [+/-] hint when selected
        let has_filter = !self.search_query.is_empty() || self.issues_label_filter.is_some();
        let per_page = self.issues_per_page;
        let current_page = (self.issues_scroll / per_page) + 1;
        let total_pages = (filtered_count as usize).div_ceil(per_page).max(1);

        // Check if issues were truncated (limited by API fetch)
        let truncated =
            snap.issues.truncated || (snap.repo.open_issues_count > snap.issues.total_open);
        let trunc_indicator = if truncated { "+" } else { "" };

        let title = if has_filter {
            format!(
                " Issues — Showing {} of {}{} (page {}/{}) ",
                filtered_count, display_total, trunc_indicator, current_page, total_pages
            )
        } else if self.selected_panel == Panel::Issues {
            format!(
                " Issues — {}{} open (page {}/{}) [+/- to change] ",
                display_total, trunc_indicator, current_page, total_pages
            )
        } else {
            format!(
                " Issues — {}{} open (page {}/{}) ",
                display_total, trunc_indicator, current_page, total_pages
            )
        };

        let block = self.panel_block(Panel::Issues, title);

        // Sort by creation date (oldest first)
        let mut sorted_issues = filtered_issues;
        sorted_issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let header = Row::new(vec!["#", "Title", "Author", "Age"])
            .style(Style::default().fg(self.theme.text_primary_color()).bold())
            .bottom_margin(0);

        // Apply scroll offset with dynamic per-page count
        let visible_issues: Vec<&Issue> = sorted_issues
            .iter()
            .skip(self.issues_scroll)
            .take(per_page)
            .copied()
            .collect();

        let rows: Vec<Row> = visible_issues
            .iter()
            .map(|issue| {
                let age = format_age(issue.created_at);
                Row::new(vec![
                    format!("#{}", issue.number),
                    truncate(&issue.title, 35),
                    truncate(&issue.author, 12),
                    age,
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(7),
            Constraint::Fill(1),
            Constraint::Length(13),
            Constraint::Length(8),
        ];

        // Show message if no matches
        if sorted_issues.is_empty() && has_filter {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let no_match = Paragraph::new("No matching issues")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, inner);
            return;
        }

        let table = Table::new(rows, widths).header(header).block(block);
        frame.render_widget(table, area);
    }

    pub(super) fn render_prs(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::PullRequests, " Pull Requests ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let pr = &snap.pull_requests;

        // Use animated counters for PR counts if animation is enabled
        let display_open = if self.animation_config.is_count_up_enabled() {
            self.get_counter_value("prs", pr.open_count)
        } else {
            pr.open_count
        };

        // Build title with per-page indicator and [+/-] hint when selected
        let per_page = self.prs_per_page;
        let title = if self.selected_panel == Panel::PullRequests {
            format!(" Pull Requests ({} per page) [+/- to change] ", per_page)
        } else {
            format!(" Pull Requests ({} per page) ", per_page)
        };

        let block = self.panel_block(Panel::PullRequests, title);

        let merge_time = pr
            .avg_time_to_merge_hours
            .map(|h| format!("{:.1}h", h))
            .unwrap_or_else(|| "N/A".to_string());

        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Open:    ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", display_open),
                    Style::default().fg(self.theme.indicator_success_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Draft:   ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", pr.draft_count),
                    Style::default().fg(self.theme.indicator_warning_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Ready:   ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", pr.ready_count),
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Merged:  ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{} (30d)", pr.merged_last_30d.len()),
                    Style::default().fg(self.theme.indicator_error_color()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Avg merge: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    merge_time,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    pub(super) fn render_contributors(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Contributors, " Contributors ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered contributors
        let filtered_contributors = self.get_filtered_contributors();
        let total_count = snap.contributors.total_unique;
        let filtered_count = filtered_contributors.len() as u64;

        // Use animated counter for contributor count if animation is in progress
        let display_total = if self.animation_config.is_count_up_enabled() {
            self.get_counter_value("contributors", total_count)
        } else {
            total_count
        };

        // Build title with limit indicator and [+/-] hint when selected
        let has_filter = !self.search_query.is_empty();
        let limit_label = self.contributors_limit.label();
        let title = if has_filter {
            format!(
                " Contributors — Showing {} of {} ({} view) ",
                filtered_count, display_total, limit_label
            )
        } else if self.selected_panel == Panel::Contributors {
            format!(
                " Contributors — {} total ({} view) [+/- to change] ",
                display_total, limit_label
            )
        } else {
            format!(
                " Contributors — {} total ({} view) ",
                display_total, limit_label
            )
        };

        let block = self.panel_block(Panel::Contributors, title);
        let contrib = &snap.contributors;

        // Apply scroll offset with dynamic limit
        let limit_count = self.contributors_limit.count();
        let mut lines: Vec<Line> = filtered_contributors
            .iter()
            .skip(self.contributors_scroll)
            .take(limit_count)
            .enumerate()
            .map(|(i, c)| {
                Line::from(vec![
                    Span::styled(
                        format!("{}. ", i + self.contributors_scroll + 1),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        c.username.as_str(),
                        Style::default().fg(self.theme.text_primary_color()),
                    ),
                    Span::styled(
                        format!(" ({})", c.commit_count),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                ])
            })
            .collect();

        // Show message if no matches
        if filtered_contributors.is_empty() && has_filter {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let no_match = Paragraph::new("No matching contributors")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, inner);
            return;
        }

        if !contrib.new_contributors_last_30d.is_empty() && !has_filter {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} new (30d)", contrib.new_contributors_last_30d.len()),
                Style::default().fg(self.theme.indicator_success_color()),
            )));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub(super) fn render_releases(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Releases, " Releases ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered releases
        let filtered_releases = self.get_filtered_releases();
        let total_count = snap.releases.len() as u64;
        let filtered_count = filtered_releases.len() as u64;

        // Use animated counter for releases count if animation is in progress
        let display_total = if self.animation_config.is_count_up_enabled() {
            self.get_counter_value("releases", total_count)
        } else {
            total_count
        };

        // Build title with limit indicator and [+/-] hint when selected
        let has_filter = !self.search_query.is_empty() || self.releases_prerelease_filter.is_some();
        let limit_count = self.releases_limit.count();
        let showing_count = filtered_releases.len().min(limit_count);
        let title = if has_filter {
            format!(
                " Releases — {} of {} matches (showing {}) ",
                filtered_count, display_total, showing_count
            )
        } else if self.selected_panel == Panel::Releases {
            format!(
                " Releases — {} total (showing {}) [+/- to change] ",
                display_total, showing_count
            )
        } else {
            format!(
                " Releases — {} total (showing {}) ",
                display_total, showing_count
            )
        };

        let block = self.panel_block(Panel::Releases, title);

        if snap.releases.is_empty() {
            let paragraph = Paragraph::new("No releases found").block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        // Show message if no matches
        if filtered_releases.is_empty() && has_filter {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let no_match = Paragraph::new("No matching releases")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, inner);
            return;
        }

        // Apply scroll offset with dynamic limit
        let limit_count = self.releases_limit.count();
        let mut lines: Vec<Line> = filtered_releases
            .iter()
            .skip(self.releases_scroll)
            .take(limit_count)
            .map(|r| {
                let name = r.name.as_deref().unwrap_or(&r.tag_name);
                let age = r
                    .days_since
                    .map(|d| format!("{}d ago", d))
                    .unwrap_or_default();

                let mut spans = vec![Span::styled(
                    name,
                    Style::default().fg(self.theme.text_primary_color()),
                )];

                if !age.is_empty() {
                    spans.push(Span::styled(
                        format!(" — {}", age),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ));
                }
                if r.prerelease {
                    spans.push(Span::styled(
                        " [pre]",
                        Style::default().fg(self.theme.indicator_warning_color()),
                    ));
                }

                Line::from(spans)
            })
            .collect();

        if let Some(avg) = snap.releases.first().and_then(|r| r.avg_interval) {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Avg interval: {:.0}d", avg),
                Style::default().fg(self.theme.text_secondary_color()),
            )));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub(super) fn render_velocity(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Velocity, " Velocity ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Build title with timeframe indicator, add [+/-] hint when selected
        let timeframe_label = self.velocity_timeframe.label();
        let title = if self.selected_panel == Panel::Velocity {
            format!(" Velocity ({}) [+/- to change] ", timeframe_label)
        } else {
            format!(" Velocity ({}) ", timeframe_label)
        };

        let block = self.panel_block(Panel::Velocity, title);
        let vel = &snap.velocity;
        let mut lines = Vec::new();

        lines.push(Line::from(Span::styled(
            "Issues (opened/closed):",
            Style::default().fg(self.theme.text_primary_color()).bold(),
        )));
        let weeks_to_show = self.velocity_timeframe.count();
        for week in vel.issues_weekly.iter().rev().take(weeks_to_show) {
            let week_label = week.week_start.format("%m/%d").to_string();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", week_label),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("+{}", week.opened),
                    Style::default().fg(self.theme.indicator_success_color()),
                ),
                Span::styled("/", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(
                    format!("-{}", week.closed),
                    Style::default().fg(self.theme.indicator_error_color()),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "PRs (opened/merged):",
            Style::default().fg(self.theme.text_primary_color()).bold(),
        )));
        for week in vel.prs_weekly.iter().rev().take(weeks_to_show) {
            let week_label = week.week_start.format("%m/%d").to_string();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", week_label),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("+{}", week.opened),
                    Style::default().fg(self.theme.indicator_success_color()),
                ),
                Span::styled("/", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(
                    format!("-{}", week.closed),
                    Style::default().fg(self.theme.indicator_error_color()),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub(super) fn render_security(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::Security, " Security ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = self.panel_block(Panel::Security, " Security ".to_string());

        let Some(ref sec) = snap.security_alerts else {
            let paragraph =
                Paragraph::new("Dependabot alerts\nnot available\n(disabled or no access)")
                    .style(Style::default().fg(self.theme.text_secondary_color()))
                    .block(block);
            frame.render_widget(paragraph, area);
            return;
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "Total:    ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", sec.total_open),
                    if sec.total_open > 0 {
                        Style::default()
                            .fg(self.theme.severity_critical_color())
                            .bold()
                    } else {
                        Style::default().fg(self.theme.indicator_success_color())
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Critical: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", sec.critical_count),
                    self.severity_style(sec.critical_count, self.theme.severity_critical_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "High:     ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", sec.high_count),
                    self.severity_style(sec.high_count, self.theme.severity_high_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Medium:   ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", sec.medium_count),
                    self.severity_style(sec.medium_count, self.theme.severity_medium_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Low:      ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", sec.low_count),
                    self.severity_style(sec.low_count, self.theme.severity_low_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    pub(super) fn render_ci(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = self.panel_block(Panel::CI, " CI Status ".to_string());
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = self.panel_block(Panel::CI, " CI Status ".to_string());

        let Some(ref ci) = snap.ci_status else {
            let paragraph = Paragraph::new("GitHub Actions\nnot available\n(disabled or no runs)")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .block(block);
            frame.render_widget(paragraph, area);
            return;
        };

        // Format success rate with color
        let success_rate_style = if ci.success_rate >= 90.0 {
            Style::default()
                .fg(self.theme.indicator_success_color())
                .bold()
        } else if ci.success_rate >= 70.0 {
            Style::default().fg(self.theme.indicator_warning_color())
        } else {
            Style::default()
                .fg(self.theme.indicator_error_color())
                .bold()
        };

        // Format average duration
        let avg_duration = if ci.avg_duration_seconds < 60 {
            format!("{}s", ci.avg_duration_seconds)
        } else if ci.avg_duration_seconds < 3600 {
            format!("{}m", ci.avg_duration_seconds / 60)
        } else {
            format!(
                "{}h {}m",
                ci.avg_duration_seconds / 3600,
                (ci.avg_duration_seconds % 3600) / 60
            )
        };

        // Get last run status with icon
        let (last_status_icon, last_status_text) = ci
            .recent_runs
            .first()
            .map(|run| {
                let icon = match run.conclusion.as_deref() {
                    Some("success") => ("✓", self.theme.indicator_success_color()),
                    Some("failure") => ("✗", self.theme.indicator_error_color()),
                    Some("cancelled") => ("⊘", self.theme.indicator_warning_color()),
                    _ => ("○", self.theme.text_secondary_color()),
                };
                let status_text = match run.conclusion.as_deref() {
                    Some("success") => "pass",
                    Some("failure") => "fail",
                    Some("cancelled") => "cancelled",
                    _ => "running",
                };
                (icon.0, icon.1, status_text)
            })
            .map(|(icon, color, text)| {
                (Span::styled(icon, Style::default().fg(color).bold()), text)
            })
            .unwrap_or_else(|| {
                (
                    Span::styled("—", Style::default().fg(self.theme.text_secondary_color())),
                    "N/A",
                )
            });

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "Success:  ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(format!("{:.1}%", ci.success_rate), success_rate_style),
            ]),
            Line::from(vec![
                Span::styled(
                    "Last:     ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                last_status_icon,
                Span::styled(
                    format!(" {}", last_status_text),
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Runs:     ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{} (30d)", ci.total_runs_30d),
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Avg time: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    avg_duration,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}
