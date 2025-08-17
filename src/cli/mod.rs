//! Command-line interface for just-mcp vector search functionality
//!
//! This module provides CLI commands for interacting with the vector search
//! functionality outside of the MCP server mode.

use clap::{Parser, Subcommand};

#[cfg(feature = "vector-search")]
use anyhow::Result;
#[cfg(feature = "vector-search")]
use std::path::PathBuf;

#[cfg(feature = "vector-search")]
use just_mcp::vector_search::{
    EmbeddingProvider, LibSqlVectorStore, MockEmbeddingProvider, OpenAIEmbeddingProvider,
    VectorSearchManager,
};

#[cfg(feature = "vector-search")]
use just_mcp::vector_search::Document;

#[cfg(all(feature = "vector-search", feature = "local-embeddings"))]
use just_mcp::vector_search::LocalEmbeddingProvider;

/// CLI arguments for just-mcp
#[derive(Parser, Debug)]
#[command(name = "just-mcp")]
#[command(version = just_mcp::VERSION)]
#[command(about = "Model Context Protocol server for justfile integration", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(
        short = 'w',
        long = "watch-dir",
        help = "Directory to watch for justfiles, optionally with name (path or path:name). Defaults to current directory if not specified"
    )]
    pub watch_dir: Vec<String>,

    #[arg(long, help = "Enable administrative tools")]
    pub admin: bool,

    #[arg(long, help = "Enable JSON output for logs")]
    pub json_logs: bool,

    #[arg(
        long,
        default_value = "info",
        help = "Log level (trace, debug, info, warn, error)"
    )]
    pub log_level: String,
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the MCP server (default mode)
    Serve,

    #[cfg(feature = "vector-search")]
    /// Vector search operations
    Search {
        #[command(subcommand)]
        search_command: SearchCommands,
    },
}

/// Vector search subcommands
#[cfg(feature = "vector-search")]
#[derive(Subcommand, Debug)]
pub enum SearchCommands {
    /// Perform semantic search on indexed documents
    Query {
        /// Search query text
        #[arg(short, long)]
        query: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Minimum similarity threshold (0.0 to 1.0)
        #[arg(short, long, default_value = "0.0")]
        threshold: f32,

        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,

        /// OpenAI API key for embeddings (if not using mock provider)
        #[arg(long, env = "OPENAI_API_KEY")]
        openai_api_key: Option<String>,

        /// Use mock embedding provider for testing
        #[arg(long)]
        mock_embeddings: bool,

        /// Use local embedding model (all-MiniLM-L6-v2) for offline text embeddings. Downloads model on first use. Alternative to OpenAI API for privacy and cost savings.
        #[cfg(feature = "local-embeddings")]
        #[arg(
            long,
            help = "Use local sentence transformer model for embeddings (offline, privacy-focused)"
        )]
        local_embeddings: bool,
    },

    /// Index justfiles from a directory into the vector database
    Index {
        /// Directory containing justfiles to index
        #[arg(short, long, default_value = ".")]
        directory: PathBuf,

        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,

        /// OpenAI API key for embeddings (if not using mock provider)
        #[arg(long, env = "OPENAI_API_KEY")]
        openai_api_key: Option<String>,

        /// Use mock embedding provider for testing
        #[arg(long)]
        mock_embeddings: bool,

        /// Use local embedding model (all-MiniLM-L6-v2) for offline indexing. Downloads model on first use. Slower than OpenAI but private and cost-free.
        #[cfg(feature = "local-embeddings")]
        #[arg(
            long,
            help = "Use local sentence transformer model for indexing (offline, no API costs)"
        )]
        local_embeddings: bool,

        /// Batch size for indexing operations
        #[arg(long, default_value = "50")]
        batch_size: usize,
    },

    /// Show database statistics
    Stats {
        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,
    },

    /// Find similar tasks to a given task
    Similar {
        /// Task content to find similar tasks for
        #[arg(short, long)]
        task: String,

        /// Maximum number of similar tasks to return
        #[arg(short, long, default_value = "5")]
        limit: usize,

        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,

        /// OpenAI API key for embeddings (if not using mock provider)
        #[arg(long, env = "OPENAI_API_KEY")]
        openai_api_key: Option<String>,

        /// Use mock embedding provider for testing
        #[arg(long)]
        mock_embeddings: bool,

        /// Use local embedding model (all-MiniLM-L6-v2) for finding similar tasks. Works offline without API calls.
        #[cfg(feature = "local-embeddings")]
        #[arg(
            long,
            help = "Use local sentence transformer model for similarity search (offline)"
        )]
        local_embeddings: bool,
    },

    /// Search by metadata filters
    Filter {
        /// Metadata filters in key=value format
        #[arg(short, long, value_parser = parse_key_val)]
        filter: Vec<(String, String)>,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,
    },

    /// Search by text content
    Text {
        /// Text pattern to search for in document content
        #[arg(short, long)]
        text: String,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Database path for vector storage
        #[arg(short = 'b', long, default_value = "vector_search.db")]
        database: PathBuf,
    },
}

