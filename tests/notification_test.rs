use just_mcp::registry::ToolRegistry;
use just_mcp::watcher::JustfileWatcher;
use std::fs;

use std::sync::Arc;
use tokio::sync::Mutex;

mod common;
use common::{cleanup_test_dir, create_test_dir_with_justfile};

#[tokio::test]
async fn test_watcher_updates_registry_on_change() {
    // Use test fixtures directory
    let (_test_dir, justfile_path) = create_test_dir_with_justfile("watcher_notification_test");

    // Create registry and watcher
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = JustfileWatcher::new(registry.clone());

    // Create initial justfile
    let justfile_content = r#"
# Test task
test:
    echo "test"
"#;
    fs::write(&justfile_path, justfile_content).unwrap();

    // Parse and update
    println!("Justfile path: {}", justfile_path.display());
    watcher
        .parse_and_update_justfile(&justfile_path)
        .await
        .unwrap();

    // Check registry has the tool
    {
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert_eq!(tools.len(), 1);
        assert!(tools[0].name.contains("test"));
    }

    // Update justfile with new task
    let updated_content = r#"
# Test task
test:
    echo "test"

# Build task
build:
    cargo build
"#;
    fs::write(&justfile_path, updated_content).unwrap();

    // Parse and update again
    watcher
        .parse_and_update_justfile(&justfile_path)
        .await
        .unwrap();

    // Check registry has both tools
    {
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert_eq!(tools.len(), 2);
        let tool_names: Vec<_> = tools.iter().map(|t| &t.name).collect();
        assert!(tool_names.iter().any(|n| n.contains("test")));
        assert!(tool_names.iter().any(|n| n.contains("build")));
    }

    // Remove justfile content (simulate deletion of all tasks)
    fs::write(&justfile_path, "").unwrap();

    // Parse and update again
    watcher
        .parse_and_update_justfile(&justfile_path)
        .await
        .unwrap();

    // Check registry is empty
    {
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        // Debug output
        println!("Tools after empty justfile: {}", tools.len());
        for tool in &tools {
            println!("  Tool: {}", tool.name);
        }
        // The watcher should have removed the tools when it found no tasks
        assert_eq!(
            tools.len(),
            0,
            "Expected empty registry after parsing empty justfile"
        );
    }

    // Cleanup
    cleanup_test_dir("watcher_notification_test");
}

#[tokio::test]
async fn test_notification_flow_simulation() {
    // This test simulates what would happen in a full notification flow
    // without actually testing the private notification infrastructure

    // The notification flow is:
    // 1. Watcher detects change
    // 2. Watcher updates registry
    // 3. Watcher sends notification (internal)
    // 4. Server receives notification (internal)
    // 5. Server sends JSON-RPC notification to client

    // We can test steps 1-2 above, and the rest would be tested
    // through integration tests with a real MCP client
}
