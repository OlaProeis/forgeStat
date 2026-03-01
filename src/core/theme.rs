use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Theme configuration with colors for all UI elements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Border color for unselected panels
    pub border_unselected: String,
    /// Border color for selected panels
    pub border_selected: String,
    /// Header border color
    pub header_border: String,
    /// Sparkline color for star history
    pub sparkline: String,
    /// Status bar live indicator color
    pub status_live: String,
    /// Status bar stale indicator color
    pub status_stale: String,
    /// Status bar offline indicator color
    pub status_offline: String,
    /// Primary text color
    pub text_primary: String,
    /// Secondary/muted text color
    pub text_secondary: String,
    /// Highlight/bold text color
    pub text_highlight: String,
    /// Success/green indicator color
    pub indicator_success: String,
    /// Warning/yellow indicator color
    pub indicator_warning: String,
    /// Error/red indicator color
    pub indicator_error: String,
    /// Info/cyan indicator color
    pub indicator_info: String,
    /// Muted/gray indicator color
    pub indicator_muted: String,
    /// Critical severity color (security alerts)
    pub severity_critical: String,
    /// High severity color
    pub severity_high: String,
    /// Medium severity color
    pub severity_medium: String,
    /// Low severity color
    pub severity_low: String,
    /// Help overlay border color
    pub help_border: String,
    /// Help overlay title color
    pub help_title: String,
    /// Use Braille characters for sparklines (2x resolution)
    #[serde(default = "default_braille_mode")]
    pub braille_mode: bool,
}

