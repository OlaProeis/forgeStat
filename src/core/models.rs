use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Main snapshot of repository data at a point in time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoSnapshot {
    pub fetched_at: DateTime<Utc>,
    /// Reference to the previous snapshot in history (None for first snapshot)
    pub previous_snapshot_at: Option<DateTime<Utc>>,
    /// Unique identifier for this snapshot in history
    pub snapshot_history_id: Uuid,
    pub repo: RepoMeta,
    pub stars: StarHistory,
    pub issues: IssueStats,
    pub pull_requests: PrStats,
    pub contributors: ContributorStats,
    pub releases: Vec<Release>,
    pub velocity: VelocityStats,
    pub security_alerts: Option<SecurityAlerts>,
    /// CI/CD status from GitHub Actions (None if Actions not enabled or no access)
    #[serde(default)]
    pub ci_status: Option<CIStatus>,
    /// Community health profile (None if API unavailable or insufficient permissions)
    #[serde(default)]
    pub community_health: Option<CommunityHealth>,
}

impl RepoSnapshot {
    /// Get the age of the oldest open issue in days
    /// Returns None if there are no open issues
    pub fn oldest_issue_age_days(&self) -> Option<i64> {
        let mut all_issues: Vec<&Issue> = Vec::new();
        for issues in self.issues.by_label.values() {
            all_issues.extend(issues.iter());
        }
        all_issues.extend(self.issues.unlabelled.iter());

        all_issues
            .iter()
            .min_by_key(|issue| issue.created_at)
            .map(|oldest| {
                let age = Utc::now().signed_duration_since(oldest.created_at);
                age.num_days()
            })
    }

    /// Get days since the last release
    /// Returns None if there are no releases
    pub fn days_since_last_release(&self) -> Option<i64> {
        self.releases.first().and_then(|release| release.days_since)
    }

    /// Get count of open issues
    pub fn open_issues_count(&self) -> u64 {
        self.issues.total_open
    }

    /// Get count of open PRs
    pub fn open_prs_count(&self) -> u64 {
        self.pull_requests.open_count
    }

    /// Get the full GitHub URL for this repository
    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}/{}", self.repo.owner, self.repo.name)
    }

    /// Format an issue reference as 'owner/repo#number'
    pub fn format_issue_reference(&self, issue_number: u64) -> String {
        format!("{}/{}#{}", self.repo.owner, self.repo.name, issue_number)
    }
}

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepoMeta {
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_branch: String,
    pub forks_count: u64,
    pub open_issues_count: u64,
    pub watchers_count: u64,
}

/// Star history with sparkline data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StarHistory {
    pub total_count: u64,
    /// Star counts for last 30 days (day-by-day)
    pub sparkline_30d: Vec<u32>,
    /// Star counts for last 90 days (aggregated by week or day)
    pub sparkline_90d: Vec<u32>,
    /// Star counts for last 365 days (aggregated by month or week)
    pub sparkline_365d: Vec<u32>,
}

/// Issue statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IssueStats {
    pub total_open: u64,
    /// Issues grouped by label
    pub by_label: HashMap<String, Vec<Issue>>,
    /// Issues with no labels
    pub unlabelled: Vec<Issue>,
    /// Whether the issue list was truncated due to the fetch limit
    #[serde(default)]
    pub truncated: bool,
}

/// Single issue representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub labels: Vec<String>,
    pub comments_count: u64,
}

/// Pull request statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrStats {
    pub open_count: u64,
    pub draft_count: u64,
    pub ready_count: u64,
    /// Merged PRs in the last 30 days
    pub merged_last_30d: Vec<MergedPr>,
    /// Average time to merge (in hours)
    pub avg_time_to_merge_hours: Option<f64>,
}

/// Merged PR details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergedPr {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub merged_at: DateTime<Utc>,
    /// Time to merge in hours
    pub time_to_merge_hours: f64,
}

/// Contributor statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributorStats {
    /// Top 10 contributors by commit count
    pub top_contributors: Vec<Contributor>,
    /// New contributors in the last 30 days
    pub new_contributors_last_30d: Vec<String>,
    /// Total unique contributors
    pub total_unique: u64,
}

