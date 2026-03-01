# Loading Screen

The forgeStat loading screen provides visual feedback during repository data fetching, with special features for large repositories that take longer to load.

## Overview

When starting forgeStat or refreshing data (`r` key), the application displays a loading screen showing:
- Real-time progress of data fetching
- Current endpoint being loaded
- Estimated time for large repositories
- An optional Pong mini-game to pass the time

## Progress Tracking

### Endpoint Progress
For most endpoints, progress is tracked at the endpoint level:
```
3/10 endpoints (30%)
```

### Page-by-Page Progress (Star History)
Star history fetching involves paginating through GitHub's API (up to 100 pages for large repos). During this phase, the progress bar shows:
```
page 12/100 (12%)
```

This provides much better feedback than staying at a fixed percentage while pages are being fetched internally.

## Large Repository Detection

Repositories with >5,000 stars are considered "large" and trigger additional UI features:

1. **Warning Message**: A yellow warning is displayed:
   > ⚠ This repo has 182.2k stars - loading may take a while!

2. **Pong Game**: The mini-game automatically appears for large repos

3. **Extended Loading Time**: Large repos can take 1-2 minutes to load due to:
   - GitHub API pagination (100 stargazers per page)
   - Rate limiting considerations
   - Processing thousands of star timestamps

## Pong Mini-Game

### Controls
- `↑` / `↓` - Move left paddle up/down
- `Space` or `Enter` - Toggle game on/off

### Gameplay
- Ball bounces off paddles and walls
- Left paddle is player-controlled (green)
- Right paddle is AI-controlled (red, moves slightly slower for fairness)
- Ball speed starts slow and increases slightly with each paddle hit
- Score tracking shows at the top of the game area

### Technical Details
- Game area: 60×12 characters (below loading info)
- Ball position updated at 60 FPS
- Paddle collision uses precise hit detection (0.5 char width)
- Spin effect: Ball angle changes based on where it hits the paddle

## Implementation Details

### Files
- `src/tui/app/loading_screen.rs` - Main loading screen implementation
- `src/core/snapshot.rs` - Progress reporting during data fetch
- `src/core/github_client.rs` - Page progress callbacks for stargazers

### Progress Data Structure
```rust
pub struct FetchProgress {
    pub total: usize,                    // Total endpoints (10)
    pub completed: usize,                // Completed endpoints
    pub current_endpoint: Option<String>, // "Star History", "Issues", etc.
    pub done: bool,
    pub error: Option<String>,
    pub star_count: Option<u64>,         // For large repo detection
    pub current_page: Option<u32>,       // Current page during pagination
    pub total_pages: Option<u32>,        // Estimated total pages
}
```

### Progress Reporting Flow

1. **Star Count Detection** (first API call)
   - Quickly fetches just the star count to detect large repos
   - Determines if Pong game should be shown

2. **Endpoint Fetching** (parallel)
   - Repository metadata
   - Star history (with page-by-page progress)
   - Issues, PRs, Contributors, Releases, Velocity, Security, CI, Community

3. **Star History Pagination**
   - Uses GraphQL for repos with >40k stars (faster, reverse chronological)
   - Uses REST for smaller repos
   - Progress callback reports `(current_page, total_pages)` after each page

### Visual Design

The loading screen features:
- Centered cyan border box with animated spinner
- Twinkling starfield background (subtle effect)
- Progress bar with percentage label
- Status text showing current operation
- Warning banner for large repos
- Game hint text (when applicable)

## Performance Considerations

### Loading Time Breakdown

Typical load times for reference:

| Repository | Stars | Cold Load | Cached Load |
|------------|-------|-----------|-------------|
| Small repo | <1k   | 5-10s     | Instant     |
| Medium repo| 1-10k | 10-20s    | Instant     |
| Large repo | 10-50k| 30-60s    | Instant     |
| Huge repo  | >100k | 1-2min    | Instant     |

### Cache Strategy
- 15-minute TTL by default
- Subsequent loads are instant (from local cache)
- Cache stored at `~/.local/share/repowatch/<owner>/<repo>/`

## Future Enhancements

Possible improvements to consider:
- Background music during loading
- Additional mini-games (Snake, Tetris)
- Estimated time remaining calculation
- Cancel/retry buttons for failed endpoints
- Parallel loading visualization