fn default_braille_mode() -> bool {
    false
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl ThemeConfig {
    /// The default color theme
    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            border_unselected: "#6b7280".to_string(), // gray-500
            border_selected: "#22d3ee".to_string(),   // cyan-400
            header_border: "#ffffff".to_string(),     // white
            sparkline: "#fbbf24".to_string(),         // amber-400
            status_live: "#4ade80".to_string(),       // green-400
            status_stale: "#facc15".to_string(),      // yellow-400
            status_offline: "#f87171".to_string(),    // red-400
            text_primary: "#ffffff".to_string(),      // white
            text_secondary: "#9ca3af".to_string(),    // gray-400
            text_highlight: "#ffffff".to_string(),    // white
            indicator_success: "#4ade80".to_string(), // green-400
            indicator_warning: "#facc15".to_string(), // yellow-400
            indicator_error: "#f87171".to_string(),   // red-400
            indicator_info: "#22d3ee".to_string(),    // cyan-400
            indicator_muted: "#6b7280".to_string(),   // gray-500
            severity_critical: "#ef4444".to_string(), // red-500
            severity_high: "#f87171".to_string(),     // light red
            severity_medium: "#facc15".to_string(),   // yellow-400
            severity_low: "#6b7280".to_string(),      // gray-500
            help_border: "#22d3ee".to_string(),       // cyan-400
            help_title: "#22d3ee".to_string(),        // cyan-400
            braille_mode: false,
        }
    }

    /// Monochrome theme (grayscale only)
    pub fn monochrome_theme() -> Self {
        Self {
            name: "monochrome".to_string(),
            border_unselected: "#6b7280".to_string(), // gray-500
            border_selected: "#ffffff".to_string(),   // white
            header_border: "#d1d5db".to_string(),     // gray-300
            sparkline: "#9ca3af".to_string(),         // gray-400
            status_live: "#9ca3af".to_string(),       // gray-400
            status_stale: "#6b7280".to_string(),      // gray-500
            status_offline: "#4b5563".to_string(),    // gray-600
            text_primary: "#ffffff".to_string(),      // white
            text_secondary: "#9ca3af".to_string(),    // gray-400
            text_highlight: "#ffffff".to_string(),    // white
            indicator_success: "#d1d5db".to_string(), // gray-300
            indicator_warning: "#9ca3af".to_string(), // gray-400
            indicator_error: "#6b7280".to_string(),   // gray-500
            indicator_info: "#d1d5db".to_string(),    // gray-300
            indicator_muted: "#4b5563".to_string(),   // gray-600
            severity_critical: "#ffffff".to_string(), // white (bold)
            severity_high: "#d1d5db".to_string(),     // gray-300
            severity_medium: "#9ca3af".to_string(),   // gray-400
            severity_low: "#6b7280".to_string(),      // gray-500
            help_border: "#d1d5db".to_string(),       // gray-300
            help_title: "#ffffff".to_string(),        // white
            braille_mode: false,
        }
    }

    /// High contrast theme (bold colors, maximum visibility)
    pub fn high_contrast_theme() -> Self {
        Self {
            name: "high-contrast".to_string(),
            border_unselected: "#808080".to_string(), // gray
            border_selected: "#00ffff".to_string(),   // bright cyan
            header_border: "#ffffff".to_string(),     // white
            sparkline: "#ffff00".to_string(),         // bright yellow
            status_live: "#00ff00".to_string(),       // bright green
            status_stale: "#ffff00".to_string(),      // bright yellow
            status_offline: "#ff0000".to_string(),    // bright red
            text_primary: "#ffffff".to_string(),      // white
            text_secondary: "#c0c0c0".to_string(),    // silver
            text_highlight: "#ffffff".to_string(),    // white
            indicator_success: "#00ff00".to_string(), // bright green
            indicator_warning: "#ffff00".to_string(), // bright yellow
            indicator_error: "#ff0000".to_string(),   // bright red
            indicator_info: "#00ffff".to_string(),    // bright cyan
            indicator_muted: "#808080".to_string(),   // gray
            severity_critical: "#ff0000".to_string(), // bright red
            severity_high: "#ff6600".to_string(),     // orange-red
            severity_medium: "#ffff00".to_string(),   // bright yellow
            severity_low: "#808080".to_string(),      // gray
            help_border: "#00ffff".to_string(),       // bright cyan
            help_title: "#00ffff".to_string(),        // bright cyan
            braille_mode: false,
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark_theme() -> Self {
        Self {
            name: "solarized-dark".to_string(),
            border_unselected: "#586e75".to_string(), // base01
            border_selected: "#2aa198".to_string(),   // cyan
            header_border: "#eee8d5".to_string(),     // base2
            sparkline: "#b58900".to_string(),         // yellow
            status_live: "#859900".to_string(),       // green
            status_stale: "#b58900".to_string(),      // yellow
            status_offline: "#dc322f".to_string(),    // red
            text_primary: "#eee8d5".to_string(),      // base2
            text_secondary: "#839496".to_string(),    // base0
            text_highlight: "#fdf6e3".to_string(),    // base3
            indicator_success: "#859900".to_string(), // green
            indicator_warning: "#b58900".to_string(), // yellow
            indicator_error: "#dc322f".to_string(),   // red
            indicator_info: "#2aa198".to_string(),    // cyan
            indicator_muted: "#586e75".to_string(),   // base01
            severity_critical: "#dc322f".to_string(), // red
            severity_high: "#cb4b16".to_string(),     // orange
            severity_medium: "#b58900".to_string(),   // yellow
            severity_low: "#586e75".to_string(),      // base01
            help_border: "#2aa198".to_string(),       // cyan
            help_title: "#2aa198".to_string(),        // cyan
            braille_mode: false,
        }
    }

    /// Dracula theme
    pub fn dracula_theme() -> Self {
        Self {
            name: "dracula".to_string(),
            border_unselected: "#6272a4".to_string(), // comment
            border_selected: "#8be9fd".to_string(),   // cyan
            header_border: "#f8f8f2".to_string(),     // foreground
            sparkline: "#f1fa8c".to_string(),         // yellow
            status_live: "#50fa7b".to_string(),       // green
            status_stale: "#f1fa8c".to_string(),      // yellow
            status_offline: "#ff5555".to_string(),    // red
            text_primary: "#f8f8f2".to_string(),      // foreground
            text_secondary: "#6272a4".to_string(),    // comment
            text_highlight: "#ffffff".to_string(),    // white
            indicator_success: "#50fa7b".to_string(), // green
            indicator_warning: "#f1fa8c".to_string(), // yellow
            indicator_error: "#ff5555".to_string(),   // red
            indicator_info: "#8be9fd".to_string(),    // cyan
            indicator_muted: "#6272a4".to_string(),   // comment
            severity_critical: "#ff5555".to_string(), // red
            severity_high: "#ff79c6".to_string(),     // pink
            severity_medium: "#f1fa8c".to_string(),   // yellow
            severity_low: "#6272a4".to_string(),      // comment
            help_border: "#8be9fd".to_string(),       // cyan
            help_title: "#8be9fd".to_string(),        // cyan
            braille_mode: false,
        }
    }

    /// Gruvbox Dark theme
    pub fn gruvbox_theme() -> Self {
        Self {
            name: "gruvbox".to_string(),
            border_unselected: "#928374".to_string(), // gray
            border_selected: "#83a598".to_string(),   // blue/cyan
            header_border: "#ebdbb2".to_string(),     // fg1
            sparkline: "#fabd2f".to_string(),         // yellow
            status_live: "#b8bb26".to_string(),       // green
            status_stale: "#fabd2f".to_string(),      // yellow
            status_offline: "#fb4934".to_string(),    // red
            text_primary: "#ebdbb2".to_string(),      // fg1
            text_secondary: "#a89984".to_string(),    // fg4
            text_highlight: "#fbf1c7".to_string(),    // fg0
            indicator_success: "#b8bb26".to_string(), // green
            indicator_warning: "#fabd2f".to_string(), // yellow
            indicator_error: "#fb4934".to_string(),   // red
            indicator_info: "#83a598".to_string(),    // blue/cyan
            indicator_muted: "#928374".to_string(),   // gray
            severity_critical: "#fb4934".to_string(), // red
            severity_high: "#fe8019".to_string(),     // orange
            severity_medium: "#fabd2f".to_string(),   // yellow
            severity_low: "#928374".to_string(),      // gray
            help_border: "#83a598".to_string(),       // blue/cyan
            help_title: "#83a598".to_string(),        // blue/cyan
            braille_mode: false,
        }
    }

    /// Get a built-in theme by name
    pub fn get_builtin(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_theme()),
            "monochrome" => Some(Self::monochrome_theme()),
            "high-contrast" => Some(Self::high_contrast_theme()),
            "solarized-dark" => Some(Self::solarized_dark_theme()),
            "dracula" => Some(Self::dracula_theme()),
            "gruvbox" => Some(Self::gruvbox_theme()),
            _ => None,
        }
    }

    /// Convert hex color string to ratatui Color
    pub fn parse_color(hex: &str) -> Color {
        if hex.starts_with('#') && hex.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[1..3], 16),
                u8::from_str_radix(&hex[3..5], 16),
                u8::from_str_radix(&hex[5..7], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
        // Fallback to white if parsing fails
        Color::White
    }

    // Helper methods to get ratatui colors directly
    pub fn border_unselected_color(&self) -> Color {
        Self::parse_color(&self.border_unselected)
    }

    pub fn border_selected_color(&self) -> Color {
        Self::parse_color(&self.border_selected)
    }

    pub fn header_border_color(&self) -> Color {
        Self::parse_color(&self.header_border)
    }

    pub fn sparkline_color(&self) -> Color {
        Self::parse_color(&self.sparkline)
    }

    pub fn status_live_color(&self) -> Color {
        Self::parse_color(&self.status_live)
    }

    pub fn status_stale_color(&self) -> Color {
        Self::parse_color(&self.status_stale)
    }

    pub fn status_offline_color(&self) -> Color {
        Self::parse_color(&self.status_offline)
    }

    pub fn text_primary_color(&self) -> Color {
        Self::parse_color(&self.text_primary)
    }

    pub fn text_secondary_color(&self) -> Color {
        Self::parse_color(&self.text_secondary)
    }

    pub fn text_highlight_color(&self) -> Color {
        Self::parse_color(&self.text_highlight)
    }

    pub fn indicator_success_color(&self) -> Color {
        Self::parse_color(&self.indicator_success)
    }

    pub fn indicator_warning_color(&self) -> Color {
        Self::parse_color(&self.indicator_warning)
    }

    pub fn indicator_error_color(&self) -> Color {
        Self::parse_color(&self.indicator_error)
    }

    pub fn indicator_info_color(&self) -> Color {
        Self::parse_color(&self.indicator_info)
    }

    pub fn indicator_muted_color(&self) -> Color {
        Self::parse_color(&self.indicator_muted)
    }

    pub fn severity_critical_color(&self) -> Color {
        Self::parse_color(&self.severity_critical)
    }

    pub fn severity_high_color(&self) -> Color {
        Self::parse_color(&self.severity_high)
    }

    pub fn severity_medium_color(&self) -> Color {
        Self::parse_color(&self.severity_medium)
    }

    pub fn severity_low_color(&self) -> Color {
        Self::parse_color(&self.severity_low)
    }

    pub fn help_border_color(&self) -> Color {
        Self::parse_color(&self.help_border)
    }

    pub fn help_title_color(&self) -> Color {
        Self::parse_color(&self.help_title)
    }

    /// Success text color (alias for indicator_success)
    pub fn text_success_color(&self) -> Color {
        Self::parse_color(&self.indicator_success)
    }

    /// Error text color (alias for indicator_error)
    pub fn text_error_color(&self) -> Color {
        Self::parse_color(&self.indicator_error)
    }

    /// Warning text color (alias for indicator_warning)
    pub fn text_warning_color(&self) -> Color {
        Self::parse_color(&self.indicator_warning)
    }
}

