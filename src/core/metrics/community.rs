use crate::core::models::CommunityHealth;
use anyhow::{Context, Result};
use serde::Deserialize;

/// Community profile response from GitHub API
#[derive(Debug, Deserialize)]
struct CommunityProfile {
    #[serde(default)]
    health_percentage: u8,
    #[serde(default)]
    files: CommunityFiles,
}

/// Community files information
#[derive(Debug, Deserialize, Default)]
struct CommunityFiles {
    #[serde(default)]
    readme: Option<FileInfo>,
    #[serde(default)]
    license: Option<FileInfo>,
    #[serde(default)]
    contributing: Option<FileInfo>,
    #[serde(default)]
    code_of_conduct: Option<FileInfo>,
    #[serde(default)]
    issue_template: Option<FileInfo>,
    #[serde(default)]
    pull_request_template: Option<FileInfo>,
    #[serde(default)]
    security: Option<FileInfo>,
}

/// Basic file information
#[derive(Debug, Deserialize)]
struct FileInfo {
    #[serde(rename = "html_url")]
    _html_url: String,
}

/// Community health metrics computation
///
/// Provides functionality to fetch community health profile from GitHub,
/// including presence of community files (README, LICENSE, CONTRIBUTING, etc.)
/// and an overall health score.
#[derive(Debug, Clone)]
pub struct CommunityMetrics {
    http: reqwest::Client,
    token: Option<String>,
}

impl CommunityMetrics {
    /// Create a new CommunityMetrics instance with the given token
    ///
    /// # Arguments
    /// * `token` - Optional GitHub personal access token
    pub fn new(token: Option<&str>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("forgeStat/0.1")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            token: token.map(|t| t.to_string()),
        }
    }

    /// Fetches community health statistics for a repository
    ///
    /// Fetches community profile from the GitHub API and extracts:
    /// - Presence of README, LICENSE, CONTRIBUTING, CODE_OF_CONDUCT
    /// - Presence of issue templates, PR template, SECURITY.md
    /// - Overall health percentage score
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Returns
    /// * `Ok(Some(CommunityHealth))` - If community profile was successfully fetched
    /// * `Ok(None)` - If the API returns 404 (repo not found) or 204 (no content)
    /// * `Err(e)` - For other API errors
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<Option<CommunityHealth>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/community/profile",
            owner, repo
        );

        let mut req = self.http.get(&url).header(
            "Accept",
            "application/vnd.github.black-panther-preview+json",
        );

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .send()
            .await
            .with_context(|| format!("Failed to fetch community health for {}/{}", owner, repo))?;

        let status = response.status();

        // The community profile API may return 204 for empty repos or 404 if unavailable
        if status == 204 || status == 404 {
            log::info!(
                "Community health unavailable ({}) for {}/{}. Skipping.",
                status,
                owner,
                repo
            );
            return Ok(None);
        }

        // Handle authentication errors gracefully
        if status == 401 || status == 403 {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::info!(
                "Community health unavailable ({}) for {}/{}: {}. Skipping.",
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

        let profile: CommunityProfile = response.json().await.with_context(|| {
            format!(
                "Failed to parse community health response for {}/{}",
                owner, repo
            )
        })?;

        let health = CommunityHealth {
            has_readme: profile.files.readme.is_some(),
            has_license: profile.files.license.is_some(),
            has_contributing: profile.files.contributing.is_some(),
            has_code_of_conduct: profile.files.code_of_conduct.is_some(),
            has_issue_templates: profile.files.issue_template.is_some(),
            has_pr_template: profile.files.pull_request_template.is_some(),
            has_security_policy: profile.files.security.is_some(),
            score: profile.health_percentage,
        };

        Ok(Some(health))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_health_default_values() {
        let health = CommunityHealth {
            has_readme: true,
            has_license: true,
            has_contributing: false,
            has_code_of_conduct: true,
            has_issue_templates: false,
            has_pr_template: true,
            has_security_policy: false,
            score: 75,
        };

        assert!(health.has_readme);
        assert!(health.has_license);
        assert!(!health.has_contributing);
        assert!(health.has_code_of_conduct);
        assert!(!health.has_issue_templates);
        assert!(health.has_pr_template);
        assert!(!health.has_security_policy);
        assert_eq!(health.score, 75);
    }

    #[test]
    fn test_community_health_all_present() {
        let health = CommunityHealth {
            has_readme: true,
            has_license: true,
            has_contributing: true,
            has_code_of_conduct: true,
            has_issue_templates: true,
            has_pr_template: true,
            has_security_policy: true,
            score: 100,
        };

        assert!(health.has_readme);
        assert!(health.has_license);
        assert!(health.has_contributing);
        assert!(health.has_code_of_conduct);
        assert!(health.has_issue_templates);
        assert!(health.has_pr_template);
        assert!(health.has_security_policy);
        assert_eq!(health.score, 100);
    }

    #[test]
    fn test_community_health_none_present() {
        let health = CommunityHealth {
            has_readme: false,
            has_license: false,
            has_contributing: false,
            has_code_of_conduct: false,
            has_issue_templates: false,
            has_pr_template: false,
            has_security_policy: false,
            score: 0,
        };

        assert!(!health.has_readme);
        assert!(!health.has_license);
        assert!(!health.has_contributing);
        assert!(!health.has_code_of_conduct);
        assert!(!health.has_issue_templates);
        assert!(!health.has_pr_template);
        assert!(!health.has_security_policy);
        assert_eq!(health.score, 0);
    }
}
