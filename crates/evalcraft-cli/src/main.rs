use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;
use evalcraft_store::Store;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run evaluations
    Run {
        /// Run in watch mode
        #[arg(short, long)]
        watch: bool,

        /// Filter tests by name
        #[arg(short, long)]
        filter: Option<String>,

        /// Path to search for tests (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Initialize/Ensure the store exists for this run
    // In a real app, we might want to locate this relative to the project root
    let _store = Store::open("eval_history.db")?;

    match &cli.command {
        Some(Commands::Run { watch, filter, path }) => {
            if *watch {
                run_watch_mode(path, filter.as_deref()).await?;
            } else {
                run_once(path, filter.as_deref()).await?;
            }
        }
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
        }
    }

    Ok(())
}

async fn run_once(_path: &PathBuf, filter: Option<&str>) -> anyhow::Result<()> {
    // For Approach 1: We assume the user has defined examples/tests in Cargo.toml.
    // We will run `cargo test` (or `cargo run --example`) and let the output stream to stdout.
    // To enable "Implicit Persistence", we set an environment variable `EVALCRAFT_DB_PATH`.
    // The `evalcraft-core` library (used by the user's test) will detect this var and write to it.

    println!("🚀 Starting evaluation run...");

    // Enable the 'persistence' feature when running tests if it's an optional feature in their Cargo.toml?
    // Actually, if they depend on evalcraft-core, they need to have the feature enabled in their Cargo.toml 
    // OR we can try to enable it via command line if it's a workspace feature.
    // For now, we assume the user has `features = ["persistence"]` or similar in their Cargo.toml
    // OR evalcraft-core enables it by default (which I set to false for now).
    // Ideally, `evalcraft run` should work even if they didn't manually enable persistence in their code,
    // but `cargo test` compiles what is in Cargo.toml.
    // We can pass `--features evalcraft-core/persistence` to `cargo test`!

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    
    // Only run examples and integration tests, not library unit tests
    // This prevents running the unit tests in evalcraft-core itself
    cmd.args(&["--examples", "--tests"]);
    
    // Enable the persistence feature in the core library dynamically
    cmd.args(&["--features", "evalcraft-core/persistence"]);
    
    // If the user is running specific examples (common for evals), we might default to running all examples
    // or specific ones if filtered.
    
    if let Some(f) = filter {
        println!("🔍 Filtering for: {}", f);
        cmd.arg(f);
    }

    // Inject the DB path so the child process knows where to save results
    let db_path = std::env::current_dir()?.join("eval_history.db");
    cmd.env("EVALCRAFT_DB_PATH", db_path);
    
    // We want to capture the status to know if tests passed
    // We let stdout/stderr inherit so the user sees the progress
    let status = cmd.status()?;

    if status.success() {
        println!("✅ Evaluation run completed successfully.");
    } else {
        println!("❌ Evaluation run failed.");
        // We don't exit with error here because we want the CLI to stay alive in watch mode,
        // but for single run we might want to propagate the failure code.
    }

    Ok(())
}

use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::sync::mpsc::channel;
use std::time::Duration;

async fn run_watch_mode(path: &PathBuf, filter: Option<&str>) -> anyhow::Result<()> {
    println!("👀 Watching for changes in {:?}...", path);

    // Initial run
    let _ = run_once(path, filter).await;

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive)?;

    loop {
        // Block until we receive an event
        match rx.recv() {
            Ok(Ok(Event { kind, paths, .. })) => {
                // Check if it's a relevant file change (e.g. .rs, .toml)
                let changed_files: Vec<_> = paths.iter()
                    .filter(|p| p.extension().map_or(false, |ext| ext == "rs" || ext == "toml"))
                    .collect();

                if !changed_files.is_empty() {
                    println!("📝 Change detected ({:?}). Re-running...", kind);
                    
                    // Simple debounce: clear queue of any other pending events for a short duration
                    while let Ok(_) = rx.recv_timeout(Duration::from_millis(100)) {}
                    
                    // Determine what to run based on what changed
                    let target_filter = determine_test_target(&changed_files);
                    
                    if let Some(specific_target) = target_filter {
                        println!("🎯 Running tests for: {}", specific_target);
                        if let Err(e) = run_specific_test(path, &specific_target).await {
                            eprintln!("Error running tests: {}", e);
                        }
                    } else {
                        // If we can't determine a specific target (e.g., Cargo.toml changed),
                        // run all tests
                        println!("🔄 Running all tests...");
                        if let Err(e) = run_once(path, filter).await {
                            eprintln!("Error running tests: {}", e);
                        }
                    }
                    
                    println!("👀 Waiting for changes...");
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
            Err(e) => eprintln!("Watch channel error: {:?}", e),
        }
    }
}

/// Determine which specific test to run based on the changed files
fn determine_test_target(changed_files: &[&std::path::PathBuf]) -> Option<String> {
    for file_path in changed_files {
        // If it's a test/example file, extract the test name
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            // Check if it's in examples/ directory
            if file_path.to_str().map_or(false, |p| p.contains("examples/")) {
                // Extract the example name (without .rs extension)
                if let Some(test_name) = file_name.strip_suffix(".rs") {
                    return Some(test_name.to_string());
                }
            }
            
            // Check if it's a test file in tests/ directory
            if file_path.to_str().map_or(false, |p| p.contains("tests/")) {
                if let Some(test_name) = file_name.strip_suffix(".rs") {
                    return Some(test_name.to_string());
                }
            }
        }
        
        // If Cargo.toml changed, we need to run everything
        if file_path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml") {
            return None; // Run all tests
        }
    }
    
    // If we changed a source file in src/, we should run all tests
    // because we don't know which tests depend on it
    None
}

/// Run a specific test by name
async fn run_specific_test(_path: &PathBuf, test_name: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    
    // Target the specific example or test
    cmd.args(&["--example", test_name]);
    
    // Enable the persistence feature
    cmd.args(&["--features", "evalcraft-core/persistence"]);
    
    // Inject the DB path
    let db_path = std::env::current_dir()?.join("eval_history.db");
    cmd.env("EVALCRAFT_DB_PATH", db_path);
    
    let status = cmd.status()?;

    if !status.success() {
        eprintln!("❌ Test '{}' failed.", test_name);
    }

    Ok(())
}