/// Single contributor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contributor {
    pub username: String,
    pub commit_count: u64,
    pub avatar_url: Option<String>,
}

/// Release information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
    pub draft: bool,
    /// Days since this release was published (relative to fetch time)
    pub days_since: Option<i64>,
    /// Average interval in days between this release and previous ones
    /// Only populated for the most recent release
    pub avg_interval: Option<f64>,
}

/// Velocity statistics (weekly activity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VelocityStats {
    /// Issues opened vs closed per week (last 8 weeks)
    pub issues_weekly: Vec<WeeklyActivity>,
    /// PRs opened vs merged per week (last 8 weeks)
    pub prs_weekly: Vec<WeeklyActivity>,
}

/// Weekly activity metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeeklyActivity {
    pub week_start: DateTime<Utc>,
    pub opened: u64,
    pub closed: u64,
}

/// Security alerts (Dependabot)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityAlerts {
    pub total_open: u64,
    pub critical_count: u64,
    pub high_count: u64,
    pub medium_count: u64,
    pub low_count: u64,
}

/// CI/CD status for GitHub Actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CIStatus {
    /// Total workflow runs in the last 30 days
    pub total_runs_30d: u64,
    /// Success rate percentage (0.0 - 100.0)
    pub success_rate: f64,
    /// Average workflow duration in seconds
    pub avg_duration_seconds: u64,
    /// Recent workflow runs (up to 10)
    pub recent_runs: Vec<WorkflowRun>,
}

/// Community health profile metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommunityHealth {
    /// Has README.md file
    pub has_readme: bool,
    /// Has LICENSE file
    pub has_license: bool,
    /// Has CONTRIBUTING.md file
    pub has_contributing: bool,
    /// Has CODE_OF_CONDUCT.md file
    pub has_code_of_conduct: bool,
    /// Has issue template(s)
    pub has_issue_templates: bool,
    /// Has pull request template
    pub has_pr_template: bool,
    /// Has SECURITY.md file
    pub has_security_policy: bool,
    /// Overall health score (0-100)
    pub score: u8,
}

/// Single workflow run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowRun {
    /// Name of the workflow
    pub name: String,
    /// Current status (e.g., "completed", "in_progress", "queued")
    pub status: String,
    /// Final conclusion (e.g., "success", "failure", "cancelled") - None if still running
    pub conclusion: Option<String>,
    /// When the run was created
    pub created_at: DateTime<Utc>,
    /// Duration of the run in seconds
    pub duration_seconds: u64,
}

/// GitHub API rate limit information (ephemeral, not cached)
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset_at: DateTime<Utc>,
}

/// Diff between two snapshots for split-screen comparison
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    /// Time when the previous snapshot was taken
    pub previous_fetched_at: DateTime<Utc>,
    /// Star count difference (current - previous)
    pub stars_delta: i64,
    /// Issue count difference (current - previous)
    pub issues_delta: i64,
    /// Open PR count difference (current - previous)
    pub prs_delta: i64,
    /// New security alerts since previous snapshot
    pub new_security_critical: u64,
    pub new_security_high: u64,
    pub new_security_medium: u64,
    pub new_security_low: u64,
    /// Contributor count difference
    pub contributors_delta: i64,
    /// Fork count difference
    pub forks_delta: i64,
    /// Watchers count difference
    pub watchers_delta: i64,
    /// Release count difference
    pub releases_delta: i64,
}

