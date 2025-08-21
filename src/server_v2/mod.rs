//! Framework Server Module (v2)
//!
//! This module provides the ultrafast-mcp framework-based server implementation
//! that replaces the custom MCP protocol handling with a proven framework.
//!
//! Key components:
//! - Framework server setup and initialization
//! - Dynamic tool management through the framework
//! - Resources and Prompts integration
//! - Seamless migration from custom implementation

use crate::error::Result;
use std::path::PathBuf;

pub mod dynamic_handler;
pub mod prompts;
pub mod resources;

// Note: Exact API structure will be determined during Task 173
// For now, we'll use a placeholder approach until the actual API is explored
#[cfg(feature = "ultrafast-framework")]
mod framework_placeholder {
    // This will be replaced with actual types during Task 173
    pub struct UltrafastMcpServer;
}

/// Framework-based MCP server implementation
///
/// This server replaces the custom MCP protocol handling with the ultrafast-mcp
/// framework, providing better maintainability and protocol compliance.
pub struct FrameworkServer {
    watch_paths: Vec<PathBuf>,
    watch_configs: Vec<(PathBuf, Option<String>)>,
    admin_enabled: bool,
    #[cfg(feature = "ultrafast-framework")]
    framework_server: Option<framework_placeholder::UltrafastMcpServer>,
}

impl FrameworkServer {
    /// Create a new framework server instance
    pub fn new() -> Self {
        Self {
            watch_paths: vec![PathBuf::from(".")],
            watch_configs: vec![(PathBuf::from("."), None)],
            admin_enabled: false,
            #[cfg(feature = "ultrafast-framework")]
            framework_server: None,
        }
    }

    /// Configure watch paths for justfile monitoring
    pub fn with_watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    /// Configure watch names for multi-directory support
    pub fn with_watch_names(mut self, configs: Vec<(PathBuf, Option<String>)>) -> Self {
        self.watch_configs = configs;
        self
    }

    /// Enable admin tools functionality
    pub fn with_admin_enabled(mut self, enabled: bool) -> Self {
        self.admin_enabled = enabled;
        self
    }

    /// Initialize the framework server
    ///
    /// Sets up the ultrafast-mcp framework with our dynamic tool handlers,
    /// resource providers, and prompt systems.
    #[cfg(feature = "ultrafast-framework")]
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing ultrafast-mcp framework server");

        // TODO: Initialize framework server
        // This will be implemented in Task 173
        tracing::warn!("Framework initialization not yet implemented");

        Ok(())
    }

    /// Initialize the framework server (feature disabled)
    ///
    /// When the ultrafast-framework feature is disabled, this returns an error
    /// indicating that the framework is not available.
    #[cfg(not(feature = "ultrafast-framework"))]
    pub async fn initialize(&mut self) -> Result<()> {
        Err(crate::error::Error::Other(
            "ultrafast-framework feature not enabled".into(),
        ))
    }

    /// Run the framework server
    ///
    /// Starts the main server loop with framework-based message handling.
    pub async fn run(&mut self) -> Result<()> {
        self.initialize().await?;

        tracing::info!("Starting framework-based MCP server");

        // TODO: Start framework server main loop
        // This will be implemented in Task 173
        tracing::warn!("Framework server loop not yet implemented");

        Ok(())
    }
}

impl Default for FrameworkServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_server_creation() {
        let server = FrameworkServer::new();
        assert_eq!(server.watch_paths.len(), 1);
        assert_eq!(server.watch_configs.len(), 1);
        assert!(!server.admin_enabled);
    }

    #[test]
    fn test_framework_server_configuration() {
        let paths = vec![PathBuf::from("test1"), PathBuf::from("test2")];
        let configs = vec![
            (PathBuf::from("test1"), Some("frontend".to_string())),
            (PathBuf::from("test2"), None),
        ];

        let server = FrameworkServer::new()
            .with_watch_paths(paths.clone())
            .with_watch_names(configs.clone())
            .with_admin_enabled(true);

        assert_eq!(server.watch_paths, paths);
        assert_eq!(server.watch_configs, configs);
        assert!(server.admin_enabled);
    }

    #[tokio::test]
    async fn test_framework_server_initialization() {
        let mut server = FrameworkServer::new();
        
        #[cfg(feature = "ultrafast-framework")]
        {
            // Framework available - should not error but warn about incomplete implementation
            let result = server.initialize().await;
            assert!(result.is_ok());
        }

        #[cfg(not(feature = "ultrafast-framework"))]
        {
            // Framework not available - should error
            let result = server.initialize().await;
            assert!(result.is_err());
        }
    }
}