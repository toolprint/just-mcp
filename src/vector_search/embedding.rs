//! Embedding provider implementations for vector search
//!
//! This module provides various embedding providers that can generate
//! vector embeddings for text content.

use crate::vector_search::error::VectorSearchError;
use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "vector-search")]
use serde::{Deserialize, Serialize};

/// OpenAI API request structure for embedding generation
#[cfg(feature = "vector-search")]
#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    /// The model to use for embedding generation
    model: String,
    /// The input text(s) to generate embeddings for
    input: OpenAIInput,
    /// The format to return embeddings in (always "float")
    encoding_format: String,
}

/// OpenAI input can be a single string or array of strings
#[cfg(feature = "vector-search")]
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OpenAIInput {
    Single(String),
    Batch(Vec<String>),
}

/// OpenAI API response structure for embedding generation
#[cfg(feature = "vector-search")]
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    /// Array of embedding objects
    data: Vec<OpenAIEmbeddingData>,
    /// Model used for generation
    model: String,
    /// Usage statistics
    usage: OpenAIUsage,
}

/// Individual embedding data
#[cfg(feature = "vector-search")]
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    /// The embedding vector
    embedding: Vec<f32>,
    /// Index in the input array
    index: usize,
}

/// Usage statistics from OpenAI API
#[cfg(feature = "vector-search")]
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    /// Number of tokens in the prompt
    prompt_tokens: u32,
    /// Total tokens used
    total_tokens: u32,
}

/// Trait for embedding providers that can generate vector embeddings
///
/// This trait provides a standardized interface for different embedding
/// services and models, allowing the application to switch between
/// providers (OpenAI, local models, etc.) without changing the core logic.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text input
    ///
    /// # Arguments
    /// * `text` - The text content to generate an embedding for
    ///
    /// # Returns
    /// A vector of floating-point values representing the text embedding
    ///
    /// # Errors
    /// Returns an error if the embedding generation fails (network issues,
    /// API limits, invalid input, etc.)
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple text inputs (batch operation)
    ///
    /// This method is more efficient than calling `embed` multiple times
    /// as it can utilize batch processing capabilities of the underlying
    /// embedding service.
    ///
    /// # Arguments
    /// * `texts` - Array of text strings to generate embeddings for
    ///
    /// # Returns
    /// A vector of embedding vectors, one for each input text
    ///
    /// # Errors
    /// Returns an error if any embedding generation fails
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get the dimension of embeddings produced by this provider
    ///
    /// This is used for validation and database schema setup.
    /// Different embedding models produce vectors of different dimensions.
    ///
    /// # Returns
    /// The number of dimensions in the embedding vectors
    fn dimension(&self) -> usize;

    /// Get the maximum number of tokens this provider can handle in a single request
    ///
    /// This helps with text chunking and batch size optimization.
    fn max_tokens(&self) -> usize {
        8192 // Default reasonable limit
    }

    /// Get the model name or identifier used by this provider
    ///
    /// This is useful for debugging, logging, and ensuring consistency
    /// when storing/retrieving embeddings.
    fn model_name(&self) -> &str;

    /// Check if the provider is healthy and ready to generate embeddings
    ///
    /// This can verify API connectivity, authentication, etc.
    async fn health_check(&self) -> Result<bool>;
}

/// Mock embedding provider for testing
///
/// This provider generates deterministic embeddings based on the input text,
/// making it perfect for unit tests and development environments where
/// consistent, reproducible results are needed.
#[cfg(feature = "vector-search")]
pub struct MockEmbeddingProvider {
    /// The dimension of embeddings this provider generates
    dimension: usize,

    /// Model name identifier
    model_name: String,
}

/// OpenAI embedding provider for production use
///
/// This provider uses OpenAI's embedding API to generate high-quality
/// vector embeddings for text content. It supports various OpenAI
/// embedding models like text-embedding-ada-002.
#[cfg(feature = "vector-search")]
pub struct OpenAIEmbeddingProvider {
    /// OpenAI API key for authentication
    api_key: String,

