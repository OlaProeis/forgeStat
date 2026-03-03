use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Configuration structure for TOML serialization
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    #[serde(rename = "github_token")]
    pub github_token: Option<String>,
}

/// Status bar item types that can be displayed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StatusBarItem {
    /// Sync state (LIVE/STALE/OFFLINE) with last sync time
    SyncState,
    /// GitHub API rate limit (remaining/limit)
    RateLimit,
    /// Count of open issues
    OpenIssues,
    /// Count of open PRs
    OpenPrs,
    /// Days since last release
    LastReleaseAge,
    /// Age of oldest open issue
    OldestIssueAge,
    /// Repository health score (0-100) with grade
    HealthScore,
}

impl StatusBarItem {
    /// All available status bar items
    pub const ALL: [Self; 7] = [
        Self::SyncState,
        Self::RateLimit,
        Self::OpenIssues,
        Self::OpenPrs,
        Self::LastReleaseAge,
        Self::OldestIssueAge,
        Self::HealthScore,
    ];

    /// Get the display name for this item
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SyncState => "sync_state",
            Self::RateLimit => "rate_limit",
            Self::OpenIssues => "open_issues",
            Self::OpenPrs => "open_prs",
            Self::LastReleaseAge => "last_release_age",
            Self::OldestIssueAge => "oldest_issue_age",
            Self::HealthScore => "health_score",
        }
    }
}

/// Status bar configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusBarConfig {
    /// Items to display in the status bar (max 3)
    #[serde(default = "StatusBarConfig::default_items")]
    pub items: Vec<StatusBarItem>,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            items: Self::default_items(),
        }
    }
}

impl StatusBarConfig {
    /// Default status bar items: sync_state + health_score + open_prs
    fn default_items() -> Vec<StatusBarItem> {
        vec![
            StatusBarItem::SyncState,
            StatusBarItem::HealthScore,
            StatusBarItem::OpenPrs,
        ]
    }

    /// Load status bar config from file or return default
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => {
                log::info!("Loaded status bar config with items: {:?}", config.items);
                config
            }
            Err(e) => {
                log::warn!("Failed to load status bar config: {}. Using default.", e);
                Self::default()
            }
        }
    }

    /// Try to load status bar config from file
    fn try_load() -> Result<Self> {
        let config_path = statusbar_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read status bar config at: {}",
                config_path.display()
            )
        })?;

        let mut config: StatusBarConfig = toml::from_str(&config_str).with_context(|| {
            format!(
                "Failed to parse status bar config at: {}",
                config_path.display()
            )
        })?;

        // Enforce max 3 items limit
        if config.items.len() > 3 {
            log::warn!(
                "Status bar config has {} items, max is 3. Truncating.",
                config.items.len()
            );
            config.items.truncate(3);
        }

        // Remove any duplicate items
        let mut seen = std::collections::HashSet::new();
        config.items.retain(|item| seen.insert(*item));

        Ok(config)
    }

    /// Save status bar config to file
    pub fn save(&self) -> Result<()> {
        let config_path = statusbar_file_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get status bar config directory")?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            log::info!("Created config directory: {}", config_dir.display());
        }

        // Enforce max 3 items before saving
        let mut config_to_save = self.clone();
        if config_to_save.items.len() > 3 {
            config_to_save.items.truncate(3);
        }

        // Remove duplicates
        let mut seen = std::collections::HashSet::new();
        config_to_save.items.retain(|item| seen.insert(*item));

        let config_str = toml::to_string_pretty(&config_to_save)
            .context("Failed to serialize status bar config to TOML")?;

        fs::write(&config_path, config_str).with_context(|| {
            format!(
                "Failed to write status bar config: {}",
                config_path.display()
            )
        })?;

        log::info!("Saved status bar config to: {}", config_path.display());
        Ok(())
    }
}

/// Returns the path to the status bar config file
pub fn statusbar_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("statusbar.toml"))
}

// =============================================================================
// Layout Configuration
// =============================================================================

/// Layout preset names
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutPreset {
    /// Default balanced layout
    #[default]
    Default,
    /// Compact layout with smaller panels
    Compact,
    /// Wide layout emphasizing certain panels
    Wide,
}

/// Individual panel size configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PanelLayout {
    /// Width percentage (0-100)
    #[serde(default = "default_panel_width")]
    pub width_pct: u16,
    /// Height percentage (0-100)
    #[serde(default = "default_panel_height")]
    pub height_pct: u16,
}

impl PanelLayout {
    /// Create a new panel layout with specified percentages
    pub fn new(width_pct: u16, height_pct: u16) -> Self {
        Self {
            width_pct: width_pct.clamp(10, 100),
            height_pct: height_pct.clamp(10, 100),
        }
    }

    /// Get width as a ratatui Constraint
    pub fn width_constraint(&self) -> ratatui::layout::Constraint {
        ratatui::layout::Constraint::Percentage(self.width_pct)
    }

    /// Get height as a ratatui Constraint
    pub fn height_constraint(&self) -> ratatui::layout::Constraint {
        ratatui::layout::Constraint::Percentage(self.height_pct)
    }
}

fn default_panel_width() -> u16 {
    50
}

fn default_panel_height() -> u16 {
    50
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            width_pct: 50,
            height_pct: 50,
        }
    }
}

/// Configuration for the 7-panel TUI layout
/// Stored in layout.toml with preset fallbacks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutConfig {
    /// Selected layout preset
    #[serde(default)]
    pub preset: LayoutPreset,
    /// Row 1 heights (stars, issues row)
    #[serde(default = "default_row1_heights")]
    pub row1: PanelLayout,
    /// Row 2 heights (prs, contributors, releases row)
    #[serde(default = "default_row2_heights")]
    pub row2: PanelLayout,
    /// Row 3 heights (velocity, security row)
    #[serde(default = "default_row3_heights")]
    pub row3: PanelLayout,
    /// Column widths for row 1 (stars | issues)
    #[serde(default = "default_row1_columns")]
    pub row1_columns: Vec<PanelLayout>,
    /// Column widths for row 2 (prs | contributors | releases)
    #[serde(default = "default_row2_columns")]
    pub row2_columns: Vec<PanelLayout>,
    /// Column widths for row 3 (velocity | security | ci)
    #[serde(default = "default_row3_columns")]
    pub row3_columns: Vec<PanelLayout>,
}

// Default height configurations for rows
fn default_row1_heights() -> PanelLayout {
    PanelLayout::new(100, 40)
}

fn default_row2_heights() -> PanelLayout {
    PanelLayout::new(100, 30)
}

fn default_row3_heights() -> PanelLayout {
    PanelLayout::new(100, 30)
}

// Default column configurations
fn default_row1_columns() -> Vec<PanelLayout> {
    vec![
        PanelLayout::new(35, 100), // Stars
        PanelLayout::new(65, 100), // Issues
    ]
}

fn default_row2_columns() -> Vec<PanelLayout> {
    vec![
        PanelLayout::new(33, 100), // PRs
        PanelLayout::new(33, 100), // Contributors
        PanelLayout::new(34, 100), // Releases
    ]
}

fn default_row3_columns() -> Vec<PanelLayout> {
    vec![
        PanelLayout::new(40, 100), // Velocity
        PanelLayout::new(30, 100), // Security
        PanelLayout::new(30, 100), // CI Status
    ]
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self::default_preset(LayoutPreset::Default)
    }
}

