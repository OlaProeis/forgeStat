use crate::core::models::{MergedPr, PrStats};
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use octocrab::Octocrab;

/// PR metrics computation and API client extension
///
/// Provides functionality to fetch PR statistics from GitHub,
/// including open PRs, draft vs ready counts, and merged PRs in the last 30 days
/// with average merge time calculations.
#[derive(Debug, Clone)]
pub struct PrsMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> PrsMetrics<'a> {
    /// Create a new PrsMetrics instance with the given octocrab client
    pub fn new(client: &'a Octocrab) -> Self {
        Self { client }
    }

    /// Fetches pull request statistics for a repository
    ///
    /// Fetches open PRs and recently merged PRs (last 30 days),
    /// calculating counts, draft vs ready status, and average merge time.
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<PrStats> {
        // Fetch open PRs
        let pulls_page = self
            .client
            .pulls(owner, repo)
            .list()
            .state(octocrab::params::State::Open)
            .per_page(100)
            .send()
            .await
            .with_context(|| format!("Failed to fetch open PRs for {}/{}", owner, repo))?;

        let pulls: Vec<_> = pulls_page.items;
        let open_count = pulls.len() as u64;

        // Count draft vs ready PRs
        let draft_count = pulls
            .iter()
            .filter(|pr| pr.draft.unwrap_or(false))
            .count() as u64;
        let ready_count = open_count - draft_count;

        // Fetch recently merged PRs (last 30 days)
        let thirty_days_ago = Utc::now() - Duration::days(30);

        let closed_pulls_page = self
            .client
            .pulls(owner, repo)
            .list()
            .state(octocrab::params::State::Closed)
            .per_page(100)
            .send()
            .await
            .with_context(|| format!("Failed to fetch closed PRs for {}/{}", owner, repo))?;

        let mut merged_last_30d: Vec<MergedPr> = Vec::new();
        let mut total_merge_hours: f64 = 0.0;
        let mut merge_count: u64 = 0;

        for pr in closed_pulls_page.items {
            // Only include actually merged PRs (not just closed) within 30 days
            if let Some(merged_at) = pr.merged_at {
                if merged_at <= thirty_days_ago {
                    continue;
                }
                let created_at = pr.created_at.unwrap_or_else(Utc::now);
                let time_to_merge_hours =
                    (merged_at - created_at).num_seconds() as f64 / 3600.0;

                // Handle optional fields for PR
                let title = pr
                    .title
                    .unwrap_or_else(|| format!("PR #{}" , pr.number));
                let author = pr
                    .user
                    .as_ref()
                    .map(|u| u.login.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                merged_last_30d.push(MergedPr {
                    number: pr.number,
                    title,
                    author,
                    created_at,
                    merged_at,
                    time_to_merge_hours,
                });

                total_merge_hours += time_to_merge_hours;
                merge_count += 1;
            }
        }

        let avg_time_to_merge_hours = if merge_count > 0 {
            Some(total_merge_hours / merge_count as f64)
        } else {
            None
        };

        Ok(PrStats {
            open_count,
            draft_count,
            ready_count,
            merged_last_30d,
            avg_time_to_merge_hours,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_stats_default_values() {
        let stats = PrStats {
            open_count: 5,
            draft_count: 1,
            ready_count: 4,
            merged_last_30d: vec![],
            avg_time_to_merge_hours: None,
        };

        assert_eq!(stats.open_count, 5);
        assert_eq!(stats.draft_count, 1);
        assert_eq!(stats.ready_count, 4);
        assert!(stats.merged_last_30d.is_empty());
        assert!(stats.avg_time_to_merge_hours.is_none());
    }

    #[test]
    fn test_pr_stats_with_merged_prs() {
        let merged_pr = MergedPr {
            number: 42,
            title: "Test PR".to_string(),
            author: "testuser".to_string(),
            created_at: Utc::now() - Duration::hours(48),
            merged_at: Utc::now(),
            time_to_merge_hours: 48.0,
        };

        let stats = PrStats {
            open_count: 3,
            draft_count: 0,
            ready_count: 3,
            merged_last_30d: vec![merged_pr.clone()],
            avg_time_to_merge_hours: Some(48.0),
        };

        assert_eq!(stats.merged_last_30d.len(), 1);
        assert_eq!(stats.merged_last_30d[0].number, 42);
        assert_eq!(stats.avg_time_to_merge_hours, Some(48.0));
    }
}