    /// OpenAI embedding model to use
    model: String,

    /// HTTP client for API requests
    client: reqwest::Client,

    /// Base URL for OpenAI API
    base_url: String,

    /// Request timeout duration
    timeout: std::time::Duration,
}

/// Hybrid embedding provider with fallback capabilities
///
/// This provider attempts to use a primary embedding provider first,
/// and falls back to a secondary provider if the primary fails.
/// This is useful for ensuring reliability and avoiding service disruptions.
#[cfg(feature = "vector-search")]
pub struct HybridEmbeddingProvider {
    /// Primary embedding provider (tried first)
    primary: Box<dyn EmbeddingProvider>,

    /// Secondary embedding provider (fallback)
    secondary: Box<dyn EmbeddingProvider>,

    /// Maximum number of retry attempts for the primary provider
    max_retries: usize,

    /// Delay between retry attempts
    retry_delay: std::time::Duration,

    /// Whether to use fallback automatically or fail fast
    auto_fallback: bool,
}

#[cfg(feature = "vector-search")]
impl MockEmbeddingProvider {
    /// Create a new mock embedding provider with default dimension
    pub fn new() -> Self {
        Self::new_with_dimension(384) // Common dimension for smaller models
    }

    /// Create a new mock embedding provider with specified dimension
    ///
    /// # Arguments
    /// * `dimension` - The dimension of embeddings to generate
    pub fn new_with_dimension(dimension: usize) -> Self {
        Self {
            dimension,
            model_name: "mock-embedding-model".to_string(),
        }
    }

    /// Create embeddings that simulate OpenAI's text-embedding-ada-002
    pub fn new_openai_compatible() -> Self {
        Self {
            dimension: 1536,
            model_name: "mock-text-embedding-ada-002".to_string(),
        }
    }

    /// Generate a deterministic embedding for the given text
    ///
    /// This uses a simple but deterministic algorithm that:
    /// 1. Converts text to bytes
    /// 2. Uses a simple hash function to distribute values
    /// 3. Normalizes the result to unit length
    fn generate_deterministic_embedding(&self, text: &str) -> Vec<f32> {
        let bytes = text.as_bytes();
        let mut embedding = Vec::with_capacity(self.dimension);

        // Use a simple but deterministic method to generate embeddings
        for i in 0..self.dimension {
            let mut value = 0.0f32;

            // Mix bytes with dimension index for variety
            for (j, &byte) in bytes.iter().enumerate() {
                let factor = ((i + j + 1) as f32).sin();
                value += (byte as f32) * factor * 0.01;
            }

            // Add some dimension-specific variation
            value += (i as f32 * 0.1).cos();

            embedding.push(value);
        }

        // Normalize to unit length for better similarity calculations
        let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for value in &mut embedding {
                *value /= magnitude;
            }
        }

        embedding
    }
}

#[cfg(feature = "vector-search")]
#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Simulate some processing time
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        Ok(self.generate_deterministic_embedding(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Simulate batch processing time
        tokio::time::sleep(std::time::Duration::from_millis(texts.len() as u64)).await;

        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.generate_deterministic_embedding(text));
        }

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn max_tokens(&self) -> usize {
        8192 // Simulate reasonable token limit
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn health_check(&self) -> Result<bool> {
        // Mock provider is always healthy
        Ok(true)
    }
}

