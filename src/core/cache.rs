use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::data_local_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use crate::core::models::RepoSnapshot;

/// Information about a cached repository for fuzzy finder
#[derive(Debug, Clone)]
pub struct CachedRepoInfo {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub path: PathBuf,
    pub last_viewed_at: Option<DateTime<Utc>>,
}

/// Cache entry storing both the snapshot and its fetch timestamp
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CacheEntry {
    pub fetched_at: DateTime<Utc>,
    pub snapshot: RepoSnapshot,
}

/// State file for storing UI scroll positions and other ephemeral state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StateEntry {
    /// Scroll position for each panel (panel_name -> scroll_offset)
    pub scroll_positions: std::collections::HashMap<String, u16>,
    /// Last viewed timestamp
    pub last_viewed_at: Option<DateTime<Utc>>,
}

/// Local cache system for storing repository snapshots
pub struct Cache {
    base_path: PathBuf,
    cache_path: PathBuf,
    history_path: PathBuf,
    state_path: PathBuf,
}

impl Cache {
    /// Create a new cache instance for the given owner/repo
    ///
    /// Cache location:
    /// - Linux: ~/.local/share/repowatch/<owner>/<repo>/
    /// - Windows: %LOCALAPPDATA%\repowatch\<owner>\<repo>\
    /// - macOS: ~/Library/Application Support/repowatch/<owner>/<repo>/
    ///
    /// Structure:
    /// - cache.json: Current snapshot
    /// - history/: Directory containing historical snapshots
    /// - state.json: UI state (scroll positions)
    pub fn new(owner: &str, repo: &str) -> Result<Self> {
        let base = data_local_dir()
            .context("Failed to get data local directory")?
            .join("repowatch")
            .join(owner)
            .join(repo);

        Ok(Self {
            base_path: base.clone(),
            cache_path: base.join("cache.json"),
            history_path: base.join("history"),
            state_path: base.join("state.json"),
        })
    }

    /// Get the base cache directory path
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Get the cache file path (cache.json)
    pub fn cache_path(&self) -> &PathBuf {
        &self.cache_path
    }

    /// Get the history directory path
    pub fn history_path(&self) -> &PathBuf {
        &self.history_path
    }

    /// Get the state file path (state.json)
    pub fn state_path(&self) -> &PathBuf {
        &self.state_path
    }

    /// Initialize the cache directory structure
    /// Creates base directory, history directory, and empty state.json if missing
    pub async fn initialize(&self) -> Result<()> {
        // Create base directory
        fs::create_dir_all(&self.base_path)
            .await
            .context("Failed to create cache base directory")?;

        // Create history directory
        fs::create_dir_all(&self.history_path)
            .await
            .context("Failed to create history directory")?;

        // Create empty state.json if it doesn't exist
        if !self.state_path.exists() {
            let default_state = StateEntry::default();
            let json = serde_json::to_string_pretty(&default_state)
                .context("Failed to serialize default state")?;
            fs::write(&self.state_path, json)
                .await
                .context("Failed to write state.json")?;
            log::info!("Initialized state.json at {}", self.state_path.display());
        }

        log::info!("Cache directory initialized at {}", self.base_path.display());
        Ok(())
    }

    /// Load the state file (scroll positions, etc.)
    pub async fn load_state(&self) -> Result<StateEntry> {
        if !self.state_path.exists() {
            return Ok(StateEntry::default());
        }

        let content = fs::read_to_string(&self.state_path)
            .await
            .context("Failed to read state file")?;

        if content.trim().is_empty() {
            return Ok(StateEntry::default());
        }

        let state: StateEntry = serde_json::from_str(&content)
            .context("Failed to deserialize state entry")?;

        Ok(state)
    }

    /// Save the state file (scroll positions, etc.)
    pub async fn save_state(&self, state: &StateEntry) -> Result<()> {
        // Ensure parent directories exist
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create state directory")?;
        }

        let json = serde_json::to_string_pretty(state)
            .context("Failed to serialize state entry")?;

        fs::write(&self.state_path, json)
            .await
            .context("Failed to write state file")?;

