//! Search Adapter for Vector Search Integration
//!
//! This module provides a bridge between the prompt system and the vector search
//! capabilities. It handles query formatting, result processing, and similarity
//! threshold filtering for semantic task discovery.

use crate::error::Result;
use crate::prompts::traits::PromptConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Search result from vector search with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The content that matched the search query
    pub content: String,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
    /// Metadata about the result
    pub metadata: SearchMetadata,
}

/// Metadata associated with a search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// Source of the content (e.g., justfile path)
    pub source: String,
    /// Task name if this result represents a justfile task
    pub task_name: Option<String>,
    /// Task description if available
    pub task_description: Option<String>,
    /// Additional metadata fields
    pub extra: std::collections::HashMap<String, String>,
}

/// Search request parameters
#[derive(Debug, Clone)]
pub struct SearchRequest {
    /// The search query
    pub query: String,
    /// Maximum number of results to return
    pub limit: usize,
    /// Minimum similarity threshold
    pub threshold: f32,
}

/// Processed search response with filtered and ranked results
#[derive(Debug, Clone)]
pub struct SearchResponse {
    /// Search results that meet the threshold
    pub results: Vec<SearchResult>,
    /// All search results regardless of threshold (for finding closest matches)
    pub all_results: Vec<SearchResult>,
    /// The highest similarity score found
    pub max_similarity: f32,
    /// Total number of results before filtering
    pub total_results: usize,
    /// Whether any results meet the confidence threshold
    pub has_confident_match: bool,
}

/// Trait for search providers that can be used by prompts
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Perform a semantic search
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse>;

    /// Check if the search provider is available
    async fn is_available(&self) -> bool;

    /// Get provider information
    fn provider_info(&self) -> SearchProviderInfo;
}

/// Information about a search provider
#[derive(Debug, Clone)]
pub struct SearchProviderInfo {
    /// Provider name
    pub name: String,
    /// Whether the provider is currently available
    pub available: bool,
    /// Provider version or model information
    pub version: Option<String>,
    /// Additional capabilities
    pub capabilities: Vec<String>,
}

/// Adapter that bridges prompts with vector search system
pub struct SearchAdapter {
    /// The underlying search provider
    provider: Option<Arc<dyn SearchProvider>>,
    /// Configuration for search behavior
    config: PromptConfig,
}

impl SearchAdapter {
    /// Create a new search adapter without a provider (for testing or when vector search is disabled)
    pub fn new(config: PromptConfig) -> Self {
        Self {
            provider: None,
            config,
        }
    }

    /// Create a search adapter with a specific provider
    pub fn with_provider(provider: Arc<dyn SearchProvider>, config: PromptConfig) -> Self {
        Self {
            provider: Some(provider),
            config,
        }
    }

    /// Set the search provider
    pub fn set_provider(&mut self, provider: Arc<dyn SearchProvider>) {
        self.provider = Some(provider);
    }

    /// Check if vector search is available
    pub async fn is_available(&self) -> bool {
        match &self.provider {
            Some(provider) => provider.is_available().await,
            None => false,
        }
    }

    /// Search for tasks matching the given query
    pub async fn search_tasks(&self, user_request: &str) -> Result<SearchResponse> {
        let query = self.format_query(user_request);

        let request = SearchRequest {
            query,
            limit: self.config.max_search_results,
            threshold: self.config.similarity_threshold,
        };

        match &self.provider {
            Some(provider) => {
                let mut response = provider.search(request).await?;
                response.has_confident_match =
                    response.max_similarity >= self.config.similarity_threshold;
                Ok(response)
            }
            None => {
                // Return empty results if no provider is available
                Ok(SearchResponse {
                    results: vec![],
                    all_results: vec![],
                    max_similarity: 0.0,
                    total_results: 0,
                    has_confident_match: false,
                })
            }
        }
    }

    /// Get the best matching task from search results
    pub fn get_best_match<'a>(&self, response: &'a SearchResponse) -> Option<&'a SearchResult> {
        response.results.first() // Results should be sorted by similarity
    }

    /// Get the closest match even if below threshold (for suggestions)
    pub fn get_closest_match<'a>(&self, response: &'a SearchResponse) -> Option<&'a SearchResult> {
        response.all_results.first()
    }

    /// Check if a search result meets the confidence threshold
    pub fn meets_threshold(&self, result: &SearchResult) -> bool {
        result.similarity >= self.config.similarity_threshold
    }

    /// Format user request into a search query
    fn format_query(&self, user_request: &str) -> String {
        // Format the query to match the expected pattern for semantic search
        format!("a command to do {}", user_request.trim())
    }

    /// Get provider information if available
    pub async fn get_provider_info(&self) -> Option<SearchProviderInfo> {
        self.provider
            .as_ref()
            .map(|provider| provider.provider_info())
    }
}