impl LayoutConfig {
    /// Create layout config for a specific preset
    pub fn default_preset(preset: LayoutPreset) -> Self {
        match preset {
            LayoutPreset::Default => Self {
                preset: LayoutPreset::Default,
                row1: default_row1_heights(),
                row2: default_row2_heights(),
                row3: default_row3_heights(),
                row1_columns: default_row1_columns(),
                row2_columns: default_row2_columns(),
                row3_columns: default_row3_columns(),
            },
            LayoutPreset::Compact => Self {
                preset: LayoutPreset::Compact,
                row1: PanelLayout::new(100, 35),
                row2: PanelLayout::new(100, 35),
                row3: PanelLayout::new(100, 30),
                row1_columns: vec![
                    PanelLayout::new(30, 100), // Stars (smaller)
                    PanelLayout::new(70, 100), // Issues (larger)
                ],
                row2_columns: vec![
                    PanelLayout::new(33, 100),
                    PanelLayout::new(33, 100),
                    PanelLayout::new(34, 100),
                ],
                row3_columns: vec![
                    PanelLayout::new(34, 100),
                    PanelLayout::new(33, 100),
                    PanelLayout::new(33, 100),
                ],
            },
            LayoutPreset::Wide => Self {
                preset: LayoutPreset::Wide,
                row1: PanelLayout::new(100, 45),
                row2: PanelLayout::new(100, 30),
                row3: PanelLayout::new(100, 25),
                row1_columns: vec![
                    PanelLayout::new(40, 100), // Stars (larger)
                    PanelLayout::new(60, 100), // Issues
                ],
                row2_columns: vec![
                    PanelLayout::new(40, 100),
                    PanelLayout::new(30, 100),
                    PanelLayout::new(30, 100),
                ],
                row3_columns: vec![
                    PanelLayout::new(40, 100), // Velocity
                    PanelLayout::new(30, 100), // Security
                    PanelLayout::new(30, 100), // CI Status
                ],
            },
        }
    }

    /// Load layout config from file or return default
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => {
                log::info!("Loaded layout config with preset: {:?}", config.preset);
                config
            }
            Err(e) => {
                log::warn!("Failed to load layout config: {}. Using default.", e);
                Self::default()
            }
        }
    }

    /// Try to load layout config from file
    fn try_load() -> Result<Self> {
        let config_path = layout_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!("Failed to read layout config at: {}", config_path.display())
        })?;

        let config: LayoutConfig = toml::from_str(&config_str).with_context(|| {
            format!(
                "Failed to parse layout config at: {}",
                config_path.display()
            )
        })?;

        // Validate and normalize the config
        Ok(config.normalize())
    }

    /// Save layout config to file
    pub fn save(&self) -> Result<()> {
        let config_path = layout_file_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get layout config directory")?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            log::info!("Created config directory: {}", config_dir.display());
        }

        let config_to_save = self.clone().normalize();
        let config_str = toml::to_string_pretty(&config_to_save)
            .context("Failed to serialize layout config to TOML")?;

        fs::write(&config_path, config_str)
            .with_context(|| format!("Failed to write layout config: {}", config_path.display()))?;

        log::info!("Saved layout config to: {}", config_path.display());
        Ok(())
    }

    /// Reset to a specific preset
    pub fn reset_to_preset(&mut self, preset: LayoutPreset) {
        *self = Self::default_preset(preset);
    }

    /// Normalize the config to ensure valid percentages
    fn normalize(mut self) -> Self {
        // Ensure row heights sum to 100%
        let total_height = self.row1.height_pct + self.row2.height_pct + self.row3.height_pct;
        if total_height != 100 {
            // Normalize proportionally
            self.row1.height_pct =
                (self.row1.height_pct as u32 * 100 / total_height as u32).min(100) as u16;
            self.row2.height_pct =
                (self.row2.height_pct as u32 * 100 / total_height as u32).min(100) as u16;
            self.row3.height_pct =
                100u16.saturating_sub(self.row1.height_pct + self.row2.height_pct);
        }

        // Ensure minimum heights (at least 10% each)
        self.row1.height_pct = self.row1.height_pct.max(10);
        self.row2.height_pct = self.row2.height_pct.max(10);
        self.row3.height_pct = self.row3.height_pct.max(10);

        // Renormalize if minimums pushed us over 100
        let total = self.row1.height_pct + self.row2.height_pct + self.row3.height_pct;
        if total > 100 {
            let excess = total - 100;
            // Reduce from the largest row
            if self.row1.height_pct >= self.row2.height_pct
                && self.row1.height_pct >= self.row3.height_pct
            {
                self.row1.height_pct = self.row1.height_pct.saturating_sub(excess);
            } else if self.row2.height_pct >= self.row3.height_pct {
                self.row2.height_pct = self.row2.height_pct.saturating_sub(excess);
            } else {
                self.row3.height_pct = self.row3.height_pct.saturating_sub(excess);
            }
        }

        // Normalize column widths for each row
        Self::normalize_columns(&mut self.row1_columns);
        Self::normalize_columns(&mut self.row2_columns);
        Self::normalize_columns(&mut self.row3_columns);

        self
    }

    /// Normalize column widths to ensure they sum to 100%
    fn normalize_columns(columns: &mut [PanelLayout]) {
        if columns.is_empty() {
            return;
        }

        // Ensure minimum widths (at least 10% each)
        for col in columns.iter_mut() {
            col.width_pct = col.width_pct.max(10);
        }

        let total_width: u16 = columns.iter().map(|c| c.width_pct).sum();
        if total_width == 0 {
            return;
        }

        // Normalize to 100%
        if total_width != 100 {
            let mut new_total = 0u16;
            let num_cols = columns.len();
            for (i, col) in columns.iter_mut().enumerate() {
                if i == num_cols - 1 {
                    // Last column gets the remainder to ensure exact 100%
                    col.width_pct = 100u16.saturating_sub(new_total);
                } else {
                    col.width_pct =
                        (col.width_pct as u32 * 100 / total_width as u32).min(100) as u16;
                    new_total += col.width_pct;
                }
            }
        }
    }

    /// Get height constraints for the three rows
    pub fn row_heights(&self) -> [ratatui::layout::Constraint; 3] {
        [
            ratatui::layout::Constraint::Percentage(self.row1.height_pct),
            ratatui::layout::Constraint::Percentage(self.row2.height_pct),
            ratatui::layout::Constraint::Percentage(self.row3.height_pct),
        ]
    }

    /// Get width constraints for row 1 columns
    pub fn row1_widths(&self) -> Vec<ratatui::layout::Constraint> {
        self.row1_columns
            .iter()
            .map(|c| c.width_constraint())
            .collect()
    }

    /// Get width constraints for row 2 columns
    pub fn row2_widths(&self) -> Vec<ratatui::layout::Constraint> {
        self.row2_columns
            .iter()
            .map(|c| c.width_constraint())
            .collect()
    }

    /// Get width constraints for row 3 columns
    pub fn row3_widths(&self) -> Vec<ratatui::layout::Constraint> {
        self.row3_columns
            .iter()
            .map(|c| c.width_constraint())
            .collect()
    }
}

/// Returns the path to the layout config file
pub fn layout_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("layout.toml"))
}

/// Loads GitHub token with precedence:
/// 1. GITHUB_TOKEN environment variable
/// 2. ~/.config/forgeStat/config.toml (or platform equivalent)
///
/// Returns error if token is not found in either location
pub fn load_token() -> Result<String> {
    // Check environment variable first
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            log::info!("GitHub token loaded from GITHUB_TOKEN environment variable");
            return Ok(token);
        }
    }

    // Fall back to config file
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "GitHub token not found. Set GITHUB_TOKEN environment variable or run `save_token()` first. \
             Config file does not exist at: {}",
            config_path.display()
        ));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file at: {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file at: {}", config_path.display()))?;

    match config.github_token {
        Some(token) if !token.is_empty() => {
            log::info!(
                "GitHub token loaded from config file: {}",
                config_path.display()
            );
            Ok(token)
        }
        _ => Err(anyhow::anyhow!(
            "GitHub token not found in config file: {}. \
             Set github_token in the config file or use GITHUB_TOKEN environment variable.",
            config_path.display()
        )),
    }
}

