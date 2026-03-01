use ratatui::{prelude::*, widgets::*};

use super::utils::{centered_rect, format_count};
use super::App;

impl App {
    pub(super) fn render_mini_map_overlay(&self, frame: &mut Frame) {
        let area = centered_rect(85, 80, frame.area());
        frame.render_widget(Clear, area);

        let block = Block::bordered()
            .title(" Mini-Map Overview (m to close, 1-8 to jump) ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into health header + 8 panel rows
        let health_constraints = vec![Constraint::Length(2)];
        let panel_constraints: Vec<Constraint> = (0..8).map(|_| Constraint::Length(3)).collect();
        let all_constraints: Vec<Constraint> = health_constraints
            .into_iter()
            .chain(panel_constraints)
            .collect();
        let areas: Vec<Rect> = Layout::vertical(all_constraints).split(inner).to_vec();

        let Some(ref snap) = self.snapshot else {
            let loading = Paragraph::new("Loading...");
            frame.render_widget(loading, inner);
            return;
        };

        // Row 0: Health Score (prominent display)
        if let Some(ref health) = self.health_score {
            let (health_color, health_emoji) = match health.grade {
                crate::core::health::HealthGrade::Excellent => {
                    (self.theme.indicator_success_color(), "✓")
                }
                crate::core::health::HealthGrade::Good => (self.theme.text_highlight_color(), "👍"),
                crate::core::health::HealthGrade::Fair => {
                    (self.theme.indicator_warning_color(), "⚠")
                }
                crate::core::health::HealthGrade::NeedsAttention => {
                    (self.theme.indicator_warning_color(), "⚡")
                }
                crate::core::health::HealthGrade::Critical => {
                    (self.theme.indicator_error_color(), "🚨")
                }
            };
            let health_text = vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {} Health Score: ", health_emoji),
                        Style::default().fg(health_color).bold(),
                    ),
                    Span::styled(
                        format!("{}/100", health.total),
                        Style::default().fg(health_color).bold(),
                    ),
                    Span::styled(
                        format!(" (Grade {} — ", health.grade.as_letter()),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        health.grade.as_label(),
                        Style::default().fg(health_color).bold(),
                    ),
                    Span::styled(
                        ")",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("   Activity: {:>2}/25 | Community: {:>2}/25 | Maintenance: {:>2}/25 | Growth: {:>2}/25",
                            health.activity, health.community, health.maintenance, health.growth),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(health_text), areas[0]);
        } else {
            let health_text = Line::from(vec![
                Span::styled(
                    " Health Score: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    "Computing...",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]);
            frame.render_widget(Paragraph::new(health_text), areas[0]);
        }

        // Row 1: Stars panel metrics
        let stars_text = vec![
            Line::from(vec![
                Span::styled(
                    "1. ★ Stars",
                    Style::default().fg(self.theme.text_primary_color()).bold(),
                ),
                Span::styled(
                    format!(" — Total: {}", format_count(snap.stars.total_count)),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   30d: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format_count(snap.stars.sparkline_30d.iter().sum::<u32>() as u64),
                    Style::default().fg(self.theme.sparkline_color()),
                ),
                Span::styled(
                    " | 90d: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format_count(snap.stars.sparkline_90d.iter().sum::<u32>() as u64),
                    Style::default().fg(self.theme.sparkline_color()),
                ),
                Span::styled(
                    " | 1y: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format_count(snap.stars.sparkline_365d.iter().sum::<u32>() as u64),
                    Style::default().fg(self.theme.sparkline_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(stars_text), areas[1]);

        // Row 2: Issues panel metrics
        let oldest_issue = snap
            .oldest_issue_age_days()
            .map(|d| format!("{}d", d))
            .unwrap_or_else(|| "N/A".to_string());
        let issues_text = vec![
            Line::from(vec![
                Span::styled(
                    "2. Issues",
                    Style::default().fg(self.theme.text_primary_color()).bold(),
                ),
                Span::styled(
                    format!(" — Open: {}", snap.issues.total_open),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Labels: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", snap.issues.by_label.len()),
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
                Span::styled(
                    " | Unlabelled: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", snap.issues.unlabelled.len()),
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
                Span::styled(
                    " | Oldest: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    oldest_issue,
                    Style::default().fg(self.theme.indicator_warning_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(issues_text), areas[2]);

        // Row 3: Pull Requests panel metrics
        let pr = &snap.pull_requests;
        let merge_time = pr
            .avg_time_to_merge_hours
            .map(|h| format!("{:.1}h", h))
            .unwrap_or_else(|| "N/A".to_string());
        let prs_text = vec![
            Line::from(vec![
                Span::styled(
                    "3. Pull Requests",
                    Style::default().fg(self.theme.text_primary_color()).bold(),
                ),
                Span::styled(
                    format!(" — Open: {}", pr.open_count),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Draft: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", pr.draft_count),
                    Style::default().fg(self.theme.indicator_warning_color()),
                ),
                Span::styled(
                    " | Ready: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", pr.ready_count),
                    Style::default().fg(self.theme.indicator_success_color()),
                ),
                Span::styled(
                    " | Merged(30d): ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    format!("{}", pr.merged_last_30d.len()),
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
                Span::styled(
                    " | Avg: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    merge_time,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(prs_text), areas[3]);

        // Row 4: Contributors panel metrics
        let contrib = &snap.contributors;
        let new_contrib = if contrib.new_contributors_last_30d.is_empty() {
            "0".to_string()
        } else {
            format!("{} new", contrib.new_contributors_last_30d.len())
        };
        let top_contrib = contrib
            .top_contributors
            .first()
            .map(|c| format!("{} ({} commits)", c.username, c.commit_count))
            .unwrap_or_else(|| "N/A".to_string());
        let contributors_text = vec![
            Line::from(vec![
                Span::styled(
                    "4. Contributors",
                    Style::default().fg(self.theme.text_primary_color()).bold(),
                ),
                Span::styled(
                    format!(" — Total: {}", contrib.total_unique),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Top: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    top_contrib,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
                Span::styled(
                    " | 30d: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    new_contrib,
                    Style::default().fg(self.theme.indicator_success_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(contributors_text), areas[4]);

        // Row 5: Releases panel metrics
        let release_days = snap
            .days_since_last_release()
            .map(|d| format!("{}d ago", d))
            .unwrap_or_else(|| "N/A".to_string());
        let avg_interval = snap
            .releases
            .first()
            .and_then(|r| r.avg_interval)
            .map(|a| format!("{:.0}d", a))
            .unwrap_or_else(|| "N/A".to_string());
        let release_count = snap.releases.len();
        let latest_release = snap
            .releases
            .first()
            .map(|r| r.tag_name.clone())
            .unwrap_or_else(|| "None".to_string());
        let releases_text = vec![
            Line::from(vec![
                Span::styled(
                    "5. Releases",
                    Style::default().fg(self.theme.text_primary_color()).bold(),
                ),
                Span::styled(
                    format!(" — Total: {}", release_count),
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "   Latest: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    latest_release,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
                Span::styled(
                    " | ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    release_days,
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
                Span::styled(
                    " | Avg interval: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    avg_interval,
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(releases_text), areas[5]);

        // Row 6: Velocity panel metrics
        let vel = &snap.velocity;
        let issues_this_week = vel
            .issues_weekly
            .last()
            .map(|w| format!("+{}/-{}", w.opened, w.closed))
            .unwrap_or_else(|| "N/A".to_string());
        let prs_this_week = vel
            .prs_weekly
            .last()
            .map(|w| format!("+{}/-{}", w.opened, w.closed))
            .unwrap_or_else(|| "N/A".to_string());
        let velocity_text = vec![
            Line::from(vec![Span::styled(
                "6. Velocity (8 weeks)",
                Style::default().fg(self.theme.text_primary_color()).bold(),
            )]),
            Line::from(vec![
                Span::styled(
                    "   Issues: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    issues_this_week,
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
                Span::styled(
                    " | PRs: ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ),
                Span::styled(
                    prs_this_week,
                    Style::default().fg(self.theme.indicator_info_color()),
                ),
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(velocity_text), areas[6]);

        // Row 7: Security panel metrics
        let security_text = if let Some(ref sec) = snap.security_alerts {
            vec![
                Line::from(vec![
                    Span::styled(
                        "7. Security",
                        Style::default().fg(self.theme.text_primary_color()).bold(),
                    ),
                    Span::styled(
                        format!(" — Total: {}", sec.total_open),
                        Style::default().fg(if sec.total_open > 0 {
                            self.theme.severity_critical_color()
                        } else {
                            self.theme.indicator_success_color()
                        }),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "   C: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        format!("{}", sec.critical_count),
                        self.severity_style_for_mini_map(
                            sec.critical_count,
                            self.theme.severity_critical_color(),
                        ),
                    ),
                    Span::styled(
                        " | H: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        format!("{}", sec.high_count),
                        self.severity_style_for_mini_map(
                            sec.high_count,
                            self.theme.severity_high_color(),
                        ),
                    ),
                    Span::styled(
                        " | M: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        format!("{}", sec.medium_count),
                        self.severity_style_for_mini_map(
                            sec.medium_count,
                            self.theme.severity_medium_color(),
                        ),
                    ),
                    Span::styled(
                        " | L: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        format!("{}", sec.low_count),
                        self.severity_style_for_mini_map(
                            sec.low_count,
                            self.theme.severity_low_color(),
                        ),
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled(
                        "7. Security",
                        Style::default().fg(self.theme.text_primary_color()).bold(),
                    ),
                    Span::styled(
                        " — Alerts unavailable",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    "   (Dependabot disabled or no access)",
                    Style::default().fg(self.theme.text_secondary_color()),
                )]),
            ]
        };
        frame.render_widget(Paragraph::new(security_text), areas[7]);

        // Row 8: CI Status panel metrics
        let ci_text = if let Some(ref ci) = snap.ci_status {
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

            let (last_status_icon, last_status_color) = ci
                .recent_runs
                .first()
                .map(|run| match run.conclusion.as_deref() {
                    Some("success") => ("✓", self.theme.indicator_success_color()),
                    Some("failure") => ("✗", self.theme.indicator_error_color()),
                    Some("cancelled") => ("⊘", self.theme.indicator_warning_color()),
                    _ => ("○", self.theme.text_secondary_color()),
                })
                .unwrap_or(("—", self.theme.text_secondary_color()));

            vec![
                Line::from(vec![
                    Span::styled(
                        "8. CI Status",
                        Style::default().fg(self.theme.text_primary_color()).bold(),
                    ),
                    Span::styled(
                        format!(" — Success: {:.1}%", ci.success_rate),
                        success_rate_style,
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        "   Last: ",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        last_status_icon,
                        Style::default().fg(last_status_color).bold(),
                    ),
                    Span::styled(
                        format!(" | Runs: {} | Avg: ", ci.total_runs_30d),
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                    Span::styled(
                        if ci.avg_duration_seconds < 60 {
                            format!("{}s", ci.avg_duration_seconds)
                        } else if ci.avg_duration_seconds < 3600 {
                            format!("{}m", ci.avg_duration_seconds / 60)
                        } else {
                            format!(
                                "{}h {}m",
                                ci.avg_duration_seconds / 3600,
                                (ci.avg_duration_seconds % 3600) / 60
                            )
                        },
                        Style::default().fg(self.theme.text_primary_color()),
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled(
                        "8. CI Status",
                        Style::default().fg(self.theme.text_primary_color()).bold(),
                    ),
                    Span::styled(
                        " — Actions unavailable",
                        Style::default().fg(self.theme.text_secondary_color()),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    "   (GitHub Actions disabled or no access)",
                    Style::default().fg(self.theme.text_secondary_color()),
                )]),
            ]
        };
        frame.render_widget(Paragraph::new(ci_text), areas[8]);
    }

    fn severity_style_for_mini_map(&self, count: u64, color: Color) -> Style {
        if count > 0 {
            Style::default().fg(color).bold()
        } else {
            Style::default().fg(self.theme.text_secondary_color())
        }
    }
}
