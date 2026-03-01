use crate::core::models::{VelocityStats, WeeklyActivity};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use octocrab::Octocrab;

const VELOCITY_MAX_OPEN_PRS: usize = 250;
const VELOCITY_MAX_CLOSED_PRS: usize = 750;
const VELOCITY_PER_PAGE: u8 = 100;

/// Velocity metrics computation and API client extension
///
/// Provides weekly velocity statistics for issues and PRs over the last 8 weeks,
/// tracking opened vs closed/merged counts per week.
#[derive(Debug, Clone)]
pub struct VelocityMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> VelocityMetrics<'a> {
    pub fn new(client: &'a Octocrab) -> Self {
        Self { client }
    }

    /// Fetches velocity statistics for the last 8 weeks
    ///
    /// Returns weekly opened-vs-closed counts for both issues and PRs.
    /// Issues use `closed_at`; PRs use `merged_at` for the "closed" metric.
    ///
    /// # Arguments
    /// * `owner` - Repository owner (username or organization)
    /// * `repo` - Repository name
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<VelocityStats> {
        let week_starts = compute_week_starts(8);
        let cutoff = week_starts[0];

        let issues_weekly = self
            .fetch_issue_velocity(owner, repo, &week_starts, cutoff)
            .await?;
        let prs_weekly = self
            .fetch_pr_velocity(owner, repo, &week_starts, cutoff)
            .await?;

        Ok(VelocityStats {
            issues_weekly,
            prs_weekly,
        })
    }

    /// Fetches issue velocity using the GitHub Search API for accurate per-week counts.
    ///
    /// The Search API supports date-range qualifiers (`created:`, `closed:`) and returns
    /// `total_count` without needing to paginate through all results. This is far more
    /// accurate than the Issues list endpoint for mega-repos where 1000 items sorted by
    /// `created DESC` only covers the most recent week.
    async fn fetch_issue_velocity(
        &self,
        owner: &str,
        repo: &str,
        week_starts: &[DateTime<Utc>],
        _cutoff: DateTime<Utc>,
    ) -> Result<Vec<WeeklyActivity>> {
        let now = Utc::now();
        let mut weekly = init_weekly(week_starts);
        let mut any_error = false;

        for (i, &ws) in week_starts.iter().enumerate() {
            let start_date = ws.format("%Y-%m-%d");
            let end_date = if i + 1 < week_starts.len() {
                (week_starts[i + 1] - Duration::days(1)).format("%Y-%m-%d").to_string()
            } else {
                now.format("%Y-%m-%d").to_string()
            };

            let opened_query = format!(
                "repo:{}/{} type:issue created:{}..{}",
                owner, repo, start_date, end_date
            );
            match self
                .client
                .search()
                .issues_and_pull_requests(&opened_query)
                .per_page(1)
                .send()
                .await
            {
                Ok(page) => {
                    weekly[i].opened = page.total_count.unwrap_or(0) as u64;
                }
                Err(e) => {
                    log::warn!("Search failed for opened issues {}: {}", start_date, e);
                    any_error = true;
                }
            }

            let closed_query = format!(
                "repo:{}/{} type:issue closed:{}..{}",
                owner, repo, start_date, end_date
            );
            match self
                .client
                .search()
                .issues_and_pull_requests(&closed_query)
                .per_page(1)
                .send()
                .await
            {
                Ok(page) => {
                    weekly[i].closed = page.total_count.unwrap_or(0) as u64;
                }
                Err(e) => {
                    log::warn!("Search failed for closed issues {}: {}", start_date, e);
                    any_error = true;
                }
            }
        }

        if any_error {
            log::warn!("Some search queries failed for {}/{} issue velocity — partial data", owner, repo);
        }

        let total_opened: u64 = weekly.iter().map(|w| w.opened).sum();
        let total_closed: u64 = weekly.iter().map(|w| w.closed).sum();

        log::info!(
            "Issue velocity for {}/{}: search-based, total opened={}, closed={}",
            owner, repo, total_opened, total_closed
        );

        Ok(weekly)
    }

    async fn fetch_pr_velocity(
        &self,
        owner: &str,
        repo: &str,
        week_starts: &[DateTime<Utc>],
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<WeeklyActivity>> {
        let mut all_open_prs = Vec::new();
        let mut all_closed_prs = Vec::new();
        let mut page: u32 = 1;

        // Paginate through open PRs
        loop {
            let prs_page = self
                .client
                .pulls(owner, repo)
                .list()
                .state(octocrab::params::State::Open)
                .per_page(VELOCITY_PER_PAGE)
                .page(page)
                .send()
                .await
                .with_context(|| {
                    format!("Failed to fetch open PRs page {} for velocity: {}/{}", page, owner, repo)
                })?;

            let prs: Vec<_> = prs_page.items;
            let fetched = prs.len();
            all_open_prs.extend(prs);

            if all_open_prs.len() >= VELOCITY_MAX_OPEN_PRS || fetched < VELOCITY_PER_PAGE as usize {
                break;
            }
            page += 1;
        }

        // Reset page for closed PRs
        page = 1;

        // Paginate through closed PRs
        loop {
            let prs_page = self
                .client
                .pulls(owner, repo)
                .list()
                .state(octocrab::params::State::Closed)
                .per_page(VELOCITY_PER_PAGE)
                .page(page)
                .send()
                .await
                .with_context(|| {
                    format!("Failed to fetch closed PRs page {} for velocity: {}/{}", page, owner, repo)
                })?;

            let prs: Vec<_> = prs_page.items;
            let fetched = prs.len();
            all_closed_prs.extend(prs);

            if all_closed_prs.len() >= VELOCITY_MAX_CLOSED_PRS || fetched < VELOCITY_PER_PAGE as usize {
                break;
            }
            page += 1;
        }

        let mut weekly = init_weekly(week_starts);

        // Count opened PRs from both open and closed sets
        for pr in all_open_prs.iter().chain(all_closed_prs.iter()) {
            if let Some(created_at) = pr.created_at {
                if created_at >= cutoff {
                    if let Some(idx) = find_week_index(week_starts, created_at) {
                        weekly[idx].opened += 1;
                    }
                }
            }
        }

        // Count merged PRs (merged_at, not just closed_at)
        for pr in &all_closed_prs {
            if let Some(merged_at) = pr.merged_at {
                if merged_at >= cutoff {
                    if let Some(idx) = find_week_index(week_starts, merged_at) {
                        weekly[idx].closed += 1;
                    }
                }
            }
        }

        Ok(weekly)
    }
}

