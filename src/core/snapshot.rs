use anyhow::{Context, Result};
use chrono::Utc;
use tokio::sync::mpsc::Sender;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::core::cache::Cache;
use crate::core::github_client::GitHubClient;
use crate::core::models::RepoSnapshot;
use crate::tui::app::FetchProgress;

const DEFAULT_CACHE_TTL_MINS: u64 = 15;
const API_TIMEOUT_SECS: u64 = 30;

/// Endpoints to fetch with their display names
const ENDPOINTS: &[(&str, &str)] = &[
    ("repository", "Repository"),
    ("stars", "Star History"),
    ("issues", "Issues"),
    ("pull_requests", "Pull Requests"),
    ("contributors", "Contributors"),
    ("releases", "Releases"),
    ("velocity", "Velocity"),
    ("security", "Security Alerts"),
    ("ci_status", "CI Status"),
    ("community", "Community Health"),
];

/// Fetches a complete [`RepoSnapshot`] with cache-first semantics and parallel metric fetching.
///
/// 1. Returns cached snapshot if fresh (< TTL) and `force_refresh` is false.
/// 2. Otherwise fetches all 9 metrics in parallel via [`tokio::try_join!`].
/// 3. Saves the assembled snapshot to cache before returning.
pub async fn fetch_snapshot(
    client: &GitHubClient,
    cache: &Cache,
    owner: &str,
    repo: &str,
    force_refresh: bool,
) -> Result<RepoSnapshot> {
    if !force_refresh && !cache.is_stale(DEFAULT_CACHE_TTL_MINS) {
        if let Some((snapshot, _)) = cache.load().await? {
            log::info!("Cache hit for {}/{}", owner, repo);
            return Ok(snapshot);
        }
    }

    log::info!("Fetching fresh snapshot for {}/{}", owner, repo);

    // Check for existing snapshot to set previous_snapshot_at
    let previous_snapshot_at = cache.load().await?.map(|(snapshot, _)| snapshot.fetched_at);

    // Fetch all metrics with individual timeouts to prevent hanging
    let (
        repo_meta,
        stars,
        issues,
        prs,
        contributors,
        releases,
        velocity,
        security,
        ci_status,
        community,
    ) = tokio::try_join!(
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.repos(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Repo metadata fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.stargazers(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Star History fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.issues(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Issues fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.pull_requests(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Pull Requests fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.contributors(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Contributors fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.releases(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Releases fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.velocity(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Velocity fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.security_alerts(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Security Alerts fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.ci_status(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("CI Status fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
        async {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.community_health(owner, repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Community Health fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        },
    )
    .with_context(|| format!("Failed to fetch metrics for {}/{}", owner, repo))?;

    let snapshot = RepoSnapshot {
        fetched_at: Utc::now(),
        previous_snapshot_at,
        snapshot_history_id: Uuid::new_v4(),
        repo: repo_meta,
        stars,
        issues,
        pull_requests: prs,
        contributors,
        releases,
        velocity,
        security_alerts: security,
        ci_status,
        community_health: community,
    };

    cache.save(&snapshot).await?;

    // Also save to history for diff/comparison functionality
    // This maintains a rolling window of up to 20 snapshots
    cache.save_to_history(&snapshot).await?;

    // Purge any history files older than 30 days
    cache.purge_history(30).await?;

    Ok(snapshot)
}

/// Fetches a complete [`RepoSnapshot`] with progress reporting via the provided channel.
///
/// Similar to [`fetch_snapshot`], but sends [`FetchProgress`] updates as each
/// endpoint completes. This allows the UI to show real-time loading progress.
pub async fn fetch_snapshot_with_progress(
    client: &GitHubClient,
    cache: &Cache,
    owner: &str,
    repo: &str,
    force_refresh: bool,
    progress_tx: Sender<FetchProgress>,
) -> Result<RepoSnapshot> {
    if !force_refresh && !cache.is_stale(DEFAULT_CACHE_TTL_MINS) {
        if let Some((snapshot, _)) = cache.load().await? {
            log::info!("Cache hit for {}/{}", owner, repo);
            // Send completion progress with star count
            let mut progress = FetchProgress::new(1);
            progress.completed = 1;
            progress.done = true;
            progress.star_count = Some(snapshot.stars.total_count);
            let _ = progress_tx.send(progress).await;
            return Ok(snapshot);
        }
    }

    log::info!("Fetching fresh snapshot for {}/{}", owner, repo);

    // Check for existing snapshot to set previous_snapshot_at
    let previous_snapshot_at = cache.load().await?.map(|(snapshot, _)| snapshot.fetched_at);

    let total_endpoints = ENDPOINTS.len();

    // Helper function to report progress
    async fn report_progress(
        tx: &Sender<FetchProgress>,
        total: usize,
        completed: usize,
        current: Option<&str>,
        star_count: Option<u64>,
    ) {
        let progress = FetchProgress {
            total,
            completed,
            current_endpoint: current.map(|s| s.to_string()),
            done: false,
            error: None,
            star_count,
            current_page: None,
            total_pages: None,
        };
        let _ = tx.send(progress).await;
    }

    // First fetch repo metadata and stars count (for large repo detection)
    report_progress(&progress_tx, total_endpoints, 0, Some("Repository"), None).await;
    let repo_meta_result = match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.repos(owner, repo)).await {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!("Repo metadata fetch timed out after {}s", API_TIMEOUT_SECS)),
    };

    // Get star count early for large repo detection - fetch from API
    let star_count = match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.fetch_repo_star_count(owner, repo)).await {
        Ok(Ok(count)) => {
            if count > 5_000 {
                log::info!("Large repo detected: {} stars (showing Pong game)", count);
            }
            Some(count)
        }
        Ok(Err(e)) => {
            log::warn!("Failed to fetch star count: {}", e);
            None
        }
        Err(_) => {
            log::warn!("Star count fetch timed out after {}s", API_TIMEOUT_SECS);
            None
        }
    };

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Star History"),
        star_count,
    )
    .await;

    // Create a progress channel for stargazer page updates
    let (page_progress_tx, mut page_progress_rx) = tokio::sync::mpsc::channel::<(u32, u32)>(100);
    let progress_tx_for_stars = progress_tx.clone();
    let star_count_for_progress = star_count;

    // Spawn a task to forward page progress updates
    let _page_progress_forwarder = tokio::spawn(async move {
        while let Some((current, total)) = page_progress_rx.recv().await {
            let progress = FetchProgress {
                total: total_endpoints,
                completed: 0,
                current_endpoint: Some("Star History".to_string()),
                done: false,
                error: None,
                star_count: star_count_for_progress,
                current_page: Some(current),
                total_pages: Some(total),
            };
            let _ = progress_tx_for_stars.send(progress).await;
        }
    });

    let stars_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        let page_progress_tx = page_progress_tx.clone();
        async move {
            let result = timeout(Duration::from_secs(API_TIMEOUT_SECS), client
                .stargazers_with_progress(
                    &owner,
                    &repo,
                    Some(Box::new(move |current, total| {
                        let _ = page_progress_tx.try_send((current, total));
                    })),
                ))
                .await;
            match result {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Star History fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(&progress_tx, total_endpoints, 0, Some("Issues"), star_count).await;
    let issues_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.issues(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Issues fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Pull Requests"),
        star_count,
    )
    .await;
    let prs_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.pull_requests(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Pull Requests fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Contributors"),
        star_count,
    )
    .await;
    let contributors_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.contributors(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Contributors fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Releases"),
        star_count,
    )
    .await;
    let releases_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.releases(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Releases fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Velocity"),
        star_count,
    )
    .await;
    let velocity_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.velocity(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Velocity fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Security Alerts"),
        star_count,
    )
    .await;
    let security_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.security_alerts(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Security Alerts fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("CI Status"),
        star_count,
    )
    .await;
    let ci_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.ci_status(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("CI Status fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    report_progress(
        &progress_tx,
        total_endpoints,
        0,
        Some("Community Health"),
        star_count,
    )
    .await;
    let community_handle = tokio::spawn({
        let client = client.clone();
        let owner = owner.to_string();
        let repo = repo.to_string();
        async move {
            match timeout(Duration::from_secs(API_TIMEOUT_SECS), client.community_health(&owner, &repo)).await {
                Ok(Ok(data)) => Ok(data),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(anyhow::anyhow!("Community Health fetch timed out after {}s", API_TIMEOUT_SECS)),
            }
        }
    });

    // Collect results with progress updates as each completes
    let repo_meta = repo_meta_result?;
    report_progress(
        &progress_tx,
        total_endpoints,
        1,
        Some("Star History"),
        star_count,
    )
    .await;

    let stars = stars_handle.await?;
    report_progress(&progress_tx, total_endpoints, 2, Some("Issues"), star_count).await;

    let issues = issues_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        3,
        Some("Pull Requests"),
        star_count,
    )
    .await;

    let prs = prs_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        4,
        Some("Contributors"),
        star_count,
    )
    .await;

    let contributors = contributors_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        5,
        Some("Releases"),
        star_count,
    )
    .await;

    let releases = releases_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        6,
        Some("Velocity"),
        star_count,
    )
    .await;

    let velocity = velocity_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        7,
        Some("Security Alerts"),
        star_count,
    )
    .await;

    let security = security_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        8,
        Some("CI Status"),
        star_count,
    )
    .await;

    let ci_status = ci_handle.await?;
    report_progress(
        &progress_tx,
        total_endpoints,
        9,
        Some("Community Health"),
        star_count,
    )
    .await;

    let community = community_handle.await?;

    // Extract results or propagate errors (already unwrapped from handles)
    let stars = stars.with_context(|| "Failed to fetch star history")?;
    let issues = issues.with_context(|| "Failed to fetch issues")?;
    let prs = prs.with_context(|| "Failed to fetch pull requests")?;
    let contributors = contributors.with_context(|| "Failed to fetch contributors")?;
    let releases = releases.with_context(|| "Failed to fetch releases")?;
    let velocity = velocity.with_context(|| "Failed to fetch velocity")?;
    let security = security.with_context(|| "Failed to fetch security alerts")?;
    let ci_status = ci_status.with_context(|| "Failed to fetch CI status")?;
    let community = community.with_context(|| "Failed to fetch community health")?;

    let snapshot = RepoSnapshot {
        fetched_at: Utc::now(),
        previous_snapshot_at,
        snapshot_history_id: Uuid::new_v4(),
        repo: repo_meta,
        stars,
        issues,
        pull_requests: prs,
        contributors,
        releases,
        velocity,
        security_alerts: security,
        ci_status,
        community_health: community,
    };

    cache.save(&snapshot).await?;
    cache.save_to_history(&snapshot).await?;
    cache.purge_history(30).await?;

    // Send final completion progress
    let final_progress = FetchProgress {
        total: total_endpoints,
        completed: total_endpoints,
        current_endpoint: None,
        done: true,
        error: None,
        star_count,
        current_page: None,
        total_pages: None,
    };
    let _ = progress_tx.send(final_progress).await;

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ttl() {
        assert_eq!(DEFAULT_CACHE_TTL_MINS, 15);
    }
}
