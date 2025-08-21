//! Configuration Data Collector
//!
//! This module handles gathering configuration data from all system components
//! to create a comprehensive view of the current runtime state.

use crate::cli::Args;
use anyhow::Result;
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

/// Collects configuration data from all system components
pub struct ConfigDataCollector {
    args: Option<Args>,
    security_config: Option<crate::security::SecurityConfig>,
    resource_limits: Option<crate::resource_limits::ResourceLimits>,
    resource_manager: Option<Arc<crate::resource_limits::ResourceManager>>,
    tool_registry: Option<Arc<Mutex<crate::registry::ToolRegistry>>>,
}

impl ConfigDataCollector {
    /// Create a new configuration data collector
    pub fn new() -> Self {
        Self {
            args: None,
            security_config: None,
            resource_limits: None,
            resource_manager: None,
            tool_registry: None,
        }
    }

    /// Set CLI arguments
    pub fn with_args(mut self, args: Args) -> Self {
        self.args = Some(args);
        self
    }

    /// Set security configuration
    pub fn with_security_config(mut self, config: crate::security::SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
    }

    /// Set resource limits
    pub fn with_resource_limits(mut self, limits: crate::resource_limits::ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    /// Set resource manager for runtime statistics
    pub fn with_resource_manager(mut self, manager: Arc<crate::resource_limits::ResourceManager>) -> Self {
        self.resource_manager = Some(manager);
        self
    }

    /// Set tool registry for tool statistics
    pub fn with_tool_registry(mut self, registry: Arc<Mutex<crate::registry::ToolRegistry>>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    /// Collect all configuration data into a JSON structure that conforms to the schema
    pub async fn collect_config_data(&self) -> Result<Value> {
        let server_info = self.collect_server_info();
        let cli_info = self.collect_cli_info();
        let security_info = self.collect_security_info();
        let resource_limits_info = self.collect_resource_limits_info().await;
        let features_info = self.collect_features_info();
        let vector_search_info = self.collect_vector_search_info();
        let model_cache_info = self.collect_model_cache_info().await;
        let environment_info = self.collect_environment_info();
        let tools_info = self.collect_tools_info().await;
        let parsing_info = self.collect_parsing_info();

        let config = json!({
            "server": server_info,
            "cli": cli_info,
            "security": security_info,
            "resource_limits": resource_limits_info,
            "features": features_info,
            "vector_search": vector_search_info,
            "model_cache": model_cache_info,
            "environment": environment_info,
            "tools": tools_info,
            "parsing": parsing_info
        });

        Ok(config)
    }

    /// Collect server identification and capabilities
    fn collect_server_info(&self) -> Value {
        json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "protocol_version": "2024-11-05",
            "capabilities": {
                "tools": {
                    "list_changed": true
                },
                "logging": {},
                "resources": {
                    "subscribe": false,
                    "list_changed": false
                },
                "resource_templates": {
                    "list_changed": false
                },
                "completion": {
                    "argument": true
                }
            }
        })
    }

    /// Collect CLI configuration
    fn collect_cli_info(&self) -> Value {
        if let Some(ref args) = self.args {
            let watch_directories: Vec<Value> = args.watch_dir
                .iter()
                .map(|dir_spec| {
                    if let Some((path, name)) = dir_spec.split_once(':') {
                        json!({
                            "path": path,
                            "name": name
                        })
                    } else {
                        json!({
                            "path": dir_spec,
                            "name": null
                        })
                    }
                })
                .collect();

            json!({
                "command": "serve",
                "watch_directories": watch_directories,
                "admin_enabled": args.admin,
                "json_logs": args.json_logs,
                "log_level": args.log_level
            })
        } else {
            json!({
                "command": null,
                "watch_directories": [],
                "admin_enabled": false,
                "json_logs": false,
                "log_level": "info"
            })
        }
    }

    /// Collect security configuration
    fn collect_security_info(&self) -> Value {
        if let Some(ref config) = self.security_config {
            let allowed_paths: Vec<String> = config.allowed_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            let forbidden_patterns: Vec<String> = config.forbidden_patterns
                .iter()
                .map(|regex| regex.as_str().to_string())
                .collect();

            json!({
                "enabled": true,
                "allowed_paths": allowed_paths,
                "max_parameter_length": config.max_parameter_length,
                "forbidden_patterns": forbidden_patterns,
                "max_parameters": config.max_parameters,
                "strict_mode": config.strict_mode
            })
        } else {
            json!({
                "enabled": false,
                "allowed_paths": ["."],
                "max_parameter_length": 1024,
                "forbidden_patterns": [],
                "max_parameters": 50,
                "strict_mode": true
            })
        }
    }

    /// Collect resource limits configuration and current usage
    async fn collect_resource_limits_info(&self) -> Value {
        if let Some(ref limits) = self.resource_limits {
            let current_executions = if let Some(ref manager) = self.resource_manager {
                manager.current_execution_count()
            } else {
                0
            };

            json!({
                "enabled": true,
                "max_execution_time_seconds": limits.max_execution_time.as_secs(),
                "max_memory_bytes": limits.max_memory_bytes,
                "max_cpu_percent": limits.max_cpu_percent,
                "max_concurrent_executions": limits.max_concurrent_executions,
                "max_output_size_bytes": limits.max_output_size,
                "enforce_hard_limits": limits.enforce_hard_limits,
                "current_executions": current_executions
            })
        } else {
            json!({
                "enabled": false,
                "max_execution_time_seconds": 300,
                "max_memory_bytes": null,
                "max_cpu_percent": null,
                "max_concurrent_executions": 10,
                "max_output_size_bytes": 10485760,
                "enforce_hard_limits": true,
                "current_executions": 0
            })
        }
    }