/// Parse a single key-value pair for metadata filters
#[cfg(feature = "vector-search")]
fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

/// Execute query with fallback logic: local embeddings -> mock embeddings -> error
#[cfg(feature = "vector-search")]
async fn query_with_fallback(
    query: &str,
    limit: usize,
    threshold: f32,
    database: &PathBuf,
    prefer_local: bool,
    prefer_mock: bool,
    openai_api_key: Option<String>,
) -> Result<()> {
    // If user explicitly requests a specific provider, use it directly
    if prefer_mock {
        let manager = create_search_manager_mock(database).await?;
        return query_search(manager, query, limit, threshold).await;
    }

    if let Some(api_key) = openai_api_key {
        let manager = create_search_manager_openai(database, api_key).await?;
        return query_search(manager, query, limit, threshold).await;
    }

    #[cfg(feature = "local-embeddings")]
    if prefer_local {
        let manager = create_search_manager_local(database).await?;
        return query_search(manager, query, limit, threshold).await;
    }

    // Fallback logic: try local -> mock -> error
    #[cfg(feature = "local-embeddings")]
    {
        match create_search_manager_local(database).await {
            Ok(manager) => {
                println!("Using local embeddings for vector search");
                return query_search(manager, query, limit, threshold).await;
            }
            Err(e) => {
                eprintln!("Failed to initialize local embeddings: {}", e);
                eprintln!("Falling back to mock embeddings...");
            }
        }
    }

    // Fall back to mock embeddings
    match create_search_manager_mock(database).await {
        Ok(manager) => {
            println!("Using mock embeddings for vector search");
            query_search(manager, query, limit, threshold).await
        }
        Err(e) => {
            #[cfg(feature = "local-embeddings")]
            return Err(anyhow::anyhow!("All embedding providers failed. Local embeddings error: see above. Mock embeddings error: {}", e));
            #[cfg(not(feature = "local-embeddings"))]
            return Err(anyhow::anyhow!("Mock embeddings failed: {}. Please provide --openai-api-key for OpenAI embeddings.", e));
        }
    }
}

