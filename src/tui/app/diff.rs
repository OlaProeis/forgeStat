use ratatui::{prelude::*, widgets::*};

use crate::core::models::{RepoSnapshot, SnapshotDiff};
use crate::tui::widgets::BrailleSparkline;
use super::utils::resample_to_width;
use super::App;

impl App {
    pub(super) fn render_diff_overlay(&mut self, frame: &mut Frame) {
        // Load previous snapshot if needed (first render)
        self.load_previous_snapshot();

        let area = frame.area();
        frame.render_widget(Clear, area);

        // Full-screen diff view
        let [header_area, content_area, status_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Render diff header
        self.render_diff_header(frame, header_area);

        // Render diff content (split screen)
        self.render_diff_content(frame, content_area);

        // Render diff status bar
        self.render_diff_status_bar(frame, status_area);
    }

    /// Render the diff mode header with "Last viewed X ago"
    fn render_diff_header(&self, frame: &mut Frame, area: Rect) {
        let diff_info = self
            .snapshot_diff
            .as_ref()
            .map(|d| format!("Last viewed {}", d.format_time_ago()))
            .unwrap_or_else(|| "No previous snapshot available".to_string());

        let block = Block::bordered()
            .title(" Diff Mode ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.header_border_color()));

        let header_text = format!("{}/{} — {} [Press Esc or 'd' to exit]", self.owner, self.repo, diff_info);

        let paragraph = Paragraph::new(header_text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render the split-screen diff content
    fn render_diff_content(&self, frame: &mut Frame, area: Rect) {
        let [left_area, right_area] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);

        // Left side: Current snapshot
        self.render_diff_side(frame, left_area, "Current", self.snapshot.as_ref(), true);

        // Right side: Previous snapshot
        self.render_diff_side(
            frame,
            right_area,
            "Previous",
            self.previous_snapshot.as_ref(),
            false,
        );
    }

    /// Render one side of the diff (current or previous)
    fn render_diff_side(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        snapshot: Option<&RepoSnapshot>,
        is_current: bool,
    ) {
        let Some(snap) = snapshot else {
            // No snapshot available
            let block = Block::bordered()
                .title(format!(" {} ", label))
                .border_style(Style::default().fg(self.theme.border_unselected_color()));

            let paragraph = Paragraph::new("No data available").block(block);
            frame.render_widget(paragraph, area);
            return;
        };

        // Get diff for change highlighting (only for current side)
        let diff = if is_current {
            self.snapshot_diff.as_ref()
        } else {
            None
        };

        // Split into sections: Stars, Issues, PRs, Security
        let [stars_area, issues_area, prs_area, security_area] = Layout::vertical([
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Fill(1),
        ])
        .areas(area);

        // Render diff stars panel
        self.render_diff_stars(frame, stars_area, snap, diff, label);

        // Render diff issues panel
        self.render_diff_issues(frame, issues_area, snap, diff, label);

        // Render diff PRs panel
        self.render_diff_prs(frame, prs_area, snap, diff, label);

        // Render diff security panel
        self.render_diff_security(frame, security_area, snap, diff, label);
    }

    /// Render stars panel in diff mode
    fn render_diff_stars(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        diff: Option<&SnapshotDiff>,
        label: &str,
    ) {
        let star_count = snap.stars.total_count;

        // Format with change indicator
        let (count_text, change_style) = if let Some(d) = diff {
            if d.stars_delta > 0 {
                (format!("{} (+{})", star_count, d.stars_delta), self.theme.text_success_color())
            } else if d.stars_delta < 0 {
                (format!("{} ({})", star_count, d.stars_delta), self.theme.text_error_color())
            } else {
                (star_count.to_string(), self.theme.text_primary_color())
            }
        } else {
            (star_count.to_string(), self.theme.text_primary_color())
        };

        let block = Block::bordered()
            .title(format!(" ★ Stars — {} ", label))
            .border_style(Style::default().fg(self.theme.border_unselected_color()));

        // Split area for sparkline
        let [text_area, spark_area] = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 0 }));

        let text = vec![
            Line::from(vec![
                Span::styled("Total: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(count_text, Style::default().fg(change_style).bold()),
            ]),
        ];

        let paragraph = Paragraph::new(text);
        frame.render_widget(paragraph, text_area);
        frame.render_widget(block.clone(), area);

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

    /// Render issues panel in diff mode
    fn render_diff_issues(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        diff: Option<&SnapshotDiff>,
        label: &str,
    ) {
        let issue_count = snap.issues.total_open;

        // Format with change indicator
        let (count_text, change_style) = if let Some(d) = diff {
            if d.issues_delta > 0 {
                (format!("{} (+{})", issue_count, d.issues_delta), self.theme.text_error_color())
            } else if d.issues_delta < 0 {
                (format!("{} ({})", issue_count, d.issues_delta), self.theme.text_success_color())
            } else {
                (issue_count.to_string(), self.theme.text_primary_color())
            }
        } else {
            (issue_count.to_string(), self.theme.text_primary_color())
        };

        let block = Block::bordered()
            .title(format!(" Issues — {} ", label))
            .border_style(Style::default().fg(self.theme.border_unselected_color()));

        // Count issues by label
        let label_count = snap.issues.by_label.len();
        let unlabelled_count = snap.issues.unlabelled.len();

        let text = vec![
            Line::from(vec![
                Span::styled("Open: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(count_text, Style::default().fg(change_style).bold()),
            ]),
            Line::from(vec![
                Span::styled("By label: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(label_count.to_string(), Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Unlabelled: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(unlabelled_count.to_string(), Style::default().fg(self.theme.text_primary_color())),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render PRs panel in diff mode
    fn render_diff_prs(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        diff: Option<&SnapshotDiff>,
        label: &str,
    ) {
        let pr_count = snap.pull_requests.open_count;

        // Format with change indicator
        let (count_text, change_style) = if let Some(d) = diff {
            if d.prs_delta > 0 {
                (format!("{} (+{})", pr_count, d.prs_delta), self.theme.text_highlight_color())
            } else if d.prs_delta < 0 {
                (format!("{} ({})", pr_count, d.prs_delta), self.theme.text_success_color())
            } else {
                (pr_count.to_string(), self.theme.text_primary_color())
            }
        } else {
            (pr_count.to_string(), self.theme.text_primary_color())
        };

        let block = Block::bordered()
            .title(format!(" Pull Requests — {} ", label))
            .border_style(Style::default().fg(self.theme.border_unselected_color()));

        let draft_count = snap.pull_requests.draft_count;
        let ready_count = snap.pull_requests.ready_count;
        let merged_30d = snap.pull_requests.merged_last_30d.len();

        let text = vec![
            Line::from(vec![
                Span::styled("Open: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(count_text, Style::default().fg(change_style).bold()),
            ]),
            Line::from(vec![
                Span::styled("Draft: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(draft_count.to_string(), Style::default().fg(self.theme.text_primary_color())),
                Span::styled(" | Ready: ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(ready_count.to_string(), Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Merged (30d): ", Style::default().fg(self.theme.text_secondary_color())),
                Span::styled(merged_30d.to_string(), Style::default().fg(self.theme.text_primary_color())),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Render security panel in diff mode
    fn render_diff_security(
        &self,
        frame: &mut Frame,
        area: Rect,
        snap: &RepoSnapshot,
        diff: Option<&SnapshotDiff>,
        label: &str,
    ) {
        let block = Block::bordered()
            .title(format!(" Security — {} ", label))
            .border_style(Style::default().fg(self.theme.border_unselected_color()));

        let security_text = if let Some(sec) = &snap.security_alerts {
            let total = sec.total_open;
            let critical = sec.critical_count;
            let high = sec.high_count;
            let medium = sec.medium_count;
            let low = sec.low_count;

            // Show new alerts indicator
            let new_alerts_text = if let Some(d) = diff {
                if d.has_new_security_alerts() {
                    format!(
                        " [+{} critical, +{} high, +{} medium, +{} low]",
                        d.new_security_critical, d.new_security_high, d.new_security_medium, d.new_security_low
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let new_alerts_style = if diff.map(|d| d.has_new_security_alerts()).unwrap_or(false) {
                self.theme.text_error_color()
            } else {
                self.theme.text_primary_color()
            };

            vec![
                Line::from(vec![
                    Span::styled("Total: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(total.to_string(), Style::default().fg(self.theme.text_primary_color()).bold()),
                    Span::styled(new_alerts_text, Style::default().fg(new_alerts_style).bold()),
                ]),
                Line::from(vec![
                    Span::styled("Critical: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(critical.to_string(), Style::default().fg(self.theme.text_error_color())),
                ]),
                Line::from(vec![
                    Span::styled("High: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(high.to_string(), Style::default().fg(self.theme.text_warning_color())),
                ]),
                Line::from(vec![
                    Span::styled("Medium: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(medium.to_string(), Style::default().fg(self.theme.text_primary_color())),
                ]),
                Line::from(vec![
                    Span::styled("Low: ", Style::default().fg(self.theme.text_secondary_color())),
                    Span::styled(low.to_string(), Style::default().fg(self.theme.text_secondary_color())),
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

    /// Render diff mode status bar
    fn render_diff_status_bar(&self, frame: &mut Frame, area: Rect) {
        let mut spans: Vec<Span> = vec![
            Span::styled("Diff Mode", Style::default().fg(self.theme.text_highlight_color()).bold()),
        ];

        if let Some(ref diff) = self.snapshot_diff {
            spans.push(Span::styled(" | ", Style::default().fg(self.theme.text_secondary_color())));

            // Stars indicator
            if diff.stars_delta > 0 {
                spans.push(Span::styled(
                    format!(" Stars +{} ", diff.stars_delta),
                    Style::default().fg(self.theme.text_success_color()).bold()
                ));
            } else if diff.stars_delta < 0 {
                spans.push(Span::styled(
                    format!(" Stars {} ", diff.stars_delta),
                    Style::default().fg(self.theme.text_error_color()).bold()
                ));
            }

            // Issues indicator
            if diff.issues_delta > 0 {
                spans.push(Span::styled(
                    format!(" Issues +{} ", diff.issues_delta),
                    Style::default().fg(self.theme.text_error_color()).bold()
                ));
            } else if diff.issues_delta < 0 {
                spans.push(Span::styled(
                    format!(" Issues {} ", diff.issues_delta),
                    Style::default().fg(self.theme.text_success_color()).bold()
                ));
            }

            // Security indicator
            if diff.has_new_security_alerts() {
                spans.push(Span::styled(
                    " NEW SECURITY ALERTS ",
                    Style::default().fg(self.theme.text_error_color()).bold().underlined()
                ));
            }

            spans.push(Span::styled(" | Press Esc or 'd' to exit", Style::default().fg(self.theme.text_secondary_color())));
        } else {
            spans.push(Span::styled(" | No previous snapshot to compare", Style::default().fg(self.theme.text_secondary_color())));
            spans.push(Span::styled(" | Press Esc or 'd' to exit", Style::default().fg(self.theme.text_secondary_color())));
        }

        let status_line = Line::from(spans);
        let paragraph = Paragraph::new(status_line).alignment(Alignment::Left);
        frame.render_widget(paragraph, area);
    }
}