/// Theme file structure for TOML parsing
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ThemeFile {
    /// Selected theme name (references built-in or custom theme)
    pub theme: Option<String>,
    /// Custom themes defined by user
    #[serde(default)]
    pub custom_themes: HashMap<String, ThemeConfig>,
    /// Global Braille mode setting (overrides theme default if set)
    #[serde(default)]
    pub braille_mode: Option<bool>,
}

/// Returns the path to the theme config file
pub fn theme_file_path() -> Result<PathBuf> {
    let config_dir = crate::core::config::config_dir()?;
    Ok(config_dir.join("theme.toml"))
}

/// Load theme configuration from file or return default
/// Falls back to 'default' theme if file is missing or invalid
pub fn load_theme() -> ThemeConfig {
    match try_load_theme() {
        Ok(theme) => {
            log::info!("Loaded theme: {}", theme.name);
            theme
        }
        Err(e) => {
            log::warn!("Failed to load theme: {}. Using default theme.", e);
            ThemeConfig::default_theme()
        }
    }
}

/// Try to load theme from config file
fn try_load_theme() -> Result<ThemeConfig> {
    let theme_path = theme_file_path()?;

    if !theme_path.exists() {
        return Ok(ThemeConfig::default_theme());
    }

    let theme_str = fs::read_to_string(&theme_path)
        .with_context(|| format!("Failed to read theme file at: {}", theme_path.display()))?;

    let theme_file: ThemeFile = toml::from_str(&theme_str)
        .with_context(|| format!("Failed to parse theme file at: {}", theme_path.display()))?;

    // Determine which theme to use
    let theme_name = theme_file.theme.as_deref().unwrap_or("default");

    let mut theme = if let Some(custom_theme) = theme_file.custom_themes.get(theme_name) {
        // First check if it's a custom theme
        let mut t = custom_theme.clone();
        t.name = theme_name.to_string();
        t
    } else if let Some(builtin_theme) = ThemeConfig::get_builtin(theme_name) {
        // Then check built-in themes
        builtin_theme
    } else {
        // Fallback to default if theme name not found
        log::warn!("Theme '{}' not found, using default", theme_name);
        ThemeConfig::default_theme()
    };

    // Apply global braille_mode override if set
    if let Some(global_braille) = theme_file.braille_mode {
        theme.braille_mode = global_braille;
    }

    Ok(theme)
}

