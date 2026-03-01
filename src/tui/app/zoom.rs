use ratatui::{prelude::*, widgets::*};

use crate::core::metrics::stars::predict_milestone;
use crate::core::models::{Contributor, Issue, MergedPr, Release};
use crate::tui::widgets::BrailleSparkline;

use super::utils::{centered_rect, format_count, format_age, truncate, trim_leading_zeros, resample_to_width};
use super::{App, Panel};

impl App {
    pub(super) fn render_zoom_overlay(&self, frame: &mut Frame, panel: Panel) {
        let area = centered_rect(80, 80, frame.area());
        frame.render_widget(Clear, area);

        match panel {
            Panel::Stars => self.render_zoom_stars(frame, area),
            Panel::Issues => self.render_zoom_issues(frame, area),
            Panel::PullRequests => self.render_zoom_prs(frame, area),
            Panel::Contributors => self.render_zoom_contributors(frame, area),
            Panel::Releases => self.render_zoom_releases(frame, area),
            Panel::Velocity => self.render_zoom_velocity(frame, area),
            Panel::Security => self.render_zoom_security(frame, area),
            Panel::CI => self.render_zoom_ci(frame, area),
        }
    }

    /// Zoomed Stars panel with 365-day chart and full detail
    fn render_zoom_stars(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" ★ Stars (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = Block::bordered()
            .title(format!(" ★ Stars — {} (Zoom) — Press Enter/Esc to close ", format_count(snap.stars.total_count)))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into sections: info, 30d chart, 90d chart, 365d chart
        let [info_area, chart_30d_area, chart_90d_area, chart_365d_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // Info line with total, timeframes, and health score
        let total_30d: u64 = snap.stars.sparkline_30d.iter().map(|&v| v as u64).sum();
        let total_90d: u64 = snap.stars.sparkline_90d.iter().map(|&v| v as u64).sum();
        let total_365d: u64 = snap.stars.sparkline_365d.iter().map(|&v| v as u64).sum();

        // Build info lines with health score if available
        let mut info_lines: Vec<Line> = vec![Line::from(vec![
            Span::styled("Total: ", Style::default().fg(self.theme.text_secondary_color())),
            Span::styled(format_count(snap.stars.total_count), Style::default().fg(self.theme.text_primary_color()).bold()),
            Span::styled(" | 30d: ", Style::default().fg(self.theme.text_secondary_color())),
            Span::styled(format_count(total_30d), Style::default().fg(self.theme.sparkline_color())),
            Span::styled(" | 90d: ", Style::default().fg(self.theme.text_secondary_color())),
            Span::styled(format_count(total_90d), Style::default().fg(self.theme.sparkline_color())),
            Span::styled(" | 1y: ", Style::default().fg(self.theme.text_secondary_color())),
            Span::styled(format_count(total_365d), Style::default().fg(self.theme.sparkline_color())),
        ])];

        // Add health score line if available
        if let Some(ref health) = self.health_score {
            let health_color = match health.grade {
                crate::core::health::HealthGrade::Excellent => self.theme.indicator_success_color(),
                crate::core::health::HealthGrade::Good => self.theme.text_highlight_color(),
                crate::core::health::HealthGrade::Fair => self.theme.indicator_warning_color(),
                crate::core::health::HealthGrade::NeedsAttention => self.theme.indicator_warning_color(),
                crate::core::health::HealthGrade::Critical => self.theme.indicator_error_color(),
            };
            info_lines.push(Line::from(vec![
                Span::styled("Health: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(
                    format!("{}/100 ({} - {})", health.total, health.grade.as_letter(), health.grade.as_label()),
                    Style::default().fg(health_color).bold(),
                ),
            ]));
        }

        // Add milestone prediction line
        if let Some(prediction) = predict_milestone(&snap.stars) {
            let milestone_str = format_count(prediction.next_milestone);
            let days_str = if prediction.estimated_days == 1 {
                "1 day".to_string()
            } else {
                format!("{} days", prediction.estimated_days)
            };
            let rate_str = format!("{:.1}/day", prediction.daily_rate);

            info_lines.push(Line::from(vec![
                Span::styled("Next milestone: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(
                    format!("{}★ in {} ({})", milestone_str, days_str, rate_str),
                    Style::default().fg(self.theme.indicator_success_color()).bold(),
                ),
            ]));
        } else {
            info_lines.push(Line::from(vec![
                Span::styled("Next milestone: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(
                    "Growth stalled".to_string(),
                    Style::default().fg(self.theme.indicator_warning_color()),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(info_lines), info_area);

        // Helper to render a sparkline section
        let render_sparkline_section = |frame: &mut Frame, area: Rect, title: &str, data: &[u32], trim_zeros: bool| {
            let [label_area, spark_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(area);

            let label = Paragraph::new(title)
                .style(Style::default().fg(self.theme.text_secondary_color()));
            frame.render_widget(label, label_area);

            let data_u64: Vec<u64> = data.iter().map(|&v| v as u64).collect();
            let trimmed = if trim_zeros {
                trim_leading_zeros(&data_u64)
            } else {
                data_u64
            };

            if !trimmed.is_empty() {
                let width = spark_area.width as usize;
                let resampled = resample_to_width(&trimmed, width.max(10));

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
        };

        render_sparkline_section(frame, chart_30d_area, "30-day trend (daily)", &snap.stars.sparkline_30d, true);
        render_sparkline_section(frame, chart_90d_area, "90-day trend (weekly)", &snap.stars.sparkline_90d, true);

        // For 1-year view: show appropriate label based on data granularity
        let bucket_count = snap.stars.sparkline_365d.len().max(1);
        let label_365d = if bucket_count <= 13 {
            format!("{}-week history (since creation)", bucket_count)
        } else {
            "1-year trend (monthly)".to_string()
        };
        render_sparkline_section(frame, chart_365d_area, &label_365d, &snap.stars.sparkline_365d, false);
    }

    /// Zoomed Issues panel with full table (number, title, author, labels, age, comments)
    fn render_zoom_issues(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Issues (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered issues
        let filtered_issues = self.get_filtered_issues();
        let total_count = snap.issues.total_open;
        let filtered_count = filtered_issues.len() as u64;

        // Build title with count indicator if filtering is active, sort indicator, and truncation warning
        let has_filter = !self.search_query.is_empty() || self.issues_label_filter.is_some();
        let sort_label = self.issues_sort.label();
        let truncated = snap.issues.truncated || (snap.repo.open_issues_count > snap.issues.total_open);
        let trunc_indicator = if truncated { "+" } else { "" };
        let title = if has_filter {
            format!(" Issues — Showing {} of {}{} (sorted by: {}) — Press Enter/Esc to close ", filtered_count, total_count, trunc_indicator, sort_label)
        } else {
            format!(" Issues — {}{} open (sorted by: {}) — Press Enter/Esc to close ", total_count, trunc_indicator, sort_label)
        };

        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Sort based on selected sort order
        let mut sorted_issues = filtered_issues;
        match self.issues_sort {
            super::IssuesSort::Number => {
                sorted_issues.sort_by(|a, b| a.number.cmp(&b.number));
            }
            super::IssuesSort::Title => {
                sorted_issues.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            }
            super::IssuesSort::Author => {
                sorted_issues.sort_by(|a, b| a.author.to_lowercase().cmp(&b.author.to_lowercase()));
            }
            super::IssuesSort::Age => {
                sorted_issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            }
            super::IssuesSort::Comments => {
                sorted_issues.sort_by(|a, b| b.comments_count.cmp(&a.comments_count)); // Descending
            }
        }

        // Header with all columns including labels and comments
        let header = Row::new(vec!["#", "Title", "Author", "Labels", "Age", "Comments"])
            .style(Style::default().fg(self.theme.text_primary_color()).bold())
            .bottom_margin(0);

        // Calculate how many rows can fit
        let row_height = 1;
        let header_height = 1;
        let available_height = inner.height.saturating_sub(header_height + 2); // +2 for padding
        let visible_count = (available_height / row_height) as usize;

        // Apply zoom scroll offset
        let visible_issues: Vec<&Issue> = sorted_issues
            .iter()
            .skip(self.zoom_issues_scroll)
            .take(visible_count.max(5))
            .copied()
            .collect();

        let rows: Vec<Row> = visible_issues
            .iter()
            .map(|issue| {
                let age = format_age(issue.created_at);
                let labels = if issue.labels.is_empty() {
                    "—".to_string()
                } else {
                    issue.labels.join(", ")
                };
                Row::new(vec![
                    format!("#{}", issue.number),
                    truncate(&issue.title, 50),
                    truncate(&issue.author, 15),
                    truncate(&labels, 25),
                    age,
                    format!("{}", issue.comments_count),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(8),
            Constraint::Fill(2),
            Constraint::Length(16),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        // Show message if no matches
        if sorted_issues.is_empty() && has_filter {
            let no_match = Paragraph::new("No matching issues")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, inner);
            return;
        }

        let table = Table::new(rows, widths).header(header);
        frame.render_widget(table, inner);
    }

    /// Zoomed PRs panel with merge time distribution view
    fn render_zoom_prs(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Pull Requests (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = Block::bordered()
            .title(" Pull Requests (Zoom) — Press Enter/Esc to close ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let pr = &snap.pull_requests;

        // Split into summary and merged PRs list
        let [summary_area, merged_area] = Layout::vertical([
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // Summary section
        let merge_time = pr
            .avg_time_to_merge_hours
            .map(|h| format!("{:.1}h", h))
            .unwrap_or_else(|| "N/A".to_string());

        let summary_text = vec![
            Line::from(vec![
                Span::styled("Open:    ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{}", pr.open_count), Style::default().fg(self.theme.indicator_success_color())),
            ]),
            Line::from(vec![
                Span::styled("Draft:   ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{}", pr.draft_count), Style::default().fg(self.theme.indicator_warning_color())),
            ]),
            Line::from(vec![
                Span::styled("Ready:   ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{}", pr.ready_count), Style::default().fg(self.theme.indicator_info_color())),
            ]),
            Line::from(vec![
                Span::styled("Merged:  ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{} (30d)", pr.merged_last_30d.len()), Style::default().fg(self.theme.indicator_error_color())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Avg merge time: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(merge_time, Style::default().fg(self.theme.text_primary_color()).bold()),
            ]),
        ];
        frame.render_widget(Paragraph::new(summary_text), summary_area);

        // Merged PRs list with merge times
        if !pr.merged_last_30d.is_empty() {
            let header = Row::new(vec!["#", "Title", "Author", "Merge Time"])
                .style(Style::default().fg(self.theme.text_primary_color()).bold());

            // Calculate visible rows
            let available_height = merged_area.height.saturating_sub(2);
            let visible_count = available_height as usize;

            let visible_prs: Vec<&MergedPr> = pr.merged_last_30d
                .iter()
                .skip(self.zoom_prs_scroll)
                .take(visible_count.max(3))
                .collect();

            let rows: Vec<Row> = visible_prs
                .iter()
                .map(|p| {
                    let time_str = if p.time_to_merge_hours < 1.0 {
                        format!("{:.0}m", p.time_to_merge_hours * 60.0)
                    } else if p.time_to_merge_hours < 24.0 {
                        format!("{:.1}h", p.time_to_merge_hours)
                    } else {
                        format!("{:.1}d", p.time_to_merge_hours / 24.0)
                    };
                    Row::new(vec![
                        format!("#{}", p.number),
                        truncate(&p.title, 45),
                        truncate(&p.author, 15),
                        time_str,
                    ])
                })
                .collect();

            let widths = [
                Constraint::Length(8),
                Constraint::Fill(2),
                Constraint::Length(16),
                Constraint::Length(12),
            ];

            let table = Table::new(rows, widths).header(header);
            frame.render_widget(table, merged_area);
        } else {
            frame.render_widget(
                Paragraph::new("No merged PRs in last 30 days")
                    .style(Style::default().fg(self.theme.text_secondary_color())),
                merged_area,
            );
        }
    }

    /// Zoomed Contributors panel with paginated list
    fn render_zoom_contributors(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Contributors (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered contributors
        let filtered_contributors = self.get_filtered_contributors();
        let total_count = snap.contributors.total_unique;
        let filtered_count = filtered_contributors.len() as u64;

        // Build title with count indicator if filtering is active
        let has_filter = !self.search_query.is_empty();
        let title = if has_filter {
            format!(" Contributors — Showing {} of {} (Zoom) — Press Enter/Esc to close ", filtered_count, total_count)
        } else {
            format!(" Contributors — {} (Zoom) — Press Enter/Esc to close ", total_count)
        };

        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let contrib = &snap.contributors;

        // Split into summary and list
        let [summary_area, list_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // Summary
        let new_contrib_text = if contrib.new_contributors_last_30d.is_empty() || has_filter {
            if has_filter {
                format!("{} matches", filtered_count)
            } else {
                "0 new".to_string()
            }
        } else {
            format!("{} new (30d)", contrib.new_contributors_last_30d.len())
        };

        let summary_text = vec![
            Line::from(vec![
                Span::styled("Total unique: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{}", contrib.total_unique), Style::default().fg(self.theme.text_primary_color()).bold()),
                Span::styled(" | ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(new_contrib_text, Style::default().fg(self.theme.indicator_success_color())),
            ]),
        ];
        frame.render_widget(Paragraph::new(summary_text), summary_area);

        // Show message if no matches
        if filtered_contributors.is_empty() && has_filter {
            let no_match = Paragraph::new("No matching contributors")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, list_area);
            return;
        }

        // Contributor list - show all top contributors with scrolling
        if !filtered_contributors.is_empty() {
            let header = Row::new(vec!["Rank", "Username", "Commits"])
                .style(Style::default().fg(self.theme.text_primary_color()).bold());

            // Calculate visible rows
            let available_height = list_area.height.saturating_sub(2);
            let visible_count = available_height as usize;

            // Use into_iter() to own the values
            let visible_contributors: Vec<(usize, &Contributor)> = filtered_contributors
                .into_iter()
                .enumerate()
                .skip(self.zoom_contributors_scroll)
                .take(visible_count.max(5))
                .collect();

            let rows: Vec<Row> = visible_contributors
                .into_iter()
                .map(|(i, c)| {
                    Row::new(vec![
                        format!("{}.", i + 1 + self.zoom_contributors_scroll),
                        c.username.clone(),
                        format!("{}", c.commit_count),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Length(6),
                Constraint::Fill(1),
                Constraint::Length(10),
            ];

            let table = Table::new(rows, widths).header(header);
            frame.render_widget(table, list_area);
        } else {
            frame.render_widget(
                Paragraph::new("No contributors data")
                    .style(Style::default().fg(self.theme.text_secondary_color())),
                list_area,
            );
        }
    }

    /// Zoomed Releases panel with detailed list
    fn render_zoom_releases(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Releases (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        // Get filtered releases
        let filtered_releases = self.get_filtered_releases();
        let total_count = snap.releases.len() as u64;
        let filtered_count = filtered_releases.len() as u64;

        // Build title with count indicator if filtering is active
        let has_filter = !self.search_query.is_empty() || self.releases_prerelease_filter.is_some();
        let title = if has_filter {
            format!(" Releases — Showing {} of {} (Zoom) — Press Enter/Esc to close ", filtered_count, total_count)
        } else {
            format!(" Releases — {} total (Zoom) — Press Enter/Esc to close ", total_count)
        };

        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if snap.releases.is_empty() {
            frame.render_widget(
                Paragraph::new("No releases found")
                    .style(Style::default().fg(self.theme.text_secondary_color())),
                inner,
            );
            return;
        }

        // Show message if no matches
        if filtered_releases.is_empty() && has_filter {
            let no_match = Paragraph::new("No matching releases")
                .style(Style::default().fg(self.theme.text_secondary_color()))
                .alignment(Alignment::Center);
            frame.render_widget(no_match, inner);
            return;
        }

        // Header
        let header = Row::new(vec!["Version", "Name", "Published", "Status"])
            .style(Style::default().fg(self.theme.text_primary_color()).bold());

        // Calculate visible rows
        let available_height = inner.height.saturating_sub(2);
        let visible_count = available_height as usize;

        // Use into_iter() to own the values
        let visible_releases: Vec<&Release> = filtered_releases
            .into_iter()
            .skip(self.zoom_releases_scroll)
            .take(visible_count.max(5))
            .collect();

        let rows: Vec<Row> = visible_releases
            .into_iter()
            .map(|r| {
                let name = r.name.as_deref().unwrap_or("—");
                let age = r.days_since.map(|d| format!("{}d ago", d)).unwrap_or_else(|| "—".to_string());
                let status = if r.prerelease {
                    "pre-release"
                } else if r.draft {
                    "draft"
                } else {
                    "stable"
                };
                Row::new(vec![
                    r.tag_name.clone(),
                    truncate(name, 40),
                    age,
                    status.to_string(),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(20),
            Constraint::Fill(1),
            Constraint::Length(12),
            Constraint::Length(12),
        ];

        let table = Table::new(rows, widths).header(header);
        frame.render_widget(table, inner);
    }

    /// Zoomed Velocity panel with full 8-week view
    fn render_zoom_velocity(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Velocity (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = Block::bordered()
            .title(" Velocity — 8 weeks (Zoom) — Press Enter/Esc to close ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let vel = &snap.velocity;

        // Split into issues and PRs sections
        let [issues_area, prs_area] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
            .areas(inner);

        // Issues section
        let mut issues_lines = vec![
            Line::from(Span::styled("Issues (opened/closed)", Style::default().fg(self.theme.text_primary_color()).bold())),
            Line::from(""),
        ];

        for week in vel.issues_weekly.iter().rev() {
            let week_label = week.week_start.format("%Y-%m-%d").to_string();
            issues_lines.push(Line::from(vec![
                Span::styled(format!("{} ", week_label), Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("+{}", week.opened), Style::default().fg(self.theme.indicator_success_color())),
                Span::styled(" / ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("-{}", week.closed), Style::default().fg(self.theme.indicator_error_color())),
            ]));
        }

        frame.render_widget(Paragraph::new(issues_lines), issues_area);

        // PRs section
        let mut prs_lines = vec![
            Line::from(Span::styled("PRs (opened/merged)", Style::default().fg(self.theme.text_primary_color()).bold())),
            Line::from(""),
        ];

        for week in vel.prs_weekly.iter().rev() {
            let week_label = week.week_start.format("%Y-%m-%d").to_string();
            prs_lines.push(Line::from(vec![
                Span::styled(format!("{} ", week_label), Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("+{}", week.opened), Style::default().fg(self.theme.indicator_success_color())),
                Span::styled(" / ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("-{}", week.closed), Style::default().fg(self.theme.indicator_error_color())),
            ]));
        }

        frame.render_widget(Paragraph::new(prs_lines), prs_area);
    }

    /// Zoomed Security panel with detailed alert view and community health
    fn render_zoom_security(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" Security (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let block = Block::bordered()
            .title(" Security (Zoom) — Press Enter/Esc to close ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into security alerts and community health sections
        let [security_area, community_area] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
            .areas(inner);

        // Security alerts section
        if let Some(ref sec) = snap.security_alerts {
            let security_lines = vec![
                Line::from(vec![
                    Span::styled("Total Open: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}", sec.total_open),
                        if sec.total_open > 0 {
                            Style::default().fg(self.theme.severity_critical_color()).bold()
                        } else {
                            Style::default().fg(self.theme.indicator_success_color())
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled("Severity Breakdown:", Style::default().fg(self.theme.text_primary_color()).bold())),
                Line::from(vec![
                    Span::styled("  Critical: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}", sec.critical_count),
                        self.severity_style(sec.critical_count, self.theme.severity_critical_color()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  High:     ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}", sec.high_count),
                        self.severity_style(sec.high_count, self.theme.severity_high_color()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Medium:   ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}", sec.medium_count),
                        self.severity_style(sec.medium_count, self.theme.severity_medium_color()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Low:      ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}", sec.low_count),
                        self.severity_style(sec.low_count, self.theme.severity_low_color()),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(security_lines), security_area);
        } else {
            let no_security = Paragraph::new(
                "Dependabot alerts not available\n(disabled or no access)"
            )
            .style(Style::default().fg(self.theme.text_secondary_color()))
            .alignment(Alignment::Center);
            frame.render_widget(no_security, security_area);
        }

        // Community Health section
        let community_lines = if let Some(ref health) = snap.community_health {
            vec![
                Line::from(Span::styled("Community Health:", Style::default().fg(self.theme.text_primary_color()).bold())),
                Line::from(vec![
                    Span::styled("Score: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(
                        format!("{}/100", health.score),
                        if health.score >= 75 {
                            Style::default().fg(self.theme.indicator_success_color()).bold()
                        } else if health.score >= 50 {
                            Style::default().fg(self.theme.indicator_warning_color())
                        } else {
                            Style::default().fg(self.theme.indicator_error_color()).bold()
                        },
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled("Health Checklist:", Style::default().fg(self.theme.text_secondary_color()))),
                Line::from(vec![
                    Span::styled(if health.has_readme { "✓" } else { "✗" },
                        if health.has_readme { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" README.md", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_license { "✓" } else { "✗" },
                        if health.has_license { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" LICENSE", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_contributing { "✓" } else { "✗" },
                        if health.has_contributing { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" CONTRIBUTING.md", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_code_of_conduct { "✓" } else { "✗" },
                        if health.has_code_of_conduct { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" CODE_OF_CONDUCT.md", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_issue_templates { "✓" } else { "✗" },
                        if health.has_issue_templates { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" Issue Templates", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_pr_template { "✓" } else { "✗" },
                        if health.has_pr_template { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" PR Template", Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled(if health.has_security_policy { "✓" } else { "✗" },
                        if health.has_security_policy { Style::default().fg(self.theme.indicator_success_color()) }
                        else { Style::default().fg(self.theme.indicator_error_color()) }),
                    Span::styled(" SECURITY.md", Style::default().fg(self.theme.text_primary_color())),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled("Community Health:", Style::default().fg(self.theme.text_primary_color()).bold())),
                Line::from(""),
                Line::from(
                    Span::styled("Community health data not available\n(insufficient permissions or API unavailable)",
                        Style::default().fg(self.theme.text_secondary_color()))
                ),
            ]
        };
        frame.render_widget(Paragraph::new(community_lines), community_area);
    }

    /// Zoomed CI panel with table of recent workflow runs
    fn render_zoom_ci(&self, frame: &mut Frame, area: Rect) {
        let Some(ref snap) = self.snapshot else {
            let block = Block::bordered()
                .title(" CI Status (Zoom) — Press Enter/Esc to close ")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(self.theme.help_border_color()));
            frame.render_widget(Paragraph::new("Loading...").block(block), area);
            return;
        };

        let Some(ref ci) = snap.ci_status else {
            let paragraph = Paragraph::new(
                "GitHub Actions not available\n(disabled or no access)"
            )
            .style(Style::default().fg(self.theme.text_secondary_color()))
            .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
            return;
        };

        // Build title with summary stats
        let title = format!(
            " CI Status — {:.1}% success ({} runs) — Press Enter/Esc to close ",
            ci.success_rate, ci.total_runs_30d
        );

        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into summary and runs list
        let [summary_area, runs_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // Summary section
        let avg_duration = if ci.avg_duration_seconds < 60 {
            format!("{}s", ci.avg_duration_seconds)
        } else if ci.avg_duration_seconds < 3600 {
            format!("{}m", ci.avg_duration_seconds / 60)
        } else {
            format!("{}h {}m", ci.avg_duration_seconds / 3600, (ci.avg_duration_seconds % 3600) / 60)
        };

        let success_rate_style = if ci.success_rate >= 90.0 {
            Style::default().fg(self.theme.indicator_success_color()).bold()
        } else if ci.success_rate >= 70.0 {
            Style::default().fg(self.theme.indicator_warning_color())
        } else {
            Style::default().fg(self.theme.indicator_error_color()).bold()
        };

        let summary_text = vec![
            Line::from(vec![
                Span::styled("Success Rate: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{:.1}%", ci.success_rate), success_rate_style),
            ]),
            Line::from(vec![
                Span::styled("Total Runs (30d): ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(format!("{}", ci.total_runs_30d), Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Avg Duration: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(avg_duration, Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(summary_text), summary_area);

        // Recent runs table
        if !ci.recent_runs.is_empty() {
            let header = Row::new(vec!["Workflow", "Status", "Conclusion", "Duration", "When"])
                .style(Style::default().fg(self.theme.text_primary_color()).bold());

            // Calculate visible rows
            let available_height = runs_area.height.saturating_sub(2);
            let visible_count = available_height as usize;

            let visible_runs: Vec<&crate::core::models::WorkflowRun> = ci.recent_runs
                .iter()
                .take(visible_count.max(5))
                .collect();

            let rows: Vec<Row> = visible_runs
                .iter()
                .map(|run| {
                    // Format status icon
                    let (status_icon, status_color) = match run.conclusion.as_deref() {
                        Some("success") => ("✓", self.theme.indicator_success_color()),
                        Some("failure") => ("✗", self.theme.indicator_error_color()),
                        Some("cancelled") => ("⊘", self.theme.indicator_warning_color()),
                        _ => ("○", self.theme.text_secondary_color()),
                    };

                    // Format duration
                    let duration = if run.duration_seconds < 60 {
                        format!("{}s", run.duration_seconds)
                    } else if run.duration_seconds < 3600 {
                        format!("{}m", run.duration_seconds / 60)
                    } else {
                        format!("{}h {}m", run.duration_seconds / 3600, (run.duration_seconds % 3600) / 60)
                    };

                    // Format when (age)
                    let when = crate::tui::app::utils::format_age(run.created_at);

                    // Format conclusion text
                    let conclusion = run.conclusion.as_deref().unwrap_or(&run.status);

                    Row::new(vec![
                        truncate(&run.name, 30),
                        format!("{} {}", status_icon, &run.status),
                        conclusion.to_string(),
                        duration,
                        when,
                    ])
                    .style(Style::default().fg(status_color))
                })
                .collect();

            let widths = [
                Constraint::Fill(2),
                Constraint::Length(15),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(10),
            ];

            let table = Table::new(rows, widths).header(header);
            frame.render_widget(table, runs_area);
        } else {
            frame.render_widget(
                Paragraph::new("No recent workflow runs")
                    .style(Style::default().fg(self.theme.text_secondary_color()))
                    .alignment(Alignment::Center),
                runs_area,
            );
        }
    }

    pub(super) fn severity_style(&self, count: u64, color: Color) -> Style {
        if count > 0 {
            Style::default().fg(color).bold()
        } else {
            Style::default().fg(self.theme.indicator_success_color())
        }
    }
}
