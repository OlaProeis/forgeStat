use ratatui::{prelude::*, widgets::*};

use super::utils::centered_rect;
use super::App;

impl App {
    pub(super) fn render_help_overlay(&self, frame: &mut Frame) {
        // Use 85% width and 90% height for the help overlay
        let area = centered_rect(85, 90, frame.area());
        frame.render_widget(Clear, area);

        // Main block with title
        let block = Block::bordered()
            .title(" Help ─ Press ? or Esc to close ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        // Render the block first to get the inner area
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split the inner area into sections
        let [top_section, cmd_section, search_section] = Layout::vertical([
            Constraint::Length(14),  // Three columns section
            Constraint::Length(7),   // Command palette section
            Constraint::Length(4),   // Search/filter section
        ])
        .margin(1)
        .areas(inner);

        // Render the title
        let title = Paragraph::new("Keyboard Shortcuts Reference")
            .style(Style::default().fg(self.theme.help_title_color()).bold())
            .alignment(Alignment::Center);
        frame.render_widget(title, top_section);

        // Split top section into three columns (below the title)
        let title_height = 2; // Title + empty line
        let cols_area = top_section.inner(Margin::new(0, title_height));

        let [col1_area, col2_area, col3_area] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(cols_area);

        // Render each column
        let col1 = Paragraph::new(self.build_global_shortcuts_text())
            .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(self.theme.help_border_color())));
        frame.render_widget(col1, col1_area);

        let col2 = Paragraph::new(self.build_navigation_shortcuts_text())
            .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(self.theme.help_border_color())));
        frame.render_widget(col2, col2_area);

        let col3 = Paragraph::new(self.build_zoom_mouse_shortcuts_text())
            .block(Block::default());
        frame.render_widget(col3, col3_area);

        // Render command palette section
        let cmd_text = self.build_command_palette_text();
        let cmd_para = Paragraph::new(cmd_text)
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(self.theme.help_border_color())));
        frame.render_widget(cmd_para, cmd_section);

        // Render search/filter section
        let search_text = self.build_search_filter_text();
        let search_para = Paragraph::new(search_text)
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(self.theme.help_border_color())));
        frame.render_widget(search_para, search_section);
    }

    /// Build Global shortcuts as text
    fn build_global_shortcuts_text(&self) -> Text<'static> {
        use ratatui::text::Span;

        let lines = vec![
            Line::from(vec![
                Span::styled("Global", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("?", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Toggle help", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Quit app", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("r", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Refresh data", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled(":", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Command palette", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("f", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Fuzzy finder", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("d", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Diff mode", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("m", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Mini-map", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("=", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("          Reset layout", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("1-8", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("        Jump panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
        ];

        Text::from(lines)
    }

    /// Build Navigation shortcuts as text
    fn build_navigation_shortcuts_text(&self) -> Text<'static> {
        use ratatui::text::Span;

        let lines = vec![
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("Tab / →", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("    Next panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Shift+Tab / ←", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled(" Prev panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("↑ / ↓", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("      Scroll lists", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("      Zoom panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Panel Adjust", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("+ / ]", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("    Increase limit/time", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("- / [", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("    Decrease limit/time", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("(Stars, Vel, Cont, Rel, Iss, PR)", Style::default().fg(self.theme.text_secondary_color()).italic()),
            ]),
        ];

        Text::from(lines)
    }

    /// Build Zoom/Mouse shortcuts as text
    fn build_zoom_mouse_shortcuts_text(&self) -> Text<'static> {
        use ratatui::text::Span;

        let lines = vec![
            Line::from(vec![
                Span::styled("Zoom Mode", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("Enter/Esc", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled(" Close zoom", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("↑ / ↓", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("     Scroll zoomed view", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("/", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("           Search in zoom", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("c", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("           Copy to clipboard", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Mouse", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("Click", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("       Select panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Double-click", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled(" Zoom panel", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Drag border", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled(" Resize panels", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("Scroll", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("      Scroll lists", Style::default().fg(self.theme.text_primary_color())),
            ]),
        ];

        Text::from(lines)
    }

    /// Build Command Palette section as text
    fn build_command_palette_text(&self) -> Text<'static> {
        use ratatui::text::Span;

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Command Palette (press : to open)", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
        ];

        // First row of commands
        lines.push(Line::from(vec![
            Span::styled(":refresh", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled("     Refresh data   ", Style::default().fg(self.theme.text_primary_color())),
            Span::styled("  :export", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled("       Export data", Style::default().fg(self.theme.text_primary_color())),
        ]));

        // Second row
        lines.push(Line::from(vec![
            Span::styled(":theme <name>", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled("  Switch theme     ", Style::default().fg(self.theme.text_primary_color())),
            Span::styled("  :layout <preset>", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled(" Switch layout", Style::default().fg(self.theme.text_primary_color())),
        ]));

        // Third row
        lines.push(Line::from(vec![
            Span::styled(":quit", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled("        Exit app       ", Style::default().fg(self.theme.text_primary_color())),
            Span::styled("  :help", Style::default().fg(self.theme.text_highlight_color()).bold()),
            Span::styled("         Show help", Style::default().fg(self.theme.text_primary_color())),
        ]));

        // Themes line
        lines.push(Line::from(vec![
            Span::styled("Themes: ", Style::default().fg(self.theme.text_secondary_color())),
            Span::styled("default, monochrome, high-contrast, solarized-dark, dracula, gruvbox", Style::default().fg(self.theme.text_secondary_color()).italic()),
        ]));

        Text::from(lines)
    }

    /// Build Search/Filter section as text
    fn build_search_filter_text(&self) -> Text<'static> {
        use ratatui::text::Span;

        let lines = vec![
            Line::from(vec![
                Span::styled("Panel-Specific Search & Filter", Style::default().fg(self.theme.help_title_color()).bold()),
            ]),
            Line::from(vec![
                Span::styled("/", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("           Open search (Issues, Contributors, Releases)", Style::default().fg(self.theme.text_primary_color())),
                Span::styled("   Esc / c", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("  Clear search/filter", Style::default().fg(self.theme.text_primary_color())),
            ]),
            Line::from(vec![
                Span::styled("l", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("           Cycle label filter (Issues only)", Style::default().fg(self.theme.text_primary_color())),
                Span::styled("                p", Style::default().fg(self.theme.text_highlight_color()).bold()),
                Span::styled("         Cycle pre-release filter (Releases)", Style::default().fg(self.theme.text_primary_color())),
            ]),
        ];

        Text::from(lines)
    }
}
