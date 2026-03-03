use anyhow::Result;
use clap::Parser;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::ExecutableCommand;
use std::io::{self, stdout, BufRead, Write};
use std::path::PathBuf;

use forgeStat::cli::report::generate_report;
use forgeStat::cli::summary::format_summary;
use forgeStat::core::cache::{Cache, CachedRepoInfo};
use forgeStat::core::config::{self, LayoutConfig, StatusBarConfig, WatchlistConfig};
use forgeStat::core::github_client::GitHubClient;
use forgeStat::core::health::compute_health_score;
use forgeStat::core::metrics::stars::predict_milestone;
use forgeStat::core::models::RepoSnapshot;
use forgeStat::core::snapshot;
use forgeStat::core::theme;
use forgeStat::tui::app::{App, AppAction, BackgroundFetchResult, LoadingScreen, SyncState};
use serde::Serialize;

/// JSON output structure including snapshot, health score, and milestone prediction
#[derive(Serialize)]
struct JsonOutput {
    #[serde(flatten)]
    snapshot: RepoSnapshot,
    health_score: forgeStat::core::health::HealthScore,
    /// Star milestone prediction (next milestone, estimated days, daily rate)
    #[serde(skip_serializing_if = "Option::is_none")]
    milestone_prediction: Option<forgeStat::core::metrics::stars::MilestonePrediction>,
}

/// forgeStat - GitHub repository metrics viewer
#[derive(Parser, Debug)]
#[command(name = "forgeStat")]
#[command(about = "GitHub repository metrics viewer")]
#[command(version)]
struct Cli {
    /// Repository in owner/repo format (optional when using --list or --from-stdin)
    repo: Option<String>,

    /// List all cached repositories
    #[arg(short, long, conflicts_with = "from_stdin")]
    list: bool,

    /// Read repository from stdin (for piping with fzf)
    #[arg(long, conflicts_with = "list")]
    from_stdin: bool,

    /// Output repository data as JSON and exit without launching TUI
    #[arg(long, conflicts_with_all = ["list", "from_stdin"])]
    json: bool,

    /// Output compact human-readable summary and exit without launching TUI
    #[arg(long, conflicts_with_all = ["list", "from_stdin", "json"])]
    summary: bool,

    /// Show multi-repo watchlist dashboard (comma-separated repos or empty=use config)
    #[arg(long, short = 'w', conflicts_with_all = ["list", "from_stdin", "json", "summary", "report", "compare"])]
    watchlist: Option<Option<String>>,

    /// Compare two repositories side-by-side (provide second repo as owner/repo)
    #[arg(long, conflicts_with_all = ["list", "from_stdin", "json", "summary", "report", "watchlist"])]
    compare: Option<String>,

    /// Output repository health report as Markdown and exit without launching TUI
    #[arg(long, conflicts_with_all = ["list", "from_stdin", "json", "summary", "watchlist"])]
    report: bool,

    /// Write report to file instead of stdout (use with --report)
    #[arg(long, requires = "report")]
    report_file: Option<PathBuf>,
}

/// List all cached repositories
async fn list_cached_repos() -> Result<Vec<CachedRepoInfo>> {
    Cache::scan_all_repos().await
}

/// Print cached repos in a format suitable for fzf
fn print_repo_list_for_fzf(repos: &[CachedRepoInfo]) {
    for repo in repos {
        let desc = repo.description.as_deref().unwrap_or("");
        let last_viewed = repo
            .last_viewed_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "never".to_string());
        println!(
            "{}\t{}\t{}\t{}",
            repo.full_name,
            desc,
            last_viewed,
            repo.path.display()
        );
    }
}

