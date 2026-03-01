//! Markdown report generation module
//!
//! Generates a formatted Markdown health report for repository snapshots.

use crate::core::health::{compute_health_score, HealthGrade, HealthScore};
use crate::core::metrics::stars::predict_milestone;
use crate::core::models::{RepoSnapshot, WeeklyActivity};

/// Generate a complete Markdown report for the repository snapshot
pub fn generate_report(snapshot: &RepoSnapshot) -> String {
    let health = compute_health_score(snapshot);
    let repo_name = format!("{}/{}", snapshot.repo.owner, snapshot.repo.name);

    let mut report = String::new();

    // Header
    report.push_str(&format!("# {} Health Report\n\n", repo_name));
    report.push_str(&format!("Generated: {} UTC\n\n", snapshot.fetched_at.format("%Y-%m-%d %H:%M")));

    // Health Score Section
    report.push_str(&format_health_section(&health));

    // Stars Section
    report.push_str(&format_stars_section(snapshot));

    // Issues Section
    report.push_str(&format_issues_section(snapshot));

    // Pull Requests Section
    report.push_str(&format_prs_section(snapshot));

    // Contributors Section
    report.push_str(&format_contributors_section(snapshot));

    // Releases Section
    report.push_str(&format_releases_section(snapshot));

    // Velocity Section
    report.push_str(&format_velocity_section(snapshot));

    // Security Section
    report.push_str(&format_security_section(snapshot));

    report
}

/// Format the health score section with detailed breakdown
fn format_health_section(health: &HealthScore) -> String {
    let mut section = String::new();

    section.push_str("## Health Score\n\n");

    // Overall score with grade emoji
    let grade_emoji = match health.grade {
        HealthGrade::Excellent => "🟢",
        HealthGrade::Good => "🔵",
        HealthGrade::Fair => "🟡",
        HealthGrade::NeedsAttention => "🟠",
        HealthGrade::Critical => "🔴",
    };

    section.push_str(&format!(
        "**Overall: {}/100** {} {} ({}\n\n",
        health.total,
        grade_emoji,
        health.grade.as_letter(),
        health.grade.as_label()
    ));

    // Sub-scores table
    section.push_str("| Category | Score | Max |\n");
    section.push_str("|----------|-------|-----|\n");
    section.push_str(&format!("| Activity | {} | 25 |\n", health.activity));
    section.push_str(&format!("| Community | {} | 25 |\n", health.community));
    section.push_str(&format!("| Maintenance | {} | 25 |\n", health.maintenance));
    section.push_str(&format!("| Growth | {} | 25 |\n", health.growth));
    section.push_str("\n");

    section
}

/// Format the stars section with milestone prediction
fn format_stars_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();

    section.push_str("## ⭐ Stars\n\n");
    section.push_str(&format!("**Total:** {}\n\n", format_number(snapshot.stars.total_count)));

    // Milestone prediction
    if let Some(prediction) = predict_milestone(&snapshot.stars) {
        let milestone = format_number(prediction.next_milestone);
        let days = if prediction.estimated_days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", prediction.estimated_days)
        };
        section.push_str(&format!(
            "📈 **Next Milestone:** {} stars estimated in {} ({:.1}/day)\n\n",
            milestone, days, prediction.daily_rate
        ));
    } else {
        section.push_str("📉 Growth stalled or no milestone data available\n\n");
    }

    // Sparkline data summary
    let stars_30d: u64 = snapshot.stars.sparkline_30d.iter().map(|&v| v as u64).sum();
    let stars_90d: u64 = snapshot.stars.sparkline_90d.iter().map(|&v| v as u64).sum();

    section.push_str("| Period | New Stars |\n");
    section.push_str("|--------|-----------|\n");
    section.push_str(&format!("| Last 30 days | +{} |\n", format_number(stars_30d)));
    section.push_str(&format!("| Last 90 days | +{} |\n", format_number(stars_90d)));
    section.push_str("\n");

    section
}

/// Format the issues section
fn format_issues_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();

    section.push_str("## 📋 Issues\n\n");
    section.push_str(&format!("**Open:** {}\n\n", snapshot.issues.total_open));

    // Oldest issue
    if let Some(oldest_days) = snapshot.oldest_issue_age_days() {
        section.push_str(&format!("⏱️ **Oldest Issue:** {} days old\n\n", oldest_days));
    }

    // Issues by label
    if !snapshot.issues.by_label.is_empty() || !snapshot.issues.unlabelled.is_empty() {
        section.push_str("### By Label\n\n");
        section.push_str("| Label | Count |\n");
        section.push_str("|-------|-------|\n");

        // Sort labels by count (descending)
        let mut labels: Vec<(&String, usize)> = snapshot
            .issues
            .by_label
            .iter()
            .map(|(label, issues)| (label, issues.len()))
            .collect();
        labels.sort_by(|a, b| b.1.cmp(&a.1));

        for (label, count) in labels {
            section.push_str(&format!("| {} | {} |\n", label, count));
        }

        if !snapshot.issues.unlabelled.is_empty() {
            section.push_str(&format!(
                "| *unlabelled* | {} |\n",
                snapshot.issues.unlabelled.len()
            ));
        }
        section.push_str("\n");
    }

    section
}

