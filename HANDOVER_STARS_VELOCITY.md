# Handover: Stars Sparkline & Velocity Data Issues

## 🎯 Problem Summary

### Stars Panel - Empty Sparklines
- **Symptom:** Stars panel shows "30d: 0 | 90d: 0 | 1y: 0" in zoom view
- **Total stars displayed:** 182.1k (correct)
- **Sparkline data:** All zeros (incorrect)
- **Affected repos:** Large/high-velocity repos like microsoft/vscode (182k stars)

### Velocity Panel - PR Data Issues
- **Symptom:** Issues velocity shows data, but PR opened/merged counts may still show 0/0
- **Last screenshot:** Showed PRs with +509/-427 (working), but need to verify consistency

---

## 🔍 Root Cause Analysis

### Stargazer Fetching Issue
The core problem is in `fetch_stargazer_timestamps()` in `src/core/github_client.rs`:

1. **API Pagination Logic:**
   - For a repo with 182k stars, `last_page = 1822` (182k/100 per page)
   - `start_page = last_page - 99 = 1723` (with 100 page limit)
   - This fetches pages 1723-1822 (most recent 10,000 stargazers)

2. **The Bug:** Even with 10k stargazers, the sparklines show zeros
   - Possible causes:
     - API authentication issue with `Accept: application/vnd.github.star+json` header
     - Date cutoff filtering too aggressively
     - Events not being parsed correctly
     - All fetched stargazers are older than 365 days (unlikely for active repo)

### Velocity Issue
In `src/core/metrics/velocity.rs`:
- Pagination was added but limits may still be too low for mega-repos
- PRs are fetched from two separate endpoints (open + closed)
- May need to increase `VELOCITY_MAX_CLOSED_PRS` further

---

## 🛠️ Changes Made (Current Session)

### Files Modified:

1. **src/core/github_client.rs**
   - Increased `STARGAZERS_MAX_PAGES` from 10 → 100 (1k → 10k stargazers)
   - Added extensive logging at INFO level
   - Added per-page debug logging

2. **src/core/metrics/velocity.rs**
   - Added pagination for issues (up to 1000)
   - Added pagination for PRs (250 open + 750 closed)
   - Added logging for fetched counts

3. **src/core/metrics/issues.rs**
   - Increased limit from 100 → 250 with pagination
   - Added `truncated` field to IssueStats

4. **src/core/models.rs**
   - Added `truncated: bool` to IssueStats
   - Added `Serialize` to MilestonePrediction

5. **src/tui/app/panels.rs**
   - Fixed releases title format
   - Added truncation indicator (+) for issues

6. **src/tui/app/zoom.rs**
   - Added sort indicator in issues zoom title
   - Added truncation warning

---

## 🔬 Debugging Steps for Next Session

### Step 1: Check Logs
Run with logging enabled:
```bash
RUST_LOG=debug cargo run -- microsoft/vscode 2>&1 | tee debug.log
```

Look for these log messages:
- "Fetching stargazers for microsoft/vscode..."
- "Stargazer pagination: last_page=X, start_page=Y..."
- "Stargazer page X: fetched Y events"
- "Stargazer fetch complete: fetched X timestamps from Y pages"
- "StarHistory for microsoft/vscode: timestamps=X, 30d_sum=Y, 90d_sum=Z..."

### Step 2: Verify API Response
Add a test to print the actual API response:
```rust
// In fetch_stargazer_timestamps, after getting events:
log::info!("First event: {:?}", events.first());
log::info!("Last event: {:?}", events.last());
```

### Step 3: Check Date Range
Verify the cutoff date logic:
```rust
let cutoff = Utc::now() - Duration::days(365);
log::info!("Cutoff date: {} (365 days ago)", cutoff);
```

### Step 4: Test Sparkline Generation
Add a test with dummy data to verify sparkline generation works:
```rust
let test_timestamps: Vec<DateTime<Utc>> = (0..30)
    .map(|i| Utc::now() - Duration::days(i))
    .collect();
let sparkline = generate_sparkline(&test_timestamps, Utc::now() - Duration::days(30), 30);
log::info!("Test sparkline: {:?}", sparkline);
```

---

## 🧪 Hypotheses to Test

