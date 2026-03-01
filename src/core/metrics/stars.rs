use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::models::StarHistory;

/// A single stargazer event returned by the GitHub Stargazers API
/// when using `Accept: application/vnd.github.star+json`.
#[derive(Debug, Deserialize)]
pub struct StargazerEvent {
    pub starred_at: DateTime<Utc>,
}

/// Milestone prediction for star growth
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MilestonePrediction {
    /// The next milestone to reach (e.g., 100, 500, 1000, etc.)
    pub next_milestone: u64,
    /// Estimated days to reach the milestone
    pub estimated_days: u64,
    /// Weighted average daily star growth rate
    pub daily_rate: f64,
}

/// Standard star milestones in ascending order
const MILESTONES: &[u64] = &[100, 500, 1000, 5000, 10000, 25000, 50000, 100000];

/// Predict the next star milestone based on 30d/90d growth trends.
///
/// Calculates a weighted average daily rate (30d: 70%, 90d: 30%) and estimates
/// days to reach the next milestone from the predefined list: [100, 500, 1k, 5k, 10k, 25k, 50k, 100k].
///
/// Returns `None` if:
/// - Daily rate is <= 0 (growth stalled or negative)
/// - Current stars already exceed all milestones
/// - No star history data is available
pub fn predict_milestone(star_history: &StarHistory) -> Option<MilestonePrediction> {
    // Calculate daily rates from sparkline data
    let rate_30d = calculate_daily_rate(&star_history.sparkline_30d, 30);
    let rate_90d = calculate_daily_rate(&star_history.sparkline_90d, 90);

    // Weighted average: 70% from 30d, 30% from 90d
    let daily_rate = rate_30d * 0.7 + rate_90d * 0.3;

    // Growth stalled if rate <= 0
    if daily_rate <= 0.0 {
        return None;
    }

    // Find the next milestone greater than current star count
    let current_stars = star_history.total_count;
    let next_milestone = MILESTONES.iter().find(|&&m| m > current_stars).copied()?;

    // Calculate estimated days to reach milestone
    let stars_needed = next_milestone - current_stars;
    let estimated_days = (stars_needed as f64 / daily_rate).ceil() as u64;

    Some(MilestonePrediction {
        next_milestone,
        estimated_days,
        daily_rate,
    })
}

/// Calculate daily growth rate from sparkline data
fn calculate_daily_rate(sparkline: &[u32], period_days: u64) -> f64 {
    if sparkline.is_empty() || period_days == 0 {
        return 0.0;
    }

    let total_stars: u64 = sparkline.iter().map(|&v| v as u64).sum();
    total_stars as f64 / period_days as f64
}

