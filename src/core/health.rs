//! Health Score computation module
//!
//! Computes a repository health score from 0-100, composed of 4 sub-scores:
//! - Activity (0-25): Commit velocity, PR merge rate and speed, recency of activity
//! - Community (0-25): Contributor diversity, new contributors, issue engagement
//! - Maintenance (0-25): Release cadence, security alerts, issue resolution rate
//! - Growth (0-25): Star trends, fork/watch counts

use crate::core::models::RepoSnapshot;
use serde::{Deserialize, Serialize};

/// Overall health score with sub-scores
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct HealthScore {
    /// Total score 0-100
    pub total: u8,
    /// Activity sub-score 0-25
    pub activity: u8,
    /// Community sub-score 0-25
    pub community: u8,
    /// Maintenance sub-score 0-25
    pub maintenance: u8,
    /// Growth sub-score 0-25
    pub growth: u8,
    /// Letter grade based on total score
    pub grade: HealthGrade,
}

/// Health grade based on total score ranges
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthGrade {
    /// 90-100 (green)
    Excellent,
    /// 75-89 (cyan)
    Good,
    /// 50-74 (yellow)
    Fair,
    /// 25-49 (orange)
    NeedsAttention,
    /// 0-24 (red)
    Critical,
}

impl HealthGrade {
    /// Get a single letter representation of the grade
    pub fn as_letter(&self) -> char {
        match self {
            HealthGrade::Excellent => 'A',
            HealthGrade::Good => 'B',
            HealthGrade::Fair => 'C',
            HealthGrade::NeedsAttention => 'D',
            HealthGrade::Critical => 'F',
        }
    }

    /// Get the full text label for the grade
    pub fn as_label(&self) -> &'static str {
        match self {
            HealthGrade::Excellent => "Excellent",
            HealthGrade::Good => "Good",
            HealthGrade::Fair => "Fair",
            HealthGrade::NeedsAttention => "Needs Attention",
            HealthGrade::Critical => "Critical",
        }
    }

    /// Get the color code for this grade (for TUI/CLI display)
    pub fn color_code(&self) -> &'static str {
        match self {
            // Green
            HealthGrade::Excellent => "\x1b[32m",
            // Cyan
            HealthGrade::Good => "\x1b[36m",
            // Yellow
            HealthGrade::Fair => "\x1b[33m",
            // Orange (bright red/yellow)
            HealthGrade::NeedsAttention => "\x1b[38;5;208m",
            // Red
            HealthGrade::Critical => "\x1b[31m",
        }
    }
}

impl From<u8> for HealthGrade {
    fn from(score: u8) -> Self {
        match score {
            90..=100 => HealthGrade::Excellent,
            75..=89 => HealthGrade::Good,
            50..=74 => HealthGrade::Fair,
            25..=49 => HealthGrade::NeedsAttention,
            _ => HealthGrade::Critical,
        }
    }
}

/// Compute the health score for a repository snapshot
pub fn compute_health_score(snapshot: &RepoSnapshot) -> HealthScore {
    let activity = compute_activity_score(snapshot);
    let community = compute_community_score(snapshot);
    let maintenance = compute_maintenance_score(snapshot);
    let growth = compute_growth_score(snapshot);

    let total = activity + community + maintenance + growth;
    let grade = HealthGrade::from(total);

    HealthScore {
        total,
        activity,
        community,
        maintenance,
        growth,
        grade,
    }
}

/// Activity sub-score (0-25)
/// Factors:
/// - Commit velocity from velocity data (issues + PRs per week)
/// - PR merge rate and speed
/// - Recency of activity
/// - CI success rate bonus
fn compute_activity_score(snapshot: &RepoSnapshot) -> u8 {
    let mut score: u8 = 0;
    let velocity = &snapshot.velocity;
    let prs = &snapshot.pull_requests;

    // Factor 1: Commit velocity (0-10 points)
    // Calculate average weekly activity (issues opened + PRs opened)
    let avg_issues_per_week: u64 = if !velocity.issues_weekly.is_empty() {
        velocity.issues_weekly.iter().map(|w| w.opened).sum::<u64>()
            / velocity.issues_weekly.len() as u64
    } else {
        0
    };

    let avg_prs_per_week: u64 = if !velocity.prs_weekly.is_empty() {
        velocity.prs_weekly.iter().map(|w| w.opened).sum::<u64>() / velocity.prs_weekly.len() as u64
    } else {
        0
    };

    let total_weekly_activity = avg_issues_per_week + avg_prs_per_week;

    // Score: 0=0, 1-5=2, 6-10=4, 11-20=6, 21-30=8, 31+=10
    let velocity_score = match total_weekly_activity {
        0 => 0,
        1..=5 => 2,
        6..=10 => 4,
        11..=20 => 6,
        21..=30 => 8,
        _ => 10,
    };
    score += velocity_score;

    // Factor 2: PR merge rate (0-8 points)
    // Based on number of merged PRs in last 30 days
    let merged_count = prs.merged_last_30d.len() as u64;
    let merge_rate_score = match merged_count {
        0 => 0,
        1..=2 => 2,
        3..=5 => 4,
        6..=10 => 6,
        _ => 8,
    };
    score += merge_rate_score;

    // Factor 3: PR merge speed (0-4 points)
    // Based on average time to merge (faster is better)
    let speed_score = if let Some(avg_hours) = prs.avg_time_to_merge_hours {
        match avg_hours as u64 {
            0..=24 => 4,    // Within 1 day
            25..=72 => 3,   // 1-3 days
            73..=168 => 2,  // 4-7 days
            169..=336 => 1, // 1-2 weeks
            _ => 0,         // More than 2 weeks
        }
    } else {
        0 // No merge data
    };
    score += speed_score;

    // Factor 4: Recency bonus (0-3 points)
    // If there was activity in the last week
    let has_recent_activity = velocity
        .issues_weekly
        .first()
        .map(|w| w.opened > 0 || w.closed > 0)
        .unwrap_or(false)
        || velocity
            .prs_weekly
            .first()
            .map(|w| w.opened > 0 || w.closed > 0)
            .unwrap_or(false);

    if has_recent_activity {
        score += 3;
    }

    // Factor 5: CI success rate bonus (up to 5 points)
    if let Some(ci_status) = &snapshot.ci_status {
        // Convert success rate percentage (0.0-100.0) to score (0-5)
        // 100% success rate = 5 points, 0% = 0 points
        let ci_bonus = ((ci_status.success_rate / 100.0) * 5.0).min(5.0) as u8;
        score = score.saturating_add(ci_bonus);
    }

    // Cap at 25
    score.min(25)
}