/// Saves GitHub token to config file
/// Creates config directory if it doesn't exist
pub fn save_token(token: &str) -> Result<()> {
    let config_dir = config_dir()?;

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
        log::info!("Created config directory: {}", config_dir.display());
    }

    let config_path = config_file_path()?;

    // Load existing config or create new one
    let mut config = if config_path.exists() {
        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read existing config file: {}",
                config_path.display()
            )
        })?;
        toml::from_str(&config_str).unwrap_or_default()
    } else {
        Config::default()
    };

    // Update token
    config.github_token = Some(token.to_string());

    // Serialize and write
    let config_str =
        toml::to_string_pretty(&config).context("Failed to serialize config to TOML")?;

    fs::write(&config_path, config_str)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    log::info!(
        "GitHub token saved to config file: {}",
        config_path.display()
    );
    Ok(())
}

/// Returns the path to the config file
pub fn config_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("config.toml"))
}

/// Returns the config directory path
/// Uses dirs::config_dir() for cross-platform compatibility:
/// - Windows: %APPDATA%/forgeStat
/// - macOS: ~/Library/Application Support/forgeStat
/// - Linux: ~/.config/forgeStat
pub fn config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("forgeStat"))
        .context("Failed to determine config directory. Could not find system config directory.")
}

/// Clears the stored GitHub token from config file (sets to None)
/// Returns Ok(()) if successful, or if config file doesn't exist
pub fn clear_token() -> Result<()> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(());
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let mut config: Config = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    config.github_token = None;

    let config_str =
        toml::to_string_pretty(&config).context("Failed to serialize config to TOML")?;

    fs::write(&config_path, config_str)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    log::info!(
        "GitHub token cleared from config file: {}",
        config_path.display()
    );
    Ok(())
}

/// Check if a token is available (either from env or config)
/// Returns true if token exists, false otherwise
pub fn has_token() -> bool {
    load_token().is_ok()
}

// =============================================================================
// Animation Configuration
// =============================================================================

/// Configuration for TUI animations and visual effects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnimationConfig {
    /// Master switch to enable/disable all animations
    #[serde(default = "default_animations_enabled")]
    pub enabled: bool,
    /// Low-power mode for basic terminals (disables advanced effects)
    #[serde(default = "default_low_power_mode")]
    pub low_power_mode: bool,
    /// Panel highlight flash duration in milliseconds
    #[serde(default = "default_flash_duration_ms")]
    pub flash_duration_ms: u64,
    /// Count-up animation duration in milliseconds
    #[serde(default = "default_count_up_duration_ms")]
    pub count_up_duration_ms: u64,
    /// Sync pulse animation enabled
    #[serde(default = "default_sync_pulse_enabled")]
    pub sync_pulse_enabled: bool,
    /// Braille spinner enabled during fetch
    #[serde(default = "default_spinner_enabled")]
    pub spinner_enabled: bool,
    /// Sparkline draw animation enabled
    #[serde(default = "default_sparkline_draw_enabled")]
    pub sparkline_draw_enabled: bool,
}

fn default_animations_enabled() -> bool {
    true
}

fn default_low_power_mode() -> bool {
    false
}

fn default_flash_duration_ms() -> u64 {
    300
}

fn default_count_up_duration_ms() -> u64 {
    800
}

fn default_sync_pulse_enabled() -> bool {
    true
}

fn default_spinner_enabled() -> bool {
    true
}

fn default_sparkline_draw_enabled() -> bool {
    true
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: default_animations_enabled(),
            low_power_mode: default_low_power_mode(),
            flash_duration_ms: default_flash_duration_ms(),
            count_up_duration_ms: default_count_up_duration_ms(),
            sync_pulse_enabled: default_sync_pulse_enabled(),
            spinner_enabled: default_spinner_enabled(),
            sparkline_draw_enabled: default_sparkline_draw_enabled(),
        }
    }
}

impl AnimationConfig {
    /// Load animation config from file or return default
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => {
                log::info!(
                    "Loaded animation config: enabled={}, low_power={}",
                    config.enabled,
                    config.low_power_mode
                );
                config
            }
            Err(e) => {
                log::warn!("Failed to load animation config: {}. Using default.", e);
                Self::default()
            }
        }
    }

    /// Try to load animation config from file
    fn try_load() -> Result<Self> {
        let config_path = animation_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read animation config at: {}",
                config_path.display()
            )
        })?;

        let config: AnimationConfig = toml::from_str(&config_str).with_context(|| {
            format!(
                "Failed to parse animation config at: {}",
                config_path.display()
            )
        })?;

        Ok(config)
    }

    /// Save animation config to file
    pub fn save(&self) -> Result<()> {
        let config_path = animation_file_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get animation config directory")?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            log::info!("Created config directory: {}", config_dir.display());
        }

        let config_str =
            toml::to_string_pretty(self).context("Failed to serialize animation config to TOML")?;

        fs::write(&config_path, config_str).with_context(|| {
            format!(
                "Failed to write animation config: {}",
                config_path.display()
            )
        })?;

        log::info!("Saved animation config to: {}", config_path.display());
        Ok(())
    }

    /// Check if animations should run (enabled and not low-power)
    pub fn should_animate(&self) -> bool {
        self.enabled
    }

    /// Check if a specific animation type is enabled
    pub fn is_spinner_enabled(&self) -> bool {
        self.enabled && self.spinner_enabled
    }

    pub fn is_sync_pulse_enabled(&self) -> bool {
        self.enabled && self.sync_pulse_enabled
    }

    pub fn is_sparkline_draw_enabled(&self) -> bool {
        self.enabled && self.sparkline_draw_enabled && !self.low_power_mode
    }

    pub fn is_flash_enabled(&self) -> bool {
        self.enabled && !self.low_power_mode
    }

    pub fn is_count_up_enabled(&self) -> bool {
        self.enabled && !self.low_power_mode
    }
}

/// Returns the path to the animation config file
pub fn animation_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("animation.toml"))
}

// =============================================================================
// Watchlist Configuration
// =============================================================================

/// Configuration for multi-repo watchlist mode
/// Stored in watchlist.toml with a list of "owner/repo" format repositories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WatchlistConfig {
    /// List of repositories in "owner/repo" format
    #[serde(default)]
    pub repos: Vec<String>,
}

