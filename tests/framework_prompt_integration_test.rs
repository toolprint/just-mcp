//! Framework Prompt Integration Tests
//!
//! This test module verifies that prompts, particularly the /just:do-it slash command,
//! work correctly through the ultrafast-mcp framework integration.

#[cfg(all(test, feature = "ultrafast-framework"))]
mod tests {
    use just_mcp::server_v2::FrameworkServer;

    #[tokio::test]
    async fn test_framework_server_prompt_integration() {
        // Create a framework server
        let mut server = FrameworkServer::new();

        // Initialize the server (which should set up prompts)
        let result = server.initialize().await;
        assert!(result.is_ok(), "Server initialization failed: {result:?}");

        // Verify prompt provider was created
        let prompt_provider = server.prompt_provider();
        assert!(prompt_provider.is_some(), "Prompt provider not initialized");

        let provider = prompt_provider.unwrap();

        // List prompts
        let prompts = provider.list_prompts().await.unwrap();
        assert!(!prompts.is_empty(), "No prompts available");
        assert!(
            prompts.contains(&"do-it".to_string()),
            "/just:do-it prompt not found"
        );

        // Verify the do-it prompt specifically
        let do_it_prompt = provider.get_prompt_by_name("do-it").await.unwrap();
        assert!(do_it_prompt.is_some(), "do-it prompt details not available");
    }

    #[tokio::test]
    async fn test_do_it_prompt_execution_through_framework() {
        use serde_json::{Map, Value};

        // Create a framework server
        let mut server = FrameworkServer::new();

        // Initialize the server
        server.initialize().await.unwrap();

        // Get the prompt provider
        let prompt_provider = server.prompt_provider().unwrap();

        // Test executing the do-it prompt with a simple request
        let mut arguments = Map::new();
        arguments.insert(
            "request".to_string(),
            Value::String("build the project".to_string()),
        );

        // Execute through the framework-compatible interface
        let result = prompt_provider
            .execute_prompt("do-it", Value::Object(arguments))
            .await;

        // Should succeed (even if no tasks match in mock provider)
        assert!(result.is_ok(), "Prompt execution failed: {result:?}");

        let response = result.unwrap();
        assert!(
            response.contains("executed successfully"),
            "Unexpected response: {response}"
        );
    }

    #[tokio::test]
    #[cfg(feature = "ultrafast-framework")]
    async fn test_framework_prompt_handler_implementation() {
        use ultrafast_mcp::PromptHandler;
        use ultrafast_mcp::{GetPromptRequest, ListPromptsRequest};

        // Create a framework server
        let mut server = FrameworkServer::new();
        server.initialize().await.unwrap();

        let prompt_provider = server.prompt_provider().unwrap();

        // Test ListPrompts through framework API
        let list_request = ListPromptsRequest { cursor: None };
        let list_response =
            PromptHandler::list_prompts(prompt_provider.as_ref(), list_request).await;
        assert!(
            list_response.is_ok(),
            "ListPrompts failed: {list_response:?}"
        );

        let prompts = list_response.unwrap().prompts;
        assert!(!prompts.is_empty(), "No prompts returned");

        // Find do-it prompt
        let do_it = prompts.iter().find(|p| p.name == "do-it");
        assert!(do_it.is_some(), "/just:do-it prompt not in list");

        let do_it_prompt = do_it.unwrap();
        assert!(do_it_prompt.description.is_some());
        assert!(do_it_prompt
            .description
            .as_ref()
            .unwrap()
            .contains("natural language"));
        assert!(do_it_prompt.arguments.is_some());

        // Test GetPrompt through framework API
        let get_request = GetPromptRequest {
            name: "do-it".to_string(),
            arguments: None,
        };
        let get_response = prompt_provider.get_prompt(get_request).await;
        assert!(get_response.is_ok(), "GetPrompt failed: {get_response:?}");

        let prompt_detail = get_response.unwrap();
        assert!(prompt_detail.description.is_some());
        assert!(!prompt_detail.messages.is_empty());

        // Verify message structure
        let messages = &prompt_detail.messages;
        assert!(messages.len() >= 1, "Expected at least one message");

        // Check first message (combined system and user due to MCP protocol limitations)
        let first_msg = &messages[0];
        match &first_msg.role {
            ultrafast_mcp::types::PromptRole::User => {}
            _ => panic!("Expected user role (MCP protocol limitation)"),
        }
        match &first_msg.content {
            ultrafast_mcp::types::PromptContent::Text { text } => {
                assert!(text.contains("justfile task execution"));
                assert!(
                    text.contains("{{request}}"),
                    "Message should contain request template"
                );
            }
            _ => panic!("Expected text content"),
        }

        // Since MCP protocol combines system and user messages, we only expect one message
        // No separate user message to check
    }

    #[tokio::test]
    async fn test_slash_command_format() {
        // Create a framework server
        let mut server = FrameworkServer::new();
        server.initialize().await.unwrap();

        let prompt_provider = server.prompt_provider().unwrap();

        // List prompts
        let prompts = prompt_provider.list_prompts().await.unwrap();

        // The prompt should be available as "do-it" (which translates to /just:do-it slash command)
        assert!(prompts.contains(&"do-it".to_string()));

        // In MCP, slash commands are typically in the format "/namespace:command"
        // The "do-it" prompt should be exposed as "/just:do-it" to clients
        // This is handled by the client's interpretation of the prompt name

        // Verify we can retrieve it
        let prompt_def = prompt_provider.get_prompt_by_name("do-it").await.unwrap();
        assert!(prompt_def.is_some());
    }
}