/// Format the pull requests section
fn format_prs_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();
    let prs = &snapshot.pull_requests;

    section.push_str("## 🔀 Pull Requests\n\n");
    section.push_str(&format!("**Open:** {}\n\n", prs.open_count));

    section.push_str("| Status | Count |\n");
    section.push_str("|--------|-------|\n");
    section.push_str(&format!("| Ready | {} |\n", prs.ready_count));
    section.push_str(&format!("| Draft | {} |\n", prs.draft_count));
    section.push_str(&format!("| Merged (30d) | {} |\n", prs.merged_last_30d.len()));
    section.push_str("\n");

    if let Some(avg_hours) = prs.avg_time_to_merge_hours {
        let avg_days = avg_hours / 24.0;
        section.push_str(&format!(
            "⏱️ **Avg Time to Merge:** {:.1} days\n\n",
            avg_days
        ));
    }

    section
}

/// Format the contributors section
fn format_contributors_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();
    let contributors = &snapshot.contributors;

    section.push_str("## 👥 Contributors\n\n");
    section.push_str(&format!("**Total Unique:** {}\n\n", format_number(contributors.total_unique)));

    // New contributors
    if !contributors.new_contributors_last_30d.is_empty() {
        section.push_str(&format!(
            "🆕 **New Contributors (30d):** {}\n\n",
            contributors.new_contributors_last_30d.len()
        ));
    }

    // Top contributors
    if !contributors.top_contributors.is_empty() {
        section.push_str("### Top Contributors\n\n");
        section.push_str("| User | Commits |\n");
        section.push_str("|------|---------|\n");

        for contributor in contributors.top_contributors.iter().take(10) {
            section.push_str(&format!(
                "| @{} | {} |\n",
                contributor.username,
                format_number(contributor.commit_count)
            ));
        }
        section.push_str("\n");
    }

    section
}

/// Format the releases section
fn format_releases_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();

    section.push_str("## 🏷️ Releases\n\n");

    if snapshot.releases.is_empty() {
        section.push_str("*No releases found*\n\n");
        return section;
    }

    section.push_str("| Version | Published | Days Since |\n");
    section.push_str("|---------|-----------|------------|\n");

    for release in snapshot.releases.iter().take(5) {
        let version = &release.tag_name;
        let published = release
            .published_at
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let days_since = release
            .days_since
            .map(|d| d.to_string())
            .unwrap_or_else(|| "-".to_string());

        section.push_str(&format!("| {} | {} | {} |\n", version, published, days_since));
    }

    // Average release interval
    if let Some(first_release) = snapshot.releases.first() {
        if let Some(avg_interval) = first_release.avg_interval {
            section.push_str(&format!("\n📅 **Avg Release Interval:** {:.1} days\n", avg_interval));
        }
    }

    section.push_str("\n");

    section
}

/// Format the velocity section
fn format_velocity_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();
    let velocity = &snapshot.velocity;

    section.push_str("## 📊 Velocity (Last 8 Weeks)\n\n");

    // Issues velocity
    if !velocity.issues_weekly.is_empty() {
        section.push_str("### Issues\n\n");
        section.push_str("| Week | Opened | Closed | Ratio |\n");
        section.push_str("|------|--------|--------|-------|\n");

        for week in &velocity.issues_weekly {
            let ratio = if week.closed > 0 {
                format!("{:.1}", week.opened as f64 / week.closed as f64)
            } else if week.opened > 0 {
                "∞".to_string()
            } else {
                "-".to_string()
            };
            section.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                week.week_start.format("%Y-%m-%d"),
                week.opened,
                week.closed,
                ratio
            ));
        }
        section.push_str("\n");
    }

    // PRs velocity
    if !velocity.prs_weekly.is_empty() {
        section.push_str("### Pull Requests\n\n");
        section.push_str("| Week | Opened | Merged | Ratio |\n");
        section.push_str("|------|--------|--------|-------|\n");

        for week in &velocity.prs_weekly {
            let ratio = if week.closed > 0 {
                format!("{:.1}", week.opened as f64 / week.closed as f64)
            } else if week.opened > 0 {
                "∞".to_string()
            } else {
                "-".to_string()
            };
            section.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                week.week_start.format("%Y-%m-%d"),
                week.opened,
                week.closed,
                ratio
            ));
        }
        section.push_str("\n");
    }

    // Averages
    let avg_issues_opened = calculate_weekly_average_opened(&velocity.issues_weekly);
    let avg_issues_closed = calculate_weekly_average_closed(&velocity.issues_weekly);
    let avg_prs_opened = calculate_weekly_average_opened(&velocity.prs_weekly);
    let avg_prs_closed = calculate_weekly_average_closed(&velocity.prs_weekly);

    section.push_str("### Averages\n\n");
    section.push_str("| Metric | Opened/Week | Closed/Merged/Week |\n");
    section.push_str("|--------|-------------|-------------------|\n");
    section.push_str(&format!(
        "| Issues | {:.1} | {:.1} |\n",
        avg_issues_opened, avg_issues_closed
    ));
    section.push_str(&format!(
        "| PRs | {:.1} | {:.1} |\n",
        avg_prs_opened, avg_prs_closed
    ));
    section.push_str("\n");

    section
}