/// Community sub-score (0-25)
/// Factors:
/// - Contributor diversity (total unique + top contributor spread)
/// - New contributors in last 30 days
/// - Issue engagement (comments per issue, age of oldest)
fn compute_community_score(snapshot: &RepoSnapshot) -> u8 {
    let mut score: u8 = 0;
    let contributors = &snapshot.contributors;
    let issues = &snapshot.issues;

    // Factor 1: Total contributors (0-10 points)
    let total_contributors = contributors.total_unique;
    let contributors_score = match total_contributors {
        0 => 0,
        1..=5 => 2,
        6..=10 => 4,
        11..=25 => 6,
        26..=50 => 8,
        _ => 10,
    };
    score += contributors_score;

    // Factor 2: Top contributor concentration (0-5 points)
    // Lower concentration (more distributed contributions) is better
    let concentration_score = if contributors.top_contributors.is_empty() {
        0
    } else {
        let top_commit_count = contributors.top_contributors[0].commit_count;
        let total_commits: u64 = contributors
            .top_contributors
            .iter()
            .map(|c| c.commit_count)
            .sum();

        if total_commits == 0 {
            0
        } else {
            let top_percentage = (top_commit_count * 100) / total_commits;
            // If top contributor has less than 40% of commits, it's healthy distribution
            match top_percentage {
                0..=39 => 5,
                40..=59 => 3,
                60..=79 => 2,
                _ => 1,
            }
        }
    };
    score += concentration_score;

    // Factor 3: New contributors (0-5 points)
    let new_contributor_count = contributors.new_contributors_last_30d.len() as u64;
    let new_contrib_score = match new_contributor_count {
        0 => 0,
        1 => 2,
        2..=3 => 3,
        4..=5 => 4,
        _ => 5,
    };
    score += new_contrib_score;

    // Factor 4: Issue engagement (0-5 points)
    // Based on average comments per issue and oldest issue age
    let total_issues = issues.total_open;
    let total_comments: u64 = issues
        .by_label
        .values()
        .flat_map(|issues| issues.iter().map(|i| i.comments_count))
        .chain(issues.unlabelled.iter().map(|i| i.comments_count))
        .sum();

    let avg_comments = if total_issues > 0 {
        total_comments as f64 / total_issues as f64
    } else {
        0.0
    };

    let oldest_age = snapshot.oldest_issue_age_days().unwrap_or(0);

    // Comments score: more than 2 comments average = good engagement
    let comments_score = if avg_comments >= 3.0 {
        3
    } else if avg_comments >= 1.0 {
        2
    } else if avg_comments > 0.0 {
        1
    } else {
        0
    };

    // Age penalty: issues older than 90 days reduce score
    let age_score = if oldest_age < 30 {
        2
    } else if oldest_age < 90 {
        1
    } else {
        0
    };

    score += (comments_score + age_score).min(5);

    // Cap at 25
    score.min(25)
}