/// Execute indexing with fallback logic: local embeddings -> mock embeddings -> error
#[cfg(feature = "vector-search")]
async fn index_with_fallback(
    directory: &PathBuf,
    database: &PathBuf,
    batch_size: usize,
    prefer_local: bool,
    prefer_mock: bool,
    openai_api_key: Option<String>,
) -> Result<()> {
    // If user explicitly requests a specific provider, use it directly
    if prefer_mock {
        let manager = create_search_manager_mock(database).await?;
        return index_documents(&manager, directory, batch_size).await;
    }

    if let Some(api_key) = openai_api_key {
        let manager = create_search_manager_openai(database, api_key).await?;
        return index_documents(&manager, directory, batch_size).await;
    }

    #[cfg(feature = "local-embeddings")]
    if prefer_local {
        let manager = create_search_manager_local(database).await?;
        return index_documents(&manager, directory, batch_size).await;
    }

    // Fallback logic: try local -> mock -> error
    #[cfg(feature = "local-embeddings")]
    {
        match create_search_manager_local(database).await {
            Ok(manager) => {
                println!("Using local embeddings for indexing");
                return index_documents(&manager, directory, batch_size).await;
            }
            Err(e) => {
                eprintln!("Failed to initialize local embeddings: {}", e);
                eprintln!("Falling back to mock embeddings...");
            }
        }
    }

    // Fall back to mock embeddings
    match create_search_manager_mock(database).await {
        Ok(manager) => {
            println!("Using mock embeddings for indexing");
            index_documents(&manager, directory, batch_size).await
        }
        Err(e) => {
            #[cfg(feature = "local-embeddings")]
            return Err(anyhow::anyhow!("All embedding providers failed. Local embeddings error: see above. Mock embeddings error: {}", e));
            #[cfg(not(feature = "local-embeddings"))]
            return Err(anyhow::anyhow!("Mock embeddings failed: {}. Please provide --openai-api-key for OpenAI embeddings.", e));
        }
    }
}

/// Execute similar task search with fallback logic: local embeddings -> mock embeddings -> error
#[cfg(feature = "vector-search")]
async fn similar_with_fallback(
    task: &str,
    limit: usize,
    database: &PathBuf,
    prefer_local: bool,
    prefer_mock: bool,
    openai_api_key: Option<String>,
) -> Result<()> {
    // If user explicitly requests a specific provider, use it directly
    if prefer_mock {
        let manager = create_search_manager_mock(database).await?;
        return similar_tasks(&manager, task, limit).await;
    }

    if let Some(api_key) = openai_api_key {
        let manager = create_search_manager_openai(database, api_key).await?;
        return similar_tasks(&manager, task, limit).await;
    }

    #[cfg(feature = "local-embeddings")]
    if prefer_local {
        let manager = create_search_manager_local(database).await?;
        return similar_tasks(&manager, task, limit).await;
    }

    // Fallback logic: try local -> mock -> error
    #[cfg(feature = "local-embeddings")]
    {
        match create_search_manager_local(database).await {
            Ok(manager) => {
                println!("Using local embeddings for similar task search");
                return similar_tasks(&manager, task, limit).await;
            }
            Err(e) => {
                eprintln!("Failed to initialize local embeddings: {}", e);
                eprintln!("Falling back to mock embeddings...");
            }
        }
    }

    // Fall back to mock embeddings
    match create_search_manager_mock(database).await {
        Ok(manager) => {
            println!("Using mock embeddings for similar task search");
            similar_tasks(&manager, task, limit).await
        }
        Err(e) => {
            #[cfg(feature = "local-embeddings")]
            return Err(anyhow::anyhow!("All embedding providers failed. Local embeddings error: see above. Mock embeddings error: {}", e));
            #[cfg(not(feature = "local-embeddings"))]
            return Err(anyhow::anyhow!("Mock embeddings failed: {}. Please provide --openai-api-key for OpenAI embeddings.", e));
        }
    }
}

/// Generic similar tasks function that works with any embedding provider
#[cfg(feature = "vector-search")]
async fn similar_tasks<E: EmbeddingProvider>(
    manager: &VectorSearchManager<E, LibSqlVectorStore>,
    task: &str,
    limit: usize,
) -> Result<()> {
    let results = manager.find_similar_tasks(task, limit).await?;

    if results.is_empty() {
        println!("No similar tasks found for: '{}'", task);
    } else {
        println!("Found {} similar tasks:", results.len());
        println!();

        for (i, result) in results.iter().enumerate() {
            println!("{}. Similarity: {:.4}", i + 1, result.score);
            if let Some(ref task_name) = result.document.task_name {
                println!("   Task: {}", task_name);
            }
            if let Some(ref justfile) = result.document.justfile_name {
                println!("   Justfile: {}", justfile);
            }
            println!("   Content: {}", result.document.content);
            println!();
        }
    }

    Ok(())
}

