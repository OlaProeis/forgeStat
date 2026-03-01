use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use octocrab::Octocrab;
use uuid::Uuid;

use crate::core::metrics::ci::CiMetrics;
use crate::core::metrics::community::CommunityMetrics;
use crate::core::metrics::contributors::ContributorsMetrics;
use crate::core::metrics::issues::IssuesMetrics;
use crate::core::metrics::prs::PrsMetrics;
use crate::core::metrics::releases::ReleasesMetrics;
use crate::core::metrics::security::SecurityMetrics;
use crate::core::metrics::stars::{self, StargazerEvent};
use crate::core::metrics::velocity::VelocityMetrics;
use crate::core::models::{
    CommunityHealth, ContributorStats, IssueStats, PrStats, RateLimitInfo, Release, RepoMeta,
    SecurityAlerts, StarHistory, VelocityStats,
};

const STARGAZERS_PER_PAGE: u32 = 100;
/// Maximum pages to fetch for stargazer history (100 pages = 10000 stargazers)
/// For very popular repos (100k+ stars), this ensures 30-day coverage
/// GitHub API limit is typically 5000 requests/hour for authenticated users
const STARGAZERS_MAX_PAGES: u32 = 100;

/// Parses the `rel="last"` page number from a GitHub API `Link` header.
///
/// GitHub caps pagination for many endpoints (e.g., stargazers at ~400 pages).
/// The `Link` header reveals the actual accessible last page, which may be far
/// lower than what `total_count / per_page` would suggest.
fn parse_link_header_last_page(link_header: &str) -> Option<u32> {
    for part in link_header.split(',') {
        if part.contains("rel=\"last\"") {
            // Match "&page=" or "?page=" to avoid matching "per_page="
            let needle = if part.contains("&page=") {
                "&page="
            } else if part.contains("?page=") {
                "?page="
            } else {
                continue;
            };
            if let Some(pos) = part.find(needle) {
                let after = &part[pos + needle.len()..];
                let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                return digits.parse().ok();
            }
        }
    }
    None
}

/// GitHub API client wrapper around octocrab, with a raw HTTP client
/// for endpoints requiring custom headers (e.g., stargazer timestamps).
#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: Octocrab,
    http: reqwest::Client,
    token: Option<String>,
}