/// Save a custom theme to the theme file
/// Creates the file if it doesn't exist
pub fn save_custom_theme(name: &str, theme: &ThemeConfig) -> Result<()> {
    let theme_path = theme_file_path()?;
    let config_dir = theme_path
        .parent()
        .context("Failed to get theme config directory")?;

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

    // Load existing theme file or create new one
    let mut theme_file = if theme_path.exists() {
        let theme_str = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed to read theme file: {}", theme_path.display()))?;
        toml::from_str(&theme_str).unwrap_or_default()
    } else {
        ThemeFile::default()
    };

    // Add/update custom theme
    let mut theme_to_save = theme.clone();
    theme_to_save.name = name.to_string();
    theme_file
        .custom_themes
        .insert(name.to_string(), theme_to_save);

    // Serialize and write
    let theme_str =
        toml::to_string_pretty(&theme_file).context("Failed to serialize theme file to TOML")?;

    fs::write(&theme_path, theme_str)
        .with_context(|| format!("Failed to write theme file: {}", theme_path.display()))?;

    log::info!("Saved custom theme '{}' to: {}", name, theme_path.display());
    Ok(())
}

/// Set the active theme in the theme file
pub fn set_active_theme(name: &str) -> Result<()> {
    let theme_path = theme_file_path()?;
    let config_dir = theme_path
        .parent()
        .context("Failed to get theme config directory")?;

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
    }

    // Load existing theme file or create new one
    let mut theme_file = if theme_path.exists() {
        let theme_str = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed to read theme file: {}", theme_path.display()))?;
        toml::from_str(&theme_str).unwrap_or_default()
    } else {
        ThemeFile::default()
    };

    theme_file.theme = Some(name.to_string());

    // Serialize and write
    let theme_str =
        toml::to_string_pretty(&theme_file).context("Failed to serialize theme file to TOML")?;

    fs::write(&theme_path, theme_str)
        .with_context(|| format!("Failed to write theme file: {}", theme_path.display()))?;

    log::info!(
        "Set active theme to '{}' in: {}",
        name,
        theme_path.display()
    );
    Ok(())
}

