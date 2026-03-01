use std::time::Duration;

use anyhow::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind},
    DefaultTerminal,
};

use super::{App, AppAction, BorderType, Panel, AUTO_REFRESH_SECS};

/// Runs the TUI event loop until the user quits or requests a refresh.
/// Auto-refreshes every 10 minutes.
pub fn run_event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<AppAction> {
    loop {
        // Update animations and redraw if needed
        let needs_redraw = app.update_animations();

        // Always draw on first iteration or when animations require it
        terminal.draw(|frame| app.render(frame))?;

        if app.last_refresh.elapsed() >= Duration::from_secs(AUTO_REFRESH_SECS) {
            return Ok(AppAction::Refresh);
        }

        // Use shorter poll interval when animations are active
        let poll_duration = if needs_redraw {
            Duration::from_millis(16) // ~60fps for smooth animations
        } else {
            Duration::from_millis(250) // Normal poll rate when idle
        };

        if event::poll(poll_duration)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Handle fuzzy mode first - highest priority
                    if app.fuzzy_mode {
                        match key.code {
                            KeyCode::Esc => {
                                app.exit_fuzzy_mode();
                                continue;
                            }
                            KeyCode::Enter => {
                                if let Some(repo) = app.get_selected_fuzzy_repo() {
                                    break Ok(AppAction::SwitchRepo(
                                        repo.owner.clone(),
                                        repo.name.clone(),
                                    ));
                                }
                                app.exit_fuzzy_mode();
                                continue;
                            }
                            KeyCode::Char(c) => {
                                app.add_fuzzy_char(c);
                                continue;
                            }
                            KeyCode::Backspace => {
                                app.backspace_fuzzy();
                                continue;
                            }
                            KeyCode::Up => {
                                app.fuzzy_prev();
                                continue;
                            }
                            KeyCode::Down => {
                                app.fuzzy_next();
                                continue;
                            }
                            _ => {
                                if key.modifiers.contains(KeyModifiers::CONTROL)
                                    && (matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
                                {
                                    app.exit_fuzzy_mode();
                                    continue;
                                }
                            }
                        }
                    }

                    // Handle command palette mode - high priority
                    if app.command_palette_mode {
                        match key.code {
                            KeyCode::Esc => {
                                app.exit_command_palette();
                                continue;
                            }
                            KeyCode::Enter => {
                                let was_showing_subcommands = app.is_showing_subcommand_options();
                                log::info!("Command palette Enter pressed. Input: {:?}, Selected: {}, Was showing subcommands: {}", 
                                    app.command_input, app.command_selected_suggestion, was_showing_subcommands);
                                let result = app.execute_command();
                                log::info!(
                                    "Execute result: {:?}, Now showing subcommands: {}",
                                    result,
                                    app.is_showing_subcommand_options()
                                );
                                match result {
                                    Ok(Some(action)) => break Ok(action),
                                    Ok(None) => {
                                        let entered_subcommand_mode = !was_showing_subcommands
                                            && app.is_showing_subcommand_options();
                                        log::info!(
                                            "Should exit palette: {}",
                                            !entered_subcommand_mode
                                        );
                                        if !entered_subcommand_mode {
                                            app.exit_command_palette();
                                        }
                                        continue;
                                    }
                                    Err(e) => {
                                        app.show_toast(format!("Error: {}", e));
                                        app.exit_command_palette();
                                        continue;
                                    }
                                }
                            }
                            KeyCode::Char(':') => {
                                continue;
                            }
                            KeyCode::Char(c) => {
                                app.add_command_char(c);
                                continue;
                            }
                            KeyCode::Backspace => {
                                app.backspace_command();
                                continue;
                            }
                            KeyCode::Tab => {
                                app.autocomplete_command();
                                continue;
                            }
                            KeyCode::Up => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.command_history_prev();
                                } else {
                                    app.command_suggestion_prev();
                                }
                                continue;
                            }
                            KeyCode::Down => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.command_history_next();
                                } else {
                                    app.command_suggestion_next();
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle token input mode - high priority
                    if app.token_input_mode {
                        match key.code {
                            KeyCode::Esc => {
                                app.exit_token_input();
                                continue;
                            }
                            KeyCode::Enter => match app.save_token() {
                                Ok(Some(action)) => break Ok(action),
                                Ok(None) => continue,
                                Err(e) => {
                                    app.show_toast(format!("Error: {}", e));
                                    app.exit_token_input();
                                    continue;
                                }
                            },
                            KeyCode::Char(c) => {
                                app.add_token_char(c);
                                continue;
                            }
                            KeyCode::Backspace => {
                                app.backspace_token();
                                continue;
                            }
                            KeyCode::Tab => {
                                app.toggle_token_mask();
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle search mode - special input handling
                    if app.search_mode {
                        match key.code {
                            KeyCode::Esc => {
                                app.clear_search();
                                continue;
                            }
                            KeyCode::Enter => {
                                app.exit_search_mode();
                                continue;
                            }
                            KeyCode::Char(c) => {
                                app.add_search_char(c);
                                continue;
                            }
                            KeyCode::Backspace => {
                                app.backspace_search();
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle diff mode - Esc exits diff mode
                    if app.is_diff_mode() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('d') => {
                                app.exit_diff_mode();
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle compare mode - highest priority when active
                    if app.is_compare_mode() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.exit_compare_mode();
                                continue;
                            }
                            KeyCode::Tab => {
                                app.toggle_compare_focus();
                                continue;
                            }
                            KeyCode::Char('r') => {
                                return Ok(AppAction::Refresh);
                            }
                            _ => {
                                // In compare mode, only allow specific keys
                                continue;
                            }
                        }
                    }

                    // Handle zoom mode - Esc and Enter exit zoom
                    if app.is_zoomed() {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                app.exit_zoom();
                                continue;
                            }
                            KeyCode::Down => {
                                app.scroll_down();
                                continue;
                            }
                            KeyCode::Up => {
                                app.scroll_up();
                                continue;
                            }
                            KeyCode::Char('/') => {
                                if app.zoom_panel == Some(Panel::Issues)
                                    || app.zoom_panel == Some(Panel::Contributors)
                                    || app.zoom_panel == Some(Panel::Releases)
                                {
                                    app.toggle_search();
                                }
                                continue;
                            }
                            KeyCode::Char('c') => {
                                app.copy_to_clipboard();
                                continue;
                            }
                            KeyCode::Char('s') => {
                                if app.zoom_panel == Some(Panel::Issues) {
                                    app.cycle_issues_sort();
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Handle Escape to close overlays first (help takes priority)
                    if key.code == KeyCode::Esc {
                        if app.show_help {
                            app.toggle_help();
                            continue;
                        }
                        if app.show_mini_map {
                            app.toggle_mini_map();
                            continue;
                        }
                    }

                    match key.code {
                        KeyCode::Char('q') => return Ok(AppAction::Quit),
                        KeyCode::Char('r') => return Ok(AppAction::Refresh),
                        KeyCode::Tab => app.next_panel(),
                        KeyCode::BackTab => app.prev_panel(),
                        KeyCode::Char('?') => app.toggle_help(),
                        KeyCode::Right => app.next_panel(),
                        KeyCode::Left => app.prev_panel(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Char('+') | KeyCode::Char(']') => match app.selected_panel {
                            Panel::Stars => app.cycle_star_timeframe_forward(),
                            Panel::Velocity => app.cycle_velocity_timeframe_forward(),
                            Panel::Contributors => app.cycle_contributors_limit_forward(),
                            Panel::Releases => app.cycle_releases_limit_forward(),
                            Panel::Issues => app.cycle_issues_per_page_forward(),
                            Panel::PullRequests => app.cycle_prs_per_page_forward(),
                            Panel::Security => {}
                            Panel::CI => {}
                        },
                        KeyCode::Char('-') | KeyCode::Char('[') => match app.selected_panel {
                            Panel::Stars => app.cycle_star_timeframe_backward(),
                            Panel::Velocity => app.cycle_velocity_timeframe_backward(),
                            Panel::Contributors => app.cycle_contributors_limit_backward(),
                            Panel::Releases => app.cycle_releases_limit_backward(),
                            Panel::Issues => app.cycle_issues_per_page_backward(),
                            Panel::PullRequests => app.cycle_prs_per_page_backward(),
                            Panel::Security => {}
                            Panel::CI => {}
                        },
                        KeyCode::Char('=') => {
                            app.reset_layout();
                        }
                        KeyCode::Char('m') => {
                            app.toggle_mini_map();
                        }
                        KeyCode::Char('f') => {
                            app.toggle_fuzzy_mode();
                        }
                        KeyCode::Char('d') => {
                            app.toggle_diff_mode();
                        }
                        KeyCode::Char('1') => app.jump_to_panel(1),
                        KeyCode::Char('2') => app.jump_to_panel(2),
                        KeyCode::Char('3') => app.jump_to_panel(3),
                        KeyCode::Char('4') => app.jump_to_panel(4),
                        KeyCode::Char('5') => app.jump_to_panel(5),
                        KeyCode::Char('6') => app.jump_to_panel(6),
                        KeyCode::Char('7') => app.jump_to_panel(7),
                        KeyCode::Char('8') => app.jump_to_panel(8),
                        KeyCode::Enter => {
                            app.toggle_zoom();
                        }
                        KeyCode::Char('/') => {
                            if app.selected_panel == Panel::Issues
                                || app.selected_panel == Panel::Contributors
                                || app.selected_panel == Panel::Releases
                            {
                                app.toggle_search();
                            }
                        }
                        KeyCode::Char('l') => {
                            if app.selected_panel == Panel::Issues {
                                app.cycle_issues_label_filter();
                            }
                        }
                        KeyCode::Char('p') => {
                            if app.selected_panel == Panel::Releases {
                                app.cycle_releases_prerelease_filter();
                            }
                        }
                        KeyCode::Char('c') => {
                            app.copy_to_clipboard();
                        }
                        KeyCode::Char(':') => {
                            app.toggle_command_palette();
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::Down(_) => {
                        if app.is_zoomed() {
                            app.exit_zoom();
                        } else {
                            if let Some(border_idx) =
                                app.get_vertical_border_at(mouse.column, mouse.row)
                            {
                                app.start_border_drag(
                                    BorderType::Vertical,
                                    border_idx,
                                    mouse.column,
                                    mouse.row,
                                );
                            } else if let Some(border_idx) =
                                app.get_horizontal_border_at(mouse.column, mouse.row)
                            {
                                app.start_border_drag(
                                    BorderType::Horizontal,
                                    border_idx,
                                    mouse.column,
                                    mouse.row,
                                );
                            } else {
                                app.handle_mouse_click(mouse.column, mouse.row);
                            }
                        }
                    }
                    MouseEventKind::Drag(_) => {
                        app.handle_drag(mouse.column, mouse.row);
                    }
                    MouseEventKind::Up(_) => {
                        app.end_drag();
                    }
                    MouseEventKind::ScrollDown => {
                        app.scroll_down();
                    }
                    MouseEventKind::ScrollUp => {
                        app.scroll_up();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