/// Maintenance sub-score (0-25)
/// Factors:
/// - Release cadence
/// - Security alerts
/// - Issue resolution rate
/// - Community health profile (README, LICENSE, etc.)
fn compute_maintenance_score(snapshot: &RepoSnapshot) -> u8 {
    let mut score: u8 = 25; // Start with perfect score and deduct
    let velocity = &snapshot.velocity;
    let releases = &snapshot.releases;

    // Factor 1: Release cadence (0-10 points, but we deduct if bad)
    // Deduct points based on days since last release
    let release_deduction = if let Some(release) = releases.first() {
        if let Some(days_since) = release.days_since {
            match days_since {
                0..=30 => 0,   // Recent release: no deduction
                31..=60 => 2,  // 1-2 months
                61..=90 => 4,  // 2-3 months
                91..=180 => 6, // 3-6 months
                _ => 10,       // More than 6 months
            }
        } else {
            5 // Unknown release date: moderate deduction
        }
    } else {
        10 // No releases at all: maximum deduction
    };
    score -= release_deduction;

    // Factor 2: Security alerts (0-8 points deduction)
    if let Some(security) = &snapshot.security_alerts {
        let security_deduction = if security.critical_count > 0 {
            8 // Critical alerts are serious
        } else if security.high_count > 0 {
            match security.high_count {
                1..=2 => 4,
                3..=5 => 6,
                _ => 8,
            }
        } else if security.medium_count > 0 {
            match security.medium_count {
                1..=3 => 2,
                4..=10 => 3,
                _ => 4,
            }
        } else if security.low_count > 0 {
            1
        } else {
            0
        };
        score -= security_deduction;
    }

    // Factor 3: Issue resolution rate (0-7 points)
    // Calculate the ratio of closed to opened issues
    let total_opened: u64 = velocity.issues_weekly.iter().map(|w| w.opened).sum();
    let total_closed: u64 = velocity.issues_weekly.iter().map(|w| w.closed).sum();

    let resolution_score = if total_opened == 0 {
        7 // No new issues is good maintenance
    } else {
        let ratio = total_closed as f64 / total_opened as f64;
        match ratio {
            r if r >= 1.2 => 7, // Closing more than opening
            r if r >= 1.0 => 6, // Keeping up
            r if r >= 0.8 => 4, // Slightly behind
            r if r >= 0.5 => 2, // Falling behind
            _ => 0,             // Not resolving issues
        }
    };
    score = score.saturating_sub(7 - resolution_score);

    // Factor 4: Community Health (0-5 points bonus)
    // Better community health profile boosts the maintenance score
    if let Some(community_health) = &snapshot.community_health {
        // Convert the 0-100 community health score to a 0-5 point bonus
        let health_bonus = match community_health.score {
            90..=100 => 5, // Excellent community health
            75..=89 => 4,  // Good community health
            60..=74 => 3,  // Fair community health
            40..=59 => 2,  // Needs some work
            20..=39 => 1,  // Needs significant work
            _ => 0,        // Poor community health
        };
        score += health_bonus;
    }

    // Ensure score doesn't go below 0 and cap at 25
    score.min(25)
}