/// Format the security section
fn format_security_section(snapshot: &RepoSnapshot) -> String {
    let mut section = String::new();

    section.push_str("## 🔒 Security\n\n");

    match &snapshot.security_alerts {
        Some(security) => {
            section.push_str(&format!("**Open Alerts:** {}\n\n", security.total_open));

            if security.total_open > 0 {
                section.push_str("### By Severity\n\n");
                section.push_str("| Severity | Count |\n");
                section.push_str("|----------|-------|\n");
                section.push_str(&format!("| 🔴 Critical | {} |\n", security.critical_count));
                section.push_str(&format!("| 🟠 High | {} |\n", security.high_count));
                section.push_str(&format!("| 🟡 Medium | {} |\n", security.medium_count));
                section.push_str(&format!("| 🟢 Low | {} |\n", security.low_count));
                section.push_str("\n");
            } else {
                section.push_str("✅ No open security alerts\n\n");
            }
        }
        None => {
            section.push_str("*Security data not available (requires authentication)*\n\n");
        }
    }

    section
}

/// Format a number with thousands separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for ch in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
        count += 1;
    }

    result.chars().rev().collect()
}

/// Calculate average opened per week
fn calculate_weekly_average_opened(weekly_data: &[WeeklyActivity]) -> f64 {
    if weekly_data.is_empty() {
        return 0.0;
    }

    let total: u64 = weekly_data.iter().map(|w| w.opened).sum();
    total as f64 / weekly_data.len() as f64
}