/// Vector search provider that integrates with the existing VectorSearchManager
#[cfg(feature = "vector-search")]
pub struct VectorSearchProvider<
    E: crate::vector_search::EmbeddingProvider,
    V: crate::vector_search::VectorStore,
> {
    manager: Arc<crate::vector_search::VectorSearchManager<E, V>>,
}

#[cfg(feature = "vector-search")]
impl<E: crate::vector_search::EmbeddingProvider, V: crate::vector_search::VectorStore>
    VectorSearchProvider<E, V>
{
    /// Create a new vector search provider
    pub fn new(manager: Arc<crate::vector_search::VectorSearchManager<E, V>>) -> Self {
        Self { manager }
    }
}

/// Type alias for the most common vector search provider configuration
#[cfg(all(feature = "vector-search", feature = "local-embeddings"))]
pub type LocalVectorSearchProvider = VectorSearchProvider<
    crate::vector_search::LocalEmbeddingProvider,
    crate::vector_search::LibSqlVectorStore,
>;

#[cfg(feature = "vector-search")]
#[async_trait]
impl<E: crate::vector_search::EmbeddingProvider, V: crate::vector_search::VectorStore>
    SearchProvider for VectorSearchProvider<E, V>
{
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        // Use the existing VectorSearchManager to perform the search
        let search_results = self
            .manager
            .search_with_threshold(&request.query, request.limit, 0.0) // Use 0.0 to get all results
            .await
            .map_err(|e| crate::error::Error::Other(format!("Vector search failed: {}", e)))?;

        let mut results = Vec::new();
        let mut all_results = Vec::new();
        let mut max_similarity: f32 = 0.0;
        let total_results = search_results.len();

        for result in search_results {
            let similarity = result.score;
            max_similarity = max_similarity.max(similarity);

            let metadata = SearchMetadata {
                source: result
                    .document
                    .metadata
                    .get("source")
                    .cloned()
                    .unwrap_or_default(),
                task_name: result.document.metadata.get("task_name").cloned(),
                task_description: result.document.metadata.get("description").cloned(),
                extra: result.document.metadata.clone(),
            };

            let search_result = SearchResult {
                content: result.document.content,
                similarity,
                metadata,
            };

            // Add to all_results
            all_results.push(search_result.clone());

            // Only include results that meet the threshold in filtered results
            if similarity >= request.threshold {
                results.push(search_result);
            }
        }

        // Sort both vectors by similarity (highest first)
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(SearchResponse {
            results,
            all_results,
            max_similarity,
            total_results,
            has_confident_match: max_similarity >= request.threshold,
        })
    }

    async fn is_available(&self) -> bool {
        // Check if the vector search manager is properly initialized
        // This is a simplified check - you might want to add a proper health check method
        true
    }

    fn provider_info(&self) -> SearchProviderInfo {
        SearchProviderInfo {
            name: "VectorSearchManager".to_string(),
            available: true,
            version: Some(crate::vector_search::VERSION.to_string()),
            capabilities: vec![
                "semantic_search".to_string(),
                "similarity_scoring".to_string(),
                "justfile_indexing".to_string(),
            ],
        }
    }
}

/// Mock search provider for testing
pub struct MockSearchProvider {
    responses: std::collections::HashMap<String, Vec<SearchResult>>,
}

impl MockSearchProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self {
            responses: std::collections::HashMap::new(),
        }
    }

    /// Add a mock response for a specific query
    pub fn add_response(&mut self, query: &str, results: Vec<SearchResult>) {
        self.responses.insert(query.to_string(), results);
    }

    /// Create a mock search result
    pub fn create_result(content: &str, similarity: f32, task_name: Option<&str>) -> SearchResult {
        SearchResult {
            content: content.to_string(),
            similarity,
            metadata: SearchMetadata {
                source: "mock".to_string(),
                task_name: task_name.map(|s| s.to_string()),
                task_description: None,
                extra: std::collections::HashMap::new(),
            },
        }
    }
}

