use crate::core::health::{compute_health_score, HealthGrade};
use crate::core::metrics::stars::predict_milestone;
use crate::core::models::RepoSnapshot;
use crate::tui::app::utils::format_count;

/// ANSI color codes for terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
}

/// Format a compact human-readable summary of all 7 metrics with ANSI colors
pub fn format_summary(snapshot: &RepoSnapshot) -> String {
    use colors::*;

    let mut output = String::new();

    // Header: Repository name
    let repo_full_name = format!("{}/{}", snapshot.repo.owner, snapshot.repo.name);
    output.push_str(&format!(
        "{}{}Repository: {}{}\n",
        BOLD, CYAN, repo_full_name, RESET
    ));

    // Separator line
    let sep_width = repo_full_name.len() + 12; // "Repository: " = 12 chars
    let separator = "━".repeat(sep_width);
    output.push_str(&format!("{}{}{}\n", DIM, separator, RESET));

    // Health Score
    let health = compute_health_score(snapshot);
    let health_color = match health.grade {
        HealthGrade::Excellent => GREEN,
        HealthGrade::Good => CYAN,
        HealthGrade::Fair => YELLOW,
        HealthGrade::NeedsAttention => "\x1b[38;5;208m", // Orange
        HealthGrade::Critical => RED,
    };
    output.push_str(&format!(
        "Health Score: {}{}/100{} (Grade: {}{}{})\n",
        health_color,
        health.total,
        RESET,
        health_color,
        health.grade.as_letter(),
        RESET
    ));
    output.push_str(&format!(
        "              Activity: {:>2}/25 | Community: {:>2}/25 | Maintenance: {:>2}/25 | Growth: {:>2}/25\n\n",
        health.activity, health.community, health.maintenance, health.growth
    ));

    // Stars metric
    let stars_total = format_count(snapshot.stars.total_count);
    let stars_change = calculate_stars_change_30d(snapshot);
    let stars_change_str = if stars_change > 0 {
        format!(" (+{} this month)", format_count(stars_change))
    } else {
        String::new()
    };

    // Milestone prediction
    let milestone_str = match predict_milestone(&snapshot.stars) {
        Some(prediction) => {
            let milestone = format_count(prediction.next_milestone);
            let days = if prediction.estimated_days == 1 {
                "1 day".to_string()
            } else {
                format!("{} days", prediction.estimated_days)
            };
            let rate = format!("{:.1}/day", prediction.daily_rate);
            format!(" | {}★ in {} ({})", milestone, days, rate)
        }
        None => " | Growth stalled".to_string(),
    };

    output.push_str(&format!(
        "{}★ Stars:{}         {:>8}{}{}\n",
        BOLD, RESET, stars_total, stars_change_str, milestone_str
    ));

    // Issues metric
    let issues_count = snapshot.issues.total_open;
    let issues_color = if issues_count == 0 {
        GREEN
    } else if issues_count < 50 {
        YELLOW
    } else {
        RED
    };
    let oldest_issue = snapshot.oldest_issue_age_days();
    let oldest_str = oldest_issue
        .map(|days| format!(" (oldest: {} days)", days))
        .unwrap_or_default();
    output.push_str(&format!(
        "{}📋 Issues:{}       {}{}{} open{}{}\n",
        BOLD, RESET, issues_color, issues_count, RESET, oldest_str, RESET
    ));

    // Pull Requests metric
    let prs = &snapshot.pull_requests;
    let pr_color = if prs.open_count == 0 {
        GREEN
    } else if prs.open_count < 10 {
        YELLOW
    } else {
        RED
    };
    let pr_detail = if prs.ready_count > 0 && prs.draft_count > 0 {
        format!(" ({} ready, {} draft)", prs.ready_count, prs.draft_count)
    } else if prs.ready_count > 0 {
        format!(" ({} ready)", prs.ready_count)
    } else if prs.draft_count > 0 {
        format!(" ({} draft)", prs.draft_count)
    } else {
        String::new()
    };
    output.push_str(&format!(
        "{}🔀 Pull Requests:{}  {}{}{} open{}{}\n",
        BOLD, RESET, pr_color, prs.open_count, RESET, pr_detail, RESET
    ));

    // Contributors metric
    let contributors = &snapshot.contributors;
    let contrib_str = format_count(contributors.total_unique);
    let new_contrib_str = if !contributors.new_contributors_last_30d.is_empty() {
        format!(
            " ({} new this month)",
            contributors.new_contributors_last_30d.len()
        )
    } else {
        String::new()
    };
    output.push_str(&format!(
        "{}👥 Contributors:{}   {:>8}{}\n",
        BOLD, RESET, contrib_str, new_contrib_str
    ));

    // Releases metric
    let release_str = if let Some(release) = snapshot.releases.first() {
        let tag = &release.tag_name;
        let days_str = release
            .days_since
            .map(|d| format!(" ({} days ago)", d))
            .unwrap_or_default();
        format!("{}{}", tag, days_str)
    } else {
        "No releases".to_string()
    };
    output.push_str(&format!(
        "{}🏷️  Releases:{}      {}\n",
        BOLD, RESET, release_str
    ));

    // Velocity metric
    let velocity = &snapshot.velocity;
    let avg_issues = calculate_weekly_average(&velocity.issues_weekly);
    let avg_prs = calculate_weekly_average(&velocity.prs_weekly);
    output.push_str(&format!(
        "{}📊 Velocity:{}       {} issues/week, {} PRs/week\n",
        BOLD, RESET, avg_issues, avg_prs
    ));

    // Security metric
    if let Some(security) = &snapshot.security_alerts {
        let security_color = if security.critical_count > 0 || security.high_count > 0 {
            RED
        } else if security.medium_count > 0 {
            YELLOW
        } else {
            GREEN
        };

        let security_detail = if security.total_open > 0 {
            let mut parts = Vec::new();
            if security.critical_count > 0 {
                parts.push(format!("{} critical", security.critical_count));
            }
            if security.high_count > 0 {
                parts.push(format!("{} high", security.high_count));
            }
            if security.medium_count > 0 {
                parts.push(format!("{} medium", security.medium_count));
            }
            if security.low_count > 0 {
                parts.push(format!("{} low", security.low_count));
            }
            format!(" ({})", parts.join(", "))
        } else {
            String::new()
        };

        output.push_str(&format!(
            "{}🔒 Security:{}      {}{}{} alert{}{}\n",
            BOLD,
            RESET,
            security_color,
            security.total_open,
            RESET,
            if security.total_open == 1 { "" } else { "s" },
            security_detail
        ));
    } else {
        output.push_str(&format!(
            "{}🔒 Security:{}      {}No data available{}\n",
            BOLD, RESET, DIM, RESET
        ));
    }

    output
}

