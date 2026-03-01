use crate::core::models::{Contributor, ContributorStats};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use octocrab::Octocrab;
use std::collections::HashMap;

/// Contributors metrics computation and API client extension
///
/// Provides functionality to fetch contributor statistics from GitHub,
/// including top contributors by commit count, new contributors in the last 30 days,
/// and total unique contributor count.
#[derive(Debug, Clone)]
pub struct ContributorsMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> ContributorsMetrics<'a> {
    /// Create a new ContributorsMetrics instance with the given octocrab client
    pub fn new(client: &'a Octocrab) -> Self {
        Self { client }
    }

    /// Fetches contributor statistics for a repository
    ///
    /// Fetches top contributors from the GitHub API and analyzes commit history
    /// to identify new contributors in the last 30 days.
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<ContributorStats> {
        // Fetch contributors list (sorted by contributions, already sorted by GitHub)
        let contributors_page = self
            .client
            .repos(owner, repo)
            .list_contributors()
            .per_page(100)
            .send()
            .await
            .with_context(|| format!("Failed to fetch contributors for {}/{}", owner, repo))?;

        let contributors: Vec<_> = contributors_page.items;
        let total_unique = contributors.len() as u64;

        // Build top contributors list (sorted by contributions, already sorted by GitHub)
        let top_contributors: Vec<Contributor> = contributors
            .iter()
            .take(10)
            .map(|c| Contributor {
                username: c.author.login.clone(),
                commit_count: c.contributions as u64,
                avatar_url: Some(c.author.avatar_url.to_string()),
            })
            .collect();

        // Identify new contributors by analyzing recent commits
        // Limit to top 10 to avoid excessive API calls on large repos
        let new_contributors_last_30d = self
            .identify_new_contributors(owner, repo, 10)
            .await?;

        Ok(ContributorStats {
            top_contributors,
            new_contributors_last_30d,
            total_unique,
        })
    }

    /// Identifies new contributors in the last 30 days by analyzing commit history
    ///
    /// Fetches recent commits and determines which contributors made their first
    /// commit within the last 30 days. Limited to max_contributors to avoid
    /// excessive API calls on large repositories.
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    /// * `max_contributors` - Maximum number of recent contributors to check
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    async fn identify_new_contributors(
        &self,
        owner: &str,
        repo: &str,
        max_contributors: usize,
    ) -> Result<Vec<String>> {
        let thirty_days_ago = Utc::now() - Duration::days(30);

        // Fetch recent commits (last 30 days) - limit to 30 to reduce API calls
        let commits_page = self
            .client
            .repos(owner, repo)
            .list_commits()
            .since(thirty_days_ago)
            .per_page(30)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to fetch recent commits for new contributor analysis {}/{}",
                    owner, repo
                )
            })?;

        // Track contributors seen in the last 30 days and their first commit date
        let mut recent_contributors: HashMap<String, DateTime<Utc>> = HashMap::new();

        for commit in commits_page.items {
            // Get the GitHub user (author) and the commit date
            if let Some(author) = commit.author {
                let username = author.login.clone();

                // Get the commit date from the git commit author info
                if let Some(commit_author) = commit.commit.author {
                    if let Some(commit_date) = commit_author.date {
                        // Keep track of the earliest commit date for each contributor
                        recent_contributors
                            .entry(username)
                            .and_modify(|date| {
                                if commit_date < *date {
                                    *date = commit_date;
                                }
                            })
                            .or_insert(commit_date);
                    }
                }
            }
        }

        // For each recent contributor, check if they have any commits before 30 days ago
        // Limit to max_contributors to avoid excessive API calls
        let mut new_contributors: Vec<String> = Vec::new();

        for (username, _first_recent_commit) in recent_contributors.iter().take(max_contributors) {
            // Check if this contributor has any commits before 30 days ago
            let has_older_commits = self
                .has_commits_before_date(owner, repo, username, thirty_days_ago)
                .await?;

            if !has_older_commits {
                new_contributors.push(username.clone());
            }
        }

        // Sort alphabetically for consistent output
        new_contributors.sort();

        Ok(new_contributors)
    }

    /// Checks if a contributor has any commits before a given date
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    /// * `username` - Contributor username to check
    /// * `before_date` - The date to check against
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    async fn has_commits_before_date(
        &self,
        owner: &str,
        repo: &str,
        username: &str,
        before_date: DateTime<Utc>,
    ) -> Result<bool> {
        // Fetch commits by this user up until the cutoff date
        // If there are any commits returned, the user has older contributions
        let commits_page = self
            .client
            .repos(owner, repo)
            .list_commits()
            .author(username)
            .until(before_date)
            .per_page(1)
            .send()
            .await;

        match commits_page {
            Ok(page) => Ok(!page.items.is_empty()),
            Err(_) => {
                // If the API call fails (e.g., user not found or permission issue),
                // assume they might be new to be conservative
                log::warn!(
                    "Failed to check older commits for contributor {} in {}/{}",
                    username,
                    owner,
                    repo
                );
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_stats_default_values() {
        let stats = ContributorStats {
            top_contributors: vec![],
            new_contributors_last_30d: vec![],
            total_unique: 0,
        };

        assert!(stats.top_contributors.is_empty());
        assert!(stats.new_contributors_last_30d.is_empty());
        assert_eq!(stats.total_unique, 0);
    }

    #[test]
    fn test_contributor_stats_with_data() {
        let contributor = Contributor {
            username: "testuser".to_string(),
            commit_count: 100,
            avatar_url: Some("https://example.com/avatar.png".to_string()),
        };

        let stats = ContributorStats {
            top_contributors: vec![contributor.clone()],
            new_contributors_last_30d: vec!["newbie".to_string()],
            total_unique: 50,
        };

        assert_eq!(stats.top_contributors.len(), 1);
        assert_eq!(stats.top_contributors[0].username, "testuser");
        assert_eq!(stats.top_contributors[0].commit_count, 100);
        assert_eq!(stats.new_contributors_last_30d.len(), 1);
        assert_eq!(stats.new_contributors_last_30d[0], "newbie");
        assert_eq!(stats.total_unique, 50);
    }

    #[test]
    fn test_contributor_sorting() {
        let contributors = vec![
            Contributor {
                username: "user1".to_string(),
                commit_count: 50,
                avatar_url: None,
            },
            Contributor {
                username: "user2".to_string(),
                commit_count: 100,
                avatar_url: None,
            },
            Contributor {
                username: "user3".to_string(),
                commit_count: 25,
                avatar_url: None,
            },
        ];

        // Simulate top 10 selection (already sorted by GitHub API)
        let top: Vec<_> = contributors.iter().take(10).cloned().collect();

        assert_eq!(top.len(), 3);
        assert_eq!(top[0].username, "user1");
        assert_eq!(top[0].commit_count, 50);
    }
}