### Hypothesis 1: Authentication Required
The `Accept: application/vnd.github.star+json` header might require authentication even for public repos.

**Test:** Check if `events` vector is empty after parsing.
```rust
if events.is_empty() {
    log::warn!("No stargazer events returned - API might require auth");
}
```

### Hypothesis 2: API Rate Limiting
Fetching 100 pages might hit rate limits.

**Test:** Check response headers in logs for `x-ratelimit-remaining`.

### Hypothesis 3: All Stargazers Are Recent
All 10k fetched stargazers might be within the last few days, and the cutoff logic filters them.

**Test:** Print min/max dates of fetched timestamps:
```rust
if let Some(first) = all_timestamps.first() {
    if let Some(last) = all_timestamps.last() {
        log::info!("Timestamp range: {} to {} ({} days)", 
            first, last, (last - first).num_days());
    }
}
```

### Hypothesis 4: Date Comparison Bug
The `event.starred_at < cutoff` comparison might be backwards.

**Current code:**
```rust
if event.starred_at < cutoff {
    reached_cutoff = true;  // Stop when we hit old stargazers
} else {
    all_timestamps.push(event.starred_at); // Keep recent ones
}
```

This logic is correct: we want to stop when we reach stargazers older than 365 days.

But wait - for a high-velocity repo, 10k stargazers might ALL be within the last 30 days!
In that case, we never hit the cutoff, fetch all 100 pages, and should have data.

### Hypothesis 5: Sparkline Generation Bug
The `generate_sparkline` function might have a bug with date range.

**Test:** Print the actual sparkline vectors:
```rust
log::info!("sparkline_30d: {:?}", sparkline_30d);
log::info!("sparkline_90d: {:?}", sparkline_90d);
log::info!("sparkline_365d: {:?}", sparkline_365d);
```

---

## 📁 Key Files to Focus On

1. **src/core/github_client.rs**
   - `stargazers()` method (line ~101)
   - `fetch_stargazer_timestamps()` method (line ~161)

2. **src/core/metrics/stars.rs**
   - `generate_sparkline()` function (line ~80)
   - `predict_milestone()` function

3. **src/core/metrics/velocity.rs**
   - `fetch_issue_velocity()` method (line ~45)
   - `fetch_pr_velocity()` method (line ~111)

4. **src/tui/app/panels.rs**
   - `render_stars()` method (line ~32)
   - `render_velocity()` method

---

## 💡 Potential Solutions

### Solution 1: Use Alternative Data Source
If stargazer API is problematic, use commit activity or release dates as proxy for repo velocity.

### Solution 2: Fetch From Beginning For Young Repos
For repos < 1 year old, fetch from page 1 instead of latest pages to get full history.

### Solution 3: Use GitHub GraphQL API
The REST API might have limitations. GraphQL could be more efficient for this use case.

### Solution 4: Cache & Incremental Updates
Store stargazer data locally and only fetch new stars since last update.

---

## ✅ Acceptance Criteria

1. Stars panel shows non-zero sparkline for microsoft/vscode
2. Zoom view shows meaningful star trend data (30d/90d/1y)
3. Velocity panel shows consistent PR activity data
4. All changes compile without warnings
5. No regression for smaller repos

---

## 📊 Test Cases

1. **Large repo:** `microsoft/vscode` (182k stars)
2. **Medium repo:** `ratatui-org/ratatui` (~10k stars)
3. **Small repo:** `torvalds/linux` (just kidding - use a real small repo)
4. **New repo:** Any repo < 30 days old
5. **Old repo:** Any repo > 5 years old

---

## 🔗 Related Issues from Previous Session

From the bug list:
- Issue #7: "if we load projects longer than a year now, we dont have any graphs, just empty" - RELATED
- Issue #11: "on larger repos, we have a problem with the / search, not showing any results" - Fixed separately

---

## 📝 Notes for Next Developer

1. The logging is already in place - use `RUST_LOG=info` or `RUST_LOG=debug`
2. The build compiles successfully with current changes
3. Focus on the stargazer timestamp collection first - that's the root cause
4. Don't forget to test with smaller repos to ensure no regression
5. Consider adding a "Test API" command to verify connectivity

---

Last Updated: 2024 (Current Session)
Status: IN PROGRESS - Needs debugging with logs
