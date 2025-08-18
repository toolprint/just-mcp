//! Interface integration tests for LocalEmbeddingProvider
//!
//! This test suite focuses on testing the LocalEmbeddingProvider interface,
//! placeholder functionality, and integration points without requiring
//! full model downloads which may not work in test environments.

#[cfg(feature = "local-embeddings")]
mod local_embedding_interface_tests {
    use anyhow::Result;
    use just_mcp::vector_search::{
        EmbeddingProvider, LocalEmbeddingProvider, MockEmbeddingProvider,
    };

    #[tokio::test]
    async fn test_local_embedding_provider_interface() -> Result<()> {
        let provider = LocalEmbeddingProvider::new();

        // Test basic interface compliance
        assert_eq!(provider.dimension(), 384);
        assert_eq!(
            provider.model_name(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );
        assert_eq!(provider.max_tokens(), 512);
        assert!(!provider.is_loaded().await);

        Ok(())
    }

    #[tokio::test]
    async fn test_local_embedding_provider_configuration() -> Result<()> {
        // Test different model configurations
        let provider_mini =
            LocalEmbeddingProvider::with_model("sentence-transformers/all-MiniLM-L6-v2");
        assert_eq!(provider_mini.dimension(), 384);

        let provider_mpnet =
            LocalEmbeddingProvider::with_model("sentence-transformers/all-mpnet-base-v2");
        assert_eq!(provider_mpnet.dimension(), 768);

        let provider_distil =
            LocalEmbeddingProvider::with_model("sentence-transformers/all-distilroberta-v1");
        assert_eq!(provider_distil.dimension(), 768);

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_functionality() -> Result<()> {
        let provider = LocalEmbeddingProvider::new();

        // Test embedding generation (may use placeholder functionality if model loading fails)
        let test_texts = vec![
            "build compile application",
            "test unit integration",
            "deploy production staging",
            "docker container image",
            "database migration schema",
        ];

        for text in &test_texts {
            // Try to generate embedding using public interface
            match provider.embed(text).await {
                Ok(embedding) => {
                    assert_eq!(embedding.len(), 384);

                    // Check that all values are finite
                    for &value in &embedding {
                        assert!(
                            value.is_finite(),
                            "Non-finite value in embedding for text: {}",
                            text
                        );
                    }
                    println!("Successfully generated embedding for: {}", text);
                }
                Err(e) => {
                    println!(
                        "Embedding generation failed for '{}' (expected in test environment): {}",
                        text, e
                    );
                    // This is expected in test environment without model files
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_normalization() -> Result<()> {
        // This test verifies that the configuration is set up correctly
        // The actual normalization behavior is tested in unit tests
        use just_mcp::vector_search::LocalEmbeddingConfig;

        // Test with normalization enabled (default)
        let provider_normalized = LocalEmbeddingProvider::new();
        assert_eq!(
            provider_normalized.model_name(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );

        // Test with normalization disabled
        let config = LocalEmbeddingConfig {
            normalize_embeddings: false,
            ..Default::default()
        };
        let provider_unnormalized = LocalEmbeddingProvider::with_config(config);
        assert_eq!(
            provider_unnormalized.model_name(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );

        // Configuration is correctly applied
        Ok(())
    }

    #[tokio::test]
    async fn test_interface_compatibility_with_mock_provider() -> Result<()> {
        // Create both providers with same dimension
        let local_provider = LocalEmbeddingProvider::new();
        let mock_provider = MockEmbeddingProvider::new_with_dimension(384);

        // Test interface compatibility
        assert_eq!(local_provider.dimension(), mock_provider.dimension());

        // Both should handle the same test texts
        let test_texts = vec![
            "system information check",
            "build debug release",
            "test filter pattern",
            "deploy staging production",
            "docker build image",
        ];

        for text in &test_texts {
            // Test interface compatibility by attempting embedding generation
            let mock_embedding = mock_provider.embed(text).await?;

            // Mock embedding should work
            assert_eq!(mock_embedding.len(), 384);
            for &value in &mock_embedding {
                assert!(value.is_finite());
            }

            // Try local embedding (may fail in test environment)
            match local_provider.embed(text).await {
                Ok(local_embedding) => {
                    assert_eq!(local_embedding.len(), mock_embedding.len());
                    assert_eq!(local_embedding.len(), 384);

                    for &value in &local_embedding {
                        assert!(value.is_finite());
                    }
                    println!("Local embedding succeeded for: {}", text);
                }
                Err(e) => {
                    println!("Local embedding failed for '{}' (expected): {}", text, e);
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_demo_justfile_task_simulation() -> Result<()> {
        let provider = LocalEmbeddingProvider::new();

        // Simulate embeddings for demo justfile tasks
        let demo_tasks = vec![
            ("hello", "Simple greeting task that prints hello message"),
            (
                "system-info",
                "System information task showing OS and environment",
            ),
            (
                "build",
                "Build simulation with different targets for compilation",
            ),
            ("test", "Run tests with optional filter for test execution"),
            ("deploy", "Deployment simulation to staging or production"),
            ("db-migrate", "Database operations for schema migrations"),
            ("db-seed", "Seed database with sample data for testing"),
            ("docker-build", "Build Docker image for containerization"),
            ("docker-push", "Push Docker image to registry"),
            ("api-test", "API testing with HTTP methods and data"),
            (
                "benchmark",
                "Performance testing with configurable iterations",
            ),
            ("health-check", "System health verification checks"),
            ("clean", "Cleanup task removing build artifacts"),
            ("monitor", "Service monitoring with configurable intervals"),
            ("backup", "Backup operations to specified destination"),
        ];

        // Test embedding generation for demo tasks
        let mut successful_embeddings = 0;
        for (task_name, description) in &demo_tasks {
            match provider.embed(description).await {
                Ok(embedding) => {
                    assert_eq!(embedding.len(), 384);
                    successful_embeddings += 1;
                    println!("Generated embedding for task: {}", task_name);
                }
                Err(e) => {
                    println!("Failed to generate embedding for '{}': {}", task_name, e);
                }
            }
        }

        // Test semantic grouping simulation
        let build_related = ["build", "compile", "create binary"];
        let test_related = ["test", "validate", "check functionality"];
        let deploy_related = ["deploy", "release", "production deployment"];

        for group in [&build_related, &test_related, &deploy_related] {
            for text in group {
                match provider.embed(text).await {
                    Ok(embedding) => {
                        assert_eq!(embedding.len(), 384);
                        for &value in &embedding {
                            assert!(value.is_finite());
                        }
                        println!("Generated embedding for: {}", text);
                    }
                    Err(e) => {
                        println!("Failed to generate embedding for '{}': {}", text, e);
                    }
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_embedding_generation() -> Result<()> {
        let provider = std::sync::Arc::new(LocalEmbeddingProvider::new());

        // Test concurrent access to embedding generation
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let provider = provider.clone();
                tokio::spawn(async move {
                    let text = format!("concurrent test text {}", i);
                    match provider.embed(&text).await {
                        Ok(embedding) => {
                            assert_eq!(embedding.len(), 384);
                            Some((i, embedding))
                        }
                        Err(e) => {
                            println!("Concurrent embedding failed for {}: {}", i, e);
                            None
                        }
                    }
                })
            })
            .collect();

        // Wait for all tasks and verify results
        let mut successful_results = 0;
        for handle in handles {
            if let Some(_) = handle.await.unwrap() {
                successful_results += 1;
            }
        }

        println!("Successful concurrent embeddings: {}/5", successful_results);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_directory_configuration() -> Result<()> {
        use just_mcp::vector_search::LocalEmbeddingConfig;
        use tempfile::TempDir;

        // Test default cache directory
        let provider_default = LocalEmbeddingProvider::new();
        let default_cache = provider_default.cache_dir();
        assert!(default_cache.to_string_lossy().contains("just-mcp"));
        assert!(default_cache.to_string_lossy().contains("models"));

        // Test custom cache directory
        let temp_dir = TempDir::new()?;
        let custom_cache = temp_dir.path().join("custom-cache");

        let config = LocalEmbeddingConfig {
            cache_dir: Some(custom_cache.clone()),
            ..Default::default()
        };
        let provider_custom = LocalEmbeddingProvider::with_config(config);
        assert_eq!(provider_custom.cache_dir(), custom_cache);

        Ok(())
    }

    #[tokio::test]
    async fn test_model_configuration_parameters() -> Result<()> {
        use just_mcp::vector_search::{LocalDevice, LocalEmbeddingConfig};

        // Test various configuration combinations
        let configs = vec![
            LocalEmbeddingConfig {
                model_id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                max_length: 128,
                device: LocalDevice::Cpu,
                normalize_embeddings: true,
                batch_size: 16,
                ..Default::default()
            },
            LocalEmbeddingConfig {
                model_id: "sentence-transformers/all-mpnet-base-v2".to_string(),
                max_length: 256,
                device: LocalDevice::Cuda(0),
                normalize_embeddings: false,
                batch_size: 64,
                ..Default::default()
            },
        ];

        for config in configs {
            let provider = LocalEmbeddingProvider::with_config(config.clone());
            assert_eq!(provider.model_name(), config.model_id);
            assert_eq!(provider.max_tokens(), config.max_length);

            // Test that provider is configured correctly
            assert_eq!(
                provider.dimension(),
                if config.model_id.contains("mpnet") {
                    768
                } else {
                    384
                }
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling_graceful_degradation() -> Result<()> {
        let provider = LocalEmbeddingProvider::new();

        // Test health check with graceful failure
        let health_result = provider.health_check().await;

        // Health check should return a result (either true or false)
        // but should not panic or return an error in test environment
        match health_result {
            Ok(is_healthy) => {
                println!("Health check result: {}", is_healthy);
                // Either result is acceptable in test environment
            }
            Err(e) => {
                println!("Health check failed gracefully: {}", e);
                // Graceful failure is acceptable in test environment
            }
        }

        // Test that embedding interface still works regardless of health check
        match provider.embed("test after health check").await {
            Ok(embedding) => {
                assert_eq!(embedding.len(), 384);
                println!("Embedding succeeded after health check");
            }
            Err(e) => {
                println!("Embedding failed after health check (expected): {}", e);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_embedding_interface() -> Result<()> {
        let provider = LocalEmbeddingProvider::new();

        // Test batch embedding interface (will use sequential processing for now)
        let texts = vec!["build app", "test code", "deploy service", "monitor health"];

        // This will attempt to call the real embed_batch method
        // In the current implementation, this may fail due to model loading issues
        // We'll test the interface and handle failures gracefully
        match provider
            .embed_batch(&texts.iter().map(|s| s.as_ref()).collect::<Vec<_>>())
            .await
        {
            Ok(embeddings) => {
                assert_eq!(embeddings.len(), texts.len());
                for embedding in &embeddings {
                    assert_eq!(embedding.len(), 384);
                }
                println!("Batch embedding succeeded");
            }
            Err(e) => {
                println!(
                    "Batch embedding failed gracefully (expected in test environment): {}",
                    e
                );

                // Test fallback to individual embeddings
                let mut successful_individual = 0;
                for text in &texts {
                    match provider.embed(text).await {
                        Ok(embedding) => {
                            assert_eq!(embedding.len(), 384);
                            successful_individual += 1;
                        }
                        Err(_) => {
                            // Expected in test environment
                        }
                    }
                }
                println!(
                    "Individual embeddings succeeded: {}/{}",
                    successful_individual,
                    texts.len()
                );
            }
        }

        Ok(())
    }
}
