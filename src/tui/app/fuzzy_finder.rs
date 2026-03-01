use ratatui::{prelude::*, widgets::*};

use super::utils::centered_rect;
use super::App;
use crate::core::cache::CachedRepoInfo;

impl App {
    pub(super) fn toggle_fuzzy_mode(&mut self) {
        self.fuzzy_mode = !self.fuzzy_mode;
        if self.fuzzy_mode {
            self.fuzzy_query.clear();
            self.fuzzy_selected_index = 0;
            self.fuzzy_repos = Vec::new();
        }
    }

    fn load_fuzzy_repos(&mut self) {
        if self.fuzzy_mode && self.fuzzy_repos.is_empty() {
            let repos_result = std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    rt.block_on(async { crate::core::cache::Cache::scan_all_repos().await.ok() })
                })
                .join()
                .ok()
                .flatten()
            });
            if let Some(repos) = repos_result {
                self.fuzzy_repos = repos;
            }
        }
    }

    fn get_filtered_fuzzy_repos(&self) -> Vec<(usize, &CachedRepoInfo)> {
        if self.fuzzy_query.is_empty() {
            return self.fuzzy_repos.iter().enumerate().collect();
        }

        let query_lower = self.fuzzy_query.to_lowercase();
        self.fuzzy_repos
            .iter()
            .enumerate()
            .filter(|(_, repo)| {
                let name_match = repo.full_name.to_lowercase().contains(&query_lower);
                let desc_match = repo
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                name_match || desc_match
            })
            .collect()
    }

    pub(super) fn add_fuzzy_char(&mut self, c: char) {
        self.fuzzy_query.push(c);
        self.fuzzy_selected_index = 0;
    }

    pub(super) fn backspace_fuzzy(&mut self) {
        self.fuzzy_query.pop();
        self.fuzzy_selected_index = 0;
    }

    pub(super) fn exit_fuzzy_mode(&mut self) {
        self.fuzzy_mode = false;
    }

    pub(super) fn fuzzy_prev(&mut self) {
        if self.fuzzy_selected_index > 0 {
            self.fuzzy_selected_index -= 1;
        }
    }

    pub(super) fn fuzzy_next(&mut self) {
        let max = self.get_filtered_fuzzy_repos().len();
        if self.fuzzy_selected_index < max.saturating_sub(1) {
            self.fuzzy_selected_index += 1;
        }
    }

    pub(super) fn get_selected_fuzzy_repo(&self) -> Option<&CachedRepoInfo> {
        let filtered = self.get_filtered_fuzzy_repos();
        filtered
            .get(self.fuzzy_selected_index)
            .map(|(_, repo)| *repo)
    }

    pub(super) fn render_fuzzy_overlay(&mut self, frame: &mut Frame) {
        self.load_fuzzy_repos();

        let area = frame.area();
        let fuzzy_area = centered_rect(80, 70, area);

        frame.render_widget(Clear, fuzzy_area);

        let [input_area, results_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(fuzzy_area);

        let input_text = format!("{}█", self.fuzzy_query);
        let input_block = Block::bordered()
            .title(" Fuzzy Finder - Select Repository ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let input_paragraph = Paragraph::new(input_text)
            .block(input_block)
            .alignment(Alignment::Left);

        frame.render_widget(input_paragraph, input_area);

        let filtered_items = self.get_filtered_fuzzy_repos();

        let results_block = Block::bordered()
            .title(format!(" Results ({}) ", filtered_items.len()))
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(self.theme.help_border_color()));

        let inner_area = results_block.inner(results_area);
        frame.render_widget(results_block, results_area);

        let visible_count = inner_area.height as usize;
        let start_idx = if self.fuzzy_selected_index > visible_count / 2 {
            self.fuzzy_selected_index.saturating_sub(visible_count / 2)
        } else {
            0
        };
        let end_idx = (start_idx + visible_count).min(filtered_items.len());

        let lines: Vec<Line> = if filtered_items.is_empty() {
            vec![Line::from(vec![Span::styled(
                if self.fuzzy_repos.is_empty() {
                    "  Loading repositories...".to_string()
                } else {
                    "  No repositories match".to_string()
                },
                Style::default().fg(self.theme.text_secondary_color()),
            )])]
        } else {
            filtered_items[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(i, (_, repo))| {
                    let actual_idx = start_idx + i;
                    let is_selected = actual_idx == self.fuzzy_selected_index;

                    let desc = repo.description.as_deref().unwrap_or("");
                    let last_viewed = repo
                        .last_viewed_at
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "never".to_string());

                    let text = if desc.is_empty() {
                        format!("{}  (viewed: {})", repo.full_name, last_viewed)
                    } else {
                        format!(
                            "{} - {}  (viewed: {})",
                            repo.full_name,
                            desc.chars().take(40).collect::<String>(),
                            last_viewed
                        )
                    };

                    let style = if is_selected {
                        Style::default()
                            .fg(self.theme.text_highlight_color())
                            .bold()
                    } else {
                        Style::default().fg(self.theme.text_primary_color())
                    };

                    Line::from(vec![Span::styled(
                        if is_selected {
                            format!("> {}", text)
                        } else {
                            format!("  {}", text)
                        },
                        style,
                    )])
                })
                .collect()
        };

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner_area);
    }
}