/// List available themes (built-in and custom)
pub fn list_available_themes() -> Result<(Vec<String>, Vec<String>)> {
    let theme_path = theme_file_path()?;

    let custom_themes = if theme_path.exists() {
        let theme_str = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed to read theme file: {}", theme_path.display()))?;
        let theme_file: ThemeFile = toml::from_str(&theme_str).unwrap_or_default();
        theme_file.custom_themes.keys().cloned().collect()
    } else {
        Vec::new()
    };

    let builtin_themes = vec![
        "default".to_string(),
        "monochrome".to_string(),
        "high-contrast".to_string(),
        "solarized-dark".to_string(),
        "dracula".to_string(),
        "gruvbox".to_string(),
    ];

    Ok((builtin_themes, custom_themes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_theme_default() {
        let theme = ThemeConfig::default_theme();
        assert_eq!(theme.name, "default");
        assert_eq!(theme.border_selected, "#22d3ee");
    }

    #[test]
    fn test_theme_get_builtin() {
        assert!(ThemeConfig::get_builtin("default").is_some());
        assert!(ThemeConfig::get_builtin("monochrome").is_some());
        assert!(ThemeConfig::get_builtin("dracula").is_some());
        assert!(ThemeConfig::get_builtin("nonexistent").is_none());
    }

    #[test]
    fn test_parse_color_valid() {
        let color = ThemeConfig::parse_color("#ff0000");
        assert_eq!(color, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_color_invalid() {
        let color = ThemeConfig::parse_color("invalid");
        assert_eq!(color, Color::White);
    }

    #[test]
    fn test_theme_color_helpers() {
        let theme = ThemeConfig::default_theme();
        assert_eq!(theme.border_selected_color(), Color::Rgb(34, 211, 238)); // #22d3ee
    }

    #[test]
    fn test_theme_file_serde() {
        let theme_file = ThemeFile {
            theme: Some("dracula".to_string()),
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&theme_file).expect("Failed to serialize");
        assert!(toml_str.contains("theme = \"dracula\""));

        let deserialized: ThemeFile = toml::from_str(&toml_str).expect("Failed to deserialize");
        assert_eq!(deserialized.theme, Some("dracula".to_string()));
    }

    #[test]
    fn test_all_builtin_themes_have_valid_colors() {
        let themes = vec![
            ThemeConfig::default_theme(),
            ThemeConfig::monochrome_theme(),
            ThemeConfig::high_contrast_theme(),
            ThemeConfig::solarized_dark_theme(),
            ThemeConfig::dracula_theme(),
            ThemeConfig::gruvbox_theme(),
        ];

        for theme in themes {
            // All colors should be valid hex
            assert!(theme.border_selected.starts_with('#'));
            assert!(theme.border_unselected.starts_with('#'));
            assert!(theme.sparkline.starts_with('#'));
            assert_eq!(theme.border_selected.len(), 7);
        }
    }

    #[test]
    fn test_theme_serde_roundtrip() {
        let theme = ThemeConfig::default_theme();
        let toml_str = toml::to_string_pretty(&theme).expect("Failed to serialize");
        let deserialized: ThemeConfig = toml::from_str(&toml_str).expect("Failed to deserialize");
        assert_eq!(theme, deserialized);
    }

    #[test]
    fn test_custom_theme_in_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let theme_file = temp_dir.path().join("theme.toml");

        // Create theme file with custom theme
        let custom_theme = ThemeConfig {
            name: "custom".to_string(),
            border_selected: "#ff00ff".to_string(),
            ..ThemeConfig::default_theme()
        };

        let theme_file_struct = ThemeFile {
            theme: Some("custom".to_string()),
            custom_themes: {
                let mut map = HashMap::new();
                map.insert("custom".to_string(), custom_theme.clone());
                map
            },
            braille_mode: None,
        };

        let toml_str = toml::to_string_pretty(&theme_file_struct).expect("Failed to serialize");
        fs::write(&theme_file, toml_str).expect("Failed to write");

        // Read back and verify
        let read_content = fs::read_to_string(&theme_file).expect("Failed to read");
        let loaded: ThemeFile = toml::from_str(&read_content).expect("Failed to parse");

        assert_eq!(loaded.theme, Some("custom".to_string()));
        assert!(loaded.custom_themes.contains_key("custom"));
        assert_eq!(loaded.custom_themes["custom"].border_selected, "#ff00ff");
    }
}
