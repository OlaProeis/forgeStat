use anyhow::Result;
use ratatui::{prelude::*, widgets::*};

use super::utils::centered_rect;
use super::{App, AppAction};

impl App {
    pub(super) fn toggle_command_palette(&mut self) {
        self.command_palette_mode = !self.command_palette_mode;
        if self.command_palette_mode {
            self.command_input.clear();
            self.command_selected_suggestion = 0;
            self.command_history_index = None;
            // Populate suggestions immediately with empty input (shows all commands)
            self.update_command_suggestions();
            log::info!("Command palette opened");
        } else {
            log::info!("Command palette closed");
        }
    }

    /// Exit command palette mode
    pub(super) fn exit_command_palette(&mut self) {
        self.command_palette_mode = false;
        self.command_input.clear();
        self.command_suggestions.clear();
        self.command_selected_suggestion = 0;
        self.command_history_index = None;
        log::info!("Command palette exited");
    }

    /// Add character to command input
    pub(super) fn add_command_char(&mut self, c: char) {
        self.command_input.push(c);
        self.command_history_index = None; // Reset history navigation when typing
        self.update_command_suggestions();
    }

    /// Remove last character from command input
    pub(super) fn backspace_command(&mut self) {
        self.command_input.pop();
        self.command_history_index = None;
        self.update_command_suggestions();
    }

    /// Update autocomplete suggestions based on current input
    fn update_command_suggestions(&mut self) {
        let input = self.command_input.trim().to_lowercase();

        // Check if we're showing subcommand options (e.g., after typing ":theme ")
        if input.starts_with(":theme ") || input == ":theme" {
            let themes = self.get_available_themes();
            self.command_suggestions = themes.iter().map(|t| format!(":theme {}", t)).collect();
            self.command_selected_suggestion = 0;
            return;
        }

        if input.starts_with(":layout ") || input == ":layout" {
            let layouts = self.get_available_layouts();
            self.command_suggestions = layouts.iter().map(|l| format!(":layout {}", l)).collect();
            self.command_selected_suggestion = 0;
            return;
        }

        let available_commands = self.get_available_commands();

        if input.is_empty() {
            // Show all commands when input is empty
            self.command_suggestions = available_commands;
        } else {
            // Normalize input: if user types "quit", also match ":quit"
            let normalized_input = if !input.starts_with(':')
                && !input.starts_with("theme ")
                && !input.starts_with("layout ")
            {
                format!(":{}", input)
            } else {
                input.clone()
            };

            self.command_suggestions = available_commands
                .into_iter()
                .filter(|cmd| {
                    let cmd_lower = cmd.to_lowercase();
                    // Match against both normalized and original input
                    cmd_lower.starts_with(&normalized_input) || cmd_lower.starts_with(&input)
                })
                .collect();
        }
        self.command_selected_suggestion = 0;
    }

    /// Get list of available commands
    fn get_available_commands(&self) -> Vec<String> {
        vec![
            ":refresh".to_string(),
            ":export".to_string(),
            ":theme <name>".to_string(),
            ":layout <preset>".to_string(),
            ":set-token".to_string(),
            ":quit".to_string(),
            ":help".to_string(),
        ]
    }

