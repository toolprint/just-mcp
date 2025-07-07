use anyhow::Result;
use clap::Parser;
use just_mcp::server::{Server, StdioTransport};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "just-mcp")]
#[command(about = "Model Context Protocol server for justfile integration", long_about = None)]
struct Args {
    #[arg(long, default_value = "justfile", help = "Path to the justfile")]
    justfile: String,

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
    tracing::info!("Using justfile: {}", args.justfile);

    // Create and run the server
    let transport = Box::new(StdioTransport::new());
    let mut server = Server::new(transport);

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
