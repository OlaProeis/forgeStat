use crate::core::models::{CIStatus, WorkflowRun};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

/// Workflow run from GitHub Actions API
#[derive(Debug, Deserialize)]
struct WorkflowRunResponse {
    #[serde(rename = "id")]
    _id: u64,
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "status")]
    status: String,
    #[serde(rename = "conclusion")]
    conclusion: Option<String>,
    #[serde(rename = "created_at")]
    created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    updated_at: DateTime<Utc>,
    #[serde(rename = "run_started_at")]
    run_started_at: Option<DateTime<Utc>>,
}

/// GitHub Actions workflow runs API response
#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    #[serde(rename = "workflow_runs")]
    workflow_runs: Vec<WorkflowRunResponse>,
}

/// CI metrics computation for GitHub Actions
///
/// Provides functionality to fetch CI/CD statistics from GitHub Actions,
/// including workflow run counts, success rates, and duration metrics.
#[derive(Debug, Clone)]
pub struct CiMetrics {
    http: reqwest::Client,
    token: Option<String>,
}

impl CiMetrics {
    /// Create a new CiMetrics instance with the given token
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

    /// Fetches CI status statistics for a repository
    ///
    /// Fetches workflow runs from the GitHub Actions API and computes:
    /// - Total runs in the last 30 days
    /// - Success rate percentage
    /// - Average workflow duration in seconds
    /// - List of recent workflow runs
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    ///
    /// # Returns
    /// * `Ok(Some(CIStatus))` - If workflow runs were successfully fetched
    /// * `Ok(None)` - If Actions is not enabled or no runs exist
    /// * `Err(e)` - For API errors
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<Option<CIStatus>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs?per_page=20",
            owner, repo
        );

        let mut req = self.http.get(&url);

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req
            .send()
            .await
            .with_context(|| format!("Failed to fetch CI status for {}/{}", owner, repo))?;

        let status = response.status();

        // Handle cases where Actions might not be enabled (404) or permission issues (403)
        if status == 404 || status == 403 {
            log::warn!(
                "GitHub Actions not available ({}) for {}/{}. Token present: {}. Skipping CI metrics.",
                status,
                owner,
                repo,
                self.token.is_some()
            );
            return Ok(None);
        }

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("Unknown error"));
            log::error!(
                "GitHub Actions API error {} for {}/{}: {}",
                status,
                owner,
                repo,
                body
            );
            return Err(anyhow::anyhow!(
                "GitHub API error {} for {}/{}: {}",
                status,
                owner,
                repo,
                body
            ));
        }

        let runs_response: WorkflowRunsResponse = response.json().await.with_context(|| {
            format!(
                "Failed to parse workflow runs response for {}/{}",
                owner, repo
            )
        })?;

        log::info!(
            "Fetched {} workflow runs for {}/{}",
            runs_response.workflow_runs.len(),
            owner,
            repo
        );

        if runs_response.workflow_runs.is_empty() {
            log::info!("No workflow runs found for {}/{}", owner, repo);
            return Ok(None);
        }

        let cutoff = Utc::now() - Duration::days(30);

        // Filter runs from the last 30 days
        let recent_runs: Vec<&WorkflowRunResponse> = runs_response
            .workflow_runs
            .iter()
            .filter(|run| run.created_at >= cutoff)
            .collect();

        let total_runs_30d = recent_runs.len() as u64;

        // Calculate success rate
        let successful_runs = recent_runs
            .iter()
            .filter(|run| {
                run.conclusion
                    .as_ref()
                    .map(|c| c == "success")
                    .unwrap_or(false)
            })
            .count() as u64;

        let success_rate = if total_runs_30d > 0 {
            (successful_runs as f64 / total_runs_30d as f64) * 100.0
        } else {
            0.0
        };

        // Calculate average duration
        let completed_runs: Vec<&WorkflowRunResponse> = recent_runs
            .iter()
            .filter(|run| run.conclusion.is_some())
            .copied()
            .collect();

        let avg_duration_seconds = if !completed_runs.is_empty() {
            let total_duration: i64 = completed_runs
                .iter()
                .map(|run| {
                    let start = run.run_started_at.unwrap_or(run.created_at);
                    let duration = run.updated_at.signed_duration_since(start);
                    duration.num_seconds().max(0)
                })
                .sum();

            (total_duration as u64) / (completed_runs.len() as u64)
        } else {
            0
        };

        // Convert recent runs to our data model (take up to 10 most recent)
        let recent_runs_model: Vec<WorkflowRun> = runs_response
            .workflow_runs
            .iter()
            .take(10)
            .map(|run| {
                let start = run.run_started_at.unwrap_or(run.created_at);
                let duration = run.updated_at.signed_duration_since(start);
                let duration_seconds = duration.num_seconds().max(0) as u64;

                WorkflowRun {
                    name: run.name.clone(),
                    status: run.status.clone(),
                    conclusion: run.conclusion.clone(),
                    created_at: run.created_at,
                    duration_seconds,
                }
            })
            .collect();

        Ok(Some(CIStatus {
            total_runs_30d,
            success_rate,
            avg_duration_seconds,
            recent_runs: recent_runs_model,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_status_default_values() {
        let ci_status = CIStatus {
            total_runs_30d: 50,
            success_rate: 85.5,
            avg_duration_seconds: 300,
            recent_runs: vec![],
        };

        assert_eq!(ci_status.total_runs_30d, 50);
        assert_eq!(ci_status.success_rate, 85.5);
        assert_eq!(ci_status.avg_duration_seconds, 300);
        assert!(ci_status.recent_runs.is_empty());
    }

    #[test]
    fn test_workflow_run_creation() {
        let run = WorkflowRun {
            name: "Test Workflow".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            created_at: Utc::now(),
            duration_seconds: 180,
        };

        assert_eq!(run.name, "Test Workflow");
        assert_eq!(run.status, "completed");
        assert_eq!(run.conclusion, Some("success".to_string()));
        assert_eq!(run.duration_seconds, 180);
    }

    #[test]
    fn test_workflow_run_with_none_conclusion() {
        let run = WorkflowRun {
            name: "Running Workflow".to_string(),
            status: "in_progress".to_string(),
            conclusion: None,
            created_at: Utc::now(),
            duration_seconds: 60,
        };

        assert_eq!(run.name, "Running Workflow");
        assert_eq!(run.status, "in_progress");
        assert!(run.conclusion.is_none());
    }

    #[test]
    fn test_ci_metrics_new() {
        let metrics = CiMetrics::new(None);
        assert!(metrics.token.is_none());

        let metrics_with_token = CiMetrics::new(Some("test_token"));
        assert_eq!(metrics_with_token.token, Some("test_token".to_string()));
    }
}
