use anyhow::Result;
use clap::Parser;
use just_mcp::server::{Server, StdioTransport};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "just-mcp")]
#[command(about = "Model Context Protocol server for justfile integration", long_about = None)]
struct Args {
    #[arg(
        short = 'w',
        long = "watch-dir",
        help = "Directory to watch for justfiles, optionally with name (path or path:name). Defaults to current directory if not specified"
    )]
    watch_dir: Vec<String>,

    #[arg(long, help = "Enable administrative tools")]
    admin: bool,

    #[arg(long, help = "Enable JSON output for logs")]
    json_logs: bool,

    #[arg(
        long,
        default_value = "info",
        help = "Log level (trace, debug, info, warn, error)"
    )]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    tracing::info!("Starting {} v{}", just_mcp::PKG_NAME, just_mcp::VERSION);

    // Parse watch directories with optional names (format: "path" or "path:name")
    let mut watch_configs = Vec::new();

    if args.watch_dir.is_empty() {
        // Default to current working directory with no name
        let cwd = std::env::current_dir()?;
        tracing::info!(
            "No --watch-dir specified, using current directory: {}",
            cwd.display()
        );
        watch_configs.push((cwd, None));
    } else {
        for dir_spec in args.watch_dir {
            if let Some(colon_pos) = dir_spec.find(':') {
                // Format: path:name
                let path = std::path::PathBuf::from(&dir_spec[..colon_pos]);
                let name = Some(dir_spec[colon_pos + 1..].to_string());
                watch_configs.push((path, name));
            } else {
                // Just a path, no name
                watch_configs.push((std::path::PathBuf::from(dir_spec), None));
            }
        }
    }

    // Convert all paths to absolute paths and extract for the server
    let mut absolute_configs = Vec::new();
    for (path, name) in watch_configs {
        let abs_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        absolute_configs.push((abs_path, name));
    }

    // Extract just the paths for the server
    let watch_paths: Vec<std::path::PathBuf> = absolute_configs
        .iter()
        .map(|(path, _)| path.clone())
        .collect();

    // Log the absolute paths being watched
    tracing::info!("Watch directories:");
    for (path, name) in &absolute_configs {
        if let Some(n) = name {
            tracing::info!("  {} (name: {})", path.display(), n);
        } else {
            tracing::info!("  {}", path.display());
        }
    }

    let watch_configs = absolute_configs;

    // Create and run the server
    let transport = Box::new(StdioTransport::new());
    let mut server = Server::new(transport)
        .with_watch_paths(watch_paths)
        .with_watch_names(watch_configs);

    server.run().await?;

    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    let fmt_layer = if args.json_logs {
        fmt::layer()
            .json()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .boxed()
    } else {
        fmt::layer()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
