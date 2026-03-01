# Config Management

## Overview

Configuration and GitHub Personal Access Token (PAT) management for Repowatch. Handles secure token storage with cross-platform config directories.

## Key Files

- `src/core/config.rs` - Token loading, saving, and config file operations

## Implementation Details

### Token Loading Precedence

1. `GITHUB_TOKEN` environment variable (checked first)
2. Platform config directory (`~/.config/forgeStat/config.toml` or equivalent)

### Platform Config Paths

| Platform | Config Path |
|----------|-------------|
| Windows | `%APPDATA%\forgeStat\config.toml` |
| macOS | `~/Library/Application Support/forgeStat/config.toml` |
| Linux | `~/.config/forgeStat/config.toml` |

### API Functions

```rust
// Load token from env or config file
pub fn load_token() -> Result<String>

// Save token to config file
pub fn save_token(token: &str) -> Result<()

// Clear token from config file
pub fn clear_token() -> Result<()>

// Check if token is available
pub fn has_token() -> bool

// Get config file path
pub fn config_file_path() -> Result<PathBuf>
```

### Config File Format (TOML)

```toml
github_token = "ghp_your_token_here"
```

## Dependencies Used

- `dirs` - Cross-platform config directory detection
- `toml` - TOML serialization/deserialization
- `serde` - Derive macros for Config struct
- `anyhow` - Error handling with context

## Usage

```rust
use forgeStat::core::config;

// Load token (env var takes precedence)
let token = config::load_token()?;

// Save token to config file
config::save_token("ghp_your_token")?;

// Check if authenticated
if config::has_token() {
    println!("Authenticated mode available");
}
```

## Error Handling

- Returns `anyhow::Result` with context on all operations
- Missing env var or config file results in descriptive error
- File I/O errors include the file path in the error message
