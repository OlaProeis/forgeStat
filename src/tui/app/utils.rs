use chrono::{DateTime, Utc};
use ratatui::prelude::*;

pub(crate) fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub(crate) fn format_age(dt: DateTime<Utc>) -> String {
    let age = Utc::now().signed_duration_since(dt);
    let days = age.num_days();
    if days > 365 {
        format!("{}y", days / 365)
    } else if days > 30 {
        format!("{}mo", days / 30)
    } else if days > 0 {
        format!("{}d", days)
    } else {
        let hours = age.num_hours();
        if hours > 0 {
            format!("{}h", hours)
        } else {
            "now".to_string()
        }
    }
}

pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_len {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        s.to_string()
    }
}

/// Trim leading zeros from data (period before repo existed/had stars)
pub(crate) fn trim_leading_zeros(data: &[u64]) -> Vec<u64> {
    let first_non_zero = data.iter().position(|&v| v > 0);
    match first_non_zero {
        Some(idx) if idx > 0 => data[idx..].to_vec(),
        _ => data.to_vec(),
    }
}

/// Resample sparkline data to fill the width with more bars.
/// Uses linear interpolation to stretch/shrink data to target buckets.
pub(crate) fn resample_to_width(data: &[u64], target_buckets: usize) -> Vec<u64> {
    if data.is_empty() || target_buckets == 0 {
        return Vec::new();
    }

    if data.len() >= target_buckets {
        let bucket_width = data.len() as f64 / target_buckets as f64;
        let mut result = Vec::with_capacity(target_buckets);

        for i in 0..target_buckets {
            let start_idx = (i as f64 * bucket_width) as usize;
            let end_idx = ((i + 1) as f64 * bucket_width) as usize;
            let end_idx = end_idx.min(data.len());

            if start_idx < end_idx {
                let avg = data[start_idx..end_idx].iter().sum::<u64>() / (end_idx - start_idx) as u64;
                result.push(avg);
            } else {
                result.push(data[start_idx.min(data.len() - 1)]);
            }
        }
        result
    } else {
        let mut result = Vec::with_capacity(target_buckets);
        let scale = (data.len() - 1) as f64 / (target_buckets - 1).max(1) as f64;

        for i in 0..target_buckets {
            let exact_idx = i as f64 * scale;
            let idx_low = exact_idx.floor() as usize;
            let idx_high = exact_idx.ceil() as usize;
            let frac = exact_idx.fract();

            let low_val = data[idx_low.min(data.len() - 1)];
            let high_val = data[idx_high.min(data.len() - 1)];

            let interpolated = low_val as f64 + frac * (high_val as f64 - low_val as f64);
            result.push(interpolated.round() as u64);
        }
        result
    }
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, center_v, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);

    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(center_v);

    center
}