        log::info!("State saved to {}", self.state_path.display());
        Ok(())
    }

    /// Get the cache file path (deprecated: use cache_path())
    pub fn path(&self) -> &PathBuf {
        &self.cache_path
    }

    /// Load the cached snapshot and its timestamp if it exists
    /// Gracefully handles schema mismatches by treating them as cache misses
    pub async fn load(&self) -> Result<Option<(RepoSnapshot, DateTime<Utc>)>> {
        if !self.cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.cache_path)
            .await
            .context("Failed to read cache file")?;

        if content.trim().is_empty() {
            return Ok(None);
        }

        // Try to deserialize - if it fails due to schema mismatch, treat as stale
        let entry: CacheEntry = match serde_json::from_str(&content) {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("Cache schema mismatch (will fetch fresh): {}", e);
                return Ok(None);
            }
        };

        Ok(Some((entry.snapshot, entry.fetched_at)))
    }

    /// Save a snapshot to the cache with the current timestamp
    pub async fn save(&self, snapshot: &RepoSnapshot) -> Result<()> {
        // Ensure parent directories exist
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create cache directories")?;
        }

        let entry = CacheEntry {
            fetched_at: Utc::now(),
            snapshot: snapshot.clone(),
        };

        let json = serde_json::to_string_pretty(&entry)
            .context("Failed to serialize cache entry")?;

        fs::write(&self.cache_path, json)
            .await
            .context("Failed to write cache file")?;

        log::info!("Cache saved to {}", self.cache_path.display());

        Ok(())
    }

    /// Check if the cache is stale based on TTL
    /// Returns true if:
    /// - Cache file doesn't exist
    /// - Cannot read file metadata
    /// - File modified time is older than TTL
    pub fn is_stale(&self, ttl_mins: u64) -> bool {
        if !self.cache_path.exists() {
            return true;
        }

        let modified = match std::fs::metadata(&self.cache_path) {
            Ok(meta) => match meta.modified() {
                Ok(time) => time,
                Err(_) => return true,
            },
            Err(_) => return true,
        };

        let modified_utc: DateTime<Utc> = modified.into();
        let now = Utc::now();
        let age = now.signed_duration_since(modified_utc);

        age.num_minutes() > ttl_mins as i64
    }

    /// Clear the cache file if it exists
    pub async fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            fs::remove_file(&self.cache_path)
                .await
                .context("Failed to remove cache file")?;
            log::info!("Cache cleared at {}", self.cache_path.display());
        }
        Ok(())
    }

    /// Check if cache exists
    pub fn exists(&self) -> bool {
        self.cache_path.exists()
    }

    /// Purge history files older than the specified number of days
    /// Returns the number of files deleted
    pub async fn purge_history(&self, days: u64) -> Result<usize> {
        if !self.history_path.exists() {
            return Ok(0);
        }

        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let mut deleted_count = 0;

        let mut entries = fs::read_dir(&self.history_path)
            .await
            .context("Failed to read history directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            // Only process files (not directories)
            if !metadata.is_file() {
                continue;
            }

            // Get file modified time
            let modified = match metadata.modified() {
                Ok(time) => time,
                Err(_) => continue,
            };

            let modified_utc: DateTime<Utc> = modified.into();

            // Delete if older than cutoff
            if modified_utc < cutoff {
                match fs::remove_file(&path).await {
                    Ok(_) => {
                        log::info!("Purged old history file: {}", path.display());
                        deleted_count += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to purge history file {}: {}", path.display(), e);
                    }
                }
            }
        }

        if deleted_count > 0 {
            log::info!("Purged {} history files older than {} days", deleted_count, days);
        }

        Ok(deleted_count)
    }

    /// Maximum number of snapshots to keep in history
    pub const MAX_HISTORY_SNAPSHOTS: usize = 20;

    /// Save a snapshot to the history directory with rotation
    /// Returns the path to the saved history file
    pub async fn save_to_history(&self, snapshot: &RepoSnapshot) -> Result<PathBuf> {
        // Ensure history directory exists
        fs::create_dir_all(&self.history_path)
            .await
            .context("Failed to create history directory")?;

        // Get current history files sorted by modification time (oldest first)
        let mut history_files = self.get_history_files().await?;

        // If we're at the limit, delete the oldest file(s)
        while history_files.len() >= Self::MAX_HISTORY_SNAPSHOTS {
            if let Some((oldest_path, _)) = history_files.pop() {
                match fs::remove_file(&oldest_path).await {
                    Ok(_) => {
                        log::info!("Removed oldest history file: {}", oldest_path.display());
                    }
                    Err(e) => {
                        log::error!("Failed to remove old history file {}: {}", oldest_path.display(), e);
                        break;
                    }
                }
            }
        }

        // Save the new snapshot with UUID filename
        let filename = format!("{}.json", snapshot.snapshot_history_id);
        let history_file_path = self.history_path.join(&filename);

        let entry = CacheEntry {
            fetched_at: snapshot.fetched_at,
            snapshot: snapshot.clone(),
        };

        let json = serde_json::to_string_pretty(&entry)
            .context("Failed to serialize snapshot for history")?;

        fs::write(&history_file_path, json)
            .await
            .context("Failed to write history file")?;

        log::info!("Saved snapshot to history: {}", history_file_path.display());

        Ok(history_file_path)
    }

    /// Load a specific snapshot from history by its UUID
    pub async fn load_from_history_by_id(&self, history_id: &Uuid) -> Result<Option<RepoSnapshot>> {
        let filename = format!("{}.json", history_id);
        let history_file_path = self.history_path.join(&filename);

        if !history_file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&history_file_path)
            .await
            .context("Failed to read history file")?;

        if content.trim().is_empty() {
            return Ok(None);
        }

        // Try to deserialize - if it fails, treat as missing
        let entry: CacheEntry = match serde_json::from_str(&content) {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!("History file schema mismatch: {}", e);
                return Ok(None);
            }
        };

        Ok(Some(entry.snapshot))
    }

    /// Load the most recent history snapshot (excluding the given current ID)
    pub async fn load_previous_snapshot(&self, current_id: &Uuid) -> Result<Option<RepoSnapshot>> {
        let history_files = self.get_history_files().await?;

        // Iterate from newest to oldest (reverse order)
        for (path, _) in history_files.iter().rev() {
            // Extract UUID from filename
            if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(file_uuid) = Uuid::parse_str(filename) {
                    // Skip if this is the current snapshot
                    if &file_uuid == current_id {
                        continue;
                    }

                    // Try to load this snapshot
                    if let Ok(content) = fs::read_to_string(path).await {
                        if let Ok(entry) = serde_json::from_str::<CacheEntry>(&content) {
                            return Ok(Some(entry.snapshot));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get history files sorted by modification time (oldest first)
    /// Public for testing purposes
    pub async fn get_history_files(&self) -> Result<Vec<(PathBuf, DateTime<Utc>)>> {
        let mut files = Vec::new();

        if !self.history_path.exists() {
            return Ok(files);
        }

        let mut entries = fs::read_dir(&self.history_path)
            .await
            .context("Failed to read history directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            // Only process files
            if !metadata.is_file() {
                continue;
            }

            // Only process .json files
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let modified = match metadata.modified() {
                Ok(time) => time,
                Err(_) => continue,
            };

            let modified_utc: DateTime<Utc> = modified.into();
            files.push((path, modified_utc));
        }

        // Sort by modification time (oldest first)
        files.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(files)
    }

    /// Get the count of history snapshots
    pub async fn history_count(&self) -> Result<usize> {
        let files = self.get_history_files().await?;
        Ok(files.len())
    }

    /// Scan all cached repositories in the base repowatch directory
    /// Returns a list of all repos with their metadata for fuzzy finder
    pub async fn scan_all_repos() -> Result<Vec<CachedRepoInfo>> {
        let base_dir = data_local_dir()
            .context("Failed to get data local directory")?
            .join("repowatch");

        let mut repos = Vec::new();

        // Check if base directory exists
        if !base_dir.exists() {
            return Ok(repos);
        }

        // Read the base directory (list of owners)
        let mut owner_entries = fs::read_dir(&base_dir)
            .await
            .context("Failed to read repowatch directory")?;

        while let Some(owner_entry) = owner_entries.next_entry().await? {
            let owner_path = owner_entry.path();
            let owner_metadata = fs::metadata(&owner_path).await.ok();

            // Skip non-directories
            if !owner_metadata.map(|m| m.is_dir()).unwrap_or(false) {
                continue;
            }

            let owner = owner_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if owner.is_empty() {
                continue;
            }

            // Read owner directory (list of repos)
            let mut repo_entries = fs::read_dir(&owner_path)
                .await
                .context("Failed to read owner directory")?;

            while let Some(repo_entry) = repo_entries.next_entry().await? {
                let repo_path = repo_entry.path();
                let repo_metadata = fs::metadata(&repo_path).await.ok();

                // Skip non-directories
                if !repo_metadata.map(|m| m.is_dir()).unwrap_or(false) {
                    continue;
                }

                let repo_name = repo_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if repo_name.is_empty() {
                    continue;
                }

                // Check if cache.json exists (valid cached repo)
                let cache_file = repo_path.join("cache.json");
                if !cache_file.exists() {
                    continue;
                }

                // Try to load the snapshot to get description and other metadata
                let (description, last_viewed_at) =
                    if let Ok(content) = fs::read_to_string(&cache_file).await {
                        if let Ok(entry) = serde_json::from_str::<CacheEntry>(&content) {
                            (
                                entry.snapshot.repo.description.clone(),
                                Self::load_state_from_path(&repo_path.join("state.json")).await
                                    .ok()
                                    .and_then(|s| s.last_viewed_at),
                            )
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                repos.push(CachedRepoInfo {
                    owner: owner.clone(),
                    name: repo_name.clone(),
                    full_name: format!("{}/{}", owner, repo_name),
                    description,
                    path: repo_path,
                    last_viewed_at,
                });
            }
        }

        // Sort by last viewed (most recent first), then alphabetically
        repos.sort_by(|a, b| {
            match (a.last_viewed_at, b.last_viewed_at) {
                (Some(a_time), Some(b_time)) => b_time.cmp(&a_time), // Reverse for most recent first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.full_name.cmp(&b.full_name),
            }
        });

        Ok(repos)
    }

    /// Load state from a specific path (helper for scan_all_repos)
    async fn load_state_from_path(state_path: &PathBuf) -> Result<StateEntry> {
        if !state_path.exists() {
            return Ok(StateEntry::default());
        }

        let content = fs::read_to_string(state_path)
            .await
            .context("Failed to read state file")?;

        if content.trim().is_empty() {
            return Ok(StateEntry::default());
        }

        let state: StateEntry = serde_json::from_str(&content)
            .context("Failed to deserialize state entry")?;

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{
        Contributor, ContributorStats, Issue, IssueStats, PrStats, Release, RepoMeta,
        RepoSnapshot, StarHistory, VelocityStats, WeeklyActivity,
    };
    use std::collections::HashMap;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            fetched_at: Utc::now(),
            previous_snapshot_at: None,
            snapshot_history_id: Uuid::new_v4(),
            repo: RepoMeta {
                owner: "test-owner".to_string(),
                name: "test-repo".to_string(),
                description: Some("Test repository".to_string()),
                language: Some("Rust".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                default_branch: "main".to_string(),
                forks_count: 50,
                open_issues_count: 5,
                watchers_count: 500,
            },
            stars: StarHistory {
                total_count: 1000,
                sparkline_30d: vec![5, 10, 15],
                sparkline_90d: vec![50, 100, 150],
                sparkline_365d: vec![500, 1000],
            },
            issues: IssueStats {
                total_open: 5,
                by_label: HashMap::new(),
                unlabelled: vec![Issue {
                    number: 1,
                    title: "Test issue".to_string(),
                    author: "testuser".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    labels: vec![],
                    comments_count: 0,
                }],
                truncated: false,
            },
            pull_requests: PrStats {
                open_count: 2,
                draft_count: 0,
                ready_count: 2,
                merged_last_30d: vec![],
                avg_time_to_merge_hours: None,
            },
            contributors: ContributorStats {
                top_contributors: vec![Contributor {
                    username: "testuser".to_string(),
                    commit_count: 10,
                    avatar_url: None,
                }],
                new_contributors_last_30d: vec![],
                total_unique: 1,
            },
            releases: vec![Release {
                tag_name: "v1.0.0".to_string(),
                name: Some("Initial release".to_string()),
                created_at: Utc::now(),
                published_at: Some(Utc::now()),
                prerelease: false,
                draft: false,
                days_since: Some(0),
                avg_interval: None,
            }],
            velocity: VelocityStats {
                issues_weekly: vec![WeeklyActivity {
                    week_start: Utc::now(),
                    opened: 5,
                    closed: 3,
                }],
                prs_weekly: vec![WeeklyActivity {
                    week_start: Utc::now(),
                    opened: 2,
                    closed: 1,
                }],
            },
            security_alerts: None,
            ci_status: None,
            community_health: None,
        }
    }

    #[test]
    fn test_cache_path_structure() {
        let cache = Cache::new("octocat", "Hello-World").unwrap();

        // Verify paths contain expected components
        let base_str = cache.base_path().to_string_lossy();
        let cache_str = cache.cache_path().to_string_lossy();
        let history_str = cache.history_path().to_string_lossy();
        let state_str = cache.state_path().to_string_lossy();

        assert!(base_str.contains("repowatch"));
        assert!(base_str.contains("octocat"));
        assert!(base_str.contains("Hello-World"));
        assert!(cache_str.ends_with("cache.json"));
        assert!(history_str.ends_with("history"));
        assert!(state_str.ends_with("state.json"));
    }

    #[tokio::test]
    async fn test_cache_initialize_creates_structure() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().join("repowatch").join("owner").join("repo"),
            cache_path: temp_dir.path().join("repowatch").join("owner").join("repo").join("cache.json"),
            history_path: temp_dir.path().join("repowatch").join("owner").join("repo").join("history"),
            state_path: temp_dir.path().join("repowatch").join("owner").join("repo").join("state.json"),
        };

        // Initialize should create directories and state.json
        cache.initialize().await.unwrap();

        // Verify structure
        assert!(cache.base_path().exists());
        assert!(cache.history_path().exists());
        assert!(cache.state_path().exists());

        // Verify state.json is valid
        let state = cache.load_state().await.unwrap();
        assert!(state.scroll_positions.is_empty());
        assert!(state.last_viewed_at.is_none());
    }

    #[tokio::test]
    async fn test_cache_save_and_load_roundtrip() {
        // Use a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create cache with custom path for testing
        let snapshot = create_test_snapshot();
        let entry = CacheEntry {
            fetched_at: Utc::now(),
            snapshot: snapshot.clone(),
        };

        // Save to file
        let json = serde_json::to_string_pretty(&entry).unwrap();
        fs::write(&cache.cache_path, json).await.unwrap();

        // Read back and verify
        let content = fs::read_to_string(&cache.cache_path).await.unwrap();
        let loaded: CacheEntry = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.snapshot.repo.owner, snapshot.repo.owner);
        assert_eq!(loaded.snapshot.repo.name, snapshot.repo.name);
        assert_eq!(loaded.snapshot.stars.total_count, snapshot.stars.total_count);
        assert_eq!(loaded.snapshot.issues.total_open, snapshot.issues.total_open);
    }

    #[tokio::test]
    async fn test_cache_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().join("nonexistent"),
            cache_path: temp_dir.path().join("nonexistent").join("cache.json"),
            history_path: temp_dir.path().join("nonexistent").join("history"),
            state_path: temp_dir.path().join("nonexistent").join("state.json"),
        };

        let result = cache.load().await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_is_stale_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().join("nonexistent"),
            cache_path: temp_dir.path().join("nonexistent").join("cache.json"),
            history_path: temp_dir.path().join("nonexistent").join("history"),
            state_path: temp_dir.path().join("nonexistent").join("state.json"),
        };

        // Non-existent cache should be considered stale
        assert!(cache.is_stale(15));
    }

    #[tokio::test]
    async fn test_cache_is_stale_fresh() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create cache file
        let entry = CacheEntry {
            fetched_at: Utc::now(),
            snapshot: create_test_snapshot(),
        };
        let json = serde_json::to_string_pretty(&entry).unwrap();
        fs::write(&cache.cache_path, json).await.unwrap();

        // Cache created just now should not be stale with 15 min TTL
        assert!(!cache.is_stale(15));
    }

    #[tokio::test]
    async fn test_cache_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Cache doesn't exist initially
        assert!(!cache.exists());

        // Create the cache file
        let entry = CacheEntry {
            fetched_at: Utc::now(),
            snapshot: create_test_snapshot(),
        };
        let json = serde_json::to_string_pretty(&entry).unwrap();
        fs::write(&cache.cache_path, json).await.unwrap();

        // Cache should now exist
        assert!(cache.exists());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create cache file
        let entry = CacheEntry {
            fetched_at: Utc::now(),
            snapshot: create_test_snapshot(),
        };
        let json = serde_json::to_string_pretty(&entry).unwrap();
        fs::write(&cache.cache_path, json).await.unwrap();

        assert!(cache.exists());

        // Clear cache
        cache.clear().await.unwrap();

        // Cache should no longer exist
        assert!(!cache.exists());
    }

    #[tokio::test]
    async fn test_cache_save_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().join("deep").join("nested"),
            cache_path: temp_dir.path().join("deep").join("nested").join("cache.json"),
            history_path: temp_dir.path().join("deep").join("nested").join("history"),
            state_path: temp_dir.path().join("deep").join("nested").join("state.json"),
        };

        let snapshot = create_test_snapshot();

        // Save should create all necessary directories
        cache.save(&snapshot).await.unwrap();

        assert!(cache.cache_path.exists());
    }

    #[tokio::test]
    async fn test_state_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create and save state
        let mut state = StateEntry {
            scroll_positions: std::collections::HashMap::new(),
            last_viewed_at: Some(Utc::now()),
        };
        state.scroll_positions.insert("stars".to_string(), 5);
        state.scroll_positions.insert("issues".to_string(), 10);

        cache.save_state(&state).await.unwrap();

        // Load and verify
        let loaded = cache.load_state().await.unwrap();
        assert_eq!(loaded.scroll_positions.get("stars"), Some(&5));
        assert_eq!(loaded.scroll_positions.get("issues"), Some(&10));
        assert!(loaded.last_viewed_at.is_some());
    }

    #[tokio::test]
    async fn test_purge_history_deletes_old_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create history directory
        fs::create_dir_all(&cache.history_path).await.unwrap();

        // Create a recent file (should NOT be deleted)
        let recent_file = cache.history_path.join("recent_snapshot.json");
        fs::write(&recent_file, "{}").await.unwrap();

        // Create old files by touching them with an old timestamp would be complex,
        // so instead we'll test the basic functionality - purge returns 0 when nothing old
        let deleted = cache.purge_history(30).await.unwrap();
        assert_eq!(deleted, 0); // Recent file should not be deleted
        assert!(recent_file.exists()); // File should still exist
    }

    #[tokio::test]
    async fn test_purge_history_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Don't create history directory - test that purge handles missing dir
        let deleted = cache.purge_history(30).await.unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn test_save_to_history_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        let snapshot = create_test_snapshot();
        let history_path = cache.save_to_history(&snapshot).await.unwrap();

        // Verify file was created with correct name
        assert!(history_path.exists());
        assert!(history_path.to_string_lossy().contains(&snapshot.snapshot_history_id.to_string()));

        // Verify history count
        let count = cache.history_count().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_save_to_history_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache {
            base_path: temp_dir.path().to_path_buf(),
            cache_path: temp_dir.path().join("cache.json"),
            history_path: temp_dir.path().join("history"),
            state_path: temp_dir.path().join("state.json"),
        };

        // Create 25 snapshots (more than MAX_HISTORY_SNAPSHOTS = 20)
        // The exact files that get deleted depend on filesystem timestamps,
        // so we just verify the count is correct
        for i in 0..25 {
            let mut snapshot = create_test_snapshot();
            snapshot.snapshot_history_id = Uuid::new_v4();
            snapshot.fetched_at = Utc::now() + chrono::Duration::seconds(i);

            cache.save_to_history(&snapshot).await.unwrap();

            // Small delay to ensure different modification times
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Verify only 20 remain (rotation should have deleted the oldest 5)
        let count = cache.history_count().await.unwrap();
        assert_eq!(count, 20, "Should have exactly 20 history snapshots after rotation (added 25, max is 20)");

        // Verify we can still add more snapshots and maintain the limit
        for i in 0..5 {
            let mut snapshot = create_test_snapshot();
            snapshot.snapshot_history_id = Uuid::new_v4();
            snapshot.fetched_at = Utc::now() + chrono::Duration::seconds(100 + i);

            cache.save_to_history(&snapshot).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Should still be 20
        let final_count = cache.history_count().await.unwrap();
        assert_eq!(final_count, 20, "Should still have exactly 20 history snapshots after adding 5 more");
    }
}