/// Growth sub-score (0-25)
/// Factors:
/// - Star trends (30d vs 90d comparison)
/// - Fork and watchers
fn compute_growth_score(snapshot: &RepoSnapshot) -> u8 {
    let mut score: u8 = 0;
    let stars = &snapshot.stars;
    let repo = &snapshot.repo;

    // Factor 1: Star velocity (0-15 points)
    // Compare 30d sparkline sum to get recent star growth
    let stars_30d: u64 = stars.sparkline_30d.iter().map(|&v| v as u64).sum();
    let stars_90d: u64 = stars.sparkline_90d.iter().map(|&v| v as u64).sum();

    // Daily average for 30d period
    let days_in_30d = stars.sparkline_30d.len().max(1) as u64;
    let daily_avg_30d = stars_30d / days_in_30d;

    // Daily average for 90d period (normalized)
    let days_in_90d = stars.sparkline_90d.len().max(1) as u64;
    let daily_avg_90d = if days_in_90d > 0 {
        stars_90d / days_in_90d
    } else {
        0
    };

    // Score based on recent daily average
    let velocity_score = match daily_avg_30d {
        0 => 0,
        1..=2 => 3,
        3..=5 => 6,
        6..=10 => 9,
        11..=20 => 12,
        _ => 15,
    };
    score += velocity_score;

    // Acceleration bonus: if 30d avg is better than 90d avg
    if daily_avg_30d > daily_avg_90d {
        score += 2;
    }

    // Factor 2: Total stars milestone (0-5 points)
    let total_stars_score = match stars.total_count {
        0..=10 => 0,
        11..=50 => 1,
        51..=100 => 2,
        101..=500 => 3,
        501..=1000 => 4,
        _ => 5,
    };
    score += total_stars_score;

    // Factor 3: Forks and watchers (0-3 points)
    // These indicate interest beyond just starring
    let forks = repo.forks_count;
    let watchers = repo.watchers_count;

    let engagement_score = match (forks, watchers) {
        (0, 0) => 0,
        (f, w) if f >= 10 || w >= 10 => 3,
        (f, w) if f >= 5 || w >= 5 => 2,
        _ => 1,
    };
    score += engagement_score;

    // Cap at 25
    score.min(25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{
        CommunityHealth, Contributor, ContributorStats, Issue, IssueStats, MergedPr, PrStats,
        Release, RepoMeta, RepoSnapshot, SecurityAlerts, StarHistory, VelocityStats,
        WeeklyActivity,
    };
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn create_test_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                default_branch: "main".to_string(),
                forks_count: 50,
                open_issues_count: 10,
                watchers_count: 100,
            },
            stars: StarHistory {
                total_count: 1000,
                sparkline_30d: vec![10, 12, 15, 8],
                sparkline_90d: vec![100, 120, 150, 80],
                sparkline_365d: vec![1000, 1200],
            },
            issues: IssueStats {
                total_open: 10,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 5,
                draft_count: 1,
                ready_count: 4,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        }
    }

    #[test]
    fn test_health_grade_from_score() {
        assert_eq!(HealthGrade::from(95), HealthGrade::Excellent);
        assert_eq!(HealthGrade::from(90), HealthGrade::Excellent);
        assert_eq!(HealthGrade::from(89), HealthGrade::Good);
        assert_eq!(HealthGrade::from(75), HealthGrade::Good);
        assert_eq!(HealthGrade::from(74), HealthGrade::Fair);
        assert_eq!(HealthGrade::from(50), HealthGrade::Fair);
        assert_eq!(HealthGrade::from(49), HealthGrade::NeedsAttention);
        assert_eq!(HealthGrade::from(25), HealthGrade::NeedsAttention);
        assert_eq!(HealthGrade::from(24), HealthGrade::Critical);
        assert_eq!(HealthGrade::from(0), HealthGrade::Critical);
    }

    #[test]
    fn test_health_grade_as_letter() {
        assert_eq!(HealthGrade::Excellent.as_letter(), 'A');
        assert_eq!(HealthGrade::Good.as_letter(), 'B');
        assert_eq!(HealthGrade::Fair.as_letter(), 'C');
        assert_eq!(HealthGrade::NeedsAttention.as_letter(), 'D');
        assert_eq!(HealthGrade::Critical.as_letter(), 'F');
    }

    #[test]
    fn test_health_grade_as_label() {
        assert_eq!(HealthGrade::Excellent.as_label(), "Excellent");
        assert_eq!(HealthGrade::NeedsAttention.as_label(), "Needs Attention");
    }

    #[test]
    fn test_compute_health_score_zero_activity() {
        let snapshot = create_test_snapshot();
        let score = compute_health_score(&snapshot);

        // With minimal activity data, scores should be present but modest
        // Activity: gets points from recency bonus even with minimal data
        assert!(
            score.activity <= 5,
            "Activity score should be low with no velocity data"
        );
        // Community: gets points from forks/watchers even without contributors
        assert!(
            score.community <= 8,
            "Community score should be low with minimal data"
        );
        // Maintenance: 25 - 10 (no releases) - resolution adjustment
        assert!(
            score.maintenance <= 17,
            "Maintenance should be reduced for no releases"
        );
        // Growth: gets points from stars (1000 total = 4) + velocity (10 stars/day = 9) + forks/watchers (3) = 16
        assert!(
            score.growth >= 10,
            "Growth should have points from stars and forks"
        );
        assert!(
            score.total <= 60,
            "Total should be modest with minimal data"
        );
    }

    #[test]
    fn test_compute_health_score_excellent() {
        let now = Utc::now();
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "excellent".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 200,
                open_issues_count: 10,
                watchers_count: 500,
            },
            stars: StarHistory {
                total_count: 10000,
                sparkline_30d: vec![50, 60, 55, 70, 65, 80, 75], // High daily avg
                sparkline_90d: vec![400, 450, 500],
                sparkline_365d: vec![5000, 6000],
            },
            issues: IssueStats {
                total_open: 20,
                by_label: {
                    let mut map = HashMap::new();
                    map.insert(
                        "bug".to_string(),
                        vec![Issue {
                            number: 1,
                            title: "Bug".to_string(),
                            author: "user".to_string(),
                            created_at: now - Duration::days(5),
                            updated_at: now,
                            labels: vec!["bug".to_string()],
                            comments_count: 3,
                        }],
                    );
                    map
                },
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 5,
                draft_count: 1,
                ready_count: 4,
                merged_last_30d: vec![
                    MergedPr {
                        number: 1,
                        title: "PR1".to_string(),
                        author: "user1".to_string(),
                        created_at: now - Duration::days(2),
                        merged_at: now,
                        time_to_merge_hours: 12.0,
                    },
                    MergedPr {
                        number: 2,
                        title: "PR2".to_string(),
                        author: "user2".to_string(),
                        created_at: now - Duration::days(1),
                        merged_at: now,
                        time_to_merge_hours: 18.0,
                    },
                    MergedPr {
                        number: 3,
                        title: "PR3".to_string(),
                        author: "user3".to_string(),
                        created_at: now - Duration::days(3),
                        merged_at: now,
                        time_to_merge_hours: 20.0,
                    },
                    MergedPr {
                        number: 4,
                        title: "PR4".to_string(),
                        author: "user4".to_string(),
                        created_at: now - Duration::days(4),
                        merged_at: now,
                        time_to_merge_hours: 16.0,
                    },
                    MergedPr {
                        number: 5,
                        title: "PR5".to_string(),
                        author: "user5".to_string(),
                        created_at: now - Duration::days(5),
                        merged_at: now,
                        time_to_merge_hours: 14.0,
                    },
                    MergedPr {
                        number: 6,
                        title: "PR6".to_string(),
                        author: "user6".to_string(),
                        created_at: now - Duration::days(6),
                        merged_at: now,
                        time_to_merge_hours: 10.0,
                    },
                ],
                avg_time_to_merge_hours: Some(15.0),
            },
            contributors: ContributorStats {
                top_contributors: vec![
                    Contributor {
                        username: "user1".to_string(),
                        commit_count: 30,
                        avatar_url: None,
                    },
                    Contributor {
                        username: "user2".to_string(),
                        commit_count: 25,
                        avatar_url: None,
                    },
                    Contributor {
                        username: "user3".to_string(),
                        commit_count: 20,
                        avatar_url: None,
                    },
                    Contributor {
                        username: "user4".to_string(),
                        commit_count: 15,
                        avatar_url: None,
                    },
                ],
                new_contributors_last_30d: vec![
                    "newbie1".to_string(),
                    "newbie2".to_string(),
                    "newbie3".to_string(),
                    "newbie4".to_string(),
                    "newbie5".to_string(),
                ],
                total_unique: 100,
            },
            releases: vec![Release {
                tag_name: "v1.0.0".to_string(),
                name: Some("Release".to_string()),
                created_at: now,
                published_at: Some(now - Duration::days(5)),
                prerelease: false,
                draft: false,
                days_since: Some(5),
                avg_interval: Some(14.0),
            }],
            velocity: VelocityStats {
                issues_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 25,
                        closed: 22,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 20,
                        closed: 19,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(21),
                        opened: 18,
                        closed: 17,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(28),
                        opened: 22,
                        closed: 20,
                    },
                ],
                prs_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 15,
                        closed: 14,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 12,
                        closed: 11,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(21),
                        opened: 10,
                        closed: 10,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(28),
                        opened: 14,
                        closed: 13,
                    },
                ],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
        };

        let score = compute_health_score(&snapshot);

        // Activity breakdown:
        // - Velocity: (15+20+10)/3 + (5+8)/2 = 15 + 6.5 = 22 weekly = 8 points
        // - Merge rate: 3 PRs merged = 4 points
        // - Merge speed: 28 hours avg = 3 points
        // - Recent activity: has data = 3 points
        // Total: 8 + 4 + 3 + 3 = 18 (capped at 25)
        assert!(
            score.activity >= 15,
            "Activity score should be high, got {}",
            score.activity
        );
        // Community: 75 contributors = 10, good distribution = 5, 2 new contributors = 3,
        //           comments/age = ~3 = 16
        assert!(
            score.community >= 12,
            "Community score should be good, got {}",
            score.community
        );
        // Maintenance: recent release (5 days) = no deduction, no security alerts = no deduction,
        //              good resolution rate (33 opened, 31 closed, ratio ~0.94) = 6 points
        // Score: 25 - 0 - 0 - (7-6) = 24
        assert!(
            score.maintenance >= 18,
            "Maintenance score should be high, got {}",
            score.maintenance
        );
        // Growth: high velocity (50+ avg/day = 15), milestone (10000 stars = 5),
        //         forks/watchers (200, 500) = 3, acceleration bonus = 2
        // Total: min(15+5+3+2, 25) = 25
        assert!(
            score.growth >= 20,
            "Growth score should be high, got {}",
            score.growth
        );
        // With all these good metrics, total should be >= 90 (Excellent)
        assert!(
            score.total >= 90,
            "Total score should be >= 90 for Excellent, got {}",
            score.total
        );
        assert_eq!(score.grade, HealthGrade::Excellent);
    }

    #[test]
    fn test_compute_health_score_critical() {
        let now = Utc::now();
        let old_date = now - Duration::days(200);
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "critical".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: old_date,
                updated_at: old_date,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 100,
                watchers_count: 5,
            },
            stars: StarHistory {
                total_count: 5,
                sparkline_30d: vec![0, 0, 0, 0],
                sparkline_90d: vec![0, 0, 0],
                sparkline_365d: vec![0],
            },
            issues: IssueStats {
                total_open: 100,
                by_label: {
                    let mut map = HashMap::new();
                    map.insert(
                        "bug".to_string(),
                        vec![Issue {
                            number: 1,
                            title: "Old bug".to_string(),
                            author: "user".to_string(),
                            created_at: now - Duration::days(180),
                            updated_at: old_date,
                            labels: vec!["bug".to_string()],
                            comments_count: 0,
                        }],
                    );
                    map
                },
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 50,
                draft_count: 10,
                ready_count: 40,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![Contributor {
                    username: "single".to_string(),
                    commit_count: 100,
                    avatar_url: None,
                }],
                new_contributors_last_30d: vec![],
                total_unique: 1,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 20,
                    closed: 0,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 5,
                    closed: 0,
                }],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 5,
                critical_count: 2,
                high_count: 3,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
        };

        let score = compute_health_score(&snapshot);

        // Activity: recent activity (20 opened, 0 closed issues) + (5 opened, 0 closed PRs) = 25 weekly = 8 points
        // But also has open issues/PRs, so gets some velocity score
        assert!(
            score.activity <= 15,
            "Activity score should be low, got {}",
            score.activity
        );
        // Community: 1 contributor = 2 points, high concentration (100%) = 1 point,
        //          no new contributors = 0, old issue (180 days) + no comments = ~1
        assert!(
            score.community <= 8,
            "Community score should be low, got {}",
            score.community
        );
        // Maintenance: no releases = -10, critical alerts (2) = -8,
        //              bad resolution (20 opened, 0 closed, ratio 0) = 0, deduction = 7
        // Score: 25 - 10 - 8 - 7 = 0
        assert!(
            score.maintenance <= 5,
            "Maintenance score should be very low, got {}",
            score.maintenance
        );
        // Growth: no stars gained (0), low total (5 stars = 0), no forks/watchers = 0
        assert!(
            score.growth <= 3,
            "Growth score should be low, got {}",
            score.growth
        );
        // With all these poor metrics, total should be critical
        assert!(
            score.total <= 30,
            "Total score should be critical, got {}",
            score.total
        );
        assert_eq!(score.grade, HealthGrade::Critical);
    }

    #[test]
    fn test_grade_boundary_89_to_90() {
        // Test that 89 = Good, 90 = Excellent boundary works
        assert_eq!(HealthGrade::from(89), HealthGrade::Good);
        assert_eq!(HealthGrade::from(90), HealthGrade::Excellent);
    }

    #[test]
    fn test_grade_boundary_74_to_75() {
        // Test that 74 = Fair, 75 = Good boundary works
        assert_eq!(HealthGrade::from(74), HealthGrade::Fair);
        assert_eq!(HealthGrade::from(75), HealthGrade::Good);
    }

    #[test]
    fn test_grade_boundary_49_to_50() {
        // Test that 49 = NeedsAttention, 50 = Fair boundary works
        assert_eq!(HealthGrade::from(49), HealthGrade::NeedsAttention);
        assert_eq!(HealthGrade::from(50), HealthGrade::Fair);
    }

    #[test]
    fn test_grade_boundary_24_to_25() {
        // Test that 24 = Critical, 25 = NeedsAttention boundary works
        assert_eq!(HealthGrade::from(24), HealthGrade::Critical);
        assert_eq!(HealthGrade::from(25), HealthGrade::NeedsAttention);
    }

    #[test]
    fn test_health_score_serde_roundtrip() {
        let score = HealthScore {
            total: 75,
            activity: 20,
            community: 18,
            maintenance: 22,
            growth: 15,
            grade: HealthGrade::Good,
        };

        let json = serde_json::to_string(&score).expect("Failed to serialize");
        let deserialized: HealthScore = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(score.total, deserialized.total);
        assert_eq!(score.activity, deserialized.activity);
        assert_eq!(score.community, deserialized.community);
        assert_eq!(score.maintenance, deserialized.maintenance);
        assert_eq!(score.growth, deserialized.growth);
        assert_eq!(score.grade, deserialized.grade);
    }

    #[test]
    fn test_activity_score_velocity_calculation() {
        let now = Utc::now();
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 15,
                        closed: 10,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 20,
                        closed: 15,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(21),
                        opened: 10,
                        closed: 8,
                    },
                ],
                prs_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 5,
                        closed: 4,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 8,
                        closed: 7,
                    },
                ],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        };

        let score = compute_activity_score(&snapshot);
        // (15+20+10)/3 + (5+8)/2 = 15 + 6.5 = ~22 weekly activity = score 8 for velocity
        // Plus recent activity bonus = 3
        // Total activity should be around 11
        assert!(
            (10..=13).contains(&score),
            "Activity score should reflect velocity"
        );
    }

    #[test]
    fn test_maintenance_score_with_security_alerts() {
        let now = Utc::now();

        // Test with critical alerts
        let critical_snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![Release {
                tag_name: "v1".to_string(),
                name: None,
                created_at: now,
                published_at: Some(now - Duration::days(5)),
                prerelease: false,
                draft: false,
                days_since: Some(5),
                avg_interval: None,
            }],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 2,
                critical_count: 1,
                high_count: 1,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
        };

        let score = compute_maintenance_score(&critical_snapshot);
        // Recent release: 0 deduction
        // Critical alert: 8 deduction
        // Max score: 25 - 8 = 17, but we also need to account for resolution score
        assert!(
            score <= 17,
            "Critical alerts should significantly reduce score"
        );

        // Test with no alerts
        let clean_snapshot = RepoSnapshot {
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
            ..critical_snapshot
        };

        let clean_score = compute_maintenance_score(&clean_snapshot);
        assert!(
            clean_score > score,
            "Clean security should have higher score"
        );
    }

    #[test]
    fn test_growth_score_acceleration_bonus() {
        let now = Utc::now();

        // Snapshot where 30d avg is better than 90d avg (acceleration)
        let accelerating = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 10,
                open_issues_count: 0,
                watchers_count: 20,
            },
            stars: StarHistory {
                total_count: 1000,
                sparkline_30d: vec![50, 50, 50, 50], // 12.5 avg per day
                sparkline_90d: vec![100, 100],       // 50 avg per 45 days = ~1.1 per day
                sparkline_365d: vec![500],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        };

        let score = compute_growth_score(&accelerating);
        // Should get acceleration bonus of +2
        assert!(score >= 20, "Accelerating growth should have high score");
    }

    #[test]
    fn test_health_score_components_sum_to_total() {
        let snapshot = create_test_snapshot();
        let score = compute_health_score(&snapshot);

        assert_eq!(
            score.total,
            score.activity + score.community + score.maintenance + score.growth,
            "Total should equal sum of components"
        );
    }

    #[test]
    fn test_maintenance_score_with_community_health_perfect() {
        let now = Utc::now();

        // Base snapshot with no releases (10 point deduction), no security alerts
        // With perfect community health (100 score = 5 point bonus)
        // Base: 25 - 10 (no releases) - 0 (security) - 7 (no resolution data) = 8
        // With bonus: 8 + 5 = 13
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: Some(CommunityHealth {
                has_readme: true,
                has_license: true,
                has_contributing: true,
                has_code_of_conduct: true,
                has_issue_templates: true,
                has_pr_template: true,
                has_security_policy: true,
                score: 100,
            }),
        };

        let score = compute_maintenance_score(&snapshot);
        // Calculation:
        // - Start: 25
        // - No releases: -10 = 15
        // - No security alerts: -0 = 15
        // - No velocity data (total_opened = 0): resolution_score = 7, so no deduction = 15
        // - Perfect community health (+5): 15 + 5 = 20
        assert!(
            (19..=21).contains(&score),
            "Maintenance score with perfect community health should be 20, got {}",
            score
        );
    }

    #[test]
    fn test_maintenance_score_with_community_health_partial() {
        let now = Utc::now();

        // Snapshot with partial community health (score 50 = 2 point bonus)
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: Some(CommunityHealth {
                has_readme: true,
                has_license: true,
                has_contributing: false,
                has_code_of_conduct: false,
                has_issue_templates: false,
                has_pr_template: false,
                has_security_policy: false,
                score: 50,
            }),
        };

        let score = compute_maintenance_score(&snapshot);
        // Calculation:
        // - Start: 25
        // - No releases: -10 = 15
        // - No security alerts: -0 = 15
        // - No velocity data: no resolution deduction = 15
        // - Partial community health 50 (+2): 15 + 2 = 17
        assert!(
            (16..=18).contains(&score),
            "Maintenance score with partial community health should be 17, got {}",
            score
        );
    }

    #[test]
    fn test_maintenance_score_with_community_health_none() {
        let now = Utc::now();

        // Snapshot with no community health data
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![],
                prs_weekly: vec![],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: None,
            community_health: None,
        };

        let score_without = compute_maintenance_score(&snapshot);

        // Calculation:
        // - Start: 25
        // - No releases: -10 = 15
        // - No security alerts: -0 = 15
        // - No velocity data: no resolution deduction = 15
        // - No community health = no bonus: 15
        assert!(
            (14..=16).contains(&score_without),
            "Maintenance score without community health should be 15, got {}",
            score_without
        );
    }

    #[test]
    fn test_activity_score_with_ci_success_rate_perfect() {
        let now = Utc::now();

        // Base snapshot with velocity data but no CI
        let base_snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 15,
                        closed: 10,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 20,
                        closed: 15,
                    },
                ],
                prs_weekly: vec![
                    WeeklyActivity {
                        week_start: now - Duration::days(7),
                        opened: 5,
                        closed: 4,
                    },
                    WeeklyActivity {
                        week_start: now - Duration::days(14),
                        opened: 8,
                        closed: 7,
                    },
                ],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        };

        let score_without_ci = compute_activity_score(&base_snapshot);

        // Snapshot with perfect CI success rate (100%)
        let snapshot_with_ci = RepoSnapshot {
            ci_status: Some(crate::core::models::CIStatus {
                total_runs_30d: 50,
                success_rate: 100.0,
                avg_duration_seconds: 300,
                recent_runs: vec![],
            }),
            ..base_snapshot.clone()
        };

        let score_with_perfect_ci = compute_activity_score(&snapshot_with_ci);

        // Perfect CI (100% success rate) should add 5 points
        assert_eq!(
            score_with_perfect_ci,
            score_without_ci + 5,
            "Perfect CI success rate should add 5 points to activity score"
        );
    }

    #[test]
    fn test_activity_score_with_ci_success_rate_partial() {
        let now = Utc::now();

        // Snapshot with velocity data and partial CI success rate (80%)
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 15,
                    closed: 10,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 5,
                    closed: 4,
                }],
            },
            security_alerts: None,
            ci_status: Some(crate::core::models::CIStatus {
                total_runs_30d: 50,
                success_rate: 80.0,
                avg_duration_seconds: 300,
                recent_runs: vec![],
            }),
            community_health: None,
        };

        let score = compute_activity_score(&snapshot);

        // 80% success rate should add 4 points (80/100 * 5 = 4)
        // Base score from velocity: (15)/1 + (5)/1 = 20 weekly = 6 points for velocity
        // Plus recent activity bonus = 3
        // Plus CI bonus (80%) = 4
        // Total = 6 + 3 + 4 = 13
        assert!(
            (12..=14).contains(&score),
            "Activity score with 80% CI success rate should include ~4 point bonus, got {}",
            score
        );
    }

    #[test]
    fn test_activity_score_with_ci_success_rate_zero() {
        let now = Utc::now();

        // Snapshot with 0% CI success rate
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 15,
                    closed: 10,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 5,
                    closed: 4,
                }],
            },
            security_alerts: None,
            ci_status: Some(crate::core::models::CIStatus {
                total_runs_30d: 50,
                success_rate: 0.0,
                avg_duration_seconds: 300,
                recent_runs: vec![],
            }),
            community_health: None,
        };

        let score_with_zero = compute_activity_score(&snapshot);

        // Snapshot without CI for comparison
        let snapshot_without_ci = RepoSnapshot {
            ci_status: None,
            ..snapshot.clone()
        };
        let score_without_ci = compute_activity_score(&snapshot_without_ci);

        // 0% CI success rate should add 0 points (same as no CI)
        assert_eq!(
            score_with_zero, score_without_ci,
            "0% CI success rate should add no bonus points"
        );
    }

    #[test]
    fn test_activity_score_with_ci_success_rate_capped() {
        let now = Utc::now();

        // Create a snapshot with high base activity score and perfect CI
        // to verify the 25-point cap is enforced
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 0,
                open_issues_count: 0,
                watchers_count: 0,
            },
            stars: StarHistory {
                total_count: 0,
                sparkline_30d: vec![],
                sparkline_90d: vec![],
                sparkline_365d: vec![],
            },
            issues: IssueStats {
                total_open: 0,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 0,
                draft_count: 0,
                ready_count: 0,
                merged_last_30d: vec![
                    crate::core::models::MergedPr {
                        number: 1,
                        title: "PR1".to_string(),
                        author: "user1".to_string(),
                        created_at: now - Duration::days(2),
                        merged_at: now,
                        time_to_merge_hours: 12.0,
                    },
                    crate::core::models::MergedPr {
                        number: 2,
                        title: "PR2".to_string(),
                        author: "user2".to_string(),
                        created_at: now - Duration::days(1),
                        merged_at: now,
                        time_to_merge_hours: 12.0,
                    },
                    crate::core::models::MergedPr {
                        number: 3,
                        title: "PR3".to_string(),
                        author: "user3".to_string(),
                        created_at: now - Duration::days(3),
                        merged_at: now,
                        time_to_merge_hours: 12.0,
                    },
                ],
                avg_time_to_merge_hours: Some(12.0),
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 50,
                    closed: 45,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 50,
                    closed: 45,
                }],
            },
            security_alerts: None,
            ci_status: Some(crate::core::models::CIStatus {
                total_runs_30d: 50,
                success_rate: 100.0,
                avg_duration_seconds: 300,
                recent_runs: vec![],
            }),
            community_health: None,
        };

        let score = compute_activity_score(&snapshot);

        // Activity score should be capped at 25 regardless of high CI bonus
        assert!(
            score <= 25,
            "Activity score should be capped at 25, got {}",
            score
        );
    }

    #[test]
    fn test_health_score_with_ci_and_community_integration() {
        let now = Utc::now();

        // Test the full integration with both CI and Community Health
        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test".to_string(),
                name: "repo".to_string(),
                description: None,
                language: None,
                created_at: now,
                updated_at: now,
                default_branch: "main".to_string(),
                forks_count: 50,
                open_issues_count: 10,
                watchers_count: 100,
            },
            stars: StarHistory {
                total_count: 1000,
                sparkline_30d: vec![10, 12, 15, 8],
                sparkline_90d: vec![100, 120, 150, 80],
                sparkline_365d: vec![1000, 1200],
            },
            issues: IssueStats {
                total_open: 10,
                by_label: HashMap::new(),
                unlabelled: vec![],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 5,
                draft_count: 1,
                ready_count: 4,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![],
                new_contributors_last_30d: vec![],
                total_unique: 0,
            },
            releases: vec![],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 25,
                    closed: 22,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: now - Duration::days(7),
                    opened: 15,
                    closed: 14,
                }],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 0,
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
            }),
            ci_status: Some(crate::core::models::CIStatus {
                total_runs_30d: 50,
                success_rate: 90.0, // 90% = 4.5 points (rounded to 4)
                avg_duration_seconds: 300,
                recent_runs: vec![],
            }),
            community_health: Some(crate::core::models::CommunityHealth {
                has_readme: true,
                has_license: true,
                has_contributing: true,
                has_code_of_conduct: true,
                has_issue_templates: true,
                has_pr_template: true,
                has_security_policy: true,
                score: 100, // Perfect score = 5 points bonus to maintenance
            }),
        };

        let score = compute_health_score(&snapshot);

        // Verify all sub-scores are within valid ranges
        assert!(score.activity <= 25, "Activity should be capped at 25");
        assert!(score.community <= 25, "Community should be capped at 25");
        assert!(
            score.maintenance <= 25,
            "Maintenance should be capped at 25"
        );
        assert!(score.growth <= 25, "Growth should be capped at 25");

        // Total should equal sum of components
        assert_eq!(
            score.total,
            score.activity + score.community + score.maintenance + score.growth,
            "Total should equal sum of components"
        );
    }
}