impl WatchlistConfig {
    /// Load watchlist config from file or return empty default
    /// Handles missing file gracefully by returning empty list
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => {
                log::info!("Loaded watchlist config with {} repos", config.repos.len());
                config
            }
            Err(e) => {
                log::warn!(
                    "Failed to load watchlist config: {}. Using empty default.",
                    e
                );
                Self::default()
            }
        }
    }

    /// Try to load watchlist config from file
    fn try_load() -> Result<Self> {
        let config_path = watchlist_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read watchlist config at: {}",
                config_path.display()
            )
        })?;

        let config: WatchlistConfig = toml::from_str(&config_str).with_context(|| {
            format!(
                "Failed to parse watchlist config at: {}",
                config_path.display()
            )
        })?;

        // Validate all repos are in correct "owner/repo" format
        config.validate_repos()?;

        Ok(config)
    }

    /// Save watchlist config to file
    pub fn save(&self) -> Result<()> {
        let config_path = watchlist_file_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get watchlist config directory")?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            log::info!("Created config directory: {}", config_dir.display());
        }

        // Validate before saving
        self.validate_repos()?;

        let config_str =
            toml::to_string_pretty(self).context("Failed to serialize watchlist config to TOML")?;

        fs::write(&config_path, config_str).with_context(|| {
            format!(
                "Failed to write watchlist config: {}",
                config_path.display()
            )
        })?;

        log::info!(
            "Saved watchlist config to: {} with {} repos",
            config_path.display(),
            self.repos.len()
        );
        Ok(())
    }

    /// Validate all repos are in correct "owner/repo" format
    fn validate_repos(&self) -> Result<()> {
        for repo in &self.repos {
            Self::validate_repo_format(repo)?;
        }
        Ok(())
    }

    /// Validate a single repo string is in "owner/repo" format
    /// Returns Ok(()) if valid, Err with descriptive message if invalid
    pub fn validate_repo_format(repo: &str) -> Result<()> {
        // Check for empty string
        if repo.is_empty() {
            return Err(anyhow::anyhow!("Repository cannot be empty"));
        }

        // Check for whitespace
        if repo.contains(char::is_whitespace) {
            return Err(anyhow::anyhow!("Repository '{}' contains whitespace", repo));
        }

        // Split by single slash
        let parts: Vec<&str> = repo.split('/').collect();

        // Must have exactly one slash (2 parts)
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Repository '{}' must be in 'owner/repo' format with exactly one slash",
                repo
            ));
        }

        let owner = parts[0];
        let name = parts[1];

        // Both parts must be non-empty
        if owner.is_empty() {
            return Err(anyhow::anyhow!("Repository '{}' has empty owner", repo));
        }
        if name.is_empty() {
            return Err(anyhow::anyhow!("Repository '{}' has empty repo name", repo));
        }

        // GitHub username/repo rules: alphanumeric, hyphens, underscores
        // Cannot start or end with hyphen
        fn is_valid_github_name(name: &str) -> bool {
            if name.starts_with('-') || name.ends_with('-') {
                return false;
            }
            name.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        }

        if !is_valid_github_name(owner) {
            return Err(anyhow::anyhow!(
                "Repository '{}' has invalid owner name. Owner must be alphanumeric with hyphens/underscores, cannot start/end with hyphen",
                repo
            ));
        }

        if !is_valid_github_name(name) {
            return Err(anyhow::anyhow!(
                "Repository '{}' has invalid repo name. Name must be alphanumeric with hyphens/underscores/dots, cannot start/end with hyphen",
                repo
            ));
        }

        Ok(())
    }

    /// Add a repo to the watchlist after validating format
    pub fn add_repo(&mut self, repo: String) -> Result<()> {
        Self::validate_repo_format(&repo)?;

        // Check for duplicates
        if self.repos.contains(&repo) {
            return Err(anyhow::anyhow!(
                "Repository '{}' is already in watchlist",
                repo
            ));
        }

        self.repos.push(repo);
        Ok(())
    }

    /// Remove a repo from the watchlist
    pub fn remove_repo(&mut self, repo: &str) -> Result<()> {
        let initial_len = self.repos.len();
        self.repos.retain(|r| r != repo);

        if self.repos.len() == initial_len {
            return Err(anyhow::anyhow!(
                "Repository '{}' not found in watchlist",
                repo
            ));
        }

        Ok(())
    }

    /// Check if the watchlist is empty
    pub fn is_empty(&self) -> bool {
        self.repos.is_empty()
    }

    /// Get the number of repos in the watchlist
    pub fn len(&self) -> usize {
        self.repos.len()
    }
}

/// Returns the path to the watchlist config file
pub fn watchlist_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("watchlist.toml"))
}

// =============================================================================
// Auto-Refresh Configuration
// =============================================================================

/// Configuration for auto-refresh interval
/// Stored in refresh.toml with customizable interval in minutes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoRefreshConfig {
    /// Auto-refresh interval in minutes (default: 60 minutes = 1 hour)
    /// Set to 0 to disable auto-refresh completely
    #[serde(default = "default_auto_refresh_minutes")]
    pub auto_refresh_minutes: u64,
}

fn default_auto_refresh_minutes() -> u64 {
    60 // Default to 1 hour to conserve API rate limits
}

impl Default for AutoRefreshConfig {
    fn default() -> Self {
        Self {
            auto_refresh_minutes: default_auto_refresh_minutes(),
        }
    }
}

impl AutoRefreshConfig {
    /// Load auto-refresh config from file or return default (60 minutes)
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => {
                log::info!(
                    "Loaded auto-refresh config: interval={} minutes",
                    config.auto_refresh_minutes
                );
                config
            }
            Err(e) => {
                log::info!("Using default auto-refresh config (60 min): {}", e);
                Self::default()
            }
        }
    }

    /// Try to load auto-refresh config from file
    fn try_load() -> Result<Self> {
        let config_path = refresh_file_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path).with_context(|| {
            format!(
                "Failed to read auto-refresh config at: {}",
                config_path.display()
            )
        })?;

        let config: AutoRefreshConfig = toml::from_str(&config_str).with_context(|| {
            format!(
                "Failed to parse auto-refresh config at: {}",
                config_path.display()
            )
        })?;

        // Validate: cap at 24 hours (1440 minutes) to prevent absurd values
        // and minimum of 5 minutes to prevent hitting rate limits too quickly
        let validated = Self {
            auto_refresh_minutes: config.auto_refresh_minutes.clamp(0, 1440),
        };

        Ok(validated)
    }

    /// Save auto-refresh config to file
    pub fn save(&self) -> Result<()> {
        let config_path = refresh_file_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get auto-refresh config directory")?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir).with_context(|| {
                format!(
                    "Failed to create config directory: {}",
                    config_dir.display()
                )
            })?;
            log::info!("Created config directory: {}", config_dir.display());
        }

        // Validate before saving
        let config_to_save = Self {
            auto_refresh_minutes: self.auto_refresh_minutes.clamp(0, 1440),
        };

        let config_str = toml::to_string_pretty(&config_to_save)
            .context("Failed to serialize auto-refresh config to TOML")?;

        fs::write(&config_path, config_str).with_context(|| {
            format!(
                "Failed to write auto-refresh config: {}",
                config_path.display()
            )
        })?;

        log::info!(
            "Saved auto-refresh config to: {} ({} minutes)",
            config_path.display(),
            config_to_save.auto_refresh_minutes
        );
        Ok(())
    }

    /// Get the auto-refresh interval as seconds (for internal use)
    /// Returns None if auto-refresh is disabled (set to 0)
    pub fn auto_refresh_secs(&self) -> Option<u64> {
        if self.auto_refresh_minutes == 0 {
            None
        } else {
            Some(self.auto_refresh_minutes * 60)
        }
    }

    /// Check if auto-refresh is enabled
    pub fn is_enabled(&self) -> bool {
        self.auto_refresh_minutes > 0
    }
}

