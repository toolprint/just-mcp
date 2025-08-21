//! Configuration Resource Module
//!
//! This module provides a virtual `config.json` resource that exposes the current
//! runtime configuration state through the Model Context Protocol (MCP) at the 
//! URI `file:///config.json`.
//!
//! # Architecture
//!
//! - **ConfigDataCollector**: Gathers configuration data from all system components
//! - **ConfigResourceProvider**: Implements ResourceProvider trait to serve virtual config.json
//!
//! # Usage
//!
//! ```rust,no_run
//! use just_mcp::config_resource::{ConfigDataCollector, ConfigResourceProvider};
//! use std::sync::Arc;
//!
//! // Create configuration data collector
//! let collector = ConfigDataCollector::new(/* configuration parameters */);
//! 
//! // Create configuration resource provider
//! let config_provider = Arc::new(ConfigResourceProvider::new(collector));
//!
//! // Provider can now be used to serve config.json via MCP protocol
//! ```

pub mod collector;
pub mod provider;
pub mod combined_provider;

pub use collector::ConfigDataCollector;
pub use provider::ConfigResourceProvider;
pub use combined_provider::CombinedResourceProvider;