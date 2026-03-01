use crate::core::models::Release;
use anyhow::{Context, Result};
use chrono::Utc;
use octocrab::Octocrab;

/// Release metrics computation and API client extension
///
/// Provides functionality to fetch release statistics from GitHub,
/// including last 5 releases, days since each release, and average interval between releases.
#[derive(Debug, Clone)]
pub struct ReleasesMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> ReleasesMetrics<'a> {
    /// Create a new ReleasesMetrics instance with the given octocrab client
    pub fn new(client: &'a Octocrab) -> Self {
        Self { client }
    }

    /// Fetches release statistics for a repository
    ///
    /// Fetches the last 5 releases from the GitHub API and computes:
    /// - Days since each release
    /// - Average interval between consecutive releases (for the most recent release)
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Errors
    /// Returns an error if the GitHub API request fails
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        let releases_page = self
            .client
            .repos(owner, repo)
            .releases()
            .list()
            .per_page(5u8)
            .send()
            .await
            .with_context(|| format!("Failed to fetch releases for {}/{}", owner, repo))?;

        let now = Utc::now();

        // Convert octocrab releases to our Release model
        let mut releases: Vec<Release> = releases_page
            .items
            .into_iter()
            .map(|r| {
                let published_at = r.published_at;
                let days_since = published_at.map(|dt| (now - dt).num_days());

                Release {
                    tag_name: r.tag_name,
                    name: r.name,
                    created_at: r.created_at.unwrap_or_else(Utc::now),
                    published_at,
                    prerelease: r.prerelease,
                    draft: r.draft,
                    days_since,
                    avg_interval: None,
                }
            })
            .collect();

        // Calculate average interval between consecutive releases
        if releases.len() > 1 {
            let intervals: Vec<i64> = releases
                .windows(2)
                .filter_map(|w| {
                    let current = w[0].published_at?;
                    let previous = w[1].published_at?;
                    Some((current - previous).num_days())
                })
                .collect();

            if !intervals.is_empty() {
                let avg_interval = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
                releases[0].avg_interval = Some(avg_interval);
            }
        }

        Ok(releases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};

    fn create_test_release_with_time(
        tag: &str,
        name: &str,
        reference_time: DateTime<Utc>,
        published_days_ago: i64,
    ) -> Release {
        let published_at = reference_time - Duration::days(published_days_ago);
        Release {
            tag_name: tag.to_string(),
            name: Some(name.to_string()),
            created_at: published_at,
            published_at: Some(published_at),
            prerelease: false,
            draft: false,
            days_since: Some(published_days_ago),
            avg_interval: None,
        }
    }

    #[test]
    fn test_releases_default_values() {
        let release = Release {
            tag_name: "v1.0.0".to_string(),
            name: Some("Initial Release".to_string()),
            created_at: Utc::now(),
            published_at: None,
            prerelease: false,
            draft: false,
            days_since: None,
            avg_interval: None,
        };

        assert_eq!(release.tag_name, "v1.0.0");
        assert!(release.days_since.is_none());
        assert!(release.avg_interval.is_none());
    }

    #[test]
    fn test_avg_interval_calculation() {
        let reference_time = Utc::now();

        // Create releases: most recent (5 days ago), then 15, 25, 35 days ago
        let releases = vec![
            create_test_release_with_time("v1.4.0", "Release 4", reference_time, 5),
            create_test_release_with_time("v1.3.0", "Release 3", reference_time, 15),
            create_test_release_with_time("v1.2.0", "Release 2", reference_time, 25),
            create_test_release_with_time("v1.1.0", "Release 1", reference_time, 35),
        ];

        // Calculate intervals: 10 days between 4-3, 10 days between 3-2, 10 days between 2-1
        // Average = (10 + 10 + 10) / 3 = 10.0
        let intervals: Vec<i64> = releases
            .windows(2)
            .filter_map(|w| {
                let current = w[0].published_at?;
                let previous = w[1].published_at?;
                Some((current - previous).num_days())
            })
            .collect();

        assert_eq!(intervals.len(), 3);
        assert_eq!(intervals[0], 10);
        assert_eq!(intervals[1], 10);
        assert_eq!(intervals[2], 10);

        let avg_interval = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
        assert_eq!(avg_interval, 10.0);
    }

    #[test]
    fn test_single_release_no_avg_interval() {
        let reference_time = Utc::now();
        let releases = [create_test_release_with_time(
            "v1.0.0",
            "Only Release",
            reference_time,
            5,
        )];

        // With only 1 release, there are no intervals to calculate
        assert_eq!(releases.len(), 1);
        assert!(releases[0].avg_interval.is_none());
    }

    #[test]
    fn test_two_releases_avg_interval() {
        let reference_time = Utc::now();

        // Create releases: 10 days apart
        let releases = vec![
            create_test_release_with_time("v1.1.0", "Second Release", reference_time, 10),
            create_test_release_with_time("v1.0.0", "First Release", reference_time, 20),
        ];

        let intervals: Vec<i64> = releases
            .windows(2)
            .filter_map(|w| {
                let current = w[0].published_at?;
                let previous = w[1].published_at?;
                Some((current - previous).num_days())
            })
            .collect();

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0], 10);

        let avg_interval = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
        assert_eq!(avg_interval, 10.0);
    }

    #[test]
    fn test_days_since_calculation() {
        let now = Utc::now();
        let five_days_ago = now - Duration::days(5);

        let release = Release {
            tag_name: "v1.0.0".to_string(),
            name: Some("Test".to_string()),
            created_at: five_days_ago,
            published_at: Some(five_days_ago),
            prerelease: false,
            draft: false,
            days_since: Some((now - five_days_ago).num_days()),
            avg_interval: None,
        };

        assert_eq!(release.days_since, Some(5));
    }
}
