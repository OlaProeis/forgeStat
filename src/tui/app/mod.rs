mod command_palette;
mod compare;
mod diff;
mod event_loop;
mod fuzzy_finder;
mod help;
mod loading_screen;
mod mini_map;
mod mouse;
mod panels;
mod token_input;
pub(crate) mod utils;
pub mod watchlist;
mod zoom;

pub use loading_screen::{FetchProgress, LoadingScreen};

pub use watchlist::{WatchlistAction, WatchlistApp};

use std::collections::HashSet;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use chrono::Utc;
use ratatui::{prelude::*, widgets::*};

use crate::core::cache::CachedRepoInfo;
use crate::core::config::{AnimationConfig, LayoutConfig, StatusBarConfig, StatusBarItem};
use crate::core::health::{compute_health_score, HealthScore};
use crate::core::models::{Contributor, Issue, RateLimitInfo, Release, RepoSnapshot, SnapshotDiff};
use crate::core::theme::ThemeConfig;
use crate::tui::app::compare::CompareFocus;
use crate::tui::widgets::{AnimatedCounter, BrailleSpinner};

const AUTO_REFRESH_SECS: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Stars,
    Issues,
    PullRequests,
    Contributors,
    Releases,
    Velocity,
    Security,
    CI,
}

impl Panel {
    const VALUES: [Self; 8] = [
        Self::Stars,
        Self::Issues,
        Self::PullRequests,
        Self::Contributors,
        Self::Releases,
        Self::Velocity,
        Self::Security,
        Self::CI,
    ];

    fn next(self) -> Self {
        match self {
            Self::Stars => Self::Issues,
            Self::Issues => Self::PullRequests,
            Self::PullRequests => Self::Contributors,
            Self::Contributors => Self::Releases,
            Self::Releases => Self::Velocity,
            Self::Velocity => Self::Security,
            Self::Security => Self::CI,
            Self::CI => Self::Stars,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Stars => Self::CI,
            Self::Issues => Self::Stars,
            Self::PullRequests => Self::Issues,
            Self::Contributors => Self::PullRequests,
            Self::Releases => Self::Contributors,
            Self::Velocity => Self::Releases,
            Self::Security => Self::Velocity,
            Self::CI => Self::Security,
        }
    }

    /// Get the display name for this panel
    fn display_name(&self) -> &'static str {
        match self {
            Self::Stars => "Stars",
            Self::Issues => "Issues",
            Self::PullRequests => "Pull Requests",
            Self::Contributors => "Contributors",
            Self::Releases => "Releases",
            Self::Velocity => "Velocity",
            Self::Security => "Security",
            Self::CI => "CI Status",
        }
    }
}

/// Tracks the current drag operation for resizing panels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DragState {
    /// Which border is being dragged (index into vertical_borders or horizontal_borders)
    border_index: usize,
    /// Whether this is a vertical (column) or horizontal (row) border drag
    border_type: BorderType,
    /// Last mouse position for calculating delta
    last_mouse_pos: (u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorderType {
    Vertical,   // Resizing columns within a row
    Horizontal, // Resizing rows
}

/// Action hint for the status bar (key binding + description)
struct ActionHint<'a> {
    key: &'a str,
    description: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarTimeframe {
    Days30,
    Days90,
    Year1,
}

impl StarTimeframe {
    fn next(self) -> Self {
        match self {
            Self::Days30 => Self::Days90,
            Self::Days90 => Self::Year1,
            Self::Year1 => Self::Days30,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Days30 => Self::Year1,
            Self::Days90 => Self::Days30,
            Self::Year1 => Self::Days90,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Days30 => "30-day trend",
            Self::Days90 => "90-day trend",
            Self::Year1 => "1-year trend",
        }
    }
}

/// Timeframe options for Velocity panel (weeks of history)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityTimeframe {
    Weeks4,
    Weeks8,
    Weeks12,
}

impl VelocityTimeframe {
    fn next(self) -> Self {
        match self {
            Self::Weeks4 => Self::Weeks8,
            Self::Weeks8 => Self::Weeks12,
            Self::Weeks12 => Self::Weeks4,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Weeks4 => Self::Weeks12,
            Self::Weeks8 => Self::Weeks4,
            Self::Weeks12 => Self::Weeks8,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Weeks4 => "4 weeks",
            Self::Weeks8 => "8 weeks",
            Self::Weeks12 => "12 weeks",
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Weeks4 => 4,
            Self::Weeks8 => 8,
            Self::Weeks12 => 12,
        }
    }
}

/// Limit options for Contributors panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributorsLimit {
    Top10,
    Top25,
    Top50,
}

impl ContributorsLimit {
    fn next(self) -> Self {
        match self {
            Self::Top10 => Self::Top25,
            Self::Top25 => Self::Top50,
            Self::Top50 => Self::Top10,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Top10 => Self::Top50,
            Self::Top25 => Self::Top10,
            Self::Top50 => Self::Top25,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Top10 => "top 10",
            Self::Top25 => "top 25",
            Self::Top50 => "top 50",
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Top10 => 10,
            Self::Top25 => 25,
            Self::Top50 => 50,
        }
    }
}

/// Limit options for Releases panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleasesLimit {
    Last5,
    Last10,
    Last15,
}