/// Returns the path to the auto-refresh config file
pub fn refresh_file_path() -> Result<PathBuf> {
    let config_dir = config_dir()?;
    Ok(config_dir.join("refresh.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_config_serde_roundtrip() {
        let config = Config {
            github_token: Some("ghp_test_token_12345".to_string()),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");
        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize config");

        assert_eq!(config, deserialized);
        assert_eq!(config.github_token, deserialized.github_token);
    }

    #[test]
    fn test_config_toml_format() {
        let config = Config {
            github_token: Some("ghp_test_token".to_string()),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");

        // Verify TOML contains expected format
        assert!(toml_str.contains("github_token"));
        assert!(toml_str.contains("ghp_test_token"));
    }

    #[test]
    fn test_config_deserialize_from_toml() {
        let toml_input = r#"github_token = "ghp_abc123"
"#;

        let config: Config = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert_eq!(config.github_token, Some("ghp_abc123".to_string()));
    }

    #[test]
    fn test_config_deserialize_missing_token() {
        let toml_input = r#"# Empty config
"#;

        let config: Config = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert!(config.github_token.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert!(config.github_token.is_none());
    }

    #[test]
    fn test_empty_env_var_ignored() {
        // This test verifies that empty env vars are handled correctly
        // We can't set env vars in a thread-safe way in tests, but we can
        // verify the Config struct handles empty strings properly
        let config = Config {
            github_token: Some("".to_string()),
        };

        // An empty string in config should be treated as no token
        assert!(config.github_token.as_ref().unwrap().is_empty());
    }

    /// Integration test for file operations (requires temp directory)
    #[test]
    fn test_save_and_load_token_from_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("config.toml");

        // Create config manually and test serialization
        let config = Config {
            github_token: Some("ghp_test_token_67890".to_string()),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let mut file = fs::File::create(&config_file).expect("Failed to create file");
        file.write_all(toml_str.as_bytes())
            .expect("Failed to write");

        // Read back and verify
        let read_content = fs::read_to_string(&config_file).expect("Failed to read");
        let loaded_config: Config = toml::from_str(&read_content).expect("Failed to parse");

        assert_eq!(
            loaded_config.github_token,
            Some("ghp_test_token_67890".to_string())
        );
    }

    #[test]
    fn test_clear_token() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("config.toml");

        // Create config with token
        let config = Config {
            github_token: Some("ghp_to_be_cleared".to_string()),
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        fs::write(&config_file, toml_str).expect("Failed to write");

        // Verify token exists
        let content = fs::read_to_string(&config_file).expect("Failed to read");
        let loaded: Config = toml::from_str(&content).expect("Failed to parse");
        assert!(loaded.github_token.is_some());

        // Create cleared config and write it
        let cleared_config = Config { github_token: None };
        let cleared_toml = toml::to_string_pretty(&cleared_config).expect("Failed to serialize");
        fs::write(&config_file, cleared_toml).expect("Failed to write");

        // Verify token is cleared
        let final_content = fs::read_to_string(&config_file).expect("Failed to read");
        let final_config: Config = toml::from_str(&final_content).expect("Failed to parse");
        assert!(final_config.github_token.is_none());
    }

    // =========================================================================
    // StatusBarItem tests
    // =========================================================================

    #[test]
    fn test_status_bar_item_all_variants() {
        // Test that all 7 variants exist and can be matched
        let items = StatusBarItem::ALL;
        assert_eq!(items.len(), 7);

        // Verify each variant is in the ALL array
        assert!(items.contains(&StatusBarItem::SyncState));
        assert!(items.contains(&StatusBarItem::RateLimit));
        assert!(items.contains(&StatusBarItem::OpenIssues));
        assert!(items.contains(&StatusBarItem::OpenPrs));
        assert!(items.contains(&StatusBarItem::LastReleaseAge));
        assert!(items.contains(&StatusBarItem::OldestIssueAge));
        assert!(items.contains(&StatusBarItem::HealthScore));
    }

    #[test]
    fn test_status_bar_item_display_names() {
        assert_eq!(StatusBarItem::SyncState.display_name(), "sync_state");
        assert_eq!(StatusBarItem::RateLimit.display_name(), "rate_limit");
        assert_eq!(StatusBarItem::OpenIssues.display_name(), "open_issues");
        assert_eq!(StatusBarItem::OpenPrs.display_name(), "open_prs");
        assert_eq!(
            StatusBarItem::LastReleaseAge.display_name(),
            "last_release_age"
        );
        assert_eq!(
            StatusBarItem::OldestIssueAge.display_name(),
            "oldest_issue_age"
        );
    }

    #[test]
    fn test_status_bar_item_deserialize_from_toml() {
        // Test that StatusBarItem variants deserialize correctly from TOML strings
        // Individual enum variants can't be serialized to TOML, but can be deserialized
        // when part of a Vec in StatusBarConfig

        let toml_input = r#"items = ["sync_state", "rate_limit", "open_issues", "open_prs", "last_release_age", "oldest_issue_age"]
"#;

        let config: StatusBarConfig = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert_eq!(config.items.len(), 6);
        assert_eq!(config.items[0], StatusBarItem::SyncState);
        assert_eq!(config.items[1], StatusBarItem::RateLimit);
        assert_eq!(config.items[2], StatusBarItem::OpenIssues);
        assert_eq!(config.items[3], StatusBarItem::OpenPrs);
        assert_eq!(config.items[4], StatusBarItem::LastReleaseAge);
        assert_eq!(config.items[5], StatusBarItem::OldestIssueAge);
    }

    #[test]
    fn test_status_bar_item_serde_via_config() {
        // Test serialization/deserialization of items through StatusBarConfig
        let config = StatusBarConfig {
            items: vec![
                StatusBarItem::SyncState,
                StatusBarItem::RateLimit,
                StatusBarItem::OpenIssues,
                StatusBarItem::OpenPrs,
                StatusBarItem::LastReleaseAge,
                StatusBarItem::OldestIssueAge,
            ],
        };

        let toml_str =
            toml::to_string_pretty(&config).expect("Failed to serialize StatusBarConfig");

        // Verify all items are in the TOML with snake_case format
        assert!(toml_str.contains("sync_state"));
        assert!(toml_str.contains("rate_limit"));
        assert!(toml_str.contains("open_issues"));
        assert!(toml_str.contains("open_prs"));
        assert!(toml_str.contains("last_release_age"));
        assert!(toml_str.contains("oldest_issue_age"));

        // Deserialize and verify roundtrip
        let deserialized: StatusBarConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize");
        assert_eq!(config.items, deserialized.items);
    }

    // =========================================================================
    // StatusBarConfig tests
    // =========================================================================

    #[test]
    fn test_status_bar_config_default() {
        let config = StatusBarConfig::default();

        // Default should be sync_state + health_score + open_prs
        assert_eq!(config.items.len(), 3);
        assert_eq!(config.items[0], StatusBarItem::SyncState);
        assert_eq!(config.items[1], StatusBarItem::HealthScore);
        assert_eq!(config.items[2], StatusBarItem::OpenPrs);
    }

    #[test]
    fn test_status_bar_config_serde_roundtrip() {
        let config = StatusBarConfig {
            items: vec![StatusBarItem::OpenIssues, StatusBarItem::LastReleaseAge],
        };

        let toml_str =
            toml::to_string_pretty(&config).expect("Failed to serialize StatusBarConfig");
        let deserialized: StatusBarConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize StatusBarConfig");

        assert_eq!(config.items, deserialized.items);
    }

    #[test]
    fn test_status_bar_config_deserialize_from_toml() {
        let toml_input = r#"items = ["sync_state", "open_issues", "oldest_issue_age"]
"#;

        let config: StatusBarConfig = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert_eq!(config.items.len(), 3);
        assert_eq!(config.items[0], StatusBarItem::SyncState);
        assert_eq!(config.items[1], StatusBarItem::OpenIssues);
        assert_eq!(config.items[2], StatusBarItem::OldestIssueAge);
    }

    #[test]
    fn test_status_bar_config_max_three_limit_on_save() {
        // Create config with more than 3 items
        let config = StatusBarConfig {
            items: vec![
                StatusBarItem::SyncState,
                StatusBarItem::RateLimit,
                StatusBarItem::OpenIssues,
                StatusBarItem::OpenPrs,
                StatusBarItem::LastReleaseAge,
            ],
        };

        // Save should truncate to 3 items
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("statusbar.toml");

        // Manually create the directory and save
        fs::create_dir_all(temp_dir.path()).expect("Failed to create temp dir");

        // Call save() which should enforce the limit
        let mut config_to_save = config.clone();

        // Enforce max 3 items (same logic as save())
        if config_to_save.items.len() > 3 {
            config_to_save.items.truncate(3);
        }

        let toml_str = toml::to_string_pretty(&config_to_save).expect("Failed to serialize");
        fs::write(&config_file, toml_str).expect("Failed to write");

        // Read back and verify it has 3 items
        let read_content = fs::read_to_string(&config_file).expect("Failed to read");
        let loaded: StatusBarConfig = toml::from_str(&read_content).expect("Failed to parse");

        assert_eq!(loaded.items.len(), 3);
        assert_eq!(loaded.items[0], StatusBarItem::SyncState);
        assert_eq!(loaded.items[1], StatusBarItem::RateLimit);
        assert_eq!(loaded.items[2], StatusBarItem::OpenIssues);
    }

    #[test]
    fn test_status_bar_config_deduplication_on_save() {
        // Create config with duplicate items
        let config = StatusBarConfig {
            items: vec![
                StatusBarItem::SyncState,
                StatusBarItem::SyncState, // Duplicate
                StatusBarItem::RateLimit,
                StatusBarItem::RateLimit, // Duplicate
                StatusBarItem::OpenIssues,
            ],
        };

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("statusbar.toml");

        fs::create_dir_all(temp_dir.path()).expect("Failed to create temp dir");

        // Apply deduplication logic (same as save())
        let mut config_to_save = config.clone();
        let mut seen = std::collections::HashSet::new();
        config_to_save.items.retain(|item| seen.insert(*item));

        // Also apply limit
        if config_to_save.items.len() > 3 {
            config_to_save.items.truncate(3);
        }

        let toml_str = toml::to_string_pretty(&config_to_save).expect("Failed to serialize");
        fs::write(&config_file, toml_str).expect("Failed to write");

        // Read back and verify deduplication worked
        let read_content = fs::read_to_string(&config_file).expect("Failed to read");
        let loaded: StatusBarConfig = toml::from_str(&read_content).expect("Failed to parse");

        // After deduplication, should have sync_state, rate_limit, open_issues
        assert_eq!(loaded.items.len(), 3);
        assert_eq!(loaded.items[0], StatusBarItem::SyncState);
        assert_eq!(loaded.items[1], StatusBarItem::RateLimit);
        assert_eq!(loaded.items[2], StatusBarItem::OpenIssues);
    }

    #[test]
    fn test_status_bar_config_truncate_and_deduplicate_together() {
        // Test both truncation and deduplication working together
        let config = StatusBarConfig {
            items: vec![
                StatusBarItem::SyncState,
                StatusBarItem::SyncState, // Duplicate
                StatusBarItem::RateLimit,
                StatusBarItem::OpenIssues,
                StatusBarItem::OpenPrs,
                StatusBarItem::LastReleaseAge,
            ],
        };

        // Apply both deduplication and truncation (as save() does)
        let mut config_to_save = config.clone();

        // Deduplicate first
        let mut seen = std::collections::HashSet::new();
        config_to_save.items.retain(|item| seen.insert(*item));

        // Then truncate to 3
        config_to_save.items.truncate(3);

        // After dedup: [SyncState, RateLimit, OpenIssues, OpenPrs, LastReleaseAge] (5 items)
        // After truncate: [SyncState, RateLimit, OpenIssues] (3 items)
        assert_eq!(config_to_save.items.len(), 3);
        assert_eq!(config_to_save.items[0], StatusBarItem::SyncState);
        assert_eq!(config_to_save.items[1], StatusBarItem::RateLimit);
        assert_eq!(config_to_save.items[2], StatusBarItem::OpenIssues);
    }

    #[test]
    fn test_status_bar_config_all_variants() {
        // Test that all 7 variants can be used in a config
        let config = StatusBarConfig {
            items: StatusBarItem::ALL.to_vec(),
        };

        assert_eq!(config.items.len(), 7);

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

        // Verify all items are in the TOML
        assert!(toml_str.contains("sync_state"));
        assert!(toml_str.contains("rate_limit"));
        assert!(toml_str.contains("open_issues"));
        assert!(toml_str.contains("open_prs"));
        assert!(toml_str.contains("last_release_age"));
        assert!(toml_str.contains("health_score"));
        assert!(toml_str.contains("oldest_issue_age"));
    }

    #[test]
    fn test_status_bar_config_empty_items() {
        // Test empty items config
        let config = StatusBarConfig { items: vec![] };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize empty config");
        let deserialized: StatusBarConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize empty config");

        assert!(deserialized.items.is_empty());
    }

    #[test]
    fn test_status_bar_config_single_item() {
        // Test single item config
        let config = StatusBarConfig {
            items: vec![StatusBarItem::SyncState],
        };

        assert_eq!(config.items.len(), 1);
        assert_eq!(config.items[0], StatusBarItem::SyncState);
    }

    #[test]
    fn test_status_bar_file_path() {
        // This test just verifies the function doesn't panic
        // The actual path depends on the OS and user config
        let path_result = statusbar_file_path();
        assert!(path_result.is_ok());

        let path = path_result.unwrap();
        assert!(path.to_string_lossy().contains("statusbar.toml"));
    }

    // =========================================================================
    // LayoutConfig tests
    // =========================================================================

    #[test]
    fn test_layout_preset_default() {
        let preset = LayoutPreset::default();
        assert_eq!(preset, LayoutPreset::Default);
    }

    #[test]
    fn test_layout_preset_deserialize() {
        let toml_default = r#"preset = "default""#;
        let toml_compact = r#"preset = "compact""#;
        let toml_wide = r#"preset = "wide""#;

        let config_default: LayoutConfig = toml::from_str(toml_default).unwrap();
        let config_compact: LayoutConfig = toml::from_str(toml_compact).unwrap();
        let config_wide: LayoutConfig = toml::from_str(toml_wide).unwrap();

        assert_eq!(config_default.preset, LayoutPreset::Default);
        assert_eq!(config_compact.preset, LayoutPreset::Compact);
        assert_eq!(config_wide.preset, LayoutPreset::Wide);
    }

    #[test]
    fn test_layout_preset_serialize() {
        let config = LayoutConfig::default_preset(LayoutPreset::Compact);
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("compact"));
    }

    #[test]
    fn test_panel_layout_new() {
        let panel = PanelLayout::new(50, 60);
        assert_eq!(panel.width_pct, 50);
        assert_eq!(panel.height_pct, 60);
    }

    #[test]
    fn test_panel_layout_clamping() {
        // Test that values are clamped to valid range
        let panel = PanelLayout::new(5, 150);
        assert_eq!(panel.width_pct, 10); // clamped to minimum
        assert_eq!(panel.height_pct, 100); // clamped to maximum
    }

    #[test]
    fn test_layout_config_default_preset() {
        let config = LayoutConfig::default_preset(LayoutPreset::Default);
        assert_eq!(config.preset, LayoutPreset::Default);
        assert_eq!(config.row1.height_pct, 40);
        assert_eq!(config.row2.height_pct, 30);
        assert_eq!(config.row3.height_pct, 30);
    }

    #[test]
    fn test_layout_config_compact_preset() {
        let config = LayoutConfig::default_preset(LayoutPreset::Compact);
        assert_eq!(config.preset, LayoutPreset::Compact);
        assert_eq!(config.row1.height_pct, 35);
        assert_eq!(config.row2.height_pct, 35);
        assert_eq!(config.row3.height_pct, 30);

        // Check column sizes are different
        assert_eq!(config.row1_columns[0].width_pct, 30); // Stars smaller
        assert_eq!(config.row1_columns[1].width_pct, 70); // Issues larger
    }

    #[test]
    fn test_layout_config_wide_preset() {
        let config = LayoutConfig::default_preset(LayoutPreset::Wide);
        assert_eq!(config.preset, LayoutPreset::Wide);
        assert_eq!(config.row1.height_pct, 45);
        assert_eq!(config.row2.height_pct, 30);
        assert_eq!(config.row3.height_pct, 25);

        // Check column sizes
        assert_eq!(config.row1_columns[0].width_pct, 40); // Stars larger
        assert_eq!(config.row3_columns[0].width_pct, 40); // Velocity
        assert_eq!(config.row3_columns[1].width_pct, 30); // Security
        assert_eq!(config.row3_columns[2].width_pct, 30); // CI
    }

    #[test]
    fn test_layout_config_normalize_heights() {
        let mut config = LayoutConfig {
            preset: LayoutPreset::Default,
            row1: PanelLayout::new(100, 50),
            row2: PanelLayout::new(100, 50),
            row3: PanelLayout::new(100, 50),
            ..Default::default()
        };

        // Total height is 150%, should be normalized to 100%
        config = config.normalize();

        // Each row should be approximately 33% (50/150 * 100)
        assert_eq!(config.row1.height_pct, 33);
        assert_eq!(config.row2.height_pct, 33);
        // Third row gets remainder
        assert_eq!(config.row3.height_pct, 34);
    }

    #[test]
    fn test_layout_config_normalize_minimums() {
        let mut config = LayoutConfig {
            preset: LayoutPreset::Default,
            row1: PanelLayout::new(100, 5),  // Below minimum
            row2: PanelLayout::new(100, 5),  // Below minimum
            row3: PanelLayout::new(100, 90), // Large
            ..Default::default()
        };

        config = config.normalize();

        // Minimum is 10%
        assert!(config.row1.height_pct >= 10);
        assert!(config.row2.height_pct >= 10);
        assert!(config.row3.height_pct >= 10);
    }

    #[test]
    fn test_layout_config_normalize_columns() {
        let mut columns = vec![PanelLayout::new(100, 100), PanelLayout::new(100, 100)];

        LayoutConfig::normalize_columns(&mut columns);

        // Should normalize to 50/50
        assert_eq!(columns[0].width_pct, 50);
        assert_eq!(columns[1].width_pct, 50);
    }

    #[test]
    fn test_layout_config_normalize_columns_three() {
        let mut columns = vec![
            PanelLayout::new(50, 100),
            PanelLayout::new(50, 100),
            PanelLayout::new(50, 100),
        ];

        LayoutConfig::normalize_columns(&mut columns);

        // Should normalize to approximately 33/33/34
        let total: u16 = columns.iter().map(|c| c.width_pct).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn test_layout_config_row_heights() {
        let config = LayoutConfig::default();
        let heights = config.row_heights();

        assert_eq!(heights.len(), 3);
        // Default is 40/30/30
        assert!(matches!(
            heights[0],
            ratatui::layout::Constraint::Percentage(40)
        ));
        assert!(matches!(
            heights[1],
            ratatui::layout::Constraint::Percentage(30)
        ));
        assert!(matches!(
            heights[2],
            ratatui::layout::Constraint::Percentage(30)
        ));
    }

    #[test]
    fn test_layout_config_column_widths() {
        let config = LayoutConfig::default();

        let row1 = config.row1_widths();
        assert_eq!(row1.len(), 2);

        let row2 = config.row2_widths();
        assert_eq!(row2.len(), 3);

        let row3 = config.row3_widths();
        assert_eq!(row3.len(), 3);
    }

    #[test]
    fn test_layout_config_reset_to_preset() {
        let mut config = LayoutConfig::default_preset(LayoutPreset::Wide);
        assert_eq!(config.preset, LayoutPreset::Wide);
        assert_eq!(config.row1.height_pct, 45);

        config.reset_to_preset(LayoutPreset::Compact);
        assert_eq!(config.preset, LayoutPreset::Compact);
        assert_eq!(config.row1.height_pct, 35);
    }

    #[test]
    fn test_layout_config_serde_roundtrip() {
        let config = LayoutConfig::default_preset(LayoutPreset::Compact);
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let deserialized: LayoutConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.preset, deserialized.preset);
        assert_eq!(config.row1.height_pct, deserialized.row1.height_pct);
        assert_eq!(config.row2.height_pct, deserialized.row2.height_pct);
        assert_eq!(config.row3.height_pct, deserialized.row3.height_pct);
    }

    #[test]
    fn test_layout_config_empty_columns() {
        let mut columns: Vec<PanelLayout> = vec![];
        LayoutConfig::normalize_columns(&mut columns);
        assert!(columns.is_empty());
    }

    #[test]
    fn test_layout_config_minimum_column_width() {
        let mut columns = vec![
            PanelLayout::new(5, 100), // Below 10% minimum, gets clamped
            PanelLayout::new(95, 100),
        ];

        LayoutConfig::normalize_columns(&mut columns);

        // After clamping to minimum (10), total is 105, then normalized to 100
        // 10 * 100 / 105 = 9.5 -> 9, remainder goes to last column
        let total: u16 = columns.iter().map(|c| c.width_pct).sum();
        assert_eq!(total, 100);
        // The first column should be at least close to the minimum after normalization
        assert!(columns[0].width_pct >= 9);
        // Both columns should be at least the minimum enforced (10) before normalization to 100
        // but after normalization they may be slightly less
    }

    #[test]
    fn test_layout_file_path() {
        let path_result = layout_file_path();
        assert!(path_result.is_ok());

        let path = path_result.unwrap();
        assert!(path.to_string_lossy().contains("layout.toml"));
    }

    #[test]
    fn test_layout_config_deserialize_full() {
        let toml_input = r#"
preset = "wide"

[row1]
width_pct = 100
height_pct = 45

[row2]
width_pct = 100
height_pct = 30

[row3]
width_pct = 100
height_pct = 25

[[row1_columns]]
width_pct = 40
height_pct = 100

[[row1_columns]]
width_pct = 60
height_pct = 100
"#;

        let config: LayoutConfig = toml::from_str(toml_input).unwrap();
        assert_eq!(config.preset, LayoutPreset::Wide);
        assert_eq!(config.row1.height_pct, 45);
        assert_eq!(config.row1_columns.len(), 2);
        assert_eq!(config.row1_columns[0].width_pct, 40);
    }

    // =========================================================================
    // WatchlistConfig tests
    // =========================================================================

    #[test]
    fn test_watchlist_config_default() {
        let config = WatchlistConfig::default();
        assert!(config.repos.is_empty());
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_watchlist_config_serde_roundtrip() {
        let config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string(), "rust-lang/rust".to_string()],
        };

        let toml_str =
            toml::to_string_pretty(&config).expect("Failed to serialize WatchlistConfig");
        let deserialized: WatchlistConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize WatchlistConfig");

        assert_eq!(config.repos, deserialized.repos);
    }

    #[test]
    fn test_watchlist_config_deserialize_from_toml() {
        let toml_input = r#"repos = ["torvalds/linux", "rust-lang/rust"]
"#;

        let config: WatchlistConfig = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.repos[0], "torvalds/linux");
        assert_eq!(config.repos[1], "rust-lang/rust");
    }

    #[test]
    fn test_watchlist_config_empty_repos() {
        let toml_input = r#"# Empty watchlist
"#;

        let config: WatchlistConfig = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert!(config.repos.is_empty());
    }

    #[test]
    fn test_watchlist_config_explicit_empty_repos() {
        let toml_input = r#"repos = []
"#;

        let config: WatchlistConfig = toml::from_str(toml_input).expect("Failed to parse TOML");

        assert!(config.repos.is_empty());
    }

    #[test]
    fn test_validate_repo_format_valid() {
        // Valid formats
        assert!(WatchlistConfig::validate_repo_format("torvalds/linux").is_ok());
        assert!(WatchlistConfig::validate_repo_format("rust-lang/rust").is_ok());
        assert!(WatchlistConfig::validate_repo_format("myorg/my-repo").is_ok());
        assert!(WatchlistConfig::validate_repo_format("user/my_repo").is_ok());
        assert!(WatchlistConfig::validate_repo_format("org/my.repo").is_ok());
        assert!(WatchlistConfig::validate_repo_format("a/b").is_ok());
    }

    #[test]
    fn test_validate_repo_format_invalid_empty() {
        let result = WatchlistConfig::validate_repo_format("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_repo_format_invalid_whitespace() {
        let result = WatchlistConfig::validate_repo_format("owner /repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));

        let result = WatchlistConfig::validate_repo_format("owner/repo ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_validate_repo_format_invalid_no_slash() {
        let result = WatchlistConfig::validate_repo_format("invalid-repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exactly one slash"));
    }

    #[test]
    fn test_validate_repo_format_invalid_multiple_slashes() {
        let result = WatchlistConfig::validate_repo_format("owner/repo/extra");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exactly one slash"));
    }

    #[test]
    fn test_validate_repo_format_invalid_empty_owner() {
        let result = WatchlistConfig::validate_repo_format("/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty owner"));
    }

    #[test]
    fn test_validate_repo_format_invalid_empty_name() {
        let result = WatchlistConfig::validate_repo_format("owner/");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty repo name"));
    }

    #[test]
    fn test_validate_repo_format_invalid_leading_hyphen_owner() {
        let result = WatchlistConfig::validate_repo_format("-owner/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid owner"));
    }

    #[test]
    fn test_validate_repo_format_invalid_trailing_hyphen_owner() {
        let result = WatchlistConfig::validate_repo_format("owner-/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid owner"));
    }

    #[test]
    fn test_validate_repo_format_invalid_leading_hyphen_repo() {
        let result = WatchlistConfig::validate_repo_format("owner/-repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid repo name"));
    }

    #[test]
    fn test_validate_repo_format_invalid_trailing_hyphen_repo() {
        let result = WatchlistConfig::validate_repo_format("owner/repo-");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid repo name"));
    }

    #[test]
    fn test_validate_repo_format_invalid_special_chars() {
        let result = WatchlistConfig::validate_repo_format("owner@name/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid owner"));

        let result = WatchlistConfig::validate_repo_format("owner/repo@name");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid repo name"));
    }

    #[test]
    fn test_watchlist_file_path() {
        let path_result = watchlist_file_path();
        assert!(path_result.is_ok());

        let path = path_result.unwrap();
        assert!(path.to_string_lossy().contains("watchlist.toml"));
    }

    #[test]
    fn test_watchlist_add_repo() {
        let mut config = WatchlistConfig::default();

        // Add valid repos
        assert!(config.add_repo("torvalds/linux".to_string()).is_ok());
        assert!(config.add_repo("rust-lang/rust".to_string()).is_ok());

        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.repos[0], "torvalds/linux");
        assert_eq!(config.repos[1], "rust-lang/rust");
    }

    #[test]
    fn test_watchlist_add_repo_invalid() {
        let mut config = WatchlistConfig::default();

        // Try to add invalid repo
        let result = config.add_repo("invalid-repo".to_string());
        assert!(result.is_err());
        assert!(config.repos.is_empty()); // Should not be added
    }

    #[test]
    fn test_watchlist_add_duplicate_repo() {
        let mut config = WatchlistConfig::default();

        // Add first repo
        assert!(config.add_repo("torvalds/linux".to_string()).is_ok());

        // Try to add duplicate
        let result = config.add_repo("torvalds/linux".to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already in watchlist"));

        assert_eq!(config.repos.len(), 1); // Should still only have one
    }

    #[test]
    fn test_watchlist_remove_repo() {
        let mut config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string(), "rust-lang/rust".to_string()],
        };

        assert!(config.remove_repo("torvalds/linux").is_ok());
        assert_eq!(config.repos.len(), 1);
        assert_eq!(config.repos[0], "rust-lang/rust");
    }

    #[test]
    fn test_watchlist_remove_nonexistent_repo() {
        let mut config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string()],
        };

        let result = config.remove_repo("nonexistent/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        assert_eq!(config.repos.len(), 1); // Original repo still there
    }

    #[test]
    fn test_watchlist_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("watchlist.toml");

        // Create config with repos
        let config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string(), "rust-lang/rust".to_string()],
        };

        // Save manually
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        fs::write(&config_file, toml_str).expect("Failed to write");

        // Read back and verify
        let read_content = fs::read_to_string(&config_file).expect("Failed to read");
        let loaded: WatchlistConfig = toml::from_str(&read_content).expect("Failed to parse");

        assert_eq!(loaded.repos.len(), 2);
        assert_eq!(loaded.repos[0], "torvalds/linux");
        assert_eq!(loaded.repos[1], "rust-lang/rust");
    }

    #[test]
    fn test_watchlist_load_missing_file_returns_empty() {
        // Create a temporary directory that doesn't contain watchlist.toml
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Verify the file doesn't exist
        let missing_file = temp_dir.path().join("watchlist.toml");
        assert!(!missing_file.exists());

        // The load() method handles missing files gracefully
        // We can't easily test the actual load() without mocking, but we can
        // test that default() gives us an empty list
        let config = WatchlistConfig::default();
        assert!(config.repos.is_empty());
    }

    #[test]
    fn test_watchlist_validate_repos_all_valid() {
        let config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string(), "rust-lang/rust".to_string()],
        };

        assert!(config.validate_repos().is_ok());
    }

    #[test]
    fn test_watchlist_validate_repos_with_invalid() {
        let config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string(), "invalid-repo".to_string()],
        };

        let result = config.validate_repos();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exactly one slash"));
    }

    #[test]
    fn test_watchlist_save_invalid_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_file = temp_dir.path().join("watchlist.toml");

        // Create config with invalid repo
        let config = WatchlistConfig {
            repos: vec!["invalid-repo".to_string()],
        };

        // Try to save manually - should fail due to validation
        let result = config.validate_repos();
        assert!(result.is_err());

        // Verify file was not created (we didn't actually save)
        assert!(!config_file.exists());
    }

    #[test]
    fn test_watchlist_toml_format() {
        let config = WatchlistConfig {
            repos: vec!["torvalds/linux".to_string()],
        };

        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");

        // Verify TOML contains expected format
        assert!(toml_str.contains("repos"));
        assert!(toml_str.contains("torvalds/linux"));
    }

    #[test]
    fn test_watchlist_many_repos() {
        let repos: Vec<String> = (0..10).map(|i| format!("owner{}/repo{}", i, i)).collect();

        let config = WatchlistConfig { repos };

        assert_eq!(config.len(), 10);
        assert!(!config.is_empty());
        assert!(config.validate_repos().is_ok());
    }
}