/// Bins star timestamps into equal-width time buckets for sparkline rendering.
///
/// Bucket periods:
/// - 30d with 30 buckets = 1 day each
/// - 90d with 13 buckets ≈ 1 week each
/// - 365d with 12 buckets ≈ 1 month each
pub fn generate_sparkline(
    timestamps: &[DateTime<Utc>],
    period_start: DateTime<Utc>,
    bucket_count: usize,
) -> Vec<u32> {
    if bucket_count == 0 {
        return Vec::new();
    }

    let now = Utc::now();
    let total_seconds = (now - period_start).num_seconds();
    if total_seconds <= 0 {
        return vec![0; bucket_count];
    }

    let bucket_width = total_seconds as f64 / bucket_count as f64;
    let mut buckets = vec![0u32; bucket_count];

    for ts in timestamps {
        if *ts >= period_start && *ts <= now {
            let elapsed = (*ts - period_start).num_seconds() as f64;
            let idx = (elapsed / bucket_width) as usize;
            buckets[idx.min(bucket_count - 1)] += 1;
        }
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_empty_timestamps() {
        let now = Utc::now();
        let result = generate_sparkline(&[], now - Duration::days(30), 30);
        assert_eq!(result.len(), 30);
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_zero_buckets() {
        let now = Utc::now();
        let result = generate_sparkline(&[now], now - Duration::days(30), 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_star_lands_in_correct_bucket() {
        let now = Utc::now();
        let star_time = now - Duration::days(15);
        let result = generate_sparkline(&[star_time], now - Duration::days(30), 30);
        assert_eq!(result.len(), 30);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, 1);
        assert!(
            result[14] == 1 || result[15] == 1,
            "star should land near bucket 14-15"
        );
    }

    #[test]
    fn test_total_count_preserved() {
        let now = Utc::now();
        let start = now - Duration::days(30);
        let timestamps: Vec<DateTime<Utc>> =
            (0..100).map(|i| start + Duration::hours(i * 7)).collect();

        let result = generate_sparkline(&timestamps, start, 30);
        assert_eq!(result.len(), 30);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, timestamps.len() as u32);
    }

    #[test]
    fn test_out_of_range_timestamps_ignored() {
        let now = Utc::now();
        let start = now - Duration::days(30);
        let timestamps = vec![
            start - Duration::days(5),
            start + Duration::days(10),
            now + Duration::days(5),
        ];

        let result = generate_sparkline(&timestamps, start, 30);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, 1);
    }

    #[test]
    fn test_90d_weekly_buckets() {
        let now = Utc::now();
        let start = now - Duration::days(90);
        let timestamps: Vec<DateTime<Utc>> = (0..90).map(|i| start + Duration::days(i)).collect();

        let result = generate_sparkline(&timestamps, start, 13);
        assert_eq!(result.len(), 13);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, 90);
    }

    #[test]
    fn test_365d_monthly_buckets() {
        let now = Utc::now();
        let start = now - Duration::days(365);
        let timestamps: Vec<DateTime<Utc>> = (0..365).map(|i| start + Duration::days(i)).collect();

        let result = generate_sparkline(&timestamps, start, 12);
        assert_eq!(result.len(), 12);
        let sum: u32 = result.iter().sum();
        assert_eq!(sum, 365);
    }

    // =========================================================================
    // Milestone prediction tests
    // =========================================================================

    fn create_star_history(
        total: u64,
        sparkline_30d: Vec<u32>,
        sparkline_90d: Vec<u32>,
    ) -> StarHistory {
        StarHistory {
            total_count: total,
            sparkline_30d,
            sparkline_90d,
            sparkline_365d: vec![],
        }
    }

    #[test]
    fn test_predict_milestone_normal_growth() {
        // 50 stars currently, growing at ~5 stars/day (150 in 30d)
        let history = create_star_history(50, vec![5; 30], vec![5; 90]);
        let prediction = predict_milestone(&history).unwrap();

        assert_eq!(prediction.next_milestone, 100);
        // 50 stars needed / 5 per day = 10 days
        assert_eq!(prediction.estimated_days, 10);
        assert!(prediction.daily_rate > 0.0);
    }

    #[test]
    fn test_predict_milestone_stalled_growth() {
        // No growth in sparklines
        let history = create_star_history(50, vec![0; 30], vec![0; 90]);
        let prediction = predict_milestone(&history);

        assert!(
            prediction.is_none(),
            "Should return None for stalled growth"
        );
    }

    #[test]
    fn test_predict_milestone_negative_growth() {
        // Negative growth should also return None
        let history = create_star_history(50, vec![0; 30], vec![0; 90]);
        let prediction = predict_milestone(&history);

        assert!(
            prediction.is_none(),
            "Should return None for negative growth"
        );
    }

    #[test]
    fn test_predict_milestone_already_past_all_milestones() {
        // 150k stars, past all defined milestones
        let history = create_star_history(150_000, vec![100; 30], vec![100; 90]);
        let prediction = predict_milestone(&history);

        assert!(
            prediction.is_none(),
            "Should return None when past all milestones"
        );
    }

    #[test]
    fn test_predict_milestone_near_milestone() {
        // 95 stars, nearly at 100 milestone
        let history = create_star_history(95, vec![1; 30], vec![1; 90]);
        let prediction = predict_milestone(&history).unwrap();

        assert_eq!(prediction.next_milestone, 100);
        // 5 stars needed / 1 per day = 5 days
        assert_eq!(prediction.estimated_days, 5);
    }

    #[test]
    fn test_predict_milestone_no_history() {
        // Empty sparklines
        let history = create_star_history(50, vec![], vec![]);
        let prediction = predict_milestone(&history);

        assert!(
            prediction.is_none(),
            "Should return None with no history data"
        );
    }

    #[test]
    fn test_predict_milestone_weighted_rate() {
        // Different rates: 30d = 10 stars/day, 90d = 2 stars/day
        // Weighted: 10 * 0.7 + 2 * 0.3 = 7 + 0.6 = 7.6 stars/day
        let history = create_star_history(100, vec![10; 30], vec![2; 90]);
        let prediction = predict_milestone(&history).unwrap();

        assert_eq!(prediction.next_milestone, 500);
        // 400 stars needed / 7.6 per day ≈ 53 days
        assert_eq!(prediction.estimated_days, 53);
        assert!((prediction.daily_rate - 7.6).abs() < 0.01);
    }

    #[test]
    fn test_predict_milestone_different_milestones() {
        // At 600 stars, next milestone should be 1000
        let history = create_star_history(600, vec![5; 30], vec![5; 90]);
        let prediction = predict_milestone(&history).unwrap();

        assert_eq!(prediction.next_milestone, 1000);
        // 400 stars needed / 5 per day = 80 days
        assert_eq!(prediction.estimated_days, 80);
    }

    #[test]
    fn test_predict_milestone_exactly_at_milestone() {
        // Exactly at 1000 stars, next should be 5000
        let history = create_star_history(1000, vec![10; 30], vec![10; 90]);
        let prediction = predict_milestone(&history).unwrap();

        assert_eq!(prediction.next_milestone, 5000);
    }

    #[test]
    fn test_calculate_daily_rate_empty() {
        let rate = calculate_daily_rate(&[], 30);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_calculate_daily_rate_zero_period() {
        let rate = calculate_daily_rate(&[1, 2, 3], 0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_calculate_daily_rate_normal() {
        // 60 stars over 30 days = 2 per day
        let rate = calculate_daily_rate(&[2; 30], 30);
        assert!((rate - 2.0).abs() < 0.01);
    }
}