    /// Collect compile-time and runtime feature availability
    fn collect_features_info(&self) -> Value {
        json!({
            "stdio_transport": cfg!(feature = "stdio"),
            "http_transport": cfg!(feature = "http"),
            "vector_search": cfg!(feature = "vector-search"),
            "local_embeddings": cfg!(feature = "local-embeddings"),
            "ast_parser": cfg!(feature = "ast-parser")
        })
    }

    /// Collect vector search configuration
    fn collect_vector_search_info(&self) -> Value {
        #[cfg(feature = "vector-search")]
        {
            json!({
                "enabled": true,
                "default_database_path": "./search_index.db",
                "default_batch_size": 32,
                "default_chunk_size": 512,
                "default_query_limit": 10
            })
        }
        #[cfg(not(feature = "vector-search"))]
        {
            Value::Null
        }
    }

    /// Collect model cache configuration and statistics
    async fn collect_model_cache_info(&self) -> Value {
        #[cfg(feature = "local-embeddings")]
        {
            // Try to get model cache stats
            let config = crate::vector_search::ModelCacheConfig::default();
            // Try to create a model cache instance to get stats
            if let Ok(cache) = crate::vector_search::ModelCache::with_config(config.clone()).await {
                let stats = cache.get_stats().await;
                json!({
                    "enabled": true,
                    "cache_directory": stats.cache_dir.to_string_lossy(),
                    "max_cache_size_bytes": config.max_cache_size,
                    "max_age_days": config.max_age_days,
                    "verify_integrity": config.verify_integrity,
                    "auto_cleanup": config.auto_cleanup,
                    "download_timeout_seconds": config.download_timeout_secs,
                    "stats": {
                        "total_models": stats.total_models,
                        "loaded_models": stats.loaded_models,
                        "total_size_bytes": stats.total_size_bytes
                    }
                })
            } else {
                json!({
                    "enabled": true,
                    "cache_directory": config.cache_dir.to_string_lossy(),
                    "max_cache_size_bytes": config.max_cache_size,
                    "max_age_days": config.max_age_days,
                    "verify_integrity": config.verify_integrity,
                    "auto_cleanup": config.auto_cleanup,
                    "download_timeout_seconds": config.download_timeout_secs,
                    "stats": {
                        "total_models": 0,
                        "loaded_models": 0,
                        "total_size_bytes": 0
                    }
                })
            }
        }
        #[cfg(not(feature = "local-embeddings"))]
        {
            Value::Null
        }
    }

    /// Collect environment and runtime information
    fn collect_environment_info(&self) -> Value {
        let working_directory = env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let platform = if cfg!(unix) {
            "unix"
        } else if cfg!(windows) {
            "windows"
        } else {
            "unknown"
        };

        let rust_log = env::var("RUST_LOG").ok();
        
        let cache_directory = dirs::cache_dir()
            .map(|p| p.to_string_lossy().to_string());

        let temp_directory = env::temp_dir().to_string_lossy().to_string();

        json!({
            "working_directory": working_directory,
            "platform": platform,
            "rust_log": rust_log,
            "cache_directory": cache_directory,
            "temp_directory": temp_directory
        })
    }

    /// Collect tool registry information
    async fn collect_tools_info(&self) -> Value {
        if let Some(ref registry) = self.tool_registry {
            let registry_guard = registry.lock().await;
            let tools = registry_guard.list_tools();
            
            let total_count = tools.len();
            let admin_tools_count = tools.iter()
                .filter(|tool| tool.name.starts_with("_admin_"))
                .count();
            let justfile_tools_count = total_count - admin_tools_count;

            // Get the last updated timestamp if available
            let last_updated = Utc::now(); // Placeholder - we'd need to track this in the registry

            json!({
                "total_count": total_count,
                "admin_tools_count": admin_tools_count,
                "justfile_tools_count": justfile_tools_count,
                "last_updated": last_updated.to_rfc3339()
            })
        } else {
            json!({
                "total_count": 0,
                "admin_tools_count": 0,
                "justfile_tools_count": 0,
                "last_updated": null
            })
        }
    }

    /// Collect parser configuration and availability
    fn collect_parsing_info(&self) -> Value {
        json!({
            "ast_parser_available": cfg!(feature = "ast-parser"),
            "cli_parser_available": true,
            "regex_parser_available": true,
            "default_parser": if cfg!(feature = "ast-parser") { "ast" } else { "cli" },
            "parser_priority": if cfg!(feature = "ast-parser") {
                vec!["ast", "cli", "regex"]
            } else {
                vec!["cli", "regex"]
            }
        })
    }
}

impl Default for ConfigDataCollector {
    fn default() -> Self {
        Self::new()
    }
}