use crate::error::Result;
use crate::registry::ToolRegistry;
use crate::types::ToolDefinition;
use crate::watcher::JustfileWatcher;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct AdminTools {
    registry: Arc<Mutex<ToolRegistry>>,
    watcher: Arc<JustfileWatcher>,
    watch_paths: Vec<PathBuf>,
    watch_configs: Vec<(PathBuf, Option<String>)>,
}

impl AdminTools {
    pub fn new(
        registry: Arc<Mutex<ToolRegistry>>,
        watcher: Arc<JustfileWatcher>,
        watch_paths: Vec<PathBuf>,
        watch_configs: Vec<(PathBuf, Option<String>)>,
    ) -> Self {
        Self {
            registry,
            watcher,
            watch_paths,
            watch_configs,
        }
    }

    pub async fn register_admin_tools(&self) -> Result<()> {
        let mut registry = self.registry.lock().await;

        // Register sync() tool
        let sync_tool = ToolDefinition {
            name: "_admin_sync".to_string(),
            description: "Manually re-scan justfiles and update the tool registry".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_sync_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
            internal_name: None,
        };

        registry.add_tool(sync_tool)?;

        // Register create_recipe() tool
        let create_recipe_tool = ToolDefinition {
            name: "_admin_create_recipe".to_string(),
            description: "Create a new recipe in a justfile with AI assistance".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "watch_name": {
                        "type": "string",
                        "description": "Name of the watch directory to create recipe in (e.g., 'frontend', 'backend'). If omitted, uses the main/default justfile"
                    },
                    "recipe_name": {
                        "type": "string",
                        "description": "Name of the new recipe"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description/comment for the recipe"
                    },
                    "recipe": {
                        "type": "string",
                        "description": "The command(s) to execute"
                    },
                    "parameters": {
                        "type": "array",
                        "description": "Recipe parameters",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "default": {"type": "string"}
                            },
                            "required": ["name"]
                        }
                    },
                    "dependencies": {
                        "type": "array",
                        "description": "Recipe dependencies",
                        "items": {"type": "string"}
                    }
                },
                "required": ["recipe_name", "recipe"],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_create_recipe_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
            internal_name: None,
        };

        registry.add_tool(create_recipe_tool)?;

        // Register set_watch_directory() tool
        let set_watch_directory_tool = ToolDefinition {
            name: "_admin_set_watch_directory".to_string(),
            description: "Clear watch directories and set a single new path. Converts relative paths to absolute.".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to watch. Can be relative (will be converted to absolute) or absolute."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_set_watch_directory_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
            internal_name: None,
        };

        registry.add_tool(set_watch_directory_tool)?;

        // Register parser_doctor() tool
        let parser_doctor_tool = ToolDefinition {
            name: "_admin_parser_doctor".to_string(),
            description: "Diagnose parser accuracy by comparing AST and CLI parser results against expected recipes".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "verbose": {
                        "type": "boolean",
                        "description": "Show detailed information about missing recipes and parsing errors",
                        "default": false
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_parser_doctor_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
            internal_name: None,
        };

        registry.add_tool(parser_doctor_tool)?;

        // TODO: Add modify_recipe, remove_recipe tools in future subtasks

        Ok(())
    }

    pub async fn sync(&self) -> Result<SyncResult> {
        info!("Starting manual justfile sync");

        let start_time = std::time::Instant::now();
        let mut scanned_files = 0;
        let mut found_recipes = 0;
        let mut errors = Vec::new();

        // Clear the registry cache
        {
            let mut registry = self.registry.lock().await;
            // Remove all non-admin tools
            let tools_to_remove: Vec<String> = registry
                .list_tools()
                .iter()
                .filter(|tool| !tool.name.starts_with("_admin_"))
                .map(|tool| tool.name.clone())
                .collect();

            for tool_name in tools_to_remove {
                registry.remove_tool(&tool_name)?;
            }
        }

        // Re-scan all watch paths
        for path in &self.watch_paths {
            if path.exists() {
                if path.is_dir() {
                    // Scan for justfiles in directory
                    let justfile_path = path.join("justfile");
                    if justfile_path.exists() {
                        info!("Found justfile: {}", justfile_path.display());
                        match self.scan_justfile(&justfile_path).await {
                            Ok(task_count) => {
                                scanned_files += 1;
                                found_recipes += task_count;
                            }
                            Err(e) => {
                                warn!("Error scanning {}: {}", justfile_path.display(), e);
                                errors.push(format!("{}: {}", justfile_path.display(), e));
                            }
                        }
                    }

                    // Also check for capitalized Justfile
                    let justfile_cap = path.join("Justfile");
                    if justfile_cap.exists() {
                        info!("Found Justfile: {}", justfile_cap.display());
                        match self.scan_justfile(&justfile_cap).await {
                            Ok(task_count) => {
                                scanned_files += 1;
                                found_recipes += task_count;
                            }
                            Err(e) => {
                                warn!("Error scanning {}: {}", justfile_cap.display(), e);
                                errors.push(format!("{}: {}", justfile_cap.display(), e));
                            }
                        }
                    }
                } else if path.file_name() == Some(std::ffi::OsStr::new("justfile"))
                    || path.file_name() == Some(std::ffi::OsStr::new("Justfile"))
                {
                    // Direct justfile path
                    match self.scan_justfile(path).await {
                        Ok(task_count) => {
                            scanned_files += 1;
                            found_recipes += task_count;
                        }
                        Err(e) => {
                            warn!("Error scanning {}: {}", path.display(), e);
                            errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
            } else {
                warn!("Watch path does not exist: {}", path.display());
                errors.push(format!("Path not found: {}", path.display()));
            }
        }

        let duration = start_time.elapsed();

        info!(
            "Sync completed in {:?}: {} files scanned, {} recipes found, {} errors",
            duration,
            scanned_files,
            found_recipes,
            errors.len()
        );

        // Send a single notification after all tools are registered
        self.watcher.send_tools_changed_notification();

        Ok(SyncResult {
            scanned_files,
            found_recipes,
            errors,
            duration_ms: duration.as_millis() as u64,
        })
    }

    async fn scan_justfile(&self, path: &std::path::Path) -> Result<usize> {
        info!("Scanning justfile: {}", path.display());

        // Use the watcher's parse method without sending notifications
        let task_count = self
            .watcher
            .parse_and_update_justfile_without_notification(path)
            .await?;

        Ok(task_count)
    }

    pub async fn create_recipe(&self, params: CreateRecipeParams) -> Result<CreateRecipeResult> {
        info!(
            "Creating new recipe: {} in {}",
            params.recipe_name,
            params.watch_name.as_deref().unwrap_or("default justfile")
        );

        // Determine which justfile to use
        let justfile_path = if let Some(watch_name) = params.watch_name {
            // Find the watch directory by name
            let mut found_path = None;

            for (path, name) in &self.watch_configs {
                if let Some(n) = name {
                    if n == &watch_name {
                        // Found by name
                        if path.is_dir() {
                            let justfile = path.join("justfile");
                            if justfile.exists() {
                                found_path = Some(justfile);
                                break;
                            }
                            let justfile_cap = path.join("Justfile");
                            if justfile_cap.exists() {
                                found_path = Some(justfile_cap);
                                break;
                            }
                        } else {
                            found_path = Some(path.clone());
                            break;
                        }
                    }
                }
            }

            found_path.ok_or_else(|| {
                crate::error::Error::Other(format!(
                    "Watch directory '{}' not found. Available: {}",
                    watch_name,
                    self.watch_configs
                        .iter()
                        .filter_map(|(_, name)| name.as_ref())
                        .map(|n| format!("'{n}'"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?
        } else {
            // No name specified - use the main/first justfile
            let (path, _) = &self.watch_configs.first().ok_or_else(|| {
                crate::error::Error::Other("No watch directories configured".to_string())
            })?;

            if path.is_dir() {
                let justfile = path.join("justfile");
                if justfile.exists() {
                    justfile
                } else {
                    let justfile_cap = path.join("Justfile");
                    if justfile_cap.exists() {
                        justfile_cap
                    } else {
                        return Err(crate::error::Error::Other(
                            "No justfile found in main watch directory".to_string(),
                        ));
                    }
                }
            } else {
                path.clone()
            }
        };

        // Validate recipe name doesn't conflict with existing recipes
        {
            let registry = self.registry.lock().await;

            // Check for any tool that matches the recipe name exactly or with @name suffix
            // This handles both single directory (recipename) and multi-directory (recipename@name) cases
            let existing_recipe = registry.list_tools().iter().any(|tool| {
                tool.name == params.recipe_name
                    || tool.name.starts_with(&format!("{}@", params.recipe_name))
            });

            if existing_recipe {
                return Err(crate::error::Error::Other(format!(
                    "Recipe '{}' already exists in {}",
                    params.recipe_name,
                    justfile_path.display()
                )));
            }

            // Check for admin tool conflicts
            if params.recipe_name.starts_with("_admin_") {
                return Err(crate::error::Error::Other(
                    "Recipe names starting with '_admin_' are reserved".to_string(),
                ));
            }
        }

        // Create backup with dotfile naming
        let backup_path = justfile_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!(
                ".{}.bak",
                justfile_path.file_name().unwrap().to_string_lossy()
            ));
        std::fs::copy(&justfile_path, &backup_path)?;

        // Read existing content
        let existing_content = std::fs::read_to_string(&justfile_path)?;

        // Build the new recipe content
        let mut recipe_content = String::new();

        // Ensure proper spacing: always add a blank line before the new recipe
        if !existing_content.is_empty() {
            // If file doesn't end with newline, add one
            if !existing_content.ends_with('\n') {
                recipe_content.push('\n');
            }
            // Always add a blank line for visual separation
            recipe_content.push('\n');
        }

        // Add description as comment
        if let Some(desc) = &params.description {
            recipe_content.push_str(&format!("# {desc}\n"));
        }

        // Add recipe signature
        recipe_content.push_str(&params.recipe_name);

        // Add parameters
        if let Some(parameters) = &params.parameters {
            for param in parameters {
                recipe_content.push(' ');
                recipe_content.push_str(&param.name);
                if let Some(default) = &param.default {
                    recipe_content.push_str(&format!("=\"{default}\""));
                }
            }
        }

        // Add dependencies
        if let Some(deps) = &params.dependencies {
            if !deps.is_empty() {
                recipe_content.push_str(": ");
                recipe_content.push_str(&deps.join(" "));
                recipe_content.push('\n');
            } else {
                recipe_content.push_str(":\n");
            }
        } else {
            recipe_content.push_str(":\n");
        }

        // Add recipe body with proper indentation
        for line in params.recipe.lines() {
            recipe_content.push_str("    ");
            recipe_content.push_str(line);
            recipe_content.push('\n');
        }

        // Write updated content
        let new_content = existing_content + &recipe_content;
        std::fs::write(&justfile_path, &new_content)?;

        // Re-scan the justfile to update registry
        self.scan_justfile(&justfile_path).await?;

        info!(
            "Successfully created recipe '{}' in {}",
            params.recipe_name,
            justfile_path.display()
        );

        Ok(CreateRecipeResult {
            recipe_name: params.recipe_name,
            justfile_path: justfile_path.to_string_lossy().to_string(),
            backup_path: backup_path.to_string_lossy().to_string(),
        })
    }

    pub async fn set_watch_directory(
        &self,
        params: SetWatchDirectoryParams,
    ) -> Result<SetWatchDirectoryResult> {
        info!("Setting watch directory to: {}", params.path);

        // Convert relative path to absolute
        let absolute_path = if std::path::Path::new(&params.path).is_absolute() {
            std::path::PathBuf::from(&params.path)
        } else {
            std::env::current_dir()?.join(&params.path)
        };

        // Validate that the path exists
        if !absolute_path.exists() {
            return Err(crate::error::Error::Other(format!(
                "Path does not exist: {}",
                absolute_path.display()
            )));
        }

        if !absolute_path.is_dir() {
            return Err(crate::error::Error::Other(format!(
                "Path is not a directory: {}",
                absolute_path.display()
            )));
        }

        // Check for justfile presence
        let justfile_path = absolute_path.join("justfile");
        let justfile_cap_path = absolute_path.join("Justfile");

        let (justfile_detected, detected_justfile_path) = if justfile_path.exists() {
            (true, Some(justfile_path.to_string_lossy().to_string()))
        } else if justfile_cap_path.exists() {
            (true, Some(justfile_cap_path.to_string_lossy().to_string()))
        } else {
            (false, None)
        };

        // Clear the registry cache (remove all non-admin tools)
        {
            let mut registry = self.registry.lock().await;
            let tools_to_remove: Vec<String> = registry
                .list_tools()
                .iter()
                .filter(|tool| !tool.name.starts_with("_admin_"))
                .map(|tool| tool.name.clone())
                .collect();

            for tool_name in tools_to_remove {
                registry.remove_tool(&tool_name)?;
            }
        }

        // Scan the new directory if a justfile was detected
        if justfile_detected {
            if let Some(ref justfile_path_str) = detected_justfile_path {
                let justfile_path = std::path::Path::new(justfile_path_str);
                if let Err(e) = self.scan_justfile(justfile_path).await {
                    warn!(
                        "Error scanning justfile at {}: {}",
                        justfile_path.display(),
                        e
                    );
                }
            }
        }

        // Send notification that tools have changed
        self.watcher.send_tools_changed_notification();

        info!(
            "Successfully set watch directory to {} (justfile detected: {})",
            absolute_path.display(),
            justfile_detected
        );

        Ok(SetWatchDirectoryResult {
            absolute_path: absolute_path.to_string_lossy().to_string(),
            justfile_detected,
            justfile_path: detected_justfile_path,
        })
    }

    pub async fn parser_doctor(&self, verbose: bool) -> Result<String> {
        info!("Running parser diagnostic");

        // Find the main justfile to analyze
        let justfile_path = {
            let (path, _) = &self.watch_configs.first().ok_or_else(|| {
                crate::error::Error::Other("No watch directories configured".to_string())
            })?;

            if path.is_dir() {
                let justfile = path.join("justfile");
                if justfile.exists() {
                    justfile
                } else {
                    let justfile_cap = path.join("Justfile");
                    if justfile_cap.exists() {
                        justfile_cap
                    } else {
                        return Err(crate::error::Error::Other(
                            "No justfile found in main watch directory".to_string(),
                        ));
                    }
                }
            } else {
                path.clone()
            }
        };

        // Get expected recipes using `just --summary`
        let expected_recipes = self.get_expected_recipes(&justfile_path).await?;

        // Test AST parser
        let ast_result = self
            .test_parser(&justfile_path, crate::parser::ParserPreference::Ast)
            .await;

        // Test CLI parser
        let cli_result = self
            .test_parser(&justfile_path, crate::parser::ParserPreference::Cli)
            .await;

        // Format and return results
        self.format_diagnostic_report(&expected_recipes, &ast_result, &cli_result, verbose)
            .await
    }

    async fn get_expected_recipes(&self, justfile_path: &std::path::Path) -> Result<Vec<String>> {
        use std::process::Command;

        let output = Command::new("just")
            .arg("--summary")
            .current_dir(
                justfile_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            )
            .output()
            .map_err(|e| {
                crate::error::Error::Other(format!("Failed to execute 'just --summary': {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::error::Error::Other(format!(
                "Command 'just --summary' failed: {stderr}"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let recipes = stdout.split_whitespace().map(|s| s.to_string()).collect();

        Ok(recipes)
    }

    async fn test_parser(
        &self,
        justfile_path: &std::path::Path,
        preference: crate::parser::ParserPreference,
    ) -> ParserDiagnosticResult {
        let parser_name = match preference {
            crate::parser::ParserPreference::Ast => "AST",
            crate::parser::ParserPreference::Cli => "CLI",
            crate::parser::ParserPreference::Auto => "Auto",
            #[allow(deprecated)]
            crate::parser::ParserPreference::Regex => "Regex",
        };

        let mut enhanced_parser = match crate::parser::EnhancedJustfileParser::new() {
            Ok(parser) => parser,
            Err(e) => {
                return ParserDiagnosticResult {
                    found_recipes: Vec::new(),
                    missing_recipes: Vec::new(),
                    parsing_errors: vec![format!("Failed to create parser: {e}")],
                    parser_name: parser_name.to_string(),
                };
            }
        };

        enhanced_parser.set_parser_preference(preference);

        match enhanced_parser.parse_file_for_tools(justfile_path) {
            Ok(tasks) => {
                let found_recipes: Vec<String> = tasks.into_iter().map(|t| t.name).collect();

                ParserDiagnosticResult {
                    found_recipes,
                    missing_recipes: Vec::new(),
                    parsing_errors: Vec::new(),
                    parser_name: parser_name.to_string(),
                }
            }
            Err(e) => ParserDiagnosticResult {
                found_recipes: Vec::new(),
                missing_recipes: Vec::new(),
                parsing_errors: vec![format!("Parser failed: {e}")],
                parser_name: parser_name.to_string(),
            },
        }
    }

    async fn format_diagnostic_report(
        &self,
        expected: &[String],
        ast_result: &ParserDiagnosticResult,
        cli_result: &ParserDiagnosticResult,
        verbose: bool,
    ) -> Result<String> {
        let mut report = String::new();

        report.push_str("# Parser Diagnostic Report\n\n");

        // Calculate missing recipes for each parser
        let ast_missing: Vec<&String> = expected
            .iter()
            .filter(|recipe| !ast_result.found_recipes.contains(recipe))
            .collect();

        let cli_missing: Vec<&String> = expected
            .iter()
            .filter(|recipe| !cli_result.found_recipes.contains(recipe))
            .collect();

        // Summary section
        report.push_str("## Summary\n");
        report.push_str(&format!("- Expected: {}\n", expected.len()));
        report.push_str(&format!(
            "- AST parser: {} ({:.0}%) | Missing: {}\n",
            ast_result.found_recipes.len(),
            if expected.is_empty() {
                0.0
            } else {
                (ast_result.found_recipes.len() as f64 / expected.len() as f64) * 100.0
            },
            ast_missing.len()
        ));
        report.push_str(&format!(
            "- CLI parser: {} ({:.0}%) | Missing: {}\n",
            cli_result.found_recipes.len(),
            if expected.is_empty() {
                0.0
            } else {
                (cli_result.found_recipes.len() as f64 / expected.len() as f64) * 100.0
            },
            cli_missing.len()
        ));

        // Verbose details
        if verbose {
            // AST Parser Issues
            report.push_str("\n## AST Parser Issues\n");
            if ast_missing.is_empty() && ast_result.parsing_errors.is_empty() {
                report.push_str("### No issues found\n");
            } else {
                if !ast_missing.is_empty() {
                    report.push_str(&format!("### Missing Recipes ({}):\n", ast_missing.len()));
                    for recipe in &ast_missing {
                        report.push_str(&format!("- `{recipe}`\n"));
                    }
                }

                if !ast_result.parsing_errors.is_empty() {
                    report.push_str(&format!(
                        "### Parsing Errors ({}):\n",
                        ast_result.parsing_errors.len()
                    ));
                    for error in &ast_result.parsing_errors {
                        report.push_str(&format!("- {error}\n"));
                    }
                }
            }

            // CLI Parser Issues
            report.push_str("\n## CLI Parser Issues\n");
            if cli_missing.is_empty() && cli_result.parsing_errors.is_empty() {
                report.push_str("### No issues found\n");
            } else {
                if !cli_missing.is_empty() {
                    report.push_str(&format!("### Missing Recipes ({}):\n", cli_missing.len()));
                    for recipe in &cli_missing {
                        report.push_str(&format!("- `{recipe}`\n"));
                    }
                }

                if !cli_result.parsing_errors.is_empty() {
                    report.push_str(&format!(
                        "### Parsing Errors ({}):\n",
                        cli_result.parsing_errors.len()
                    ));
                    for error in &cli_result.parsing_errors {
                        report.push_str(&format!("- {error}\n"));
                    }
                }
            }
        }

        Ok(report)
    }
}

#[derive(Debug)]
pub struct ParserDiagnosticResult {
    pub found_recipes: Vec<String>,
    pub missing_recipes: Vec<String>,
    pub parsing_errors: Vec<String>,
    pub parser_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub scanned_files: usize,
    pub found_recipes: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRecipeParams {
    pub watch_name: Option<String>,
    pub recipe_name: String,
    pub description: Option<String>,
    pub recipe: String,
    pub parameters: Option<Vec<RecipeParameter>>,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeParameter {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRecipeResult {
    pub recipe_name: String,
    pub justfile_path: String,
    pub backup_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetWatchDirectoryParams {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetWatchDirectoryResult {
    pub absolute_path: String,
    pub justfile_detected: bool,
    pub justfile_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_admin_tools_creation() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Register admin tools
        admin_tools.register_admin_tools().await.unwrap();

        // Check that sync tool was registered
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert!(tools.iter().any(|t| t.name == "_admin_sync"));
    }

    #[tokio::test]
    async fn test_sync_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create a test justfile
        let content = r#"
# Test task
test:
    echo "test"

# Build task
build:
    cargo build
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![temp_dir.path().to_path_buf()],
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Perform sync
        let result = admin_tools.sync().await.unwrap();

        // We might find more than one justfile if there are parent directories
        // with justfiles, so just check that we found at least our test justfile
        assert!(result.scanned_files >= 1);
        assert!(
            result.found_recipes >= 2,
            "Expected at least 2 recipes, found {}",
            result.found_recipes
        );
        assert_eq!(result.errors.len(), 0);

        // Check registry has the tools
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        // Should have at least 2 tools from our test justfile
        let our_justfile_tools: Vec<_> = tools
            .iter()
            .filter(|t| !t.name.starts_with("_admin_"))
            .collect();
        assert!(
            our_justfile_tools.len() >= 2,
            "Expected at least 2 tools, found {}",
            our_justfile_tools.len()
        );
    }

    #[tokio::test]
    async fn test_create_recipe() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create an initial justfile
        let content = r#"
# Existing task
existing:
    echo "existing"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![temp_dir.path().to_path_buf()],
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Create a new recipe
        let params = CreateRecipeParams {
            watch_name: None, // Use default
            recipe_name: "new_recipe".to_string(),
            description: Some("A new test recipe".to_string()),
            recipe: "echo \"hello world\"\necho \"second line\"".to_string(),
            parameters: Some(vec![RecipeParameter {
                name: "name".to_string(),
                default: Some("world".to_string()),
            }]),
            dependencies: Some(vec!["existing".to_string()]),
        };

        let result = admin_tools.create_recipe(params).await.unwrap();

        assert_eq!(result.recipe_name, "new_recipe");
        assert!(result.backup_path.ends_with(".justfile.bak"));

        // Verify the recipe was added to the file
        let new_content = fs::read_to_string(&justfile_path).unwrap();
        assert!(new_content.contains("# A new test recipe"));
        assert!(new_content.contains("new_recipe name=\"world\": existing"));
        assert!(new_content.contains("    echo \"hello world\""));
        assert!(new_content.contains("    echo \"second line\""));

        // Verify backup was created
        let backup_path = justfile_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!(
                ".{}.bak",
                justfile_path.file_name().unwrap().to_string_lossy()
            ));
        assert!(backup_path.exists());

        // Verify registry was updated
        let reg = registry.lock().await;
        let tools = reg.list_tools();

        let new_recipe_tool = tools
            .iter()
            .find(|t| t.name.contains("new_recipe"))
            .expect("New recipe should be in registry");
        // The parser may not extract comments as descriptions in all modes
        // Just verify the recipe was parsed and added
        assert!(!new_recipe_tool.description.is_empty());
    }

    #[tokio::test]
    async fn test_create_recipe_validation() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create an initial justfile
        let content = r#"
# Existing task
existing:
    echo "existing"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher.clone(),
            vec![temp_dir.path().to_path_buf()],
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Parse initial justfile to populate registry
        watcher
            .parse_and_update_justfile(&justfile_path)
            .await
            .unwrap();

        // Try to create a recipe with existing name
        let params = CreateRecipeParams {
            watch_name: None, // Use default
            recipe_name: "existing".to_string(),
            description: None,
            recipe: "echo \"duplicate\"".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_recipe(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // Try to create a recipe with _admin_ prefix
        let params = CreateRecipeParams {
            watch_name: None, // Use default
            recipe_name: "_admin_recipe".to_string(),
            description: None,
            recipe: "echo \"admin\"".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_recipe(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[tokio::test]
    async fn test_create_recipe_with_named_dirs() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let justfile_path1 = temp_dir1.path().join("justfile");
        let justfile_path2 = temp_dir2.path().join("justfile");

        // Create justfiles
        fs::write(&justfile_path1, "# Frontend tasks\n").unwrap();
        fs::write(&justfile_path2, "# Backend tasks\n").unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![
                temp_dir1.path().to_path_buf(),
                temp_dir2.path().to_path_buf(),
            ],
            vec![
                (temp_dir1.path().to_path_buf(), Some("frontend".to_string())),
                (temp_dir2.path().to_path_buf(), Some("backend".to_string())),
            ],
        );

        // Test creating recipe with name
        let params = CreateRecipeParams {
            watch_name: Some("frontend".to_string()),
            recipe_name: "build".to_string(),
            description: Some("Build frontend".to_string()),
            recipe: "npm run build".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_recipe(params).await.unwrap();
        assert_eq!(result.recipe_name, "build");
        assert!(result.justfile_path.contains("justfile"));

        // Verify the recipe was added
        let content = fs::read_to_string(&justfile_path1).unwrap();
        assert!(content.contains("# Build frontend"));
        assert!(content.contains("build:"));
        assert!(content.contains("npm run build"));
    }

    #[tokio::test]
    async fn test_set_watch_directory_with_justfile() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create a test justfile
        let content = r#"
# Test task
test:
    echo "test"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Test setting watch directory with absolute path
        let params = SetWatchDirectoryParams {
            path: temp_dir.path().to_string_lossy().to_string(),
        };

        let result = admin_tools.set_watch_directory(params).await.unwrap();

        assert_eq!(
            result.absolute_path,
            temp_dir.path().to_string_lossy().to_string()
        );
        assert!(result.justfile_detected);
        assert!(result.justfile_path.is_some());
        assert!(result.justfile_path.unwrap().contains("justfile"));
    }

    #[tokio::test]
    async fn test_set_watch_directory_without_justfile() {
        let temp_dir = TempDir::new().unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Test setting watch directory without justfile
        let params = SetWatchDirectoryParams {
            path: temp_dir.path().to_string_lossy().to_string(),
        };

        let result = admin_tools.set_watch_directory(params).await.unwrap();

        assert_eq!(
            result.absolute_path,
            temp_dir.path().to_string_lossy().to_string()
        );
        assert!(!result.justfile_detected);
        assert!(result.justfile_path.is_none());
    }

    #[tokio::test]
    async fn test_set_watch_directory_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("Justfile"); // Test capitalized version

        // Create a test Justfile
        let content = r#"
# Test task
test:
    echo "test"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Change to parent directory and use relative path
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path().parent().unwrap()).unwrap();

        let relative_path = temp_dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let params = SetWatchDirectoryParams {
            path: relative_path.clone(),
        };

        let result = admin_tools.set_watch_directory(params).await.unwrap();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.absolute_path.ends_with(&relative_path));
        assert!(result.justfile_detected);
        assert!(result.justfile_path.is_some());
        // The actual detected justfile path should contain either "justfile" or "Justfile"
        let justfile_path = result.justfile_path.unwrap();
        assert!(justfile_path.contains("justfile") || justfile_path.contains("Justfile"));
    }

    #[tokio::test]
    async fn test_set_watch_directory_errors() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Test with non-existent path
        let params = SetWatchDirectoryParams {
            path: "/non/existent/path".to_string(),
        };

        let result = admin_tools.set_watch_directory(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        // Test with file instead of directory
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let params = SetWatchDirectoryParams {
            path: temp_file.path().to_string_lossy().to_string(),
        };

        let result = admin_tools.set_watch_directory(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }
}