/// Calculate average closed per week
fn calculate_weekly_average_closed(weekly_data: &[WeeklyActivity]) -> f64 {
    if weekly_data.is_empty() {
        return 0.0;
    }

    let total: u64 = weekly_data.iter().map(|w| w.closed).sum();
    total as f64 / weekly_data.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{
        Contributor, ContributorStats, Issue, IssueStats, MergedPr, PrStats, Release, RepoMeta,
        RepoSnapshot, SecurityAlerts, StarHistory, VelocityStats, WeeklyActivity,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_snapshot() -> RepoSnapshot {
        let now = Utc::now();
        RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "octocat".to_string(),
                name: "Hello-World".to_string(),
                description: Some("Test repo".to_string()),
                language: Some("Rust".to_string()),
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 100,
                open_issues_count: 10,
                watchers_count: 1000,
            },
            stars: StarHistory {
                total_count: 5234,
                sparkline_30d: vec![10, 12, 15, 8, 20, 18, 22, 18],
                sparkline_90d: vec![100, 120, 150, 80, 200],
                sparkline_365d: vec![1000, 1200, 1500, 800, 2000],
            },
            issues: IssueStats {
                total_open: 42,
                by_label: {
                    let mut map = HashMap::new();
                    map.insert(
                        "bug".to_string(),
                        vec![
                            Issue {
                                number: 1,
                                title: "Bug 1".to_string(),
                                author: "user1".to_string(),
                                created_at: now - chrono::Duration::days(5),
                                updated_at: now,
                                labels: vec!["bug".to_string()],
                                comments_count: 3,
                            },
                            Issue {
                                number: 2,
                                title: "Bug 2".to_string(),
                                author: "user2".to_string(),
                                created_at: now - chrono::Duration::days(10),
                                updated_at: now,
                                labels: vec!["bug".to_string()],
                                comments_count: 1,
                            },
                        ],
                    );
                    map.insert(
                        "enhancement".to_string(),
                        vec![Issue {
                            number: 3,
                            title: "Feature request".to_string(),
                            author: "user3".to_string(),
                            created_at: now - chrono::Duration::days(3),
                            updated_at: now,
                            labels: vec!["enhancement".to_string()],
                            comments_count: 5,
                        }],
                    );
                    map
                },
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 8,
                draft_count: 2,
                ready_count: 6,
                merged_last_30d: vec![
                    MergedPr {
                        number: 101,
                        title: "Fix bug".to_string(),
                        author: "user1".to_string(),
                        created_at: now - chrono::Duration::days(2),
                        merged_at: now,
                        time_to_merge_hours: 24.0,
                    },
                    MergedPr {
                        number: 102,
                        title: "Add feature".to_string(),
                        author: "user2".to_string(),
                        created_at: now - chrono::Duration::days(5),
                        merged_at: now - chrono::Duration::days(1),
                        time_to_merge_hours: 96.0,
                    },
                ],
                avg_time_to_merge_hours: Some(60.0),
            },
            contributors: ContributorStats {
                top_contributors: vec![
                    Contributor {
                        username: "alice".to_string(),
                        commit_count: 150,
                        avatar_url: None,
                    },
                    Contributor {
                        username: "bob".to_string(),
                        commit_count: 75,
                        avatar_url: None,
                    },
                ],
                new_contributors_last_30d: vec!["newbie1".to_string(), "newbie2".to_string()],
                total_unique: 156,
            },
            releases: vec![
                Release {
                    tag_name: "v1.2.0".to_string(),
                    name: Some("Version 1.2".to_string()),
                    created_at: now - chrono::Duration::days(14),
                    published_at: Some(now - chrono::Duration::days(14)),
                    prerelease: false,
                    draft: false,
                    days_since: Some(14),
                    avg_interval: Some(30.0),
                },
                Release {
                    tag_name: "v1.1.0".to_string(),
                    name: Some("Version 1.1".to_string()),
                    created_at: now - chrono::Duration::days(45),
                    published_at: Some(now - chrono::Duration::days(45)),
                    prerelease: false,
                    draft: false,
                    days_since: Some(45),
                    avg_interval: None,
                },
            ],
            velocity: VelocityStats {
                issues_weekly: vec![
                    WeeklyActivity {
                        week_start: now - chrono::Duration::days(7),
                        opened: 5,
                        closed: 3,
                    },
                    WeeklyActivity {
                        week_start: now - chrono::Duration::days(14),
                        opened: 3,
                        closed: 4,
                    },
                ],
                prs_weekly: vec![
                    WeeklyActivity {
                        week_start: now - chrono::Duration::days(7),
                        opened: 2,
                        closed: 1,
                    },
                    WeeklyActivity {
                        week_start: now - chrono::Duration::days(14),
                        opened: 1,
                        closed: 2,
                    },
                ],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 2,
                critical_count: 0,
                high_count: 1,
                medium_count: 1,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
        }
    }

    #[test]
    fn test_generate_report_includes_header() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("# octocat/Hello-World Health Report"));
        assert!(report.contains("Generated:"));
    }

    #[test]
    fn test_generate_report_includes_health_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## Health Score"));
        assert!(report.contains("Overall:"));
        assert!(report.contains("/100"));
        assert!(report.contains("Activity"));
        assert!(report.contains("Community"));
        assert!(report.contains("Maintenance"));
        assert!(report.contains("Growth"));
    }

    #[test]
    fn test_generate_report_includes_stars_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## ⭐ Stars"));
        assert!(report.contains("5,234"));
    }

    #[test]
    fn test_generate_report_includes_issues_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 📋 Issues"));
        assert!(report.contains("42"));
        assert!(report.contains("bug"));
        assert!(report.contains("enhancement"));
    }

    #[test]
    fn test_generate_report_includes_prs_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 🔀 Pull Requests"));
        assert!(report.contains("8")); // open count
        assert!(report.contains("Ready"));
        assert!(report.contains("Draft"));
    }

    #[test]
    fn test_generate_report_includes_contributors_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 👥 Contributors"));
        assert!(report.contains("156"));
        assert!(report.contains("@alice"));
        assert!(report.contains("@bob"));
    }

    #[test]
    fn test_generate_report_includes_releases_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 🏷️ Releases"));
        assert!(report.contains("v1.2.0"));
    }

    #[test]
    fn test_generate_report_includes_velocity_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 📊 Velocity"));
        assert!(report.contains("Issues"));
        assert!(report.contains("Pull Requests"));
    }

    #[test]
    fn test_generate_report_includes_security_section() {
        let snapshot = create_test_snapshot();
        let report = generate_report(&snapshot);

        assert!(report.contains("## 🔒 Security"));
        assert!(report.contains("Critical"));
        assert!(report.contains("High"));
    }

    #[test]
    fn test_generate_report_no_security_data() {
        let mut snapshot = create_test_snapshot();
        snapshot.security_alerts = None;
        let report = generate_report(&snapshot);

        assert!(report.contains("Security data not available"));
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(5234), "5,234");
        assert_eq!(format_number(42), "42");
    }
}