    /// Get available themes for autocomplete
    fn get_available_themes(&self) -> Vec<&'static str> {
        vec![
            "default",
            "monochrome",
            "high-contrast",
            "solarized-dark",
            "dracula",
            "gruvbox",
        ]
    }

    /// Get available layout presets for autocomplete
    fn get_available_layouts(&self) -> Vec<&'static str> {
        vec!["default", "compact", "wide"]
    }

    /// Autocomplete the current command
    pub(super) fn autocomplete_command(&mut self) {
        let input = self.command_input.trim().to_lowercase();

        // Check if we're autocompleting a theme or layout argument
        if input.starts_with(":theme ") {
            let partial = input[7..].trim().to_lowercase();
            let themes = self.get_available_themes();
            if let Some(theme) = themes.iter().find(|t| t.starts_with(&partial)) {
                self.command_input = format!(":theme {}", theme);
            }
        } else if input.starts_with(":layout ") {
            let partial = input[8..].trim().to_lowercase();
            let layouts = self.get_available_layouts();
            if let Some(layout) = layouts.iter().find(|l| l.starts_with(&partial)) {
                self.command_input = format!(":layout {}", layout);
            }
        } else if !self.command_suggestions.is_empty() {
            // Autocomplete the command itself
            let suggestion = &self.command_suggestions[self.command_selected_suggestion];
            // If suggestion has a placeholder like <name>, strip it and just use the base command
            let autocompleted = if suggestion.contains("<name>") || suggestion.contains("<preset>")
            {
                suggestion
                    .split_whitespace()
                    .next()
                    .unwrap_or(suggestion)
                    .to_string()
            } else {
                suggestion.clone()
            };
            self.command_input = autocompleted;
            // Update suggestions to show subcommand options if applicable
            self.update_command_suggestions();
        }
    }

    /// Navigate to previous command in history
    pub(super) fn command_history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        if let Some(index) = self.command_history_index {
            if index > 0 {
                self.command_history_index = Some(index - 1);
                self.command_input = self.command_history[index - 1].clone();
            }
        } else {
            // Start from the most recent command
            let last_idx = self.command_history.len() - 1;
            self.command_history_index = Some(last_idx);
            self.command_input = self.command_history[last_idx].clone();
        }
        self.update_command_suggestions();
    }

    /// Navigate to next command in history
    pub(super) fn command_history_next(&mut self) {
        if let Some(index) = self.command_history_index {
            if index < self.command_history.len() - 1 {
                self.command_history_index = Some(index + 1);
                self.command_input = self.command_history[index + 1].clone();
            } else {
                // Clear input when going past the most recent command
                self.command_history_index = None;
                self.command_input.clear();
            }
            self.update_command_suggestions();
        }
    }

    /// Navigate to previous suggestion
    pub(super) fn command_suggestion_prev(&mut self) {
        if !self.command_suggestions.is_empty() && self.command_selected_suggestion > 0 {
            self.command_selected_suggestion -= 1;
        }
    }

    /// Navigate to next suggestion
    pub(super) fn command_suggestion_next(&mut self) {
        if !self.command_suggestions.is_empty() {
            let max = self.command_suggestions.len().saturating_sub(1);
            if self.command_selected_suggestion < max {
                self.command_selected_suggestion += 1;
            }
        }
    }

    /// Check if we're currently showing subcommand options (theme or layout choices)
    pub(super) fn is_showing_subcommand_options(&self) -> bool {
        let input = self.command_input.trim().to_lowercase();
        input == ":theme"
            || input.starts_with(":theme ")
            || input == ":layout"
            || input.starts_with(":layout ")
    }

    /// Execute the current command and return an optional AppAction
    pub(super) fn execute_command(&mut self) -> Result<Option<AppAction>> {
        let mut input = self.command_input.trim();

        // Use the selected suggestion when input is empty or is just a base
        // command that expects a subcommand argument (e.g. ":theme" or ":layout"
        // without a specific value).
        let input_lower = input.to_lowercase();
        let is_base_command = input_lower == ":theme"
            || input_lower == ":layout"
            || input_lower == "theme"
            || input_lower == "layout";
        if (input.is_empty() || is_base_command) && !self.command_suggestions.is_empty() {
            input = self.command_suggestions[self.command_selected_suggestion].trim();
        }

        // Don't execute empty commands
        if input.is_empty() {
            return Ok(None);
        }

        // Add to history if not already the most recent
        if self.command_history.last().map(|s| s.as_str()) != Some(input) {
            self.command_history.push(input.to_string());
            // Keep history size reasonable (last 50 commands)
            if self.command_history.len() > 50 {
                self.command_history.remove(0);
            }
        }

        let cmd = input.to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(None);
        }

        // Normalize command - add colon prefix if missing
        let command = if !parts[0].starts_with(':') {
            format!(":{}", parts[0])
        } else {
            parts[0].to_string()
        };

        match command.as_str() {
            ":refresh" => {
                log::info!("Command palette: refresh");
                return Ok(Some(AppAction::Refresh));
            }
            ":export" => {
                log::info!("Command palette: export");
                // TODO: Implement export functionality
                self.show_toast("Export not yet implemented".to_string());
                return Ok(None);
            }
            ":theme" => {
                if parts.len() < 2 || parts[1].starts_with('<') {
                    // Switch to showing theme options instead of error
                    self.command_input = ":theme ".to_string();
                    self.update_command_suggestions();
                    return Ok(None);
                }
                let theme_name = parts[1];
                log::info!("Command palette: theme {}", theme_name);
                if let Some(theme) = crate::core::theme::ThemeConfig::get_builtin(theme_name) {
                    self.theme = theme;
                    crate::core::theme::set_active_theme(theme_name)?;
                    self.show_toast(format!("Theme changed to: {}", theme_name));
                } else {
                    return Err(anyhow::anyhow!("Unknown theme: {}. Available: default, monochrome, high-contrast, solarized-dark, dracula, gruvbox", theme_name));
                }
                return Ok(None);
            }
            ":layout" => {
                if parts.len() < 2 || parts[1].starts_with('<') {
                    // Switch to showing layout options instead of error
                    self.command_input = ":layout ".to_string();
                    self.update_command_suggestions();
                    return Ok(None);
                }
                let layout_name = parts[1];
                log::info!("Command palette: layout {}", layout_name);
                let preset = match layout_name {
                    "default" => crate::core::config::LayoutPreset::Default,
                    "compact" => crate::core::config::LayoutPreset::Compact,
                    "wide" => crate::core::config::LayoutPreset::Wide,
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unknown layout: {}. Available: default, compact, wide",
                            layout_name
                        ))
                    }
                };
                self.layout_config.reset_to_preset(preset);
                self.layout_config.save()?;
                self.show_toast(format!("Layout changed to: {}", layout_name));
                return Ok(None);
            }
            ":set-token" => {
                log::info!("Command palette: set-token");
                self.exit_command_palette();
                self.toggle_token_input();
                return Ok(None);
            }
            ":quit" | ":q" => {
                log::info!("Command palette: quit");
                return Ok(Some(AppAction::Quit));
            }
            ":help" => {
                log::info!("Command palette: help");
                self.show_help = true;
                return Ok(None);
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown command: {}. Try :refresh, :export, :theme <name>, :layout <preset>, :set-token, :quit, :help", input));
            }
        }
    }

    /// Render the command palette modal
    pub(super) fn render_command_palette(&self, frame: &mut Frame) {
        let area = frame.area();

        // Create a centered modal that takes up 80% of width and 40% of height
        let palette_area = centered_rect(80, 40, area);

        frame.render_widget(Clear, palette_area);

        // Split into input area, suggestions area, and help text
        let [input_area, suggestions_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .areas(palette_area);

        // Render input block with command
        let input_text = format!("{}█", self.command_input);
        let input_block = Block::bordered()
            .title(" Command Palette ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let input_paragraph = Paragraph::new(input_text)
            .block(input_block)
            .alignment(Alignment::Left);

        frame.render_widget(input_paragraph, input_area);

        // Render suggestions
        let suggestions_block = Block::bordered()
            .title(format!(
                " Suggestions ({}) ",
                self.command_suggestions.len()
            ))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.border_unselected_color()));

        let inner_area = suggestions_block.inner(suggestions_area);
        frame.render_widget(suggestions_block, suggestions_area);

        // Render suggestion items
        let visible_count = inner_area.height as usize;
        let lines: Vec<Line> = if self.command_suggestions.is_empty() {
            vec![Line::from(vec![Span::styled(
                "  Type a command...",
                Style::default().fg(self.theme.text_secondary_color()),
            )])]
        } else {
            self.command_suggestions
                .iter()
                .take(visible_count)
                .enumerate()
                .map(|(i, cmd)| {
                    let is_selected = i == self.command_selected_suggestion;
                    let style = if is_selected {
                        Style::default()
                            .fg(self.theme.text_highlight_color())
                            .bold()
                    } else {
                        Style::default().fg(self.theme.text_primary_color())
                    };

                    let prefix = if is_selected { "> " } else { "  " };
                    Line::from(vec![Span::styled(format!("{}{}", prefix, cmd), style)])
                })
                .collect()
        };

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner_area);

        // Render help text
        let help_text =
            "Tab: autocomplete | ↑/↓: select | Ctrl+↑/↓: history | Enter: execute | Esc: close";
        let help_paragraph = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .fg(self.theme.text_secondary_color());
        frame.render_widget(help_paragraph, help_area);
    }
}