/// Generic indexing function that works with any embedding provider
#[cfg(feature = "vector-search")]
async fn index_documents<E: EmbeddingProvider>(
    manager: &VectorSearchManager<E, LibSqlVectorStore>,
    directory: &PathBuf,
    batch_size: usize,
) -> Result<()> {
    // Find all justfiles in the directory
    let justfiles = find_justfiles(directory)?;

    if justfiles.is_empty() {
        println!("No justfiles found in directory: {}", directory.display());
        return Ok(());
    }

    println!("Found {} justfiles to index", justfiles.len());

    // Parse justfiles and create documents
    let mut documents = Vec::new();

    for justfile_path in justfiles {
        let tasks = parse_justfile(&justfile_path)?;
        for task in tasks {
            documents.push(task);
        }
    }

    if documents.is_empty() {
        println!("No tasks found in justfiles");
        return Ok(());
    }

    println!("Found {} tasks to index", documents.len());

    // Index in chunks
    let chunk_count = (documents.len() + batch_size - 1) / batch_size;
    println!(
        "Indexing in {} batches of up to {} documents each",
        chunk_count, batch_size
    );

    let mut total_indexed = 0;
    for (chunk_idx, chunk) in documents.chunks(batch_size).enumerate() {
        println!("Processing batch {}/{}", chunk_idx + 1, chunk_count);
        let chunk_docs = chunk.to_vec();
        let ids = manager
            .index_documents_chunked(chunk_docs, batch_size, "tasks")
            .await?;
        total_indexed += ids.len();
        println!("Indexed {} documents (total: {})", ids.len(), total_indexed);
    }

    println!("Successfully indexed {} tasks", total_indexed);
    Ok(())
}