impl SnapshotDiff {
    /// Compute diff between current and previous snapshot
    pub fn compute(current: &RepoSnapshot, previous: &RepoSnapshot) -> Self {
        let current_sec = current.security_alerts.as_ref();
        let previous_sec = previous.security_alerts.as_ref();

        Self {
            previous_fetched_at: previous.fetched_at,
            stars_delta: current.stars.total_count as i64 - previous.stars.total_count as i64,
            issues_delta: current.issues.total_open as i64 - previous.issues.total_open as i64,
            prs_delta: current.pull_requests.open_count as i64
                - previous.pull_requests.open_count as i64,
            new_security_critical: current_sec
                .map(|c| c.critical_count)
                .unwrap_or(0)
                .saturating_sub(previous_sec.map(|p| p.critical_count).unwrap_or(0)),
            new_security_high: current_sec
                .map(|c| c.high_count)
                .unwrap_or(0)
                .saturating_sub(previous_sec.map(|p| p.high_count).unwrap_or(0)),
            new_security_medium: current_sec
                .map(|c| c.medium_count)
                .unwrap_or(0)
                .saturating_sub(previous_sec.map(|p| p.medium_count).unwrap_or(0)),
            new_security_low: current_sec
                .map(|c| c.low_count)
                .unwrap_or(0)
                .saturating_sub(previous_sec.map(|p| p.low_count).unwrap_or(0)),
            contributors_delta: current.contributors.total_unique as i64
                - previous.contributors.total_unique as i64,
            forks_delta: current.repo.forks_count as i64 - previous.repo.forks_count as i64,
            watchers_delta: current.repo.watchers_count as i64
                - previous.repo.watchers_count as i64,
            releases_delta: current.releases.len() as i64 - previous.releases.len() as i64,
        }
    }

    /// Format the "last viewed X ago" string
    pub fn format_time_ago(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.previous_fetched_at);