impl Default for MockSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchProvider for MockSearchProvider {
    async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let all_results = self
            .responses
            .get(&request.query)
            .cloned()
            .unwrap_or_default();

        let max_similarity = all_results.iter().map(|r| r.similarity).fold(0.0, f32::max);

        let filtered_results = all_results
            .iter()
            .filter(|r| r.similarity >= request.threshold)
            .take(request.limit)
            .cloned()
            .collect::<Vec<_>>();

        // Sort all_results by similarity (highest first)
        let mut sorted_all_results = all_results;
        sorted_all_results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(SearchResponse {
            results: filtered_results,
            all_results: sorted_all_results,
            max_similarity,
            total_results: self
                .responses
                .get(&request.query)
                .map(|r| r.len())
                .unwrap_or(0),
            has_confident_match: max_similarity >= request.threshold,
        })
    }

    async fn is_available(&self) -> bool {
        true
    }

    fn provider_info(&self) -> SearchProviderInfo {
        SearchProviderInfo {
            name: "MockSearchProvider".to_string(),
            available: true,
            version: Some("1.0.0".to_string()),
            capabilities: vec!["testing".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_adapter_without_provider() {
        let config = PromptConfig::default();
        let adapter = SearchAdapter::new(config);

        assert!(!adapter.is_available().await);

        let response = adapter.search_tasks("build the project").await.unwrap();
        assert_eq!(response.results.len(), 0);
        assert_eq!(response.all_results.len(), 0);
        assert!(!response.has_confident_match);
    }

    #[tokio::test]
    async fn test_mock_search_provider() {
        let mut mock = MockSearchProvider::new();
        mock.add_response(
            "a command to do build the project",
            vec![MockSearchProvider::create_result(
                "build task",
                0.9,
                Some("just_build"),
            )],
        );

        let config = PromptConfig::default();
        let adapter = SearchAdapter::with_provider(Arc::new(mock), config);

        assert!(adapter.is_available().await);

        let response = adapter.search_tasks("build the project").await.unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.all_results.len(), 1);
        assert!(response.has_confident_match);
        assert_eq!(response.max_similarity, 0.9);

        let best_match = adapter.get_best_match(&response).unwrap();
        assert_eq!(best_match.content, "build task");
        assert_eq!(
            best_match.metadata.task_name,
            Some("just_build".to_string())
        );
    }

    #[tokio::test]
    async fn test_search_adapter_threshold_filtering() {
        let mut mock = MockSearchProvider::new();
        mock.add_response(
            "a command to do test the code",
            vec![
                MockSearchProvider::create_result("test task", 0.9, Some("just_test")),
                MockSearchProvider::create_result("lint task", 0.5, Some("just_lint")),
            ],
        );

        let config = PromptConfig::default().with_similarity_threshold(0.8);
        let adapter = SearchAdapter::with_provider(Arc::new(mock), config);

        let response = adapter.search_tasks("test the code").await.unwrap();

        // Only the high-confidence result should be returned in results
        assert_eq!(response.results.len(), 1);
        // But all results should be in all_results
        assert_eq!(response.all_results.len(), 2);
        assert!(response.has_confident_match);
        assert_eq!(response.results[0].content, "test task");
    }

    #[test]
    fn test_query_formatting() {
        let config = PromptConfig::default();
        let adapter = SearchAdapter::new(config);

        let query = adapter.format_query("build the project");
        assert_eq!(query, "a command to do build the project");

        let query = adapter.format_query("  test everything  ");
        assert_eq!(query, "a command to do test everything");
    }

    #[test]
    fn test_search_result_construction() {
        let result = SearchResult {
            content: "test content".to_string(),
            similarity: 0.85,
            metadata: SearchMetadata {
                source: "test.just".to_string(),
                task_name: Some("test_task".to_string()),
                task_description: Some("Test description".to_string()),
                extra: std::collections::HashMap::new(),
            },
        };

        assert_eq!(result.content, "test content");
        assert_eq!(result.similarity, 0.85);
        assert_eq!(result.metadata.task_name, Some("test_task".to_string()));
    }

    #[test]
    fn test_search_request_creation() {
        let request = SearchRequest {
            query: "test query".to_string(),
            limit: 10,
            threshold: 0.8,
        };

        assert_eq!(request.query, "test query");
        assert_eq!(request.limit, 10);
        assert_eq!(request.threshold, 0.8);
    }
}