/// Handle vector search CLI commands
#[cfg(feature = "vector-search")]
pub async fn handle_search_command(search_command: SearchCommands) -> Result<()> {
    match search_command {
        SearchCommands::Query {
            query,
            limit,
            threshold,
            database,
            openai_api_key,
            mock_embeddings,
            #[cfg(feature = "local-embeddings")]
            local_embeddings,
        } => {
            #[cfg(feature = "local-embeddings")]
            {
                query_with_fallback(
                    &query,
                    limit,
                    threshold,
                    &database,
                    local_embeddings,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }

            #[cfg(not(feature = "local-embeddings"))]
            {
                query_with_fallback(
                    &query,
                    limit,
                    threshold,
                    &database,
                    false,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }
        }

        SearchCommands::Index {
            directory,
            database,
            openai_api_key,
            mock_embeddings,
            #[cfg(feature = "local-embeddings")]
            local_embeddings,
            batch_size,
        } => {
            println!("Indexing justfiles from: {}", directory.display());

            #[cfg(feature = "local-embeddings")]
            {
                index_with_fallback(
                    &directory,
                    &database,
                    batch_size,
                    local_embeddings,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }

            #[cfg(not(feature = "local-embeddings"))]
            {
                index_with_fallback(
                    &directory,
                    &database,
                    batch_size,
                    false,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }
        }

        SearchCommands::Stats { database } => {
            let manager = create_search_manager_mock(&database).await?;

            let count = manager.get_document_count().await?;
            let healthy = manager.health_check().await?;

            println!("Vector Database Statistics");
            println!("==========================");
            println!("Database path: {}", database.display());
            println!("Total documents: {}", count);
            println!(
                "Health status: {}",
                if healthy { "Healthy" } else { "Unhealthy" }
            );

            // Additional stats could be added here
        }

        SearchCommands::Similar {
            task,
            limit,
            database,
            openai_api_key,
            mock_embeddings,
            #[cfg(feature = "local-embeddings")]
            local_embeddings,
        } => {
            #[cfg(feature = "local-embeddings")]
            {
                similar_with_fallback(
                    &task,
                    limit,
                    &database,
                    local_embeddings,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }

            #[cfg(not(feature = "local-embeddings"))]
            {
                similar_with_fallback(
                    &task,
                    limit,
                    &database,
                    false,
                    mock_embeddings,
                    openai_api_key,
                )
                .await?;
            }
        }

        SearchCommands::Filter {
            filter,
            limit,
            database,
        } => {
            let manager = create_search_manager_mock(&database).await?;

            let filter_refs: Vec<(&str, &str)> = filter
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let results = manager.search_by_metadata(&filter_refs, limit).await?;

            if results.is_empty() {
                println!("No documents found matching filters");
            } else {
                println!("Found {} documents matching filters:", results.len());
                println!();

                for (i, doc) in results.iter().enumerate() {
                    println!("{}. ID: {}", i + 1, doc.id);
                    if let Some(ref task_name) = doc.task_name {
                        println!("   Task: {}", task_name);
                    }
                    if let Some(ref justfile) = doc.justfile_name {
                        println!("   Justfile: {}", justfile);
                    }
                    println!(
                        "   Content: {}",
                        doc.content.chars().take(100).collect::<String>()
                    );
                    if doc.content.len() > 100 {
                        println!("   ...");
                    }
                    println!();
                }
            }
        }

        SearchCommands::Text {
            text,
            limit,
            database,
        } => {
            let manager = create_search_manager_mock(&database).await?;

            let results = manager.search_by_content(&text, limit).await?;

            if results.is_empty() {
                println!("No documents found containing text: '{}'", text);
            } else {
                println!(
                    "Found {} documents containing text '{}':",
                    results.len(),
                    text
                );
                println!();

                for (i, doc) in results.iter().enumerate() {
                    println!("{}. ID: {}", i + 1, doc.id);
                    if let Some(ref task_name) = doc.task_name {
                        println!("   Task: {}", task_name);
                    }
                    if let Some(ref justfile) = doc.justfile_name {
                        println!("   Justfile: {}", justfile);
                    }

                    // Highlight the search text in the content
                    let highlighted = doc.content.replace(&text, &format!("**{}**", text));
                    println!(
                        "   Content: {}",
                        highlighted.chars().take(200).collect::<String>()
                    );
                    if highlighted.len() > 200 {
                        println!("   ...");
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Create a vector search manager with mock embedding provider
#[cfg(feature = "vector-search")]
async fn create_search_manager_mock(
    database_path: &PathBuf,
) -> Result<VectorSearchManager<MockEmbeddingProvider, LibSqlVectorStore>> {
    // Create mock embedding provider
    let embedding_provider = MockEmbeddingProvider::new_openai_compatible();

    // Create vector store
    let dimension = embedding_provider.dimension();
    let vector_store =
        LibSqlVectorStore::new(database_path.to_string_lossy().to_string(), dimension);

    // Create and initialize manager
    let mut manager = VectorSearchManager::new(embedding_provider, vector_store);
    manager.initialize().await?;

    Ok(manager)
}

/// Create a vector search manager with OpenAI embedding provider
#[cfg(feature = "vector-search")]
async fn create_search_manager_openai(
    database_path: &PathBuf,
    api_key: String,
) -> Result<VectorSearchManager<OpenAIEmbeddingProvider, LibSqlVectorStore>> {
    // Create OpenAI embedding provider
    let embedding_provider = OpenAIEmbeddingProvider::new(api_key);

    // Create vector store
    let dimension = embedding_provider.dimension();
    let vector_store =
        LibSqlVectorStore::new(database_path.to_string_lossy().to_string(), dimension);

    // Create and initialize manager
    let mut manager = VectorSearchManager::new(embedding_provider, vector_store);
    manager.initialize().await?;

    Ok(manager)
}

/// Create a vector search manager with local embedding provider
#[cfg(all(feature = "vector-search", feature = "local-embeddings"))]
async fn create_search_manager_local(
    database_path: &PathBuf,
) -> Result<VectorSearchManager<LocalEmbeddingProvider, LibSqlVectorStore>> {
    // Create local embedding provider with default config
    let embedding_provider = LocalEmbeddingProvider::new();

    // Create vector store
    let dimension = embedding_provider.dimension();
    let vector_store =
        LibSqlVectorStore::new(database_path.to_string_lossy().to_string(), dimension);

    // Create and initialize manager
    let mut manager = VectorSearchManager::new(embedding_provider, vector_store);
    manager.initialize().await?;

    Ok(manager)
}

/// Helper function for query search operations
#[cfg(feature = "vector-search")]
async fn query_search<E: just_mcp::vector_search::EmbeddingProvider>(
    manager: VectorSearchManager<E, LibSqlVectorStore>,
    query: &str,
    limit: usize,
    threshold: f32,
) -> Result<()> {
    let results = manager
        .search_with_threshold(query, limit, threshold)
        .await?;

    if results.is_empty() {
        println!("No results found for query: '{}'", query);
    } else {
        println!("Found {} results for query: '{}'", results.len(), query);
        println!();

        for (i, result) in results.iter().enumerate() {
            println!("{}. Score: {:.4}", i + 1, result.score);
            println!("   ID: {}", result.document.id);
            if let Some(ref task_name) = result.document.task_name {
                println!("   Task: {}", task_name);
            }
            if let Some(ref justfile) = result.document.justfile_name {
                println!("   Justfile: {}", justfile);
            }
            println!(
                "   Content: {}",
                result
                    .document
                    .content
                    .chars()
                    .take(100)
                    .collect::<String>()
            );
            if result.document.content.len() > 100 {
                println!("   ...");
            }
            println!();
        }
    }

    Ok(())
}

/// Find all justfiles in a directory recursively
#[cfg(feature = "vector-search")]
fn find_justfiles(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut justfiles = Vec::new();

    if !dir.exists() {
        return Err(anyhow::anyhow!(
            "Directory does not exist: {}",
            dir.display()
        ));
    }

    if !dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Path is not a directory: {}",
            dir.display()
        ));
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                if filename == "justfile" || filename == "Justfile" {
                    justfiles.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(justfiles)
}

/// Parse a justfile and extract tasks as documents
#[cfg(feature = "vector-search")]
fn parse_justfile(justfile_path: &PathBuf) -> Result<Vec<Document>> {
    use just_mcp::parser::JustfileParser;
    use uuid::Uuid;

    let content = std::fs::read_to_string(justfile_path)?;
    let parser = JustfileParser::new()?;
    let tasks = parser.parse_content(&content)?;

    let mut documents = Vec::new();
    let justfile_name = justfile_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());
    let source_path = justfile_path.to_string_lossy().to_string();

    for task in tasks {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("type".to_string(), "justfile_task".to_string());

        if let Some(ref justfile) = justfile_name {
            metadata.insert("justfile_name".to_string(), justfile.clone());
        }

        metadata.insert("source_path".to_string(), source_path.clone());
        metadata.insert("line_number".to_string(), task.line_number.to_string());

        // Create content from task details
        let mut content_parts = vec![task.name.clone()];
        if !task.comments.is_empty() {
            content_parts.extend(task.comments.clone());
        }
        if !task.parameters.is_empty() {
            content_parts.push(format!(
                "Parameters: {}",
                task.parameters
                    .iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !task.dependencies.is_empty() {
            content_parts.push(format!("Dependencies: {}", task.dependencies.join(", ")));
        }

        let document = Document {
            id: Uuid::new_v4().to_string(),
            content: content_parts.join(" - "),
            metadata,
            source_path: Some(source_path.clone()),
            justfile_name: justfile_name.clone(),
            task_name: Some(task.name.clone()),
        };

        documents.push(document);
    }

    Ok(documents)
}