fn init_weekly(week_starts: &[DateTime<Utc>]) -> Vec<WeeklyActivity> {
    week_starts
        .iter()
        .map(|&ws| WeeklyActivity {
            week_start: ws,
            opened: 0,
            closed: 0,
        })
        .collect()
}

/// Computes the start of each week (Monday 00:00:00 UTC) for the last `n` weeks,
/// returned in chronological order (oldest first).
fn compute_week_starts(n: usize) -> Vec<DateTime<Utc>> {
    let now = Utc::now();
    let days_since_monday = now.weekday().num_days_from_monday() as i64;
    let current_monday = (now - Duration::days(days_since_monday))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let current_monday = Utc.from_utc_datetime(&current_monday);

    (0..n)
        .map(|i| current_monday - Duration::weeks(i as i64))
        .rev()
        .collect()
}

/// Finds which week bucket a timestamp falls into.
/// Week i covers `[week_starts[i], week_starts[i+1])` for i < len-1,
/// and `[week_starts[last], ∞)` for the last bucket.
fn find_week_index(week_starts: &[DateTime<Utc>], dt: DateTime<Utc>) -> Option<usize> {
    for (i, ws) in week_starts.iter().enumerate().rev() {
        if dt >= *ws {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_compute_week_starts_returns_eight_weeks() {
        let weeks = compute_week_starts(8);
        assert_eq!(weeks.len(), 8);
    }

    #[test]
    fn test_compute_week_starts_all_mondays_at_midnight() {
        let weeks = compute_week_starts(8);
        for ws in &weeks {
            assert_eq!(ws.weekday(), chrono::Weekday::Mon);
            assert_eq!(ws.hour(), 0);
            assert_eq!(ws.minute(), 0);
            assert_eq!(ws.second(), 0);
        }
    }

    #[test]
    fn test_compute_week_starts_chronological_order() {
        let weeks = compute_week_starts(8);
        for pair in weeks.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }

    #[test]
    fn test_compute_week_starts_seven_day_intervals() {
        let weeks = compute_week_starts(8);
        for pair in weeks.windows(2) {
            assert_eq!(pair[1] - pair[0], Duration::weeks(1));
        }
    }

    #[test]
    fn test_eight_weeks_span() {
        let weeks = compute_week_starts(8);
        let span = *weeks.last().unwrap() - *weeks.first().unwrap();
        assert_eq!(span, Duration::weeks(7));
    }

    #[test]
    fn test_find_week_index_last_week() {
        let weeks = compute_week_starts(8);
        let recent = weeks[7] + Duration::hours(1);
        assert_eq!(find_week_index(&weeks, recent), Some(7));
    }

    #[test]
    fn test_find_week_index_first_week() {
        let weeks = compute_week_starts(8);
        let in_first = weeks[0] + Duration::hours(12);
        assert_eq!(find_week_index(&weeks, in_first), Some(0));
    }

    #[test]
    fn test_find_week_index_before_range_returns_none() {
        let weeks = compute_week_starts(8);
        let before = weeks[0] - Duration::hours(1);
        assert_eq!(find_week_index(&weeks, before), None);
    }

    #[test]
    fn test_find_week_index_exact_boundary() {
        let weeks = compute_week_starts(8);
        assert_eq!(find_week_index(&weeks, weeks[3]), Some(3));
    }

    #[test]
    fn test_find_week_index_mid_week() {
        let weeks = compute_week_starts(8);
        let wednesday = weeks[4] + Duration::days(2);
        assert_eq!(find_week_index(&weeks, wednesday), Some(4));
    }

    #[test]
    fn test_init_weekly_zeroed() {
        let weeks = compute_week_starts(8);
        let weekly = init_weekly(&weeks);
        assert_eq!(weekly.len(), 8);
        for (i, entry) in weekly.iter().enumerate() {
            assert_eq!(entry.week_start, weeks[i]);
            assert_eq!(entry.opened, 0);
            assert_eq!(entry.closed, 0);
        }
    }
}