        if duration.num_minutes() < 60 {
            format!("{} min ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{} hours ago", duration.num_hours())
        } else {
            format!("{} days ago", duration.num_days())
        }
    }

    /// Check if there are any new security alerts
    pub fn has_new_security_alerts(&self) -> bool {
        self.new_security_critical > 0
            || self.new_security_high > 0
            || self.new_security_medium > 0
            || self.new_security_low > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "octocat".to_string(),
                name: "Hello-World".to_string(),
                description: Some("My first repository".to_string()),
                language: Some("Rust".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                default_branch: "main".to_string(),
                forks_count: 100,
                open_issues_count: 10,
                watchers_count: 1000,
            },
            stars: StarHistory {
                total_count: 5000,
                sparkline_30d: vec![10, 12, 15, 8, 20],
                sparkline_90d: vec![100, 120, 150, 80, 200],
                sparkline_365d: vec![1000, 1200, 1500, 800, 2000],
            },
            issues: IssueStats {
                total_open: 10,
                by_label: {
                    let mut map = HashMap::new();
                    map.insert(
                        "bug".to_string(),
                        vec![Issue {
                            number: 1,
                            title: "Test bug".to_string(),
                            author: "user1".to_string(),
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            labels: vec!["bug".to_string()],
                            comments_count: 5,
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
                merged_last_30d: vec![MergedPr {
                    number: 42,
                    title: "Fix something".to_string(),
                    author: "contributor".to_string(),
                    created_at: Utc::now(),
                    merged_at: Utc::now(),
                    time_to_merge_hours: 24.0,
                }],
                avg_time_to_merge_hours: Some(48.0),
            },
            contributors: ContributorStats {
                top_contributors: vec![Contributor {
                    username: "octocat".to_string(),
                    commit_count: 100,
                    avatar_url: None,
                }],
                new_contributors_last_30d: vec!["newbie".to_string()],
                total_unique: 50,
            },
            releases: vec![Release {
                tag_name: "v1.0.0".to_string(),
                name: Some("First Release".to_string()),
                created_at: Utc::now(),
                published_at: Some(Utc::now()),
                prerelease: false,
                draft: false,
                days_since: Some(5),
                avg_interval: Some(14.0),
            }],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: Utc::now(),
                    opened: 10,
                    closed: 8,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: Utc::now(),
                    opened: 5,
                    closed: 4,
                }],
            },
            security_alerts: Some(SecurityAlerts {
                total_open: 2,
                critical_count: 0,
                high_count: 1,
                medium_count: 1,
                low_count: 0,
            }),
            ci_status: Some(CIStatus {
                total_runs_30d: 50,
                success_rate: 85.5,
                avg_duration_seconds: 300,
                recent_runs: vec![WorkflowRun {
                    name: "CI".to_string(),
                    status: "completed".to_string(),
                    conclusion: Some("success".to_string()),
                    created_at: Utc::now(),
                    duration_seconds: 180,
                }],
            }),
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
        }
    }

    #[test]
    fn test_repo_snapshot_serde_roundtrip() {
        let original = create_sample_snapshot();
        let json = serde_json::to_string(&original).expect("Failed to serialize");
        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.fetched_at, deserialized.fetched_at);
        assert_eq!(original.repo.owner, deserialized.repo.owner);
        assert_eq!(original.repo.name, deserialized.repo.name);
        assert_eq!(original.stars.total_count, deserialized.stars.total_count);
        assert_eq!(original.issues.total_open, deserialized.issues.total_open);
        assert_eq!(
            original.pull_requests.open_count,
            deserialized.pull_requests.open_count
        );
        assert_eq!(
            original.contributors.total_unique,
            deserialized.contributors.total_unique
        );
        assert_eq!(original.releases.len(), deserialized.releases.len());
        assert_eq!(
            original.security_alerts.as_ref().map(|s| s.total_open),
            deserialized.security_alerts.as_ref().map(|s| s.total_open)
        );
    }

    #[test]
    fn test_all_fields_present() {
        let snapshot = create_sample_snapshot();
        let json = serde_json::to_string_pretty(&snapshot).expect("Failed to serialize");

        // Verify all main fields are present in JSON
        assert!(json.contains("fetched_at"));
        assert!(json.contains("repo"));
        assert!(json.contains("stars"));
        assert!(json.contains("issues"));
        assert!(json.contains("pull_requests"));
        assert!(json.contains("contributors"));
        assert!(json.contains("releases"));
        assert!(json.contains("velocity"));
        assert!(json.contains("security_alerts"));
        assert!(json.contains("ci_status"));
    }

    #[test]
    fn test_nested_struct_serialization() {
        let issue = Issue {
            number: 123,
            title: "Test Issue".to_string(),
            author: "testuser".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            labels: vec!["bug".to_string(), "help wanted".to_string()],
            comments_count: 3,
        };

        let json = serde_json::to_string(&issue).expect("Failed to serialize issue");
        let deserialized: Issue = serde_json::from_str(&json).expect("Failed to deserialize issue");

        assert_eq!(issue.number, deserialized.number);
        assert_eq!(issue.title, deserialized.title);
        assert_eq!(issue.labels, deserialized.labels);
    }

    #[test]
    fn test_option_security_alerts_none() {
        let snapshot = RepoSnapshot {
            security_alerts: None,
            ..create_sample_snapshot()
        };

        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(deserialized.security_alerts.is_none());
    }

    #[test]
    fn test_option_ci_status_none() {
        let snapshot = RepoSnapshot {
            ci_status: None,
            ..create_sample_snapshot()
        };

        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(deserialized.ci_status.is_none());
    }

    #[test]
    fn test_ci_status_roundtrip() {
        let snapshot = create_sample_snapshot();

        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(deserialized.ci_status.is_some());
        let ci = deserialized.ci_status.unwrap();
        assert_eq!(ci.total_runs_30d, 50);
        assert_eq!(ci.success_rate, 85.5);
        assert_eq!(ci.avg_duration_seconds, 300);
        assert_eq!(ci.recent_runs.len(), 1);
    }

    #[test]
    fn test_hashmap_serialization() {
        let mut by_label = HashMap::new();
        by_label.insert(
            "enhancement".to_string(),
            vec![Issue {
                number: 1,
                title: "Feature request".to_string(),
                author: "user".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                labels: vec!["enhancement".to_string()],
                comments_count: 0,
            }],
        );

        let stats = IssueStats {
            total_open: 1,
            by_label,
            unlabelled: vec![],
            truncated: false,
        };

        let json = serde_json::to_string(&stats).expect("Failed to serialize");
        let deserialized: IssueStats = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.total_open, 1);
        assert!(deserialized.by_label.contains_key("enhancement"));
    }

    #[test]
    fn test_new_fields_serialization() {
        let snapshot = create_sample_snapshot();
        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");

        // Verify new fields are present in JSON
        assert!(json.contains("previous_snapshot_at"));
        assert!(json.contains("snapshot_history_id"));

        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify UUID roundtrip
        assert_eq!(
            snapshot.snapshot_history_id,
            deserialized.snapshot_history_id
        );
        assert_eq!(
            snapshot.previous_snapshot_at,
            deserialized.previous_snapshot_at
        );
    }

    #[test]
    fn test_snapshot_with_previous() {
        let now = Utc::now();
        let previous = now - chrono::Duration::hours(1);

        let snapshot = RepoSnapshot {
            fetched_at: now,
            previous_snapshot_at: Some(previous),
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
            security_alerts: None,
            ci_status: None,
            community_health: None,
        };

        let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
        let deserialized: RepoSnapshot =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.previous_snapshot_at, Some(previous));
        assert!(deserialized.snapshot_history_id.to_string().len() > 0);
    }

    #[test]
    fn test_uuid_uniqueness() {
        // Generate multiple UUIDs and verify uniqueness
        let uuids: Vec<_> = (0..100).map(|_| Uuid::new_v4()).collect();
        let unique_count = uuids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 100);
    }

    // =========================================================================
    // RepoSnapshot helper method tests
    // =========================================================================

    #[test]
    fn test_open_issues_count() {
        let snapshot = create_sample_snapshot();
        assert_eq!(snapshot.open_issues_count(), 10);
    }

    #[test]
    fn test_open_prs_count() {
        let snapshot = create_sample_snapshot();
        assert_eq!(snapshot.open_prs_count(), 5);
    }

    #[test]
    fn test_days_since_last_release_with_releases() {
        let snapshot = create_sample_snapshot();
        assert_eq!(snapshot.days_since_last_release(), Some(5));
    }

    #[test]
    fn test_days_since_last_release_no_releases() {
        let mut snapshot = create_sample_snapshot();
        snapshot.releases = vec![];
        assert_eq!(snapshot.days_since_last_release(), None);
    }

    #[test]
    fn test_oldest_issue_age_days_with_issues() {
        let snapshot = create_sample_snapshot();
        // The oldest issue is from the sample data
        // We can't assert exact age since Utc::now() changes, but we can verify it's >= 0
        let age = snapshot.oldest_issue_age_days();
        assert!(age.is_some());
        assert!(age.unwrap() >= 0);
    }

    #[test]
    fn test_oldest_issue_age_days_no_issues() {
        let mut snapshot = create_sample_snapshot();
        snapshot.issues.total_open = 0;
        snapshot.issues.by_label.clear();
        snapshot.issues.unlabelled.clear();
        assert_eq!(snapshot.oldest_issue_age_days(), None);
    }

    #[test]
    fn test_oldest_issue_age_days_only_unlabelled() {
        let mut snapshot = create_sample_snapshot();
        snapshot.issues.by_label.clear();
        snapshot.issues.unlabelled = vec![
            Issue {
                number: 1,
                title: "Old unlabelled".to_string(),
                author: "user".to_string(),
                created_at: Utc::now() - chrono::Duration::days(30),
                updated_at: Utc::now(),
                labels: vec![],
                comments_count: 0,
            },
            Issue {
                number: 2,
                title: "Newer unlabelled".to_string(),
                author: "user".to_string(),
                created_at: Utc::now() - chrono::Duration::days(10),
                updated_at: Utc::now(),
                labels: vec![],
                comments_count: 0,
            },
        ];

        let age = snapshot.oldest_issue_age_days();
        assert!(age.is_some());
        // Should be approximately 30 days
        assert!(age.unwrap() >= 29);
    }

    #[test]
    fn test_oldest_issue_age_days_mixed_issues() {
        let mut snapshot = create_sample_snapshot();
        snapshot.issues.unlabelled = vec![Issue {
            number: 100,
            title: "Old unlabelled".to_string(),
            author: "user".to_string(),
            created_at: Utc::now() - chrono::Duration::days(45),
            updated_at: Utc::now(),
            labels: vec![],
            comments_count: 0,
        }];

        // The by_label issues are from create_sample_snapshot and are from Utc::now()
        // So the oldest should be the unlabelled one (45 days)
        let age = snapshot.oldest_issue_age_days();
        assert!(age.is_some());
        assert!(age.unwrap() >= 44);
    }
}