impl GitHubClient {
    /// Creates a new GitHub client with optional authentication token
    ///
    /// # Arguments
    /// * `token` - Optional GitHub personal access token
    ///
    /// # Errors
    /// Returns an error if the client builder fails
    pub fn new(token: Option<&str>) -> Result<Self> {
        let builder = Octocrab::builder();

        let builder = if let Some(t) = token {
            builder.personal_token(t.to_string())
        } else {
            builder
        };

        let client = builder.build().context("Failed to build GitHub client")?;

        let http = reqwest::Client::builder()
            .user_agent("forgeStat/0.1")
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            http,
            token: token.map(|t| t.to_string()),
        })
    }

    /// Fetches repository metadata
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    pub async fn repos(&self, owner: &str, repo: &str) -> Result<RepoMeta> {
        let repo_data = self
            .client
            .repos(owner, repo)
            .get()
            .await
            .with_context(|| format!("Failed to fetch repo metadata for {}/{}", owner, repo))?;

        // Convert language from Option<Value> to Option<String>
        let language = repo_data
            .language
            .as_ref()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        Ok(RepoMeta {
            owner: owner.to_string(),
            name: repo.to_string(),
            description: repo_data.description,
            language,
            created_at: repo_data.created_at.unwrap_or_else(Utc::now),
            updated_at: repo_data.updated_at.unwrap_or_else(Utc::now),
            default_branch: repo_data
                .default_branch
                .unwrap_or_else(|| "main".to_string()),
            forks_count: repo_data.forks_count.unwrap_or(0) as u64,
            open_issues_count: repo_data.open_issues_count.unwrap_or(0) as u64,
            watchers_count: repo_data.watchers_count.unwrap_or(0) as u64,
        })
    }

    /// Fetches star history with real sparkline data from the GitHub Stargazers API.
    ///
    /// Uses the `Accept: application/vnd.github.star+json` header to get
    /// `starred_at` timestamps, then bins them into sparkline buckets.
    /// Paginates backwards from the newest stargazers, capped at
    /// [`STARGAZERS_MAX_PAGES`] pages to stay within rate limits.
    pub async fn stargazers(&self, owner: &str, repo: &str) -> Result<StarHistory> {
        self.stargazers_with_progress(owner, repo, None).await
    }

    /// Fetches star history with optional progress callback for page-by-page updates.
    ///
    /// The progress callback receives (current_page, total_pages) for paginated fetches.
    pub async fn stargazers_with_progress(
        &self,
        owner: &str,
        repo: &str,
        progress_cb: Option<Box<dyn Fn(u32, u32) + Send>>,
    ) -> Result<StarHistory> {
        let repo_data = self
            .client
            .repos(owner, repo)
            .get()
            .await
            .with_context(|| format!("Failed to fetch star count for {}/{}", owner, repo))?;

        let total_count = repo_data.stargazers_count.unwrap_or(0) as u64;
        let now = Utc::now();
        let repo_created_at = repo_data
            .created_at
            .unwrap_or_else(|| now - Duration::days(365));
        let repo_age_days = (now - repo_created_at).num_days();

        // For repos older than 1 year, fetch more history to show meaningful 1-year sparkline
        // Cap at 365 days even for old repos to avoid excessive API calls
        let cutoff_days = if repo_age_days > 365 {
            365
        } else {
            repo_age_days.max(30)
        };

        log::info!(
            "Fetching stargazers for {}/{}: total_count={}, repo_age_days={}, cutoff_days={}",
            owner,
            repo,
            total_count,
            repo_age_days,
            cutoff_days
        );

        // Use Arc to share the callback between potential GraphQL and REST paths
        let timestamps = if total_count == 0 {
            Vec::new()
        } else {
            let calculated_last_page =
                ((total_count as f64) / (STARGAZERS_PER_PAGE as f64)).ceil() as u32;
            let estimated_pages = calculated_last_page.min(STARGAZERS_MAX_PAGES);

            // Try GraphQL first for large repos
            if self.token.is_some() && calculated_last_page > 400 {
                match self
                    .fetch_stargazer_timestamps_graphql_with_progress(
                        owner,
                        repo,
                        cutoff_days,
                        estimated_pages,
                        progress_cb,
                    )
                    .await
                {
                    Ok(ts) => ts,
                    Err(e) => {
                        log::warn!(
                            "GraphQL stargazer fetch failed for {}/{}, falling back to REST: {}",
                            owner,
                            repo,
                            e
                        );
                        // Fall back to REST (without progress callback since it was consumed)
                        self.fetch_stargazer_timestamps_rest(owner, repo, total_count, cutoff_days)
                            .await?
                    }
                }
            } else {
                // Use REST directly with progress
                self.fetch_stargazer_timestamps_rest_with_progress(
                    owner,
                    repo,
                    total_count,
                    cutoff_days,
                    estimated_pages,
                    progress_cb,
                )
                .await?
            }
        };

        log::info!(
            "Fetched {} stargazer timestamps for {}/{} (covering {} days)",
            timestamps.len(),
            owner,
            repo,
            cutoff_days
        );

        // Timestamps are already sorted chronologically by fetch_stargazer_timestamps
        let sparkline_30d = stars::generate_sparkline(&timestamps, now - Duration::days(30), 30);
        let sparkline_90d = stars::generate_sparkline(&timestamps, now - Duration::days(90), 13);

        // For 1-year view: use weekly buckets for projects younger than 1 year
        // to maintain good visual density like the 90-day view.
        // Start from repo creation date so the chart shows the full ramp from 0 stars.
        // For older repos, always show full 365 days with 12 monthly buckets.
        let (sparkline_365d_start, sparkline_365d_buckets) =
            if repo_age_days < 365 && !timestamps.is_empty() {
                let active_period_days = (now - repo_created_at).num_days().max(1);
                let weeks = (active_period_days / 7).clamp(1, 52) as usize;
                (repo_created_at, weeks)
            } else {
                (now - Duration::days(365), 12)
            };
        let sparkline_365d =
            stars::generate_sparkline(&timestamps, sparkline_365d_start, sparkline_365d_buckets);

        let total_30d: u32 = sparkline_30d.iter().sum();
        let total_90d: u32 = sparkline_90d.iter().sum();
        let total_365d: u32 = sparkline_365d.iter().sum();

        log::info!(
            "StarHistory for {}/{}: timestamps={}, 30d_sum={}, 90d_sum={}, 365d_sum={}",
            owner,
            repo,
            timestamps.len(),
            total_30d,
            total_90d,
            total_365d
        );

        Ok(StarHistory {
            total_count,
            sparkline_30d,
            sparkline_90d,
            sparkline_365d,
        })
    }

    /// Fetches stargazer timestamps, choosing the best strategy based on repo size.
    ///
    /// For repos with >40k stars (where REST pagination is capped at ~400 pages of
    /// the OLDEST stargazers), uses the GraphQL API to fetch recent stargazers in
    /// reverse chronological order. Falls back to REST for smaller repos or when
    /// GraphQL is unavailable (e.g., unauthenticated users).
    #[allow(dead_code)]
    async fn fetch_stargazer_timestamps(
        &self,
        owner: &str,
        repo: &str,
        total_count: u64,
        cutoff_days: i64,
    ) -> Result<Vec<DateTime<Utc>>> {
        if total_count == 0 {
            return Ok(Vec::new());
        }

        let calculated_last_page =
            ((total_count as f64) / (STARGAZERS_PER_PAGE as f64)).ceil() as u32;

        // REST API paginates oldest-first with a cap of ~400 pages.
        // For repos above that cap, the newest stargazers are simply unreachable
        // via REST. Use GraphQL which supports orderBy: DESC.
        if self.token.is_some() && calculated_last_page > 400 {
            match self
                .fetch_stargazer_timestamps_graphql(owner, repo, cutoff_days)
                .await
            {
                Ok(ts) => return Ok(ts),
                Err(e) => {
                    log::warn!(
                        "GraphQL stargazer fetch failed for {}/{}, falling back to REST: {}",
                        owner,
                        repo,
                        e
                    );
                }
            }
        }

        self.fetch_stargazer_timestamps_rest(owner, repo, total_count, cutoff_days)
            .await
    }

    /// Fetches recent stargazer timestamps via the GitHub GraphQL API with progress.
    ///
    /// Uses `orderBy: { field: STARRED_AT, direction: DESC }` to get the most
    /// recent stargazers first, then paginates forward (going further back in time)
    /// until we pass the cutoff or exhaust the page budget.
    #[allow(dead_code)]
    async fn fetch_stargazer_timestamps_graphql(
        &self,
        owner: &str,
        repo: &str,
        cutoff_days: i64,
    ) -> Result<Vec<DateTime<Utc>>> {
        let token = self
            .token
            .as_ref()
            .context("GraphQL stargazer fetch requires authentication")?;

        let cutoff = Utc::now() - Duration::days(cutoff_days);
        let mut all_timestamps = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages_fetched: u32 = 0;

        log::info!(
            "Fetching stargazers via GraphQL for {}/{} (cutoff: {} days)",
            owner,
            repo,
            cutoff_days
        );

        static QUERY: &str = r#"query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    stargazers(first: 100, after: $cursor, orderBy: {field: STARRED_AT, direction: DESC}) {
      edges { starredAt }
      pageInfo { hasNextPage endCursor }
    }
  }
}"#;

        loop {
            if pages_fetched >= STARGAZERS_MAX_PAGES {
                log::info!("Reached GraphQL page limit ({} pages)", pages_fetched);
                break;
            }

            let body = serde_json::json!({
                "query": QUERY,
                "variables": {
                    "owner": owner,
                    "name": repo,
                    "cursor": cursor,
                }
            });

            let resp = self
                .http
                .post("https://api.github.com/graphql")
                .header("Authorization", format!("Bearer {}", token))
                .json(&body)
                .send()
                .await
                .with_context(|| {
                    format!("GraphQL stargazer request failed for {}/{}", owner, repo)
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!(
                    "GraphQL returned {} for {}/{}: {}",
                    status,
                    owner,
                    repo,
                    text
                );
            }

            let data: serde_json::Value = resp
                .json()
                .await
                .context("Failed to parse GraphQL stargazer response")?;

            if let Some(errors) = data.get("errors") {
                anyhow::bail!("GraphQL errors for {}/{}: {}", owner, repo, errors);
            }

            let stargazers = &data["data"]["repository"]["stargazers"];
            let edges = stargazers["edges"]
                .as_array()
                .context("Missing edges in GraphQL stargazer response")?;

            if edges.is_empty() {
                break;
            }

            let mut reached_cutoff = false;
            for edge in edges {
                if let Some(ts_str) = edge["starredAt"].as_str() {
                    if let Ok(ts) = ts_str.parse::<DateTime<Utc>>() {
                        if ts < cutoff {
                            reached_cutoff = true;
                        } else {
                            all_timestamps.push(ts);
                        }
                    }
                }
            }

            pages_fetched += 1;

            if reached_cutoff {
                log::info!("GraphQL: reached cutoff at page {}", pages_fetched);
                break;
            }

            let page_info = &stargazers["pageInfo"];
            if !page_info["hasNextPage"].as_bool().unwrap_or(false) {
                break;
            }

            cursor = page_info["endCursor"].as_str().map(|s| s.to_string());
        }

        all_timestamps.sort();

        log::info!(
            "GraphQL stargazer fetch complete: {} timestamps from {} pages",
            all_timestamps.len(),
            pages_fetched
        );

        Ok(all_timestamps)
    }

    /// GraphQL stargazer fetch with progress callback.
    async fn fetch_stargazer_timestamps_graphql_with_progress(
        &self,
        owner: &str,
        repo: &str,
        cutoff_days: i64,
        estimated_pages: u32,
        progress_cb: Option<Box<dyn Fn(u32, u32) + Send>>,
    ) -> Result<Vec<DateTime<Utc>>> {
        let token = self
            .token
            .as_ref()
            .context("GraphQL stargazer fetch requires authentication")?;

        let cutoff = Utc::now() - Duration::days(cutoff_days);
        let mut all_timestamps = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages_fetched: u32 = 0;

        log::info!(
            "Fetching stargazers via GraphQL for {}/{} (cutoff: {} days, est. {} pages)",
            owner,
            repo,
            cutoff_days,
            estimated_pages
        );

        static QUERY: &str = r#"query($owner: String!, $name: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    stargazers(first: 100, after: $cursor, orderBy: {field: STARRED_AT, direction: DESC}) {
      edges { starredAt }
      pageInfo { hasNextPage endCursor }
    }
  }
}"#;

        loop {
            if pages_fetched >= STARGAZERS_MAX_PAGES {
                log::info!("Reached GraphQL page limit ({} pages)", pages_fetched);
                break;
            }

            // Report progress before fetching this page
            if let Some(ref cb) = progress_cb {
                cb(pages_fetched + 1, estimated_pages);
            }

            let body = serde_json::json!({
                "query": QUERY,
                "variables": {
                    "owner": owner,
                    "name": repo,
                    "cursor": cursor,
                }
            });

            let resp = self
                .http
                .post("https://api.github.com/graphql")
                .header("Authorization", format!("Bearer {}", token))
                .json(&body)
                .send()
                .await
                .with_context(|| {
                    format!("GraphQL stargazer request failed for {}/{}", owner, repo)
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                anyhow::bail!(
                    "GraphQL returned {} for {}/{}: {}",
                    status,
                    owner,
                    repo,
                    text
                );
            }

            let data: serde_json::Value = resp
                .json()
                .await
                .context("Failed to parse GraphQL stargazer response")?;

            if let Some(errors) = data.get("errors") {
                anyhow::bail!("GraphQL errors for {}/{}: {}", owner, repo, errors);
            }

            let stargazers = &data["data"]["repository"]["stargazers"];
            let edges = stargazers["edges"]
                .as_array()
                .context("Missing edges in GraphQL stargazer response")?;

            if edges.is_empty() {
                break;
            }

            let mut reached_cutoff = false;
            for edge in edges {
                if let Some(ts_str) = edge["starredAt"].as_str() {
                    if let Ok(ts) = ts_str.parse::<DateTime<Utc>>() {
                        if ts < cutoff {
                            reached_cutoff = true;
                        } else {
                            all_timestamps.push(ts);
                        }
                    }
                }
            }

            pages_fetched += 1;

            if reached_cutoff {
                log::info!("GraphQL: reached cutoff at page {}", pages_fetched);
                break;
            }

            let page_info = &stargazers["pageInfo"];
            if !page_info["hasNextPage"].as_bool().unwrap_or(false) {
                break;
            }

            cursor = page_info["endCursor"].as_str().map(|s| s.to_string());
        }

        all_timestamps.sort();

        log::info!(
            "GraphQL stargazer fetch complete: {} timestamps from {} pages",
            all_timestamps.len(),
            pages_fetched
        );

        Ok(all_timestamps)
    }

    /// REST-based stargazer timestamp fetching (oldest-first pagination).
    ///
    /// Works well for repos with ≤40k stars where all pages are accessible.
    /// Uses the `Link` header to discover the actual last accessible page.
    async fn fetch_stargazer_timestamps_rest(
        &self,
        owner: &str,
        repo: &str,
        total_count: u64,
        cutoff_days: i64,
    ) -> Result<Vec<DateTime<Utc>>> {
        let calculated_last_page =
            ((total_count as f64) / (STARGAZERS_PER_PAGE as f64)).ceil() as u32;
        let cutoff = Utc::now() - Duration::days(cutoff_days);

        let last_page = self
            .discover_stargazer_last_page(owner, repo, calculated_last_page)
            .await;

        let start_page = last_page.saturating_sub(STARGAZERS_MAX_PAGES - 1).max(1);

        let mut all_timestamps = Vec::new();
        let mut pages_fetched = 0;

        log::info!(
            "REST stargazer pagination: calculated_last={}, api_last={}, start_page={}, cutoff={} ({} days ago)",
            calculated_last_page, last_page, start_page, cutoff, cutoff_days
        );

        for page in (start_page..=last_page).rev() {
            pages_fetched += 1;
            let url = format!(
                "https://api.github.com/repos/{}/{}/stargazers?per_page={}&page={}",
                owner, repo, STARGAZERS_PER_PAGE, page
            );

            let mut req = self
                .http
                .get(&url)
                .header("Accept", "application/vnd.github.star+json");

            if let Some(ref token) = self.token {
                req = req.header("Authorization", format!("Bearer {}", token));
            }

            let response = req.send().await.with_context(|| {
                format!(
                    "Failed to fetch stargazers page {} for {}/{}",
                    page, owner, repo
                )
            })?;

            if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                if let Ok(r) = remaining.to_str().unwrap_or("").parse::<u64>() {
                    if r < 10 {
                        log::warn!("GitHub API rate limit low: {} requests remaining", r);
                    }
                }
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                log::warn!(
                    "Stargazer API returned {} for {}/{} page {}: body={}",
                    status,
                    owner,
                    repo,
                    page,
                    body
                );
                break;
            }

            let events: Vec<StargazerEvent> = response.json().await.with_context(|| {
                format!("Failed to parse stargazer response for {}/{}", owner, repo)
            })?;

            let mut reached_cutoff = false;
            for event in events {
                if event.starred_at < cutoff {
                    reached_cutoff = true;
                } else {
                    all_timestamps.push(event.starred_at);
                }
            }

            if reached_cutoff {
                log::info!("REST: reached cutoff at page {}", page);
                break;
            }
        }

        all_timestamps.sort();

        log::info!(
            "REST stargazer fetch complete: {} timestamps from {} pages (cutoff: {} days)",
            all_timestamps.len(),
            pages_fetched,
            cutoff_days
        );

        Ok(all_timestamps)
    }

    /// REST-based stargazer fetch with progress callback.
    async fn fetch_stargazer_timestamps_rest_with_progress(
        &self,
        owner: &str,
        repo: &str,
        total_count: u64,
        cutoff_days: i64,
        estimated_pages: u32,
        progress_cb: Option<Box<dyn Fn(u32, u32) + Send>>,
    ) -> Result<Vec<DateTime<Utc>>> {
        let calculated_last_page =
            ((total_count as f64) / (STARGAZERS_PER_PAGE as f64)).ceil() as u32;
        let cutoff = Utc::now() - Duration::days(cutoff_days);

        let last_page = self
            .discover_stargazer_last_page(owner, repo, calculated_last_page)
            .await;

        let start_page = last_page.saturating_sub(STARGAZERS_MAX_PAGES - 1).max(1);

        let total_pages_to_fetch = last_page - start_page + 1;
        let display_total = estimated_pages.max(total_pages_to_fetch);

        let mut all_timestamps = Vec::new();
        let mut pages_fetched = 0;

        log::info!(
            "REST stargazer pagination: calculated_last={}, api_last={}, start_page={}, cutoff={} ({} days ago), est_pages={}",
            calculated_last_page, last_page, start_page, cutoff, cutoff_days, display_total
        );

        for page in (start_page..=last_page).rev() {
            pages_fetched += 1;

            // Report progress before fetching this page
            if let Some(ref cb) = progress_cb {
                cb(pages_fetched, display_total);
            }

            let url = format!(
                "https://api.github.com/repos/{}/{}/stargazers?per_page={}&page={}",
                owner, repo, STARGAZERS_PER_PAGE, page
            );

            let mut req = self
                .http
                .get(&url)
                .header("Accept", "application/vnd.github.star+json");

            if let Some(ref token) = self.token {
                req = req.header("Authorization", format!("Bearer {}", token));
            }

            let response = req.send().await.with_context(|| {
                format!(
                    "Failed to fetch stargazers page {} for {}/{}",
                    page, owner, repo
                )
            })?;

            if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                if let Ok(r) = remaining.to_str().unwrap_or("").parse::<u64>() {
                    if r < 10 {
                        log::warn!("GitHub API rate limit low: {} requests remaining", r);
                    }
                }
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                log::warn!(
                    "Stargazer API returned {} for {}/{} page {}: body={}",
                    status,
                    owner,
                    repo,
                    page,
                    body
                );
                break;
            }

            let events: Vec<StargazerEvent> = response.json().await.with_context(|| {
                format!("Failed to parse stargazer response for {}/{}", owner, repo)
            })?;

            let mut reached_cutoff = false;
            for event in events {
                if event.starred_at < cutoff {
                    reached_cutoff = true;
                } else {
                    all_timestamps.push(event.starred_at);
                }
            }

            if reached_cutoff {
                log::info!("REST: reached cutoff at page {}", page);
                break;
            }
        }

        all_timestamps.sort();

        log::info!(
            "REST stargazer fetch complete: {} timestamps from {} pages (cutoff: {} days)",
            all_timestamps.len(),
            pages_fetched,
            cutoff_days
        );

        Ok(all_timestamps)
    }

    /// Probes the stargazer API to discover the real last page from the `Link` header.
    ///
    /// Falls back to `calculated_last_page` if the probe fails or no `Link` header
    /// is present (single-page repos).
    async fn discover_stargazer_last_page(
        &self,
        owner: &str,
        repo: &str,
        calculated_last_page: u32,
    ) -> u32 {
        let url = format!(
            "https://api.github.com/repos/{}/{}/stargazers?per_page={}&page=1",
            owner, repo, STARGAZERS_PER_PAGE
        );

        let mut req = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github.star+json");

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        match req.send().await {
            Ok(resp) => {
                let api_last = resp
                    .headers()
                    .get("link")
                    .and_then(|h| h.to_str().ok())
                    .and_then(parse_link_header_last_page);

                match api_last {
                    Some(page) => {
                        if page < calculated_last_page {
                            log::info!(
                                "GitHub API caps stargazer pagination at page {} (calculated {})",
                                page,
                                calculated_last_page
                            );
                        }
                        page
                    }
                    None => {
                        // No Link header → single page or very small repo
                        calculated_last_page.min(1)
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Stargazer probe request failed, using calculated page: {}",
                    e
                );
                calculated_last_page
            }
        }
    }

    /// Fetches issue statistics for a repository
    ///
    /// Fetches open issues and groups them by label, sorted by age (oldest first).
    /// Uses the issues metrics module for the actual computation.
    pub async fn issues(&self, owner: &str, repo: &str) -> Result<IssueStats> {
        let metrics = IssuesMetrics::new(&self.client);
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches pull request statistics for a repository
    ///
    /// Fetches open PRs and recently merged PRs (last 30 days).
    /// Uses the PR metrics module for the actual computation.
    pub async fn pull_requests(&self, owner: &str, repo: &str) -> Result<PrStats> {
        let metrics = PrsMetrics::new(&self.client);
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches contributor statistics for a repository
    ///
    /// Fetches top contributors and identifies new contributors (last 30 days).
    /// Uses the contributors metrics module for the actual computation.
    pub async fn contributors(&self, owner: &str, repo: &str) -> Result<ContributorStats> {
        let metrics = ContributorsMetrics::new(&self.client);
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches releases for a repository
    ///
    /// Fetches the last 5 releases and computes cadence metrics including
    /// days since each release and average interval between releases.
    pub async fn releases(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        let metrics = ReleasesMetrics::new(&self.client);
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches velocity statistics for a repository
    ///
    /// Returns weekly opened-vs-closed counts for issues and PRs over the last 8 weeks.
    pub async fn velocity(&self, owner: &str, repo: &str) -> Result<VelocityStats> {
        let metrics = VelocityMetrics::new(&self.client);
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches security alert statistics for a repository
    ///
    /// Returns Dependabot alert counts by severity if the token has `security_events` scope.
    /// Returns `None` if the token lacks the required scope or is unauthenticated.
    pub async fn security_alerts(&self, owner: &str, repo: &str) -> Result<Option<SecurityAlerts>> {
        let metrics = SecurityMetrics::new(self.token.as_deref());
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches CI status statistics for a repository
    ///
    /// Returns GitHub Actions workflow run statistics including:
    /// - Total runs in the last 30 days
    /// - Success rate percentage
    /// - Average workflow duration
    /// - Recent workflow runs
    ///
    /// Returns `None` if Actions is not enabled or no runs exist.
    pub async fn ci_status(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<crate::core::models::CIStatus>> {
        let metrics = CiMetrics::new(self.token.as_deref());
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches community health profile for a repository
    ///
    /// Returns community health metrics including:
    /// - Presence of README, LICENSE, CONTRIBUTING, CODE_OF_CONDUCT
    /// - Presence of issue templates, PR template, SECURITY.md
    /// - Overall health percentage score
    ///
    /// Returns `None` if the API is unavailable or returns 404/204.
    pub async fn community_health(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<CommunityHealth>> {
        let metrics = CommunityMetrics::new(self.token.as_deref());
        metrics.fetch_stats(owner, repo).await
    }

    /// Fetches just the star count for a repository (quick API call).
    ///
    /// This is much faster than stargazers() which fetches all timestamps.
    /// Used for detecting large repos before starting the slow fetch.
    pub async fn fetch_repo_star_count(&self, owner: &str, repo: &str) -> Result<u64> {
        let repo_data = self
            .client
            .repos(owner, repo)
            .get()
            .await
            .with_context(|| format!("Failed to fetch repo info for {}/{}", owner, repo))?;

        Ok(repo_data.stargazers_count.unwrap_or(0) as u64)
    }

    /// Fetches current GitHub API rate limit using the `/rate_limit` endpoint.
    ///
    /// This endpoint is free and does not count against the rate limit.
    pub async fn fetch_rate_limit(&self) -> Result<RateLimitInfo> {
        let mut req = self.http.get("https://api.github.com/rate_limit");
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .context("Failed to fetch rate limit info")?;

        if !resp.status().is_success() {
            anyhow::bail!("Rate limit API returned {}", resp.status());
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse rate limit response")?;

        let rate = body
            .get("rate")
            .context("Missing 'rate' field in rate limit response")?;
        let limit = rate.get("limit").and_then(|v| v.as_u64()).unwrap_or(0);
        let remaining = rate.get("remaining").and_then(|v| v.as_u64()).unwrap_or(0);
        let reset_ts = rate.get("reset").and_then(|v| v.as_i64()).unwrap_or(0);
        let reset_at = DateTime::from_timestamp(reset_ts, 0).unwrap_or_else(Utc::now);

        Ok(RateLimitInfo {
            limit,
            remaining,
            reset_at,
        })
    }

    /// Fetches complete repository snapshot including all metrics
    ///
    /// This combines all individual fetch operations into a single snapshot
    pub async fn fetch_snapshot(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<crate::core::models::RepoSnapshot> {
        use crate::core::models::RepoSnapshot;

        let repo_meta = self.repos(owner, repo).await?;
        let stars = self.stargazers(owner, repo).await?;
        let issues = self.issues(owner, repo).await?;
        let pull_requests = self.pull_requests(owner, repo).await?;
        let contributors = self.contributors(owner, repo).await?;
        let releases = self.releases(owner, repo).await?;
        let velocity = self.velocity(owner, repo).await?;

        // Security alerts require a token with security_events scope
        // Returns None gracefully if the scope is missing
        let security_alerts = self.security_alerts(owner, repo).await?;

        Ok(RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: repo_meta,
            stars,
            issues,
            pull_requests,
            contributors,
            releases,
            velocity,
            security_alerts,
            ci_status: None, // CI status is fetched separately in the main snapshot flow
            community_health: None, // Community health is fetched separately in the main snapshot flow
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_github_client_new_unauthenticated() {
        let client = GitHubClient::new(None);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_github_client_new_with_token() {
        let client = GitHubClient::new(Some("test_token"));
        assert!(client.is_ok());
    }

    #[test]
    fn test_parse_link_header_last_page_typical() {
        let header = r#"<https://api.github.com/repos/microsoft/vscode/stargazers?per_page=100&page=2>; rel="next", <https://api.github.com/repos/microsoft/vscode/stargazers?per_page=100&page=400>; rel="last""#;
        assert_eq!(parse_link_header_last_page(header), Some(400));
    }

    #[test]
    fn test_parse_link_header_last_page_small_repo() {
        let header = r#"<https://api.github.com/repos/user/repo/stargazers?per_page=100&page=2>; rel="next", <https://api.github.com/repos/user/repo/stargazers?per_page=100&page=3>; rel="last""#;
        assert_eq!(parse_link_header_last_page(header), Some(3));
    }

    #[test]
    fn test_parse_link_header_no_last_rel() {
        let header = r#"<https://api.github.com/repos/user/repo/stargazers?per_page=100&page=2>; rel="next""#;
        assert_eq!(parse_link_header_last_page(header), None);
    }

    #[test]
    fn test_parse_link_header_empty() {
        assert_eq!(parse_link_header_last_page(""), None);
    }

    #[test]
    fn test_parse_link_header_page_1_only() {
        let header = r#"<https://api.github.com/repos/user/repo/stargazers?per_page=100&page=1>; rel="last""#;
        assert_eq!(parse_link_header_last_page(header), Some(1));
    }
}
