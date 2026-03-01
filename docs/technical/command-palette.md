# Command Palette

Vim-style command palette with autocomplete and history, accessible via the `:` key.

## Overview

The command palette provides quick access to common actions through typed commands. It features:

- **Instant command list** - All commands shown immediately when pressing `:`
- **Smart filtering** - Type to filter the command list
- **Subcommand hints** - Commands with arguments show available options
- **Arrow key selection** - Navigate suggestions with Up/Down arrows
- **Tab autocomplete** - Complete commands and arguments
- **Command history** - Navigate previous commands with Ctrl+Up/Down
- **Flexible input** - Commands work with or without the `:` prefix
- **Error feedback** - Toast notifications for invalid commands

## Usage

1. Press `:` to open the command palette (shows all available commands)
2. Type to filter commands (e.g., `th` shows `:theme`)
3. Press `↑`/`↓` to select, then `Enter` to execute
4. For commands with arguments (like `:theme`), the palette shows available options
5. Press `Esc` to close without executing

## Available Commands

| Command | Shorthand | Description | Arguments |
|---------|-----------|-------------|-----------|
| `:refresh` | `refresh` | Refresh repository data from GitHub API | None |
| `:export` | `export` | Export repository data (placeholder) | None |
| `:theme <name>` | `theme <name>` | Switch color theme | See themes below |
| `:layout <preset>` | `layout <preset>` | Switch layout preset | See layouts below |
| `:quit` | `quit`, `q` | Exit the application | None |
| `:help` | `help` | Show the help overlay | None |

### Themes

When you type `:theme` or select it, the palette shows all available themes:

- `default` - Standard color scheme
- `monochrome` - Grayscale only
- `high-contrast` - Bold colors for visibility
- `solarized-dark` - Solarized color palette
- `dracula` - Dracula theme
- `gruvbox` - Gruvbox dark theme

**Usage:** Type `:theme d` + `Tab` → `:theme default`

### Layouts

When you type `:layout` or select it, the palette shows all available presets:

- `default` - Balanced panel sizes
- `compact` - Smaller panels, more content
- `wide` - Larger panels for readability

**Usage:** Type `:layout c` + `Tab` → `:layout compact`

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `:` | Open command palette |
| `Esc` | Close palette without executing |
| `Enter` | Execute selected/current command |
| `Tab` | Autocomplete command or argument |
| `↑` / `↓` | Navigate through suggestions |
| `Ctrl + ↑` / `Ctrl + ↓` | Navigate through command history |
| `Backspace` | Delete last character |
| Any character | Type into input |

## Workflow Examples

### Changing Theme

1. Press `:`
2. Type `theme` (list filters to show `:theme`)
3. Press `Enter` or `↓` to select
4. The list updates to show all available themes:
   ```
   > :theme default
     :theme monochrome
     :theme high-contrast
     ...
   ```
5. Navigate with `↑`/`↓` and press `Enter` to apply

### Quick Layout Switch

1. Press `:`
2. Type `lay comp` (filters to `:layout compact`)
3. Press `Tab` to autocomplete
4. Press `Enter` to apply

### Using History

1. Execute a few commands (e.g., `:theme dracula`, `:layout compact`)
2. Press `:` then `Ctrl+↑` to recall previous commands
3. Press `Enter` to re-execute

## Implementation

### State Management

```rust
command_palette_mode: bool,        // Whether palette is open
command_input: String,             // Current input text
command_history: Vec<String>,      // History of executed commands (last 50)
command_history_index: Option<usize>, // Current position in history
command_suggestions: Vec<String>,  // Current autocomplete suggestions
command_selected_suggestion: usize, // Selected suggestion index
```

### Smart Subcommand Detection

The palette detects when you're typing a command with arguments:

```rust
if input.starts_with(":theme ") || input == ":theme" {
    // Show all available themes as suggestions
    self.command_suggestions = themes.iter()
        .map(|t| format!(":theme {}", t))
        .collect();
}
```

### Command Execution

When input is empty but a suggestion is selected, the palette uses the selected suggestion:

```rust
// If input is empty but there's a selected suggestion, use that
if input.is_empty() && !self.command_suggestions.is_empty() {
    input = self.command_suggestions[self.command_selected_suggestion].trim();
}
```

### Rendering

The command palette renders as a centered modal with three sections:

1. **Input area** - Shows current command with cursor
2. **Suggestions area** - Lists matching commands/options with selection indicator (`>`)
3. **Help text** - Shows keyboard shortcuts

## Files

- `src/tui/app/command_palette.rs` — Command palette logic, suggestions, history, rendering
- `src/tui/app/mod.rs` — App state fields for command palette
- `src/tui/app/event_loop.rs` — Key binding dispatch for `:` mode
