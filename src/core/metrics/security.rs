use crate::core::models::SecurityAlerts;
use anyhow::{Context, Result};
use serde::Deserialize;

/// Dependabot alert from GitHub API
#[derive(Debug, Deserialize)]
struct DependabotAlert {
    #[serde(rename = "number")]
    _number: u64,
    #[serde(rename = "state")]
    _state: String,
    #[serde(rename = "security_advisory")]
    security_advisory: SecurityAdvisory,
}

/// Security advisory details within a Dependabot alert
#[derive(Debug, Deserialize)]
struct SecurityAdvisory {
    #[serde(rename = "severity")]
    severity: String,
}

/// Security metrics computation for Dependabot alerts
///
/// Provides functionality to fetch security alert statistics from GitHub,
/// including total open alerts and counts by severity (critical, high, medium, low).
/// Requires a GitHub token with `security_events` scope.
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    http: reqwest::Client,
    token: Option<String>,
}

impl SecurityMetrics {
    /// Create a new SecurityMetrics instance with the given token
    ///
    /// # Arguments
    /// * `token` - Optional GitHub personal access token (required for security_events scope)
    pub fn new(token: Option<&str>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("repowatch/0.1")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            token: token.map(|t| t.to_string()),
        }
    }

    /// Fetches security alert statistics for a repository
    ///
    /// Fetches Dependabot alerts from the GitHub API and computes:
    /// - Total open alerts count
    /// - Counts by severity: critical, high, medium, low
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Returns
    /// * `Ok(Some(SecurityAlerts))` - If alerts were successfully fetched
    /// * `Ok(None)` - If the token lacks `security_events` scope or is unauthenticated
    /// * `Err(e)` - For other API errors
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<Option<SecurityAlerts>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/dependabot/alerts?state=open&per_page=100",
            owner, repo
        );

        let mut req = self.http.get(&url);

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .send()
            .await
            .with_context(|| format!("Failed to fetch security alerts for {}/{}", owner, repo))?;

        let status = response.status();

        // Security alerts require elevated permissions (admin access or
        // security_events scope). Return None gracefully for any auth error
        // so the rest of the snapshot fetch isn't blocked.
        if status == 401 || status == 403 || status == 404 {
            // Log the actual error message to help diagnose permission issues
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::info!(
                "Security alerts unavailable ({}) for {}/{}: {}. Skipping.",
                status,
                owner,
                repo,
                error_body
            );
            return Ok(None);
        }

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("Unknown error"));
            return Err(anyhow::anyhow!(
                "GitHub API error {} for {}/{}: {}",
                status,
                owner,
                repo,
                body
            ));
        }

        let alerts: Vec<DependabotAlert> = response.json().await.with_context(|| {
            format!(
                "Failed to parse security alerts response for {}/{}",
                owner, repo
            )
        })?;

        // Count alerts by severity
        let mut total_open = 0u64;
        let mut critical_count = 0u64;
        let mut high_count = 0u64;
        let mut medium_count = 0u64;
        let mut low_count = 0u64;

        for alert in alerts {
            total_open += 1;

            match alert.security_advisory.severity.as_str() {
                "critical" => critical_count += 1,
                "high" => high_count += 1,
                "medium" => medium_count += 1,
                "low" => low_count += 1,
                _ => {
                    // Unknown severity - log and ignore
                    log::debug!(
                        "Unknown severity '{}' for alert in {}/{}",
                        alert.security_advisory.severity,
                        owner,
                        repo
                    );
                }
            }
        }

        Ok(Some(SecurityAlerts {
            total_open,
            critical_count,
            high_count,
            medium_count,
            low_count,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_alerts_default_values() {
        let alerts = SecurityAlerts {
            total_open: 5,
            critical_count: 1,
            high_count: 2,
            medium_count: 1,
            low_count: 1,
        };

        assert_eq!(alerts.total_open, 5);
        assert_eq!(alerts.critical_count, 1);
        assert_eq!(alerts.high_count, 2);
        assert_eq!(alerts.medium_count, 1);
        assert_eq!(alerts.low_count, 1);
    }

    #[test]
    fn test_security_alerts_zero_counts() {
        let alerts = SecurityAlerts {
            total_open: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
        };

        assert_eq!(alerts.total_open, 0);
        assert_eq!(alerts.critical_count, 0);
        assert_eq!(alerts.high_count, 0);
        assert_eq!(alerts.medium_count, 0);
        assert_eq!(alerts.low_count, 0);
    }
}
