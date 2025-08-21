pub mod admin;
pub mod cli;
pub mod config_resource;
pub mod embedded_content;
pub mod error;
pub mod executor;
mod notification;
pub mod parser;
pub mod prompts;
pub mod registry;
pub mod resource_limits;
pub mod security;
pub mod server;
pub mod types;
pub mod watcher;

#[cfg(feature = "vector-search")]
pub mod vector_search;

#[cfg(feature = "ultrafast-framework")]
pub mod server_v2;

pub use error::{Error, Result};
pub use server::Server;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
