use crate::core::models::{Issue, IssueStats};
use anyhow::{Context, Result};
use octocrab::Octocrab;
use std::collections::HashMap;

/// Default maximum number of issues to fetch (to avoid excessive API calls on huge repos)
const DEFAULT_MAX_ISSUES: usize = 250;
/// Issues per page for pagination
const ISSUES_PER_PAGE: u8 = 100;

/// Issues metrics computation and API client extension
///
/// Provides functionality to fetch open issues from GitHub,
/// group them by label, and sort them by age (oldest first).
#[derive(Debug, Clone)]
pub struct IssuesMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> IssuesMetrics<'a> {
    /// Create a new IssuesMetrics instance with the given octocrab client
    pub fn new(client: &'a Octocrab) -> Self {
        Self { client }
    }

    /// Fetches open issues and returns statistics grouped by label
    ///
    /// Fetches open issues from the GitHub API with pagination up to DEFAULT_MAX_ISSUES,
    /// groups them by label, and sorts issues within each group by creation time (oldest first).
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<IssueStats> {
        let mut all_issues = Vec::new();
        let mut page: u32 = 1;

        // Paginate through issues up to the limit
        loop {
            let issues_page = self
                .client
                .issues(owner, repo)
                .list()
                .state(octocrab::params::State::Open)
                .per_page(ISSUES_PER_PAGE)
                .page(page)
                .send()
                .await
                .with_context(|| format!("Failed to fetch issues page {} for {}/{}", page, owner, repo))?;

            let issues: Vec<_> = issues_page.items;
            let fetched = issues.len();
            all_issues.extend(issues);

            // Stop if we've reached the limit or there are no more pages
            if all_issues.len() >= DEFAULT_MAX_ISSUES || fetched < ISSUES_PER_PAGE as usize {
                break;
            }

            page += 1;
        }

        let total_open = all_issues.len() as u64;
        // Check if we hit the limit (indicates truncation)
        let truncated = total_open as usize >= DEFAULT_MAX_ISSUES;

        let mut by_label: HashMap<String, Vec<Issue>> = HashMap::new();
        let mut unlabelled: Vec<Issue> = Vec::new();

        for issue in all_issues {
            let created_at = if issue.created_at.timestamp() > 0 {
                issue.created_at
            } else {
                chrono::Utc::now()
            };

            let updated_at = if issue.updated_at.timestamp() > 0 {
                issue.updated_at
            } else {
                created_at
            };

            let issue_data = Issue {
                number: issue.number,
                title: issue.title,
                author: issue.user.login.clone(),
                created_at,
                updated_at,
                labels: issue
                    .labels
                    .iter()
                    .map(|l| l.name.clone())
                    .collect(),
                comments_count: issue.comments as u64,
            };

            if issue.labels.is_empty() {
                unlabelled.push(issue_data);
            } else {
                for label in &issue.labels {
                    by_label
                        .entry(label.name.clone())
                        .or_default()
                        .push(issue_data.clone());
                }
            }
        }

        // Sort issues within each group by created_at ascending (oldest first)
        sort_issues_by_age_ascending(&mut by_label);
        sort_vec_by_age_ascending(&mut unlabelled);

        Ok(IssueStats {
            total_open,
            by_label,
            unlabelled,
            truncated,
        })
    }
}

/// Sorts issues within each label group by created_at ascending (oldest first)
fn sort_issues_by_age_ascending(by_label: &mut HashMap<String, Vec<Issue>>) {
    for issues in by_label.values_mut() {
        sort_vec_by_age_ascending(issues);
    }
}

/// Sorts a vector of issues by created_at ascending (oldest first)
fn sort_vec_by_age_ascending(issues: &mut Vec<Issue>) {
    issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_test_issue(number: u64, title: &str, age_days: i64) -> Issue {
        let created_at = Utc::now() - Duration::days(age_days);
        Issue {
            number,
            title: title.to_string(),
            author: "testuser".to_string(),
            created_at,
            updated_at: created_at,
            labels: vec![],
            comments_count: 0,
        }
    }

    fn create_test_issue_with_labels(number: u64, title: &str, age_days: i64, labels: Vec<&str>) -> Issue {
        let created_at = Utc::now() - Duration::days(age_days);
        Issue {
            number,
            title: title.to_string(),
            author: "testuser".to_string(),
            created_at,
            updated_at: created_at,
            labels: labels.into_iter().map(|s| s.to_string()).collect(),
            comments_count: 0,
        }
    }

    #[test]
    fn test_sort_vec_by_age_ascending() {
        let mut issues = vec![
            create_test_issue(1, "Newest", 1),
            create_test_issue(2, "Oldest", 10),
            create_test_issue(3, "Middle", 5),
        ];

        sort_vec_by_age_ascending(&mut issues);

        // After sorting: oldest first (highest age_days = oldest)
        assert_eq!(issues[0].number, 2); // Oldest (10 days)
        assert_eq!(issues[1].number, 3); // Middle (5 days)
        assert_eq!(issues[2].number, 1); // Newest (1 day)
    }

    #[test]
    fn test_sort_issues_by_age_ascending() {
        let mut by_label: HashMap<String, Vec<Issue>> = HashMap::new();

        by_label.insert(
            "bug".to_string(),
            vec![
                create_test_issue_with_labels(1, "Recent bug", 1, vec!["bug"]),
                create_test_issue_with_labels(2, "Old bug", 10, vec!["bug"]),
                create_test_issue_with_labels(3, "Medium bug", 5, vec!["bug"]),
            ],
        );

        by_label.insert(
            "enhancement".to_string(),
            vec![
                create_test_issue_with_labels(4, "Recent feature", 2, vec!["enhancement"]),
                create_test_issue_with_labels(5, "Old feature", 15, vec!["enhancement"]),
            ],
        );

        sort_issues_by_age_ascending(&mut by_label);

        // Verify bug issues are sorted oldest first
        let bugs = by_label.get("bug").unwrap();
        assert_eq!(bugs[0].number, 2); // Oldest
        assert_eq!(bugs[1].number, 3); // Middle
        assert_eq!(bugs[2].number, 1); // Newest

        // Verify enhancement issues are sorted oldest first
        let enhancements = by_label.get("enhancement").unwrap();
        assert_eq!(enhancements[0].number, 5); // Oldest
        assert_eq!(enhancements[1].number, 4); // Newest
    }

    #[test]
    fn test_empty_vec_sort() {
        let mut issues: Vec<Issue> = vec![];
        sort_vec_by_age_ascending(&mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_single_issue_sort() {
        let mut issues = vec![create_test_issue(1, "Only", 5)];
        sort_vec_by_age_ascending(&mut issues);
        assert_eq!(issues[0].number, 1);
    }
}
