//! Framework Resource Integration Test
//!
//! Tests that the ultrafast-mcp framework resource integration works correctly.

#[cfg(feature = "ultrafast-framework")]
mod framework_tests {
    use just_mcp::registry::ToolRegistry;
    use just_mcp::server_v2::resources::create_framework_resource_provider;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_framework_resource_provider_creation() {
        // Test that we can create a framework resource provider
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));

        let result = create_framework_resource_provider(
            None, // args
            None, // security_config
            None, // resource_limits
            registry,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should be able to create framework resource provider"
        );
        let _provider = result.unwrap();
        // If we get here, the provider was created successfully
    }

    #[tokio::test]
    async fn test_framework_resource_handler_trait() {
        use ultrafast_mcp::{ListResourcesRequest, ResourceHandler};

        // Test that the ResourceHandler trait is properly implemented
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));

        let provider = create_framework_resource_provider(None, None, None, registry)
            .await
            .expect("Should create provider");

        // Cast to ResourceHandler trait and test list_resources
        let handler: &dyn ResourceHandler = &provider;
        let request = ListResourcesRequest { cursor: None };
        let result = handler.list_resources(request).await;

        assert!(result.is_ok(), "Should be able to list resources");
        let _response = result.unwrap();
        // Resources list should be valid (non-null)
        // Length comparison >= 0 removed as it's always true for usize
    }

    #[tokio::test]
    async fn test_embedded_resources_available() {
        use ultrafast_mcp::{ListResourcesRequest, ResourceHandler};

        // Test that embedded resources are available through the framework
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));

        let provider = create_framework_resource_provider(None, None, None, registry)
            .await
            .expect("Should create provider");

        // Cast to ResourceHandler trait and test list_resources
        let handler: &dyn ResourceHandler = &provider;
        let request = ListResourcesRequest { cursor: None };
        let response = handler
            .list_resources(request)
            .await
            .expect("Should list resources");

        // Check if we have embedded resources (like the best practices guide)
        let has_embedded_resources = response
            .resources
            .iter()
            .any(|r| r.uri.starts_with("file:///docs/guides/"));

        assert!(
            has_embedded_resources,
            "Should have embedded resources available"
        );
    }

    #[tokio::test]
    async fn test_resource_read_functionality() {
        use ultrafast_mcp::{ListResourcesRequest, ReadResourceRequest, ResourceHandler};

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));

        let provider = create_framework_resource_provider(None, None, None, registry)
            .await
            .expect("Should create provider");

        // Cast to ResourceHandler trait
        let handler: &dyn ResourceHandler = &provider;

        // First, get the list of resources
        let list_request = ListResourcesRequest { cursor: None };
        let list_response = handler
            .list_resources(list_request)
            .await
            .expect("Should list resources");

        if let Some(resource) = list_response.resources.first() {
            // Try to read the first resource
            let read_request = ReadResourceRequest {
                uri: resource.uri.clone(),
            };
            let read_result = handler.read_resource(read_request).await;

            assert!(
                read_result.is_ok(),
                "Should be able to read resource: {}",
                resource.uri
            );
            let read_response = read_result.unwrap();
            assert!(!read_response.contents.is_empty(), "Should have content");
        }
    }
}

// Dummy test when framework feature is not enabled
#[cfg(not(feature = "ultrafast-framework"))]
#[tokio::test]
async fn test_framework_not_available() {
    // This test just ensures the file compiles when the framework feature is disabled
    assert!(true, "Framework feature not enabled - this is expected");
}