impl ReleasesLimit {
    fn next(self) -> Self {
        match self {
            Self::Last5 => Self::Last10,
            Self::Last10 => Self::Last15,
            Self::Last15 => Self::Last5,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Last5 => Self::Last15,
            Self::Last10 => Self::Last5,
            Self::Last15 => Self::Last10,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Last5 => "5",
            Self::Last10 => "10",
            Self::Last15 => "15",
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Last5 => 5,
            Self::Last10 => 10,
            Self::Last15 => 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Live,
    Stale,
    Offline,
}

/// Sort options for Issues panel in zoom mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssuesSort {
    Number,
    Title,
    Author,
    Age,
    Comments,
}

impl IssuesSort {
    fn next(self) -> Self {
        match self {
            Self::Number => Self::Title,
            Self::Title => Self::Author,
            Self::Author => Self::Age,
            Self::Age => Self::Comments,
            Self::Comments => Self::Number,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Number => "#",
            Self::Title => "title",
            Self::Author => "author",
            Self::Age => "age",
            Self::Comments => "comments",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Quit,
    Refresh,
    /// Switch to a different repository (owner, repo)
    SwitchRepo(String, String),
}

pub struct App {
    pub snapshot: Option<RepoSnapshot>,
    selected_panel: Panel,
    sync_state: SyncState,
    show_help: bool,
    owner: String,
    repo: String,
    rate_limit: Option<RateLimitInfo>,
    pub last_refresh: Instant,
    /// Computed health score from current snapshot
    health_score: Option<HealthScore>,
    star_timeframe: StarTimeframe,
    // Scroll offsets for scrollable panels
    issues_scroll: usize,
    contributors_scroll: usize,
    releases_scroll: usize,
    // Panel areas for mouse click detection
    panel_areas: [Option<Rect>; 8],
    // Border areas for resize detection (vertical and horizontal borders)
    vertical_borders: [Option<Rect>; 5], // 1 in row1, 2 in row2, 2 in row3
    horizontal_borders: [Option<Rect>; 2], // Between row1/row2 and row2/row3
    // Drag state for resizing
    drag_state: Option<DragState>,
    // Theme configuration
    theme: ThemeConfig,
    // Status bar configuration
    statusbar_config: StatusBarConfig,
    // Layout configuration
    layout_config: LayoutConfig,
    // Mini-map overview mode
    show_mini_map: bool,
    // Zoom modal state - which panel is zoomed, if any
    zoom_panel: Option<Panel>,
    // Zoom-specific scroll offsets (independent from normal view)
    zoom_issues_scroll: usize,
    zoom_contributors_scroll: usize,
    zoom_releases_scroll: usize,
    zoom_stars_scroll: usize,
    zoom_prs_scroll: usize,
    // Search/Filter state
    search_mode: bool,
    search_query: String,
    // Label filter for Issues panel (cycles through available labels + "all")
    issues_label_filter: Option<String>,
    // Sort order for Issues panel in zoom mode
    issues_sort: IssuesSort,
    // Pre-release filter for Releases panel
    releases_prerelease_filter: Option<bool>, // None = all, Some(true) = prerelease only, Some(false) = stable only
    // Panel-specific timeframe/list size controls
    velocity_timeframe: VelocityTimeframe,
    contributors_limit: ContributorsLimit,
    releases_limit: ReleasesLimit,
    // Pagination for issues and PRs (items per page)
    issues_per_page: usize,
    prs_per_page: usize,
    // Fuzzy finder state
    fuzzy_mode: bool,
    fuzzy_query: String,
    fuzzy_repos: Vec<CachedRepoInfo>,
    fuzzy_selected_index: usize,
    // Diff mode state
    diff_mode: bool,
    previous_snapshot: Option<RepoSnapshot>,
    snapshot_diff: Option<SnapshotDiff>,
    // Compare mode state
    compare_mode: bool,
    compare_snapshot: Option<RepoSnapshot>,
    compare_health_score: Option<HealthScore>,
    compare_focus: compare::CompareFocus,
    // Clipboard copy toast notification
    clipboard_toast: Option<(String, Instant)>,
    toast_duration: Duration,
    // Command palette state
    command_palette_mode: bool,
    command_input: String,
    command_history: Vec<String>,
    command_history_index: Option<usize>,
    command_suggestions: Vec<String>,
    command_selected_suggestion: usize,
    // Animation configuration
    animation_config: AnimationConfig,
    // Braille spinner for live indicator
    live_spinner: BrailleSpinner,
    last_spinner_update: Instant,
    // Panel highlight flash state
    panel_flash: Option<(Panel, Instant)>, // Panel being flashed and when it started
    // Count-up animation state for metrics
    animated_counters: std::collections::HashMap<String, AnimatedCounter>,
    last_animation_tick: Instant,
    // Sync pulse animation state
    sync_pulse_active: bool,
    sync_pulse_start: Option<Instant>,
    // Token input dialog state
    token_input_mode: bool,
    token_input: String,
    token_error: Option<String>,
    token_input_masked: bool,
    // Double-click tracking for zoom
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
}

impl App {
    pub fn new(
        owner: String,
        repo: String,
        theme: ThemeConfig,
        statusbar_config: StatusBarConfig,
        layout_config: LayoutConfig,
    ) -> Self {
        Self {
            snapshot: None,
            selected_panel: Panel::Stars,
            sync_state: SyncState::Offline,
            show_help: false,
            owner,
            repo,
            rate_limit: None,
            last_refresh: Instant::now(),
            star_timeframe: StarTimeframe::Days30,
            issues_scroll: 0,
            contributors_scroll: 0,
            releases_scroll: 0,
            panel_areas: [None; 8],
            vertical_borders: [None; 5],
            horizontal_borders: [None; 2],
            drag_state: None,
            theme,
            statusbar_config,
            layout_config,
            show_mini_map: false,
            zoom_panel: None,
            zoom_issues_scroll: 0,
            zoom_contributors_scroll: 0,
            zoom_releases_scroll: 0,
            zoom_stars_scroll: 0,
            zoom_prs_scroll: 0,
            search_mode: false,
            search_query: String::new(),
            issues_label_filter: None,
            issues_sort: IssuesSort::Number,
            releases_prerelease_filter: None,
            velocity_timeframe: VelocityTimeframe::Weeks8,
            contributors_limit: ContributorsLimit::Top10,
            releases_limit: ReleasesLimit::Last5,
            issues_per_page: 15,
            prs_per_page: 15,
            fuzzy_mode: false,
            fuzzy_query: String::new(),
            fuzzy_repos: Vec::new(),
            fuzzy_selected_index: 0,
            diff_mode: false,
            previous_snapshot: None,
            snapshot_diff: None,
            compare_mode: false,
            compare_snapshot: None,
            compare_health_score: None,
            compare_focus: CompareFocus::Left,
            clipboard_toast: None,
            toast_duration: Duration::from_secs(2),
            command_palette_mode: false,
            command_input: String::new(),
            command_history: Vec::new(),
            command_history_index: None,
            command_suggestions: Vec::new(),
            command_selected_suggestion: 0,
            animation_config: AnimationConfig::load(),
            live_spinner: BrailleSpinner::new(),
            last_spinner_update: Instant::now(),
            panel_flash: None,
            animated_counters: std::collections::HashMap::new(),
            last_animation_tick: Instant::now(),
            sync_pulse_active: false,
            sync_pulse_start: None,
            health_score: None,
            token_input_mode: false,
            token_input: String::new(),
            token_error: None,
            token_input_masked: true,
            last_click_time: None,
            last_click_pos: None,
        }
    }

    /// Toggle diff mode on/off
    pub fn toggle_diff_mode(&mut self) {
        if self.diff_mode {
            // Turn off diff mode
            self.diff_mode = false;
            log::info!("Diff mode disabled");
        } else {
            // Try to enable diff mode
            self.try_enable_diff_mode();
        }
    }

    /// Try to enable diff mode by loading previous snapshot
    fn try_enable_diff_mode(&mut self) {
        let Some(ref current) = self.snapshot else {
            log::warn!("Cannot enable diff mode: no current snapshot");
            return;
        };

        // Check if there's a previous snapshot reference
        if current.previous_snapshot_at.is_none() {
            log::warn!("Cannot enable diff mode: no previous snapshot reference");
            return;
        }

        // Enable diff mode - the previous snapshot will be loaded in render_diff_overlay
        self.diff_mode = true;
        log::info!("Diff mode enabled");
    }

    /// Load the previous snapshot from history (called from render if needed)
    fn load_previous_snapshot(&mut self) {
        if self.diff_mode && self.previous_snapshot.is_none() {
            let Some(ref current) = self.snapshot else {
                return;
            };

            // Get the previous snapshot ID if available
            // For now, we'll load the most recent history snapshot that's not the current one
            let current_id = current.snapshot_history_id;

            // Use blocking task to load previous snapshot from history
            let owner = self.owner.clone();
            let repo = self.repo.clone();

            let prev_result = std::thread::scope(|s| {
                s.spawn(move || {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(async {
                        let cache = crate::core::cache::Cache::new(&owner, &repo).ok()?;
                        cache
                            .load_previous_snapshot(&current_id)
                            .await
                            .ok()
                            .flatten()
                    })
                })
                .join()
                .ok()
                .flatten()
            });

            if let Some(prev) = prev_result {
                let diff = SnapshotDiff::compute(current, &prev);
                self.snapshot_diff = Some(diff);
                self.previous_snapshot = Some(prev);
                log::info!("Loaded previous snapshot for diff mode");
            } else {
                log::warn!("No previous snapshot found in history for diff mode");
            }
        }
    }

    /// Set the previous snapshot and compute diff
    pub fn set_previous_snapshot(&mut self, snapshot: RepoSnapshot) {
        if let Some(ref current) = self.snapshot {
            let diff = SnapshotDiff::compute(current, &snapshot);
            self.snapshot_diff = Some(diff);
        }
        self.previous_snapshot = Some(snapshot);
    }

    /// Check if diff mode is active
    pub fn is_diff_mode(&self) -> bool {
        self.diff_mode
    }

    /// Exit diff mode
    pub fn exit_diff_mode(&mut self) {
        self.diff_mode = false;
        // Clear previous snapshot to reload on next entry
        self.previous_snapshot = None;
        self.snapshot_diff = None;
    }

    // Compare mode methods

    /// Enter compare mode with a second repository snapshot
    pub fn enter_compare_mode(&mut self, snapshot: RepoSnapshot) {
        log::info!(
            "Entering compare mode with {}/{}",
            snapshot.repo.owner,
            snapshot.repo.name
        );
        self.compare_health_score = Some(compute_health_score(&snapshot));
        self.compare_snapshot = Some(snapshot);
        self.compare_mode = true;
        self.compare_focus = CompareFocus::Left;
    }

    /// Exit compare mode
    pub fn exit_compare_mode(&mut self) {
        self.compare_mode = false;
        self.compare_snapshot = None;
        self.compare_health_score = None;
        self.compare_focus = CompareFocus::Left;
        log::info!("Exiting compare mode");
    }

    /// Toggle compare focus between left and right panels
    pub fn toggle_compare_focus(&mut self) {
        self.compare_focus = self.compare_focus.toggle();
        log::debug!("Compare focus switched to {:?}", self.compare_focus);
    }

    /// Check if compare mode is active
    pub fn is_compare_mode(&self) -> bool {
        self.compare_mode
    }

    /// Copy contextual content to clipboard based on current panel
    pub fn copy_to_clipboard(&mut self) {
        let content = match self.selected_panel {
            Panel::Issues => self.get_all_filtered_issue_references(),
            Panel::Contributors => self.get_selected_contributor_username(),
            Panel::Releases => self.get_selected_release_tag(),
            _ => self.snapshot.as_ref().map(|s| s.repo_url()),
        };

        if let Some(text) = content {
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    if let Err(e) = clipboard.set_text(&text) {
                        log::error!("Failed to copy to clipboard: {}", e);
                        self.show_toast("Copy failed!".to_string());
                    } else {
                        log::info!("Copied to clipboard: {}", text);
                        // Show count in toast for issues, preview for others
                        let toast_msg = match self.selected_panel {
                            Panel::Issues => {
                                let count = self.get_filtered_issues().len();
                                format!("Copied {} issue references", count)
                            }
                            _ => format!("Copied: {}", text),
                        };
                        self.show_toast(toast_msg);
                    }
                }
                Err(e) => {
                    log::error!("Failed to create clipboard: {}", e);
                    self.show_toast("Clipboard unavailable".to_string());
                }
            }
        } else {
            log::warn!("Nothing to copy for current panel");
            self.show_toast("Nothing to copy".to_string());
        }
    }