#[cfg(feature = "vector-search")]
impl OpenAIEmbeddingProvider {
    /// Create a new OpenAI embedding provider
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key for authentication
    ///
    /// # Returns
    /// A new OpenAI embedding provider instance
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, "text-embedding-ada-002".to_string())
    }

    /// Create a new OpenAI embedding provider with a specific model
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key for authentication
    /// * `model` - OpenAI embedding model to use
    ///
    /// # Returns
    /// A new OpenAI embedding provider instance
    pub fn with_model(api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            client,
            base_url: "https://api.openai.com/v1".to_string(),
            timeout: std::time::Duration::from_secs(30),
        }
    }

    /// Create a new OpenAI embedding provider with custom configuration
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key for authentication
    /// * `model` - OpenAI embedding model to use
    /// * `base_url` - Custom base URL for OpenAI API (useful for proxies)
    /// * `timeout` - Request timeout duration
    ///
    /// # Returns
    /// A new OpenAI embedding provider instance
    pub fn with_config(
        api_key: String,
        model: String,
        base_url: String,
        timeout: std::time::Duration,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            client,
            base_url,
            timeout,
        }
    }

    /// Make a request to the OpenAI embedding API
    ///
    /// # Arguments
    /// * `input` - The input text(s) to generate embeddings for
    ///
    /// # Returns
    /// The API response containing embeddings
    ///
    /// # Errors
    /// Returns an error if the API request fails
    async fn make_embedding_request(&self, input: OpenAIInput) -> Result<OpenAIEmbeddingResponse> {
        let request = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input,
            encoding_format: "float".to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "OpenAI API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let embedding_response: OpenAIEmbeddingResponse = response.json().await?;
        Ok(embedding_response)
    }

    /// Get the dimension for the configured model
    ///
    /// This returns the known dimensions for common OpenAI models.
    /// For unknown models, it defaults to 1536 (ada-002 dimension).
    fn get_model_dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-ada-002" => 1536,
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            _ => 1536, // Default to ada-002 dimension
        }
    }

    /// Get the maximum tokens for the configured model
    ///
    /// This returns the known token limits for common OpenAI models.
    fn get_model_max_tokens(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-ada-002" => 8191,
            "text-embedding-3-small" => 8191,
            "text-embedding-3-large" => 8191,
            _ => 8191, // Default to common limit
        }
    }
}

