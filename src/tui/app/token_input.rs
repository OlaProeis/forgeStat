use anyhow::Result;
use ratatui::{prelude::*, widgets::*};

use super::utils::centered_rect;
use super::{App, AppAction};

impl App {
    /// Toggle token input dialog
    pub(super) fn toggle_token_input(&mut self) {
        self.token_input_mode = !self.token_input_mode;
        if self.token_input_mode {
            self.token_input.clear();
            self.token_error = None;
            log::info!("Token input dialog opened");
        } else {
            log::info!("Token input dialog closed");
        }
    }

    /// Exit token input mode
    pub(super) fn exit_token_input(&mut self) {
        self.token_input_mode = false;
        self.token_input.clear();
        self.token_error = None;
        log::info!("Token input dialog exited");
    }

    /// Add character to token input
    pub(super) fn add_token_char(&mut self, c: char) {
        self.token_input.push(c);
        self.token_error = None; // Clear error when user types
    }

    /// Remove last character from token input
    pub(super) fn backspace_token(&mut self) {
        self.token_input.pop();
        self.token_error = None;
    }

    /// Toggle password masking
    pub(super) fn toggle_token_mask(&mut self) {
        self.token_input_masked = !self.token_input_masked;
    }

    /// Save the entered token to config
    pub(super) fn save_token(&mut self) -> Result<Option<AppAction>> {
        let token = self.token_input.trim();

        if token.is_empty() {
            self.token_error = Some("Token cannot be empty".to_string());
            return Ok(None);
        }

        // Basic validation - GitHub tokens typically start with 'ghp_' or 'github_pat_'
        if !token.starts_with("ghp_") && !token.starts_with("github_pat_") {
            self.token_error = Some(
                "Token should start with 'ghp_' (classic) or 'github_pat_' (fine-grained)"
                    .to_string(),
            );
            return Ok(None);
        }

        // Save to config file
        match crate::core::config::save_token(token) {
            Ok(_) => {
                self.show_toast("GitHub token saved successfully!".to_string());
                self.exit_token_input();
                // Return Refresh action to reload with new token
                Ok(Some(AppAction::Refresh))
            }
            Err(e) => {
                self.token_error = Some(format!("Failed to save token: {}", e));
                Ok(None)
            }
        }
    }

    /// Render the token input dialog
    pub(super) fn render_token_input(&self, frame: &mut Frame) {
        let area = centered_rect(60, 40, frame.area());

        // Clear background
        frame.render_widget(Clear, area);

        // Create block with title
        let block = Block::default()
            .title(" Set GitHub Token ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border_selected_color()));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Build content
        let mut text_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Enter your GitHub Personal Access Token to access:",
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "  • Higher rate limits (5,000 req/hr vs 60)",
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
            Line::from(vec![Span::styled(
                "  • Private repository access",
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
            Line::from(vec![Span::styled(
                "  • Security alerts and CI status",
                Style::default().fg(self.theme.text_secondary_color()),
            )]),
            Line::from(""),
        ];

        // Add input field display
        let display_token = if self.token_input_masked {
            "•".repeat(self.token_input.len())
        } else {
            self.token_input.clone()
        };

        let input_line = if self.token_input.is_empty() {
            Line::from(vec![
                Span::styled(
                    "Token: ",
                    Style::default().fg(self.theme.text_primary_color()),
                ),
                Span::styled(
                    "ghp_... or github_pat_...",
                    Style::default()
                        .fg(self.theme.text_secondary_color())
                        .add_modifier(Modifier::DIM),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "Token: ",
                    Style::default().fg(self.theme.text_primary_color()),
                ),
                Span::styled(
                    display_token,
                    Style::default().fg(self.theme.text_primary_color()),
                ),
            ])
        };
        text_lines.push(input_line);

        text_lines.push(Line::from(""));

        // Add error message if present
        if let Some(ref error) = self.token_error {
            text_lines.push(Line::from(vec![
                Span::styled(
                    "✗ ",
                    Style::default().fg(self.theme.indicator_error_color()),
                ),
                Span::styled(
                    error.clone(),
                    Style::default().fg(self.theme.indicator_error_color()),
                ),
            ]));
            text_lines.push(Line::from(""));
        }

        // Add instructions
        text_lines.push(Line::from(vec![Span::styled(
            "Press Enter to save, Esc to cancel, Tab to toggle mask",
            Style::default()
                .fg(self.theme.text_secondary_color())
                .add_modifier(Modifier::DIM),
        )]));

        let paragraph = Paragraph::new(Text::from(text_lines))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, inner);
    }
}