    /// Get all filtered issue references as a newline-separated list
    fn get_all_filtered_issue_references(&self) -> Option<String> {
        let snapshot = self.snapshot.as_ref()?;
        let filtered_issues = self.get_filtered_issues();

        if filtered_issues.is_empty() {
            return None;
        }

        // Format all filtered issues as a list
        let references: Vec<String> = filtered_issues
            .iter()
            .map(|issue| snapshot.format_issue_reference(issue.number))
            .collect();

        Some(references.join("\n"))
    }

    /// Get the contributor username at the current scroll position
    fn get_selected_contributor_username(&self) -> Option<String> {
        let filtered_contributors = self.get_filtered_contributors();

        // Get the contributor at the current scroll position
        filtered_contributors
            .get(self.contributors_scroll)
            .map(|c| c.username.clone())
    }

    /// Get the release tag name at the current scroll position
    fn get_selected_release_tag(&self) -> Option<String> {
        let filtered_releases = self.get_filtered_releases();

        // Get the release at the current scroll position
        filtered_releases
            .get(self.releases_scroll)
            .map(|r| r.tag_name.clone())
    }

    /// Show a toast notification
    fn show_toast(&mut self, message: String) {
        self.clipboard_toast = Some((message, Instant::now()));
    }

    /// Check if toast is active and get its message
    fn get_toast_message(&self) -> Option<&str> {
        self.clipboard_toast.as_ref().and_then(|(msg, start)| {
            if start.elapsed() < self.toast_duration {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// Reset layout to the current preset
    pub fn reset_layout(&mut self) {
        self.layout_config
            .reset_to_preset(self.layout_config.preset);
        // Save the reset layout
        if let Err(e) = self.layout_config.save() {
            log::warn!("Failed to save layout config after reset: {}", e);
        }
    }

    /// Get a reference to the layout config
    pub fn layout_config(&self) -> &LayoutConfig {
        &self.layout_config
    }

    pub fn set_snapshot(&mut self, snapshot: RepoSnapshot, state: SyncState) {
        // Trigger panel flash for all panels when new data arrives
        if self.snapshot.is_some() && self.animation_config.is_flash_enabled() {
            // Flash the currently selected panel prominently
            self.trigger_panel_flash(self.selected_panel);
        }

        // Initialize count-up animations for metrics
        if self.animation_config.is_count_up_enabled() {
            self.init_counter("stars", snapshot.stars.total_count);
            self.init_counter("issues", snapshot.open_issues_count());
            self.init_counter("prs", snapshot.open_prs_count());
            self.init_counter("contributors", snapshot.contributors.total_unique);
            self.init_counter("releases", snapshot.releases.len() as u64);
        }

        // Trigger sync pulse if going live
        if state == SyncState::Live && self.sync_state != SyncState::Live {
            self.trigger_sync_pulse();
        }

        // Compute health score from snapshot
        self.health_score = Some(compute_health_score(&snapshot));

        self.snapshot = Some(snapshot);
        self.sync_state = state;
        self.last_refresh = Instant::now();
    }

    pub fn set_offline(&mut self) {
        self.sync_state = SyncState::Offline;
        self.last_refresh = Instant::now();
    }

    pub fn set_rate_limit(&mut self, info: RateLimitInfo) {
        self.rate_limit = Some(info);
    }

    // Animation methods

    /// Get animation config reference
    pub fn animation_config(&self) -> &AnimationConfig {
        &self.animation_config
    }

    /// Trigger a panel highlight flash animation
    pub fn trigger_panel_flash(&mut self, panel: Panel) {
        if self.animation_config.is_flash_enabled() {
            self.panel_flash = Some((panel, Instant::now()));
        }
    }

    /// Check if a panel should be flashing and get flash intensity (0.0-1.0)
    pub fn get_flash_intensity(&self, panel: Panel) -> f64 {
        if !self.animation_config.is_flash_enabled() {
            return 0.0;
        }

        if let Some((flashing_panel, start)) = self.panel_flash {
            if flashing_panel == panel {
                let elapsed = start.elapsed().as_millis() as u64;
                let duration = self.animation_config.flash_duration_ms;

                if (elapsed) < duration {
                    // Calculate flash intensity that decays over time
                    let progress = elapsed as f64 / duration as f64;
                    // Easing: start bright, fade out quickly
                    return (1.0 - progress).powf(2.0);
                } else {
                    return 0.0;
                }
            }
        }
        0.0
    }

    /// Check if any panel is currently flashing
    pub fn is_any_panel_flashing(&self) -> bool {
        if !self.animation_config.is_flash_enabled() {
            return false;
        }
        self.panel_flash.map_or(false, |(_, start)| {
            let elapsed = start.elapsed().as_millis() as u64;
            elapsed < self.animation_config.flash_duration_ms
        })
    }

    /// Update animations - call this regularly (e.g., 60fps)
    /// Returns true if any animation needs a redraw
    pub fn update_animations(&mut self) -> bool {
        let mut needs_redraw = false;

        // Update live spinner if fetching
        if self.sync_state == SyncState::Live {
            let spinner_interval = Duration::from_millis(80);
            if self.last_spinner_update.elapsed() >= spinner_interval {
                self.live_spinner.next_frame();
                self.last_spinner_update = Instant::now();
                needs_redraw = true;
            }
        }

        // Update panel flash
        if self.is_any_panel_flashing() {
            needs_redraw = true;
        } else {
            // Clear completed flash
            self.panel_flash = None;
        }

        // Update count-up animations
        if self.animation_config.is_count_up_enabled() {
            let counter_interval = Duration::from_millis(16); // ~60fps
            if self.last_animation_tick.elapsed() >= counter_interval {
                let mut any_counter_updated = false;
                for counter in self.animated_counters.values_mut() {
                    if counter.step() {
                        any_counter_updated = true;
                    }
                }
                if any_counter_updated {
                    needs_redraw = true;
                }
                self.last_animation_tick = Instant::now();
            }
        }

        // Update sync pulse
        if self.sync_pulse_active {
            if let Some(start) = self.sync_pulse_start {
                let pulse_duration = Duration::from_millis(500);
                if start.elapsed() >= pulse_duration {
                    self.sync_pulse_active = false;
                    self.sync_pulse_start = None;
                } else {
                    needs_redraw = true;
                }
            }
        }

        needs_redraw
    }

    /// Trigger a sync pulse animation
    pub fn trigger_sync_pulse(&mut self) {
        if self.animation_config.is_sync_pulse_enabled() {
            self.sync_pulse_active = true;
            self.sync_pulse_start = Some(Instant::now());
        }
    }

    /// Get sync pulse intensity (0.0-1.0)
    pub fn get_sync_pulse_intensity(&self) -> f64 {
        if !self.sync_pulse_active || self.sync_pulse_start.is_none() {
            return 0.0;
        }

        let start = self.sync_pulse_start.unwrap();
        let elapsed = start.elapsed().as_millis() as f64;
        let duration = 500.0; // ms

        if elapsed >= duration {
            0.0
        } else {
            // Pulse effect: fade in then out
            let progress = elapsed / duration;
            if progress < 0.5 {
                // Fade in
                progress * 2.0
            } else {
                // Fade out
                (1.0 - progress) * 2.0
            }
        }
    }

    /// Initialize or reset a counter for count-up animation
    pub fn init_counter(&mut self, name: &str, target: u64) {
        if self.animation_config.is_count_up_enabled() && target > 0 {
            let counter = AnimatedCounter::new(target);
            self.animated_counters.insert(name.to_string(), counter);
        }
    }

    /// Get current counter value (returns target if no animation)
    pub fn get_counter_value(&self, name: &str, target: u64) -> u64 {
        if !self.animation_config.is_count_up_enabled() {
            return target;
        }

        self.animated_counters
            .get(name)
            .map(|c| c.current())
            .unwrap_or(target)
    }

    /// Get live spinner character for status bar
    pub fn get_spinner_char(&self) -> char {
        if self.animation_config.is_spinner_enabled() {
            self.live_spinner.current_char()
        } else {
            '●'
        }
    }

    fn next_panel(&mut self) {
        self.selected_panel = self.selected_panel.next();
    }

    fn prev_panel(&mut self) {
        self.selected_panel = self.selected_panel.prev();
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    fn toggle_mini_map(&mut self) {
        self.show_mini_map = !self.show_mini_map;
    }

    fn jump_to_panel(&mut self, panel_index: usize) {
        if panel_index > 0 && panel_index <= Panel::VALUES.len() {
            self.selected_panel = Panel::VALUES[panel_index - 1];
            // Auto-close mini-map when jumping to a panel
            self.show_mini_map = false;
        }
    }

    fn toggle_zoom(&mut self) {
        // Toggle zoom for the currently selected panel
        if self.zoom_panel == Some(self.selected_panel) {
            // Already zoomed on this panel - exit zoom
            self.zoom_panel = None;
        } else {
            // Zoom into the selected panel
            self.zoom_panel = Some(self.selected_panel);
            // Auto-close mini-map and help when zooming
            self.show_mini_map = false;
            self.show_help = false;
        }
    }

    fn exit_zoom(&mut self) {
        self.zoom_panel = None;
    }

    fn is_zoomed(&self) -> bool {
        self.zoom_panel.is_some()
    }

    fn cycle_star_timeframe_forward(&mut self) {
        self.star_timeframe = self.star_timeframe.next();
    }

    fn cycle_star_timeframe_backward(&mut self) {
        self.star_timeframe = self.star_timeframe.prev();
    }

    fn cycle_velocity_timeframe_forward(&mut self) {
        self.velocity_timeframe = self.velocity_timeframe.next();
    }

    fn cycle_velocity_timeframe_backward(&mut self) {
        self.velocity_timeframe = self.velocity_timeframe.prev();
    }

    fn cycle_contributors_limit_forward(&mut self) {
        self.contributors_limit = self.contributors_limit.next();
    }

    fn cycle_contributors_limit_backward(&mut self) {
        self.contributors_limit = self.contributors_limit.prev();
    }

    fn cycle_releases_limit_forward(&mut self) {
        self.releases_limit = self.releases_limit.next();
    }

    fn cycle_releases_limit_backward(&mut self) {
        self.releases_limit = self.releases_limit.prev();
    }

    fn cycle_issues_per_page_forward(&mut self) {
        self.issues_per_page = match self.issues_per_page {
            10 => 15,
            15 => 25,
            25 => 50,
            _ => 15,
        };
    }

    fn cycle_issues_per_page_backward(&mut self) {
        self.issues_per_page = match self.issues_per_page {
            10 => 50,
            15 => 10,
            25 => 15,
            50 => 25,
            _ => 15,
        };
    }

    fn cycle_prs_per_page_forward(&mut self) {
        self.prs_per_page = match self.prs_per_page {
            10 => 15,
            15 => 25,
            25 => 50,
            _ => 15,
        };
    }

    fn cycle_prs_per_page_backward(&mut self) {
        self.prs_per_page = match self.prs_per_page {
            10 => 50,
            15 => 10,
            25 => 15,
            50 => 25,
            _ => 15,
        };
    }

    fn cycle_issues_sort(&mut self) {
        self.issues_sort = self.issues_sort.next();
        // Reset scroll when sort changes to show the top of the newly sorted list
        self.zoom_issues_scroll = 0;
    }

    fn scroll_down(&mut self) {
        if let Some(zoom_panel) = self.zoom_panel {
            // When zoomed, scroll the zoom-specific offset
            match zoom_panel {
                Panel::Issues => {
                    self.zoom_issues_scroll = self.zoom_issues_scroll.saturating_add(1)
                }
                Panel::Contributors => {
                    self.zoom_contributors_scroll = self.zoom_contributors_scroll.saturating_add(1)
                }
                Panel::Releases => {
                    self.zoom_releases_scroll = self.zoom_releases_scroll.saturating_add(1)
                }
                Panel::Stars => self.zoom_stars_scroll = self.zoom_stars_scroll.saturating_add(1),
                Panel::PullRequests => {
                    self.zoom_prs_scroll = self.zoom_prs_scroll.saturating_add(1)
                }
                _ => {}
            }
        } else {
            // Normal mode - scroll the selected panel
            match self.selected_panel {
                Panel::Issues => self.issues_scroll = self.issues_scroll.saturating_add(1),
                Panel::Contributors => {
                    self.contributors_scroll = self.contributors_scroll.saturating_add(1)
                }
                Panel::Releases => self.releases_scroll = self.releases_scroll.saturating_add(1),
                _ => {}
            }
        }
    }

    fn scroll_up(&mut self) {
        if let Some(zoom_panel) = self.zoom_panel {
            // When zoomed, scroll the zoom-specific offset
            match zoom_panel {
                Panel::Issues => {
                    self.zoom_issues_scroll = self.zoom_issues_scroll.saturating_sub(1)
                }
                Panel::Contributors => {
                    self.zoom_contributors_scroll = self.zoom_contributors_scroll.saturating_sub(1)
                }
                Panel::Releases => {
                    self.zoom_releases_scroll = self.zoom_releases_scroll.saturating_sub(1)
                }
                Panel::Stars => self.zoom_stars_scroll = self.zoom_stars_scroll.saturating_sub(1),
                Panel::PullRequests => {
                    self.zoom_prs_scroll = self.zoom_prs_scroll.saturating_sub(1)
                }
                _ => {}
            }
        } else {
            // Normal mode - scroll the selected panel
            match self.selected_panel {
                Panel::Issues => self.issues_scroll = self.issues_scroll.saturating_sub(1),
                Panel::Contributors => {
                    self.contributors_scroll = self.contributors_scroll.saturating_sub(1)
                }
                Panel::Releases => self.releases_scroll = self.releases_scroll.saturating_sub(1),
                _ => {}
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        // When in search mode, reserve space for search modal at bottom
        let has_search_modal = self.search_mode;
        let status_height = if has_search_modal { 3 } else { 1 };

        let [header_area, content_area, status_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(status_height),
        ])
        .areas(frame.area());

        self.render_header(frame, header_area);
        self.render_content(frame, content_area);
        self.render_status_bar(frame, status_area);

        if self.show_help {
            self.render_help_overlay(frame);
        }

        if self.show_mini_map {
            self.render_mini_map_overlay(frame);
        }

        // Render zoom overlay on top of everything if zoomed
        if let Some(zoom_panel) = self.zoom_panel {
            self.render_zoom_overlay(frame, zoom_panel);
        }

        // Render search modal on top of everything if in search mode
        if self.search_mode {
            self.render_search_modal(frame);
        }

        // Render fuzzy finder overlay on top of everything if in fuzzy mode
        if self.fuzzy_mode {
            self.render_fuzzy_overlay(frame);
        }

        // Render diff mode overlay on top of everything if in diff mode
        if self.diff_mode {
            self.render_diff_overlay(frame);
        }

        // Render compare mode overlay on top of everything if in compare mode
        if self.compare_mode {
            self.render_compare_overlay(frame);
        }

        // Render command palette on top of everything if in command palette mode
        if self.command_palette_mode {
            self.render_command_palette(frame);
        }

        // Render token input dialog on top of everything if in token input mode
        if self.token_input_mode {
            self.render_token_input(frame);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let desc = self
            .snapshot
            .as_ref()
            .and_then(|s| s.repo.description.as_deref())
            .unwrap_or("");

        let lang = self
            .snapshot
            .as_ref()
            .and_then(|s| s.repo.language.as_deref())
            .unwrap_or("");

        let header_text = match (desc.is_empty(), lang.is_empty()) {
            (false, false) => format!("{}/{} — {} [{}]", self.owner, self.repo, desc, lang),
            (false, true) => format!("{}/{} — {}", self.owner, self.repo, desc),
            _ => format!("{}/{}", self.owner, self.repo),
        };

        let block = Block::bordered()
            .title(" forgeStat ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.header_border_color()));

        let paragraph = Paragraph::new(header_text)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect) {
        // Use configurable layout from layout.toml
        let [row1, row2, row3] = Layout::vertical(self.layout_config.row_heights()).areas(area);

        let [stars_area, issues_area] =
            Layout::horizontal(self.layout_config.row1_widths()).areas(row1);

        let [prs_area, contrib_area, releases_area] =
            Layout::horizontal(self.layout_config.row2_widths()).areas(row2);

        let [velocity_area, security_area, ci_area] =
            Layout::horizontal(self.layout_config.row3_widths()).areas(row3);

        // Store areas for mouse click detection
        self.panel_areas[0] = Some(stars_area);
        self.panel_areas[1] = Some(issues_area);
        self.panel_areas[2] = Some(prs_area);
        self.panel_areas[3] = Some(contrib_area);
        self.panel_areas[4] = Some(releases_area);
        self.panel_areas[5] = Some(velocity_area);
        self.panel_areas[6] = Some(security_area);
        self.panel_areas[7] = Some(ci_area);

        // Calculate and store border areas for resize detection
        // Vertical borders (5 total): row1 has 1, row2 has 2, row3 has 2
        const BORDER_GRAB_WIDTH: u16 = 2; // Width of the grab area on each side of the border

        // Row 1 vertical border: between stars and issues
        if stars_area.right() > 0 && issues_area.left() < area.width {
            let border_x = stars_area.right().min(issues_area.left());
            self.vertical_borders[0] = Some(Rect::new(
                border_x.saturating_sub(BORDER_GRAB_WIDTH),
                row1.y,
                BORDER_GRAB_WIDTH * 2,
                row1.height,
            ));
        }

        // Row 2 vertical borders: between prs/contributors and contributors/releases
        if prs_area.right() > 0 && contrib_area.left() < area.width {
            let border_x1 = prs_area.right().min(contrib_area.left());
            self.vertical_borders[1] = Some(Rect::new(
                border_x1.saturating_sub(BORDER_GRAB_WIDTH),
                row2.y,
                BORDER_GRAB_WIDTH * 2,
                row2.height,
            ));
        }
        if contrib_area.right() > 0 && releases_area.left() < area.width {
            let border_x2 = contrib_area.right().min(releases_area.left());
            self.vertical_borders[2] = Some(Rect::new(
                border_x2.saturating_sub(BORDER_GRAB_WIDTH),
                row2.y,
                BORDER_GRAB_WIDTH * 2,
                row2.height,
            ));
        }

        // Row 3 vertical borders: between velocity/security and security/ci
        if velocity_area.right() > 0 && security_area.left() < area.width {
            let border_x1 = velocity_area.right().min(security_area.left());
            self.vertical_borders[3] = Some(Rect::new(
                border_x1.saturating_sub(BORDER_GRAB_WIDTH),
                row3.y,
                BORDER_GRAB_WIDTH * 2,
                row3.height,
            ));
        }
        if security_area.right() > 0 && ci_area.left() < area.width {
            let border_x2 = security_area.right().min(ci_area.left());
            self.vertical_borders[4] = Some(Rect::new(
                border_x2.saturating_sub(BORDER_GRAB_WIDTH),
                row3.y,
                BORDER_GRAB_WIDTH * 2,
                row3.height,
            ));
        }

        // Horizontal borders (2 total): between rows
        const BORDER_GRAB_HEIGHT: u16 = 1; // Height of the grab area on each side of the border

        // Between row1 and row2
        if row1.bottom() > 0 && row2.top() < area.height {
            let border_y1 = row1.bottom().min(row2.top());
            self.horizontal_borders[0] = Some(Rect::new(
                area.x,
                border_y1.saturating_sub(BORDER_GRAB_HEIGHT),
                area.width,
                BORDER_GRAB_HEIGHT * 2 + 1, // Slightly taller for horizontal borders
            ));
        }

        // Between row2 and row3
        if row2.bottom() > 0 && row3.top() < area.height {
            let border_y2 = row2.bottom().min(row3.top());
            self.horizontal_borders[1] = Some(Rect::new(
                area.x,
                border_y2.saturating_sub(BORDER_GRAB_HEIGHT),
                area.width,
                BORDER_GRAB_HEIGHT * 2 + 1,
            ));
        }

        self.render_stars(frame, stars_area);
        self.render_issues(frame, issues_area);
        self.render_prs(frame, prs_area);
        self.render_contributors(frame, contrib_area);
        self.render_releases(frame, releases_area);
        self.render_velocity(frame, velocity_area);
        self.render_security(frame, security_area);
        self.render_ci(frame, ci_area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        // Check if toast should be displayed
        if let Some(toast_msg) = self.get_toast_message() {
            let toast = Paragraph::new(Line::from(vec![
                Span::styled(
                    "✓ ",
                    Style::default()
                        .fg(self.theme.indicator_success_color())
                        .bold(),
                ),
                Span::styled(
                    toast_msg,
                    Style::default()
                        .fg(self.theme.text_highlight_color())
                        .bold(),
                ),
            ]));
            frame.render_widget(toast, area);
            return;
        }

        let mut spans: Vec<Span> = Vec::new();
        let mut first_item = true;

        // Render configured status bar items (max 3)
        for item in &self.statusbar_config.items {
            if !first_item {
                spans.push(Span::styled(
                    " | ",
                    Style::default().fg(self.theme.text_secondary_color()),
                ));
            }
            first_item = false;

            match item {
                StatusBarItem::SyncState => {
                    let (text, style) = match self.sync_state {
                        SyncState::Live => {
                            let mins = self
                                .snapshot
                                .as_ref()
                                .map(|s| {
                                    Utc::now().signed_duration_since(s.fetched_at).num_minutes()
                                })
                                .unwrap_or(0);
                            // Add spinner character for live state when animations enabled
                            let spinner = if self.animation_config.is_spinner_enabled() {
                                format!("{} ", self.get_spinner_char())
                            } else {
                                String::new()
                            };
                            (
                                format!("{}LIVE {}m", spinner, mins),
                                Style::default().fg(self.theme.status_live_color()),
                            )
                        }
                        SyncState::Stale => {
                            let mins = self
                                .snapshot
                                .as_ref()
                                .map(|s| {
                                    Utc::now().signed_duration_since(s.fetched_at).num_minutes()
                                })
                                .unwrap_or(0);
                            (
                                format!("STALE {}m", mins),
                                Style::default().fg(self.theme.status_stale_color()),
                            )
                        }
                        SyncState::Offline => (
                            "OFFLINE".to_string(),
                            Style::default().fg(self.theme.status_offline_color()),
                        ),
                    };
                    spans.push(Span::styled(text, style));
                }
                StatusBarItem::RateLimit => {
                    if let Some(ref rl) = self.rate_limit {
                        let color = if rl.remaining < 10 {
                            self.theme.indicator_error_color()
                        } else if rl.remaining < rl.limit / 10 {
                            self.theme.indicator_warning_color()
                        } else {
                            self.theme.text_secondary_color()
                        };
                        spans.push(Span::styled(
                            format!("API: {}/{}", rl.remaining, rl.limit),
                            Style::default().fg(color),
                        ));
                    } else {
                        spans.push(Span::styled(
                            "API: --/--",
                            Style::default().fg(self.theme.text_secondary_color()),
                        ));
                    }
                }
                StatusBarItem::OpenIssues => {
                    let count = self
                        .snapshot
                        .as_ref()
                        .map(|s| s.open_issues_count())
                        .unwrap_or(0);
                    spans.push(Span::styled(
                        format!("Issues: {}", count),
                        Style::default().fg(self.theme.text_primary_color()),
                    ));
                }
                StatusBarItem::OpenPrs => {
                    let count = self
                        .snapshot
                        .as_ref()
                        .map(|s| s.open_prs_count())
                        .unwrap_or(0);
                    spans.push(Span::styled(
                        format!("PRs: {}", count),
                        Style::default().fg(self.theme.text_primary_color()),
                    ));
                }
                StatusBarItem::LastReleaseAge => {
                    if let Some(days) = self
                        .snapshot
                        .as_ref()
                        .and_then(|s| s.days_since_last_release())
                    {
                        spans.push(Span::styled(
                            format!("Release: {}d", days),
                            Style::default().fg(self.theme.text_primary_color()),
                        ));
                    } else {
                        spans.push(Span::styled(
                            "Release: N/A",
                            Style::default().fg(self.theme.text_secondary_color()),
                        ));
                    }
                }
                StatusBarItem::OldestIssueAge => {
                    if let Some(days) = self
                        .snapshot
                        .as_ref()
                        .and_then(|s| s.oldest_issue_age_days())
                    {
                        spans.push(Span::styled(
                            format!("Oldest: {}d", days),
                            Style::default().fg(self.theme.text_primary_color()),
                        ));
                    } else {
                        spans.push(Span::styled(
                            "Oldest: N/A",
                            Style::default().fg(self.theme.text_secondary_color()),
                        ));
                    }
                }
                StatusBarItem::HealthScore => {
                    if let Some(ref health) = self.health_score {
                        let color = match health.grade {
                            crate::core::health::HealthGrade::Excellent => {
                                self.theme.indicator_success_color()
                            }
                            crate::core::health::HealthGrade::Good => {
                                self.theme.text_highlight_color()
                            }
                            crate::core::health::HealthGrade::Fair => {
                                self.theme.indicator_warning_color()
                            }
                            crate::core::health::HealthGrade::NeedsAttention => {
                                self.theme.indicator_warning_color()
                            }
                            crate::core::health::HealthGrade::Critical => {
                                self.theme.indicator_error_color()
                            }
                        };
                        spans.push(Span::styled(
                            format!(
                                "Health: {}/100 ({})",
                                health.total,
                                health.grade.as_letter()
                            ),
                            Style::default().fg(color).bold(),
                        ));
                    } else {
                        spans.push(Span::styled(
                            "Health: N/A",
                            Style::default().fg(self.theme.text_secondary_color()),
                        ));
                    }
                }
            }
        }

        // Add separator and contextual action hints
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }

        // Get panel name and context-aware hints
        let panel_name = self.selected_panel.display_name();
        let context_hints = self.get_context_hints();

        // Add panel name (bold highlight)
        spans.push(Span::styled(
            panel_name,
            Style::default()
                .fg(self.theme.text_highlight_color())
                .bold(),
        ));

        // Add hints with theme colors
        for hint in context_hints {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                hint.key,
                Style::default()
                    .fg(self.theme.text_highlight_color())
                    .bold(),
            ));
            spans.push(Span::styled(
                format!(" {}", hint.description),
                Style::default().fg(self.theme.text_secondary_color()),
            ));
        }

        // Add global shortcuts hint
        spans.push(Span::styled(
            "  Tab/←/→:cycle  ?:help  q:quit",
            Style::default().fg(self.theme.text_secondary_color()),
        ));

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Get context-aware action hints based on the currently selected panel and app state
    fn get_context_hints(&self) -> Vec<ActionHint<'_>> {
        let mut hints = Vec::new();

        // Fuzzy mode indicator (if active)
        if self.fuzzy_mode {
            hints.push(ActionHint {
                key: "Esc",
                description: "close",
            });
            hints.push(ActionHint {
                key: "Enter",
                description: "select repo",
            });
            hints.push(ActionHint {
                key: "↑/↓",
                description: "navigate",
            });
            return hints; // When in fuzzy mode, only show fuzzy-specific hints
        }

        // Command palette mode indicator (if active)
        if self.command_palette_mode {
            hints.push(ActionHint {
                key: "Esc",
                description: "close",
            });
            hints.push(ActionHint {
                key: "Enter",
                description: "execute",
            });
            hints.push(ActionHint {
                key: "Tab",
                description: "complete",
            });
            hints.push(ActionHint {
                key: "↑/↓",
                description: "select",
            });
            hints.push(ActionHint {
                key: "Ctrl+↑/↓",
                description: "history",
            });
            return hints; // When in command palette mode, only show palette-specific hints
        }

        // Zoom mode indicator (if active)
        if let Some(ref zoom_panel) = self.zoom_panel {
            hints.push(ActionHint {
                key: "Enter/Esc",
                description: "exit zoom",
            });
            hints.push(ActionHint {
                key: "↑/↓",
                description: "scroll",
            });
            // Add copy hint based on zoomed panel type
            match zoom_panel {
                Panel::Issues => {
                    hints.push(ActionHint {
                        key: "c",
                        description: "copy all",
                    });
                    hints.push(ActionHint {
                        key: "s",
                        description: "sort",
                    });
                }
                Panel::Contributors => {
                    hints.push(ActionHint {
                        key: "c",
                        description: "copy user",
                    });
                }
                Panel::Releases => {
                    hints.push(ActionHint {
                        key: "c",
                        description: "copy tag",
                    });
                }
                _ => {
                    hints.push(ActionHint {
                        key: "c",
                        description: "copy repo",
                    });
                }
            }
            return hints; // When zoomed, only show zoom-specific hints
        }

        // Mini-map mode indicator
        if self.show_mini_map {
            hints.push(ActionHint {
                key: "m",
                description: "close map",
            });
            hints.push(ActionHint {
                key: "1-8",
                description: "jump",
            });
            return hints;
        }

        // Panel-specific hints
        match self.selected_panel {
            Panel::Stars => {
                hints.push(ActionHint {
                    key: "+/-",
                    description: "timeframe",
                });
                hints.push(ActionHint {
                    key: "c",
                    description: "copy repo",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::Issues => {
                hints.push(ActionHint {
                    key: "/",
                    description: "search",
                });
                hints.push(ActionHint {
                    key: "l",
                    description: "filter",
                });
                hints.push(ActionHint {
                    key: "c",
                    description: "copy issue",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::PullRequests => {
                hints.push(ActionHint {
                    key: "c",
                    description: "copy repo",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::Contributors => {
                hints.push(ActionHint {
                    key: "/",
                    description: "search",
                });
                hints.push(ActionHint {
                    key: "c",
                    description: "copy user",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::Releases => {
                hints.push(ActionHint {
                    key: "/",
                    description: "search",
                });
                hints.push(ActionHint {
                    key: "p",
                    description: "filter",
                });
                hints.push(ActionHint {
                    key: "c",
                    description: "copy tag",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::Velocity => {
                hints.push(ActionHint {
                    key: "c",
                    description: "copy repo",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::Security => {
                hints.push(ActionHint {
                    key: "c",
                    description: "copy repo",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
            Panel::CI => {
                hints.push(ActionHint {
                    key: "c",
                    description: "copy repo",
                });
                hints.push(ActionHint {
                    key: "r",
                    description: "refresh",
                });
            }
        }

        // Add zoom and scroll hints (available in normal mode for scrollable panels)
        if !hints.iter().any(|h| h.key == "Enter") {
            hints.push(ActionHint {
                key: "Enter",
                description: "zoom",
            });
        }

        // Add fuzzy finder hint
        hints.push(ActionHint {
            key: "f",
            description: "find repo",
        });

        hints.push(ActionHint {
            key: ":",
            description: "commands",
        });

        // Add scroll hint for scrollable panels
        match self.selected_panel {
            Panel::Issues | Panel::Contributors | Panel::Releases => {
                if !hints.iter().any(|h| h.key == "↑/↓") {
                    hints.push(ActionHint {
                        key: "↑/↓",
                        description: "scroll",
                    });
                }
            }
            _ => {}
        }

        hints
    }

    /// Render the search modal at the bottom of the screen
    fn render_search_modal(&self, frame: &mut Frame) {
        let area = frame.area();
        // Place search modal at bottom, spanning full width
        let search_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(3),
            width: area.width,
            height: 3,
        };

        frame.render_widget(Clear, search_area);

        let panel_name = self.selected_panel.display_name();
        let search_prompt = format!("Search {}: ", panel_name);

        let search_text = format!("{}{}", search_prompt, self.search_query);

        // Add a blinking cursor indicator (using block character)
        let text_with_cursor = format!("{}█", search_text);

        let block = Block::bordered()
            .title(" Search ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let paragraph = Paragraph::new(text_with_cursor)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, search_area);
    }

    /// Toggle search mode on/off
    fn toggle_search(&mut self) {
        match self.selected_panel {
            Panel::Issues | Panel::Contributors | Panel::Releases => {
                self.search_mode = !self.search_mode;
                if self.search_mode {
                    self.search_query.clear();
                }
            }
            _ => {}
        }
    }

    /// Clear search and exit search mode
    fn clear_search(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
    }

    /// Exit search mode but keep query (for ESC while keeping filter)
    fn exit_search_mode(&mut self) {
        self.search_mode = false;
    }

    /// Add character to search query and reset scroll positions
    fn add_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.reset_search_scrolls();
    }

    /// Remove last character from search query and reset scroll positions
    fn backspace_search(&mut self) {
        self.search_query.pop();
        self.reset_search_scrolls();
    }

    /// Reset scroll positions when search changes to prevent "empty list" bug
    fn reset_search_scrolls(&mut self) {
        // Reset normal view scrolls
        self.issues_scroll = 0;
        self.contributors_scroll = 0;
        self.releases_scroll = 0;
        // Reset zoom view scrolls
        self.zoom_issues_scroll = 0;
        self.zoom_contributors_scroll = 0;
        self.zoom_releases_scroll = 0;
    }

    /// Cycle through label filters for Issues panel
    fn cycle_issues_label_filter(&mut self) {
        if let Some(ref snap) = self.snapshot {
            let mut labels: Vec<&str> = snap.issues.by_label.keys().map(|s| s.as_str()).collect();
            labels.sort();
            labels.insert(0, "all"); // Add "all" at the beginning

            let current = self.issues_label_filter.as_deref().unwrap_or("all");
            let current_idx = labels.iter().position(|&l| l == current).unwrap_or(0);
            let next_idx = (current_idx + 1) % labels.len();

            self.issues_label_filter = if labels[next_idx] == "all" {
                None
            } else {
                Some(labels[next_idx].to_string())
            };
            // Reset scroll when filter changes
            self.issues_scroll = 0;
            self.zoom_issues_scroll = 0;
        }
    }

    /// Cycle through prerelease filters for Releases panel
    fn cycle_releases_prerelease_filter(&mut self) {
        self.releases_prerelease_filter = match self.releases_prerelease_filter {
            None => Some(false),       // All -> Stable only
            Some(false) => Some(true), // Stable -> Prerelease only
            Some(true) => None,        // Prerelease -> All
        };
        // Reset scroll when filter changes
        self.releases_scroll = 0;
        self.zoom_releases_scroll = 0;
    }

    /// Get filtered issues based on search query and label filter
    fn get_filtered_issues(&self) -> Vec<&Issue> {
        let Some(ref snap) = self.snapshot else {
            return Vec::new();
        };

        // Collect all issues first (deduplicate by issue number since
        // issues can appear multiple times if they have multiple labels)
        let mut all_issues: Vec<&Issue> = Vec::new();
        let mut seen_numbers: HashSet<u64> = HashSet::new();
        for issues in snap.issues.by_label.values() {
            for issue in issues.iter() {
                if seen_numbers.insert(issue.number) {
                    all_issues.push(issue);
                }
            }
        }
        // Unlabelled issues are already unique, add them directly
        all_issues.extend(snap.issues.unlabelled.iter());

        // Apply label filter first
        let label_filtered: Vec<&Issue> = if let Some(ref label) = self.issues_label_filter {
            all_issues
                .into_iter()
                .filter(|i| i.labels.contains(label))
                .collect()
        } else {
            all_issues
        };

        // Then apply search query filter (title search)
        if self.search_query.is_empty() {
            label_filtered
        } else {
            let query = self.search_query.to_lowercase();
            label_filtered
                .into_iter()
                .filter(|i| i.title.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Get filtered contributors based on search query
    fn get_filtered_contributors(&self) -> Vec<&Contributor> {
        let Some(ref snap) = self.snapshot else {
            return Vec::new();
        };

        if self.search_query.is_empty() {
            snap.contributors.top_contributors.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            snap.contributors
                .top_contributors
                .iter()
                .filter(|c| c.username.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Get filtered releases based on search query and prerelease filter
    fn get_filtered_releases(&self) -> Vec<&Release> {
        let Some(ref snap) = self.snapshot else {
            return Vec::new();
        };

        // Apply prerelease filter first
        let prerelease_filtered: Vec<&Release> = match self.releases_prerelease_filter {
            Some(true) => snap.releases.iter().filter(|r| r.prerelease).collect(),
            Some(false) => snap.releases.iter().filter(|r| !r.prerelease).collect(),
            None => snap.releases.iter().collect(),
        };

        // Then apply search query filter (tag name search)
        if self.search_query.is_empty() {
            prerelease_filtered
        } else {
            let query = self.search_query.to_lowercase();
            prerelease_filtered
                .into_iter()
                .filter(|r| r.tag_name.to_lowercase().contains(&query))
                .collect()
        }
    }
}

pub use event_loop::run_event_loop;