#[cfg(feature = "vector-search")]
#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let input = OpenAIInput::Single(text.to_string());
        let response = self.make_embedding_request(input).await?;

        if response.data.is_empty() {
            return Err(anyhow::anyhow!("No embeddings returned from OpenAI API"));
        }

        Ok(response.data[0].embedding.clone())
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let input = OpenAIInput::Batch(texts.iter().map(|&s| s.to_string()).collect());
        let response = self.make_embedding_request(input).await?;

        if response.data.len() != texts.len() {
            return Err(anyhow::anyhow!(
                "Expected {} embeddings, got {}",
                texts.len(),
                response.data.len()
            ));
        }

        // Sort by index to ensure correct order
        let mut sorted_data = response.data;
        sorted_data.sort_by_key(|d| d.index);

        Ok(sorted_data.into_iter().map(|d| d.embedding).collect())
    }

    fn dimension(&self) -> usize {
        self.get_model_dimension()
    }

    fn max_tokens(&self) -> usize {
        self.get_model_max_tokens()
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> Result<bool> {
        // Test with a simple embedding request
        match self.embed("test").await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("OpenAI embedding provider health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(feature = "vector-search")]
impl HybridEmbeddingProvider {
    /// Create a new hybrid embedding provider
    ///
    /// # Arguments
    /// * `primary` - Primary embedding provider to try first
    /// * `secondary` - Secondary embedding provider to fallback to
    ///
    /// # Returns
    /// A new hybrid embedding provider instance
    pub fn new(primary: Box<dyn EmbeddingProvider>, secondary: Box<dyn EmbeddingProvider>) -> Self {
        Self {
            primary,
            secondary,
            max_retries: 2,
            retry_delay: std::time::Duration::from_millis(500),
            auto_fallback: true,
        }
    }

    /// Create a new hybrid embedding provider with custom configuration
    ///
    /// # Arguments
    /// * `primary` - Primary embedding provider to try first
    /// * `secondary` - Secondary embedding provider to fallback to
    /// * `max_retries` - Maximum retry attempts for primary provider
    /// * `retry_delay` - Delay between retry attempts
    /// * `auto_fallback` - Whether to automatically fallback on primary failure
    ///
    /// # Returns
    /// A new hybrid embedding provider instance
    pub fn with_config(
        primary: Box<dyn EmbeddingProvider>,
        secondary: Box<dyn EmbeddingProvider>,
        max_retries: usize,
        retry_delay: std::time::Duration,
        auto_fallback: bool,
    ) -> Self {
        Self {
            primary,
            secondary,
            max_retries,
            retry_delay,
            auto_fallback,
        }
    }

    /// Attempt to use primary provider with retries
    ///
    /// # Arguments
    /// * `operation` - Async closure that performs the embedding operation
    ///
    /// # Returns
    /// Result from the primary provider or an error if all retries failed
    async fn try_primary_with_retries<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.max_retries {
                        tracing::warn!(
                            "Primary embedding provider failed (attempt {}/{}), retrying in {:?}: {}",
                            attempt + 1,
                            self.max_retries + 1,
                            self.retry_delay,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(self.retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Primary provider failed without error")))
    }

    /// Attempt embedding with primary, fallback to secondary if needed
    ///
    /// # Arguments
    /// * `primary_op` - Operation to perform with primary provider
    /// * `secondary_op` - Operation to perform with secondary provider
    ///
    /// # Returns
    /// Result from primary or secondary provider
    async fn try_with_fallback<T, F1, F2, Fut1, Fut2>(
        &self,
        primary_op: F1,
        secondary_op: F2,
    ) -> Result<T>
    where
        F1: Fn() -> Fut1,
        F2: Fn() -> Fut2,
        Fut1: std::future::Future<Output = Result<T>>,
        Fut2: std::future::Future<Output = Result<T>>,
    {
        // Try primary provider with retries
        match self.try_primary_with_retries(primary_op).await {
            Ok(result) => Ok(result),
            Err(primary_error) => {
                if !self.auto_fallback {
                    return Err(primary_error);
                }

                tracing::warn!(
                    "Primary embedding provider failed after {} retries, falling back to secondary: {}", 
                    self.max_retries,
                    primary_error
                );

                // Try secondary provider
                match secondary_op().await {
                    Ok(result) => {
                        tracing::info!("Successfully fell back to secondary embedding provider");
                        Ok(result)
                    }
                    Err(secondary_error) => {
                        Err(anyhow::anyhow!(
                            "Both primary and secondary embedding providers failed. Primary: {}. Secondary: {}",
                            primary_error,
                            secondary_error
                        ))
                    }
                }
            }
        }
    }
}

#[cfg(feature = "vector-search")]
#[async_trait]
impl EmbeddingProvider for HybridEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.try_with_fallback(|| self.primary.embed(text), || self.secondary.embed(text))
            .await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.try_with_fallback(
            || self.primary.embed_batch(texts),
            || self.secondary.embed_batch(texts),
        )
        .await
    }

    fn dimension(&self) -> usize {
        // Use primary provider's dimension
        self.primary.dimension()
    }

    fn max_tokens(&self) -> usize {
        // Use the minimum of both providers to ensure compatibility
        std::cmp::min(self.primary.max_tokens(), self.secondary.max_tokens())
    }

    fn model_name(&self) -> &str {
        // Return primary provider's model name (could be enhanced to show both)
        self.primary.model_name()
    }

    async fn health_check(&self) -> Result<bool> {
        // Check primary provider first
        let primary_healthy = self.primary.health_check().await.unwrap_or(false);

        if primary_healthy {
            return Ok(true);
        }

        // If primary is not healthy, check secondary
        if self.auto_fallback {
            let secondary_healthy = self.secondary.health_check().await.unwrap_or(false);
            if secondary_healthy {
                tracing::warn!(
                    "Primary embedding provider is unhealthy, but secondary is available"
                );
                return Ok(true);
            }
        }

        Ok(false)
    }
}
