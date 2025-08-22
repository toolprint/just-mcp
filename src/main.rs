use anyhow::Result;
use clap::Parser;
use just_mcp::server::{Server, StdioTransport};
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use just_mcp::cli::{Args, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    // Handle different commands
    match args.command {
        #[cfg(feature = "vector-search")]
        Some(Commands::Search { search_command }) => {
            just_mcp::cli::handle_search_command(search_command).await?;
        }
        Some(Commands::Serve) | None => {
            // Check if we should use custom server (legacy mode) or framework server (default)
            let use_legacy = args.use_legacy 
                || std::env::var("JUST_MCP_USE_LEGACY").is_ok();
            
            if use_legacy {
                // Legacy mode: use custom MCP server
                start_mcp_server(&args).await?;
            } else {
                // Default behavior: start framework server
                start_framework_server(&args).await?;
            }
        }
    }

    Ok(())
}

/// Start the MCP server with the given arguments
async fn start_mcp_server(args: &Args) -> Result<()> {
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
        for dir_spec in &args.watch_dir {
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

    // Initialize empty prompt registry (no prompts registered yet)
    let prompt_config = just_mcp::prompts::traits::PromptConfig::default();

    // Create search adapter with MockSearchProvider
    let mock_provider = just_mcp::prompts::search_adapter::MockSearchProvider::new();
    let search_adapter = Arc::new(
        just_mcp::prompts::search_adapter::SearchAdapter::with_provider(
            Arc::new(mock_provider),
            prompt_config.clone(),
        ),
    );

    // Build empty registry (with_defaults = false to prevent auto-registration)
    let prompt_registry = Arc::new(
        just_mcp::prompts::registry::PromptRegistryBuilder::new()
            .with_config(prompt_config)
            .with_search_adapter(search_adapter)
            .with_defaults(false) // KEY: Keep registry empty
            .build()
            .await?,
    );

    // Create and run the server
    let transport = Box::new(StdioTransport::new());
    let mut server = Server::new(transport)
        .with_watch_paths(watch_paths)
        .with_watch_names(watch_configs)
        .with_admin_enabled(args.admin)
        .with_args(args.clone())
        .with_prompt_registry(prompt_registry);

    server.run().await?;

    Ok(())
}

/// Start the framework-based MCP server with the given arguments
async fn start_framework_server(args: &Args) -> Result<()> {
    #[cfg(feature = "ultrafast-framework")]
    {
        tracing::info!("Starting {} v{} with ultrafast-mcp framework", just_mcp::PKG_NAME, just_mcp::VERSION);

        // Parse watch directories with optional names (same logic as custom server)
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
            for dir_spec in &args.watch_dir {
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

        // Convert all paths to absolute paths
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
        tracing::info!("Watch directories (framework server):");
        for (path, name) in &absolute_configs {
            if let Some(n) = name {
                tracing::info!("  {} (name: {})", path.display(), n);
            } else {
                tracing::info!("  {}", path.display());
            }
        }

        // Create and configure the framework server
        let mut framework_server = just_mcp::server_v2::FrameworkServer::new()
            .with_watch_paths(watch_paths)
            .with_watch_names(absolute_configs)
            .with_admin_enabled(args.admin);

        // Run the framework server
        framework_server.run().await?;
        Ok(())
    }

    #[cfg(not(feature = "ultrafast-framework"))]
    {
        tracing::warn!("Framework server not available (ultrafast-framework feature not enabled)");
        tracing::info!("Falling back to legacy server");
        start_mcp_server(args).await
    }
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
