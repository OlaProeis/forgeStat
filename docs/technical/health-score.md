# Health Score

## Overview

The Health Score module provides a comprehensive repository health assessment from 0-100, composed of four sub-scores. It helps maintainers and contributors quickly understand the overall health of an open source project at a glance.

## Key Files

- `src/core/health.rs` - Core health score computation logic
- `src/cli/summary.rs` - CLI summary display with health score

## Implementation Details

### Data Model

```rust
pub struct HealthScore {
    pub total: u8,        // 0-100
    pub activity: u8,     // 0-25
    pub community: u8,    // 0-25
    pub maintenance: u8,  // 0-25
    pub growth: u8,       // 0-25
    pub grade: HealthGrade,
}

pub enum HealthGrade {
    Excellent,      // 90-100 (A)
    Good,          // 75-89 (B)
    Fair,          // 50-74 (C)
    NeedsAttention, // 25-49 (D)
    Critical,      // 0-24 (F)
}
```

### Scoring Algorithm

#### Activity (0-25 points)

| Factor | Points | Criteria |
|--------|--------|----------|
| Velocity | 0-10 | Weekly issues+PRs activity (0=0pts, 31+=10pts) |
| Merge Rate | 0-8 | PRs merged last 30d (0=0pts, 11+=8pts) |
| Merge Speed | 0-4 | Avg time to merge (24h=4pts, >2wks=0pts) |
| Recency | 0-3 | Activity in last week |
| CI Success Rate | 0-5 | GitHub Actions success % (100%=5pts, 0%=0pts) |

#### Community (0-25 points)

| Factor | Points | Criteria |
|--------|--------|----------|
| Contributors | 0-10 | Total unique (0=0pts, 50+=10pts) |
| Concentration | 0-5 | Top contributor % (<40%=5pts, >80%=1pt) |
| New Contributors | 0-5 | Last 30d (0=0pts, 6+=5pts) |
| Engagement | 0-5 | Comments per issue + oldest issue age |

#### Maintenance (0-25 points, starts at 25, deducts/bonuses)

| Factor | Deduction/Bonus | Criteria |
|--------|-----------------|----------|
| Release Age | 0-10 | Days since release (<30d=0, >6mo=10) |
| Security Alerts | 0-8 | Critical=8pts, High=4-8pts, Medium=2-4pts |
| Resolution Rate | 0-7 | Opened vs closed ratio (1.2x=0pts, 0.5x=7pts) |
| Community Health | 0-5 bonus | Health score 0-100 (100=+5pts, 75=+4pts, etc.) |

#### Growth (0-25 points)

| Factor | Points | Criteria |
|--------|--------|----------|
| Star Velocity | 0-15 | Daily avg 30d (0=0pts, 21+=15pts) |
| Acceleration | 0-2 | 30d avg > 90d avg |
| Milestone | 0-5 | Total stars (0=0pts, 1000+=5pts) |
| Engagement | 0-3 | Forks + watchers (10+ each = 3pts) |

### Grade Boundaries

| Grade | Score Range | Color |
|-------|-------------|-------|
| A (Excellent) | 90-100 | Green |
| B (Good) | 75-89 | Cyan |
| C (Fair) | 50-74 | Yellow |
| D (Needs Attention) | 25-49 | Orange |
| F (Critical) | 0-24 | Red |

## Usage

### Compute Health Score

```rust
use forgeStat::core::health::compute_health_score;

let score = compute_health_score(&snapshot);
println!("Health: {}/100 (Grade: {})", score.total, score.grade.as_letter());
```

### CLI Summary View

```bash
forgeStat --summary owner/repo
```

Output includes:
- Total score with color-coded grade
- All four sub-scores
- Seven core metrics

## Testing

Comprehensive unit tests covering:
- Edge cases (zero activity, max values)
- Grade boundary conditions (89→90, 74→75, etc.)
- Formula verification for each sub-score
- Serialization roundtrip tests

Run tests:
```bash
cargo test core::health
```
