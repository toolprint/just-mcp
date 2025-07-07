pub mod admin;
pub mod error;
pub mod executor;
mod notification;
pub mod parser;
pub mod registry;
pub mod server;
pub mod types;
pub mod watcher;

pub use error::{Error, Result};
pub use server::Server;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