/// Get repository argument from CLI (positional, --from-stdin, or error)
fn get_repo_arg(cli: &Cli) -> Result<String> {
    if cli.from_stdin {
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        match lines.next() {
            Some(Ok(line)) => {
                let parts: Vec<&str> = line.split('\t').collect();
                if !parts.is_empty() {
                    Ok(parts[0].trim().to_string())
                } else {
                    Ok(line.trim().to_string())
                }
            }
            _ => {
                anyhow::bail!("No repository provided via stdin. Pipe a repo like: echo 'owner/repo' | forgeStat --from-stdin");
            }
        }
    } else {
        match cli.repo.as_ref() {
            Some(repo) => Ok(repo.trim().to_string()),
            None => {
                anyhow::bail!("Repository argument is required.\n\nUsage:\n  forgeStat <owner/repo>\n  forgeStat --list\n  forgeStat --from-stdin (pipe with fzf)\n\nExample: forgeStat ratatui-org/ratatui");
            }
        }
    }
}

/// Parse and validate repository format "owner/repo"
fn parse_repo(repo_arg: &str) -> Result<(String, String)> {
    if repo_arg.is_empty() {
        anyhow::bail!("Repository argument is empty. Please provide a repository in 'owner/repo' format.\n\nExample: forgeStat ratatui-org/ratatui");
    }

    let parts: Vec<&str> = repo_arg.split('/').collect();

    if parts.len() < 2 {
        anyhow::bail!(
            "Invalid repository format: '{}'
\nRepository must be in 'owner/repo' format with a forward slash.\n\nExamples:\n  forgeStat ratatui-org/ratatui\n  forgeStat torvalds/linux",
            repo_arg
        );
    }

    if parts.len() > 2 {
        anyhow::bail!(
            "Invalid repository format: '{}'
\nRepository must be in 'owner/repo' format with exactly one forward slash.\n\nExamples:\n  forgeStat ratatui-org/ratatui\n  forgeStat torvalds/linux",
            repo_arg
        );
    }

    let owner = parts[0];
    let repo_name = parts[1];

    if owner.is_empty() {
        anyhow::bail!(
            "Invalid repository format: '{}'
\nThe owner name is empty. Repository must be in 'owner/repo' format.\n\nExample: forgeStat ratatui-org/ratatui",
            repo_arg
        );
    }

    if repo_name.is_empty() {
        anyhow::bail!(
            "Invalid repository format: '{}'
\nThe repository name is empty. Repository must be in 'owner/repo' format.\n\nExample: forgeStat ratatui-org/ratatui",
            repo_arg
        );
    }

    Ok((owner.to_string(), repo_name.to_string()))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (silently ignore if missing)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    // Handle --list flag
    if cli.list {
        let repos = list_cached_repos().await?;
        if repos.is_empty() {
            println!("No cached repositories found.");
            println!("Use: forgeStat <owner/repo> to cache a repository.");
        } else {
            println!("Cached repositories ({} found):", repos.len());
            println!();
            print_repo_list_for_fzf(&repos);
        }
        return Ok(());
    }

    // Handle --json flag
    if cli.json {
        let repo_arg = get_repo_arg(&cli)?;
        let (owner, repo_name) = parse_repo(&repo_arg)?;

        let token = config::load_token().ok();
        let client = GitHubClient::new(token.as_deref())?;
        let cache = Cache::new(&owner, &repo_name)?;

        match snapshot::fetch_snapshot(&client, &cache, &owner, &repo_name, false).await {
            Ok(snapshot) => {
                // Compute health score and milestone prediction for JSON output
                let health = compute_health_score(&snapshot);
                let milestone = predict_milestone(&snapshot.stars);
                let output = JsonOutput {
                    snapshot,
                    health_score: health,
                    milestone_prediction: milestone,
                };
                let json = serde_json::to_string_pretty(&output).unwrap();
                println!("{}", json);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    // Handle --summary flag
    if cli.summary {
        let repo_arg = get_repo_arg(&cli)?;
        let (owner, repo_name) = parse_repo(&repo_arg)?;

        let token = config::load_token().ok();
        let client = GitHubClient::new(token.as_deref())?;
        let cache = Cache::new(&owner, &repo_name)?;

        match snapshot::fetch_snapshot(&client, &cache, &owner, &repo_name, false).await {
            Ok(snapshot) => {
                let summary = format_summary(&snapshot);
                println!("{}", summary);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    // Handle --report flag
    if cli.report {
        let repo_arg = get_repo_arg(&cli)?;
        let (owner, repo_name) = parse_repo(&repo_arg)?;

        let token = config::load_token().ok();
        let client = GitHubClient::new(token.as_deref())?;
        let cache = Cache::new(&owner, &repo_name)?;

        match snapshot::fetch_snapshot(&client, &cache, &owner, &repo_name, false).await {
            Ok(snapshot) => {
                let report = generate_report(&snapshot);

                if let Some(path) = cli.report_file {
                    // Write to file
                    match std::fs::File::create(&path) {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(report.as_bytes()) {
                                eprintln!("Failed to write report to file: {}", e);
                                std::process::exit(1);
                            }
                            println!("Report written to: {}", path.display());
                        }
                        Err(e) => {
                            eprintln!("Failed to create report file: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    // Output to stdout
                    println!("{}", report);
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    // Handle --watchlist flag
    if let Some(watchlist_arg) = cli.watchlist {
        // Get repos from CLI arg or config file
        let repos = match watchlist_arg {
            Some(repo_list) => {
                // Parse comma-separated repos
                repo_list
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
            }
            None => {
                // Use config file
                let config = WatchlistConfig::load();
                config.repos
            }
        };

        if repos.is_empty() {
            eprintln!("No repositories specified for watchlist.");
            eprintln!("Usage:");
            eprintln!("  forgeStat --watchlist owner/repo1,owner/repo2,owner/repo3");
            eprintln!("  forgeStat --watchlist (uses ~/.config/forgeStat/watchlist.toml)");
            std::process::exit(1);
        }

        // Validate all repos
        for repo in &repos {
            if let Err(e) = WatchlistConfig::validate_repo_format(repo) {
                eprintln!("Invalid repository format '{}': {}", repo, e);
                std::process::exit(1);
            }
        }

        // Run watchlist dashboard
        return run_watchlist_loop(&repos).await;
    }

    // Handle --compare flag
    if let Some(ref compare_repo) = cli.compare {
        let repo_arg = get_repo_arg(&cli)?;
        let (owner1, repo1) = parse_repo(&repo_arg)?;
        let (owner2, repo2) = parse_repo(compare_repo)?;

        return run_compare_mode(&owner1, &repo1, &owner2, &repo2).await;
    }

    // Get repository argument and parse it
    let repo_arg = get_repo_arg(&cli)?;
    let (owner, repo_name) = parse_repo(&repo_arg)?;

    // Run the main app loop with support for switching repositories
    run_app_loop(&owner, &repo_name).await
}

/// Run compare mode with two repositories side-by-side
async fn run_compare_mode(owner1: &str, repo1: &str, owner2: &str, repo2: &str) -> Result<()> {
    let token = config::load_token().ok();
    let client = GitHubClient::new(token.as_deref())?;

    let cache1 = Cache::new(owner1, repo1)?;
    let cache2 = Cache::new(owner2, repo2)?;

    // Load theme configuration
    let theme = theme::load_theme();

    // Load status bar configuration
    let statusbar_config = StatusBarConfig::load();

    // Load layout configuration
    let layout_config = LayoutConfig::load();

    let mut app = App::new(
        owner1.to_string(),
        repo1.to_string(),
        theme,
        statusbar_config,
        layout_config,
    );

    // Initialize terminal early for loading screen
    let mut terminal = ratatui::init();

    // Create loading screen for both repos
    let mut loading_screen = LoadingScreen::new(
        format!("{} & {}", owner1, owner2),
        format!("{} & {}", repo1, repo2),
    );

    // Create progress channels for both repos
    let (progress_tx1, progress_rx1) = tokio::sync::mpsc::channel(20);
    let (progress_tx2, _progress_rx2) = tokio::sync::mpsc::channel(20);

    // Spawn fetch tasks with progress reporting for both repos
    let client_clone1 = client.clone();
    let client_clone2 = client.clone();
    let cache_clone1 = Cache::new(owner1, repo1)?;
    let cache_clone2 = Cache::new(owner2, repo2)?;
    let owner1_string = owner1.to_string();
    let repo1_string = repo1.to_string();
    let owner2_string = owner2.to_string();
    let repo2_string = repo2.to_string();

    let fetch_handle1 = tokio::spawn(async move {
        snapshot::fetch_snapshot_with_progress(
            &client_clone1,
            &cache_clone1,
            &owner1_string,
            &repo1_string,
            false,
            progress_tx1,
        )
        .await
    });

    let fetch_handle2 = tokio::spawn(async move {
        snapshot::fetch_snapshot_with_progress(
            &client_clone2,
            &cache_clone2,
            &owner2_string,
            &repo2_string,
            false,
            progress_tx2,
        )
        .await
    });

    // Run loading screen showing first repo's progress
    let fetch_result = match loading_screen.run(&mut terminal, progress_rx1).await {
        Ok(_) => {
            // Wait for both fetches to complete
            let (result1, result2) = tokio::join!(fetch_handle1, fetch_handle2);
            match (result1, result2) {
                (Ok(Ok(snap1)), Ok(Ok(snap2))) => Ok((Ok(snap1), Ok(snap2))),
                (Ok(Err(e)), _) => Ok((Err(e), Err(anyhow::anyhow!("Fetch failed")))),
                (_, Ok(Err(e))) => Ok((Err(anyhow::anyhow!("Fetch failed")), Err(e))),
                (Err(e), _) => Err(anyhow::anyhow!("Task join error: {}", e)),
                (_, Err(e)) => Err(anyhow::anyhow!("Task join error: {}", e)),
            }
        }
        Err(e) => {
            fetch_handle1.abort();
            fetch_handle2.abort();
            Err(anyhow::anyhow!("Loading screen error: {}", e))
        }
    };

    // Handle first repo result
    let snap2_for_compare = match &fetch_result {
        Ok((_, Ok(snap2))) => Some(snap2.clone()),
        _ => None,
    };

    match fetch_result {
        Ok((Ok(snap1), _)) => {
            app.set_snapshot(snap1, SyncState::Live);
            if let Ok(rl) = client.fetch_rate_limit().await {
                app.set_rate_limit(rl);
            }
        }
        Ok((Err(e), _)) | Err(e) => {
            eprintln!("Failed to fetch {}/{}: {}", owner1, repo1, e);
            match cache1.load().await {
                Ok(Some((snap, _))) => {
                    let state = if cache1.is_stale(15) {
                        SyncState::Stale
                    } else {
                        SyncState::Live
                    };
                    app.set_snapshot(snap, state);
                }
                _ => {
                    anyhow::bail!("No cached data available for {}/{}", owner1, repo1);
                }
            }
        }
    }

    // Handle second repo
    match snap2_for_compare {
        Some(snap2) => {
            app.enter_compare_mode(snap2);
        }
        None => {
            // Try to load from cache on failure
            match cache2.load().await {
                Ok(Some((snap, _))) => {
                    app.enter_compare_mode(snap);
                }
                _ => {
                    anyhow::bail!("No cached data available for {}/{}", owner2, repo2);
                }
            }
        }
    }

    // Enable mouse capture
    stdout().execute(EnableMouseCapture)?;

    // No background refresh in compare mode (too complex with two repos)
    let mut background_rx: Option<tokio::sync::mpsc::Receiver<BackgroundFetchResult>> = None;

    let action = loop {
        match forgeStat::tui::app::run_event_loop(&mut terminal, &mut app, &mut background_rx) {
            Ok(AppAction::Quit) => break Ok(AppAction::Quit),
            Ok(AppAction::Refresh) => {
                // Disable mouse capture and clear terminal for loading screen
                let _ = stdout().execute(DisableMouseCapture);
                terminal.clear()?;

                // Create loading screen for refresh (shows first repo name as primary)
                let mut loading_screen = LoadingScreen::new(
                    format!("{} & {}", owner1, owner2),
                    format!("{} & {}", repo1, repo2),
                );

                // Create progress channels for both repos
                let (progress_tx1, progress_rx1) = tokio::sync::mpsc::channel(20);
                let (progress_tx2, _progress_rx2) = tokio::sync::mpsc::channel(20);

                // Spawn fetch tasks with progress reporting for both repos
                let client_clone1 = client.clone();
                let client_clone2 = client.clone();
                let cache_clone1 = Cache::new(owner1, repo1)?;
                let cache_clone2 = Cache::new(owner2, repo2)?;
                let owner1_string = owner1.to_string();
                let repo1_string = repo1.to_string();
                let owner2_string = owner2.to_string();
                let repo2_string = repo2.to_string();

                let fetch_handle1 = tokio::spawn(async move {
                    snapshot::fetch_snapshot_with_progress(
                        &client_clone1,
                        &cache_clone1,
                        &owner1_string,
                        &repo1_string,
                        true, // force refresh
                        progress_tx1,
                    )
                    .await
                });

                let fetch_handle2 = tokio::spawn(async move {
                    snapshot::fetch_snapshot_with_progress(
                        &client_clone2,
                        &cache_clone2,
                        &owner2_string,
                        &repo2_string,
                        true, // force refresh
                        progress_tx2,
                    )
                    .await
                });

                // For compare mode, we'll just show the first repo's progress
                // and wait for both to complete
                let fetch_result = match loading_screen.run(&mut terminal, progress_rx1).await {
                    Ok(_) => {
                        // Wait for both fetches to complete
                        let (result1, result2) = tokio::join!(fetch_handle1, fetch_handle2);
                        Ok((result1.ok().transpose(), result2.ok().transpose()))
                    }
                    Err(e) => {
                        fetch_handle1.abort();
                        fetch_handle2.abort();
                        Err(anyhow::anyhow!("Loading screen error: {}", e))
                    }
                };

                // Re-enable mouse capture after loading screen
                stdout().execute(EnableMouseCapture)?;

                // Process fetch results
                match fetch_result {
                    Ok((Ok(Some(snap1)), Ok(Some(snap2)))) => {
                        app.set_snapshot(snap1, SyncState::Live);
                        app.enter_compare_mode(snap2);
                        if let Ok(rl) = client.fetch_rate_limit().await {
                            app.set_rate_limit(rl);
                        }
                    }
                    Ok((result1, result2)) => {
                        // Handle partial success - at least update what we got
                        if let Ok(Some(snap1)) = result1 {
                            app.set_snapshot(snap1, SyncState::Live);
                        }
                        if let Ok(Some(snap2)) = result2 {
                            app.enter_compare_mode(snap2);
                        }
                        if let Ok(rl) = client.fetch_rate_limit().await {
                            app.set_rate_limit(rl);
                        }
                    }
                    Err(e) => {
                        log::warn!("Refresh failed: {}", e);
                    }
                }
            }
            Ok(AppAction::BackgroundRefresh) => {
                // Skip auto-refresh in compare mode (too complex with two repos)
                // Just reset the timer to avoid constant triggering
                app.last_refresh = std::time::Instant::now();
            }
            Ok(AppAction::BackgroundFetchComplete(_)) => {
                // Should not happen in compare mode, but handle gracefully
            }
            Ok(AppAction::SwitchRepo(new_owner, new_repo)) => {
                break Ok(AppAction::SwitchRepo(new_owner, new_repo))
            }
            Err(e) => break Err(e),
        }
    };

    // Disable mouse capture before restoring terminal
    let _ = stdout().execute(DisableMouseCapture);
    ratatui::restore();

    // Handle the action
    match action {
        Ok(AppAction::Quit) => Ok(()),
        Ok(AppAction::SwitchRepo(new_owner, new_repo)) => {
            // Switch to single repo mode
            run_app_loop(&new_owner, &new_repo).await
        }
        Err(e) => Err(e),
        _ => Ok(()),
    }
}

/// Main application loop that supports switching between repositories
async fn run_app_loop(initial_owner: &str, initial_repo: &str) -> Result<()> {
    let token = config::load_token().ok();
    let client = GitHubClient::new(token.as_deref())?;

    let mut current_owner = initial_owner.to_string();
    let mut current_repo = initial_repo.to_string();

    loop {
        let cache = Cache::new(&current_owner, &current_repo)?;

        // Load theme configuration
        let theme = theme::load_theme();

        // Load status bar configuration
        let statusbar_config = StatusBarConfig::load();

        // Load layout configuration
        let layout_config = LayoutConfig::load();

        let mut app = App::new(
            current_owner.clone(),
            current_repo.clone(),
            theme,
            statusbar_config,
            layout_config,
        );

        // Check if we can use cache (skip loading screen for cache hits)
        let use_cache = !cache.is_stale(15);
        let cached_snapshot = if use_cache {
            cache.load().await.ok().flatten()
        } else {
            None
        };

        let mut snapshot_result: Option<RepoSnapshot> = None;
        let mut sync_state = SyncState::Live;

        if let Some((snap, _)) = cached_snapshot {
            // Use cached snapshot directly - no loading screen needed
            log::info!(
                "Using cached snapshot for {}/{}",
                current_owner,
                current_repo
            );
            snapshot_result = Some(snap);
        } else {
            // Initialize terminal early for loading screen
            let mut terminal = ratatui::init();

            // Create loading screen
            let mut loading_screen =
                LoadingScreen::new(current_owner.clone(), current_repo.clone());

            // Create progress channel
            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(20);

            // Spawn fetch task with progress reporting
            let client_clone = client.clone();
            let owner_clone = current_owner.clone();
            let repo_clone = current_repo.clone();
            let cache_clone = Cache::new(&current_owner, &current_repo)?;
            let fetch_handle = tokio::spawn(async move {
                snapshot::fetch_snapshot_with_progress(
                    &client_clone,
                    &cache_clone,
                    &owner_clone,
                    &repo_clone,
                    false,
                    progress_tx,
                )
                .await
            });

            // Run loading screen until fetch completes
            let fetch_result = match loading_screen.run(&mut terminal, progress_rx).await {
                Ok(_) => fetch_handle.await.ok().transpose(),
                Err(e) => {
                    // Loading screen error - cancel fetch and continue
                    fetch_handle.abort();
                    Err(anyhow::anyhow!("Loading screen error: {}", e))
                }
            };

            // Restore terminal before continuing to main app
            let _ = stdout().execute(DisableMouseCapture);
            ratatui::restore();

            match fetch_result {
                Ok(Some(snap)) => {
                    snapshot_result = Some(snap);
                }
                Ok(None) => {
                    // Fetch was cancelled or failed, try to load from cache
                    match cache.load().await {
                        Ok(Some((snap, _))) => {
                            snapshot_result = Some(snap);
                            sync_state = SyncState::Stale;
                        }
                        _ => {
                            sync_state = SyncState::Offline;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Initial fetch failed: {e:#}");
                    // Try to load from cache as fallback
                    match cache.load().await {
                        Ok(Some((snap, _))) => {
                            snapshot_result = Some(snap);
                            sync_state = if cache.is_stale(15) {
                                SyncState::Stale
                            } else {
                                SyncState::Live
                            };
                        }
                        _ => {
                            sync_state = SyncState::Offline;
                        }
                    }
                }
            }
        }

        // Set snapshot and rate limit
        if let Some(snapshot) = snapshot_result {
            app.set_snapshot(snapshot, sync_state);
        } else {
            app.set_offline();
        }

        if let Ok(rl) = client.fetch_rate_limit().await {
            app.set_rate_limit(rl);
        }

        // Initialize main TUI
        let mut terminal = ratatui::init();
        stdout().execute(EnableMouseCapture)?;

        // Channel for background fetch results
        let mut background_rx: Option<tokio::sync::mpsc::Receiver<BackgroundFetchResult>> = None;

        // Run main event loop
        let action = loop {
            match forgeStat::tui::app::run_event_loop(&mut terminal, &mut app, &mut background_rx) {
                Ok(AppAction::Quit) => break Ok(AppAction::Quit),
                Ok(AppAction::Refresh) => {
                    // Disable mouse capture and clear terminal for loading screen
                    let _ = stdout().execute(DisableMouseCapture);
                    terminal.clear()?;

                    // Create loading screen for refresh
                    let mut loading_screen =
                        LoadingScreen::new(current_owner.clone(), current_repo.clone());

                    // Create progress channel
                    let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(20);

                    // Spawn fetch task with progress reporting
                    let client_clone = client.clone();
                    let owner_clone = current_owner.clone();
                    let repo_clone = current_repo.clone();
                    let cache_clone = Cache::new(&current_owner, &current_repo)?;
                    let fetch_handle = tokio::spawn(async move {
                        snapshot::fetch_snapshot_with_progress(
                            &client_clone,
                            &cache_clone,
                            &owner_clone,
                            &repo_clone,
                            true, // force refresh
                            progress_tx,
                        )
                        .await
                    });

                    // Run loading screen until fetch completes
                    let fetch_result = match loading_screen.run(&mut terminal, progress_rx).await {
                        Ok(_) => fetch_handle.await.ok().transpose(),
                        Err(e) => {
                            fetch_handle.abort();
                            Err(anyhow::anyhow!("Loading screen error: {}", e))
                        }
                    };

                    // Re-enable mouse capture after loading screen
                    stdout().execute(EnableMouseCapture)?;

                    // Process fetch result
                    match fetch_result {
                        Ok(Some(snap)) => {
                            app.set_snapshot(snap, SyncState::Live);
                            if let Ok(rl) = client.fetch_rate_limit().await {
                                app.set_rate_limit(rl);
                            }
                        }
                        Ok(None) => {
                            log::warn!("Refresh fetch was cancelled");
                            // Keep existing snapshot, just update rate limit
                            if let Ok(rl) = client.fetch_rate_limit().await {
                                app.set_rate_limit(rl);
                            }
                        }
                        Err(e) => {
                            log::warn!("Refresh failed: {}", e);
                            app.set_offline();
                        }
                    }
                }
                Ok(AppAction::BackgroundRefresh) => {
                    // Start a background refresh without loading screen
                    if app.background_refresh_in_progress {
                        continue; // Already refreshing, skip
                    }

                    app.start_background_refresh();

                    // Create channel for background fetch result
                    let (bg_tx, bg_rx) = tokio::sync::mpsc::channel(1);
                    background_rx = Some(bg_rx);

                    // Spawn fetch task in background (no progress reporting needed)
                    let client_clone = client.clone();
                    let owner_clone = current_owner.clone();
                    let repo_clone = current_repo.clone();
                    let cache_clone = match Cache::new(&current_owner, &current_repo) {
                        Ok(c) => c,
                        Err(e) => {
                            app.fail_background_refresh(format!("Cache error: {}", e));
                            background_rx = None;
                            continue;
                        }
                    };

                    tokio::spawn(async move {
                        match snapshot::fetch_snapshot(
                            &client_clone,
                            &cache_clone,
                            &owner_clone,
                            &repo_clone,
                            true, // force refresh
                        )
                        .await
                        {
                            Ok(snapshot) => {
                                let _ = bg_tx.send(BackgroundFetchResult::Success(snapshot)).await;
                            }
                            Err(e) => {
                                let _ = bg_tx
                                    .send(BackgroundFetchResult::Failed(e.to_string()))
                                    .await;
                            }
                        }
                    });
                }
                Ok(AppAction::BackgroundFetchComplete(snapshot)) => {
                    // Background fetch completed successfully
                    app.complete_background_refresh();
                    app.set_snapshot(snapshot, SyncState::Live);
                    background_rx = None; // Channel consumed

                    // Update rate limit
                    if let Ok(rl) = client.fetch_rate_limit().await {
                        app.set_rate_limit(rl);
                    }

                    log::info!("Background refresh completed successfully");
                }
                Ok(AppAction::SwitchRepo(new_owner, new_repo)) => {
                    break Ok(AppAction::SwitchRepo(new_owner, new_repo))
                }
                Err(e) => break Err(e),
            }
        };

        // Disable mouse capture before restoring terminal
        let _ = stdout().execute(DisableMouseCapture);
        ratatui::restore();

        // Handle the action
        match action {
            Ok(AppAction::Quit) => return Ok(()),
            Ok(AppAction::SwitchRepo(new_owner, new_repo)) => {
                // Update current owner/repo and restart the loop
                current_owner = new_owner;
                current_repo = new_repo;
                continue;
            }
            Err(e) => return Err(e),
            _ => return Ok(()),
        }
    }
}

/// Run the watchlist dashboard loop for multi-repo mode
async fn run_watchlist_loop(repos: &[String]) -> Result<()> {
    use forgeStat::tui::app::{WatchlistAction, WatchlistApp};

    let token = config::load_token().ok();
    let client = GitHubClient::new(token.as_deref())?;

    // Load theme configuration
    let theme = theme::load_theme();

    let mut app = WatchlistApp::new(repos.to_vec(), theme);

    let mut terminal = ratatui::init();

    // Enable mouse capture
    stdout().execute(EnableMouseCapture)?;

    loop {
        // Fetch snapshots in parallel
        app.set_fetching(true);
        app.render(&mut terminal)?;

        let fetch_start = std::time::Instant::now();

        // Fetch all repos in parallel
        let futures: Vec<_> = repos
            .iter()
            .map(|repo| {
                let parts: Vec<&str> = repo.split('/').collect();
                let owner = parts[0].to_string();
                let repo_name = parts[1].to_string();
                let client = &client;

                async move {
                    let cache = Cache::new(&owner, &repo_name).ok()?;
                    let result =
                        snapshot::fetch_snapshot(client, &cache, &owner, &repo_name, false).await;
                    Some((repo.clone(), result))
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        for (repo, snapshot_result) in results.into_iter().flatten() {
            match snapshot_result {
                Ok(snapshot) => {
                    app.add_snapshot(repo, snapshot);
                }
                Err(e) => {
                    log::warn!("Failed to fetch {}: {}", repo, e);
                    // Try to load from cache
                    let parts: Vec<&str> = repo.split('/').collect();
                    if let Ok(cache) = Cache::new(parts[0], parts[1]) {
                        if let Ok(Some((snapshot, _))) = cache.load().await {
                            app.add_snapshot(repo, snapshot);
                        } else {
                            app.add_error(repo, e.to_string());
                        }
                    } else {
                        app.add_error(repo, e.to_string());
                    }
                }
            }
        }

        app.set_fetching(false);
        let fetch_duration = fetch_start.elapsed();
        log::info!(
            "Watchlist fetched {} repos in {:?}",
            repos.len(),
            fetch_duration
        );

        // Re-render to show the fetched data before waiting for input
        app.render(&mut terminal)?;

        // Run the event loop
        let action = app.run_event_loop(&mut terminal).await;

        match action {
            Ok(WatchlistAction::Quit) => {
                let _ = stdout().execute(DisableMouseCapture);
                ratatui::restore();
                return Ok(());
            }
            Ok(WatchlistAction::Refresh) => {
                // Clear snapshots and re-fetch
                app.clear_snapshots();
                continue;
            }
            Ok(WatchlistAction::SelectRepo(owner, repo)) => {
                // Switch to single repo mode
                let _ = stdout().execute(DisableMouseCapture);
                ratatui::restore();
                return run_app_loop(&owner, &repo).await;
            }
            Err(e) => {
                let _ = stdout().execute(DisableMouseCapture);
                ratatui::restore();
                return Err(e);
            }
        }
    }
}