/// Calculate stars change in the last 30 days from sparkline data
fn calculate_stars_change_30d(snapshot: &RepoSnapshot) -> u64 {
    let sparkline = &snapshot.stars.sparkline_30d;
    if sparkline.len() < 2 {
        return 0;
    }

    // Sum the sparkline values to get approximate stars gained in last 30 days
    sparkline.iter().map(|&v| v as u64).sum()
}

/// Calculate weekly average from velocity data
fn calculate_weekly_average(weekly_data: &[crate::core::models::WeeklyActivity]) -> u64 {
    if weekly_data.is_empty() {
        return 0;
    }

    let total: u64 = weekly_data.iter().map(|w| w.opened).sum();
    total / weekly_data.len() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{
        ContributorStats, IssueStats, PrStats, RepoMeta, RepoSnapshot, SecurityAlerts, StarHistory,
        VelocityStats,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "octocat".to_string(),
                name: "Hello-World".to_string(),
                description: None,
                language: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
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
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 8,
                draft_count: 4,
                ready_count: 4,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: Some(48.0),
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec!["newbie1".to_string(), "newbie2".to_string()],
                total_unique: 156,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
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
    fn test_calculate_stars_change_30d() {
        let snapshot = create_test_snapshot();
        let change = calculate_stars_change_30d(&snapshot);
        // Sum of [10, 12, 15, 8, 20, 18, 22, 18] = 123
        assert_eq!(change, 123);
    }

    #[test]
    fn test_calculate_weekly_average_empty() {
        let avg = calculate_weekly_average(&[]);
        assert_eq!(avg, 0);
    }

    #[test]
    fn test_format_summary_includes_repo_name() {
        let snapshot = create_test_snapshot();
        let output = format_summary(&snapshot);
        assert!(output.contains("octocat/Hello-World"));
    }

    #[test]
    fn test_format_summary_includes_health_score() {
        let snapshot = create_test_snapshot();
        let output = format_summary(&snapshot);
        assert!(output.contains("Health Score"));
        // Should now show actual score instead of "Calculating..."
        assert!(output.contains("/100"));
        assert!(output.contains("Grade:"));
        // Check for sub-score breakdown
        assert!(output.contains("Activity:"));
        assert!(output.contains("Community:"));
        assert!(output.contains("Maintenance:"));
        assert!(output.contains("Growth:"));
    }

    #[test]
    fn test_format_summary_includes_all_metrics() {
        let snapshot = create_test_snapshot();
        let output = format_summary(&snapshot);

        // Check all 7 metrics are present
        assert!(output.contains("Stars"));
        assert!(output.contains("Issues"));
        assert!(output.contains("Pull Requests"));
        assert!(output.contains("Contributors"));
        assert!(output.contains("Releases"));
        assert!(output.contains("Velocity"));
        assert!(output.contains("Security"));
    }
}
