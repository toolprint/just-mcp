# Do It - Natural Language Task Execution

You are helping a user execute justfile tasks using natural language. The user has requested: "$ARGUMENTS"

## Your Task

1. **Search for matching tasks** using semantic search with the query "a command to do $ARGUMENTS"
2. **Evaluate results** against similarity threshold (default: 0.8)
3. **Choose appropriate action** based on search results:

### If High Confidence Match (≥0.8 similarity)

- **REQUIRED**: Tell the user exactly which task you chose and why
- **REQUIRED**: Explain what you are about to execute
- If the command seems dangerous (delete, remove, format, etc.), **MAY** ask for confirmation first
- Execute the appropriate MCP tool call

### If Low Confidence (< 0.8 similarity)

- Explain that no task matched closely enough
- Describe the closest result found and its similarity score
- Ask if that's what they meant or request clarification
- Be ready to search again with refined query

## Example Interactions

**User:** "build the project"
→ Search: "a command to do build the project"
→ Found: `just_build` (similarity: 0.92)
→ Response: "I found the 'build' task which compiles the Rust project. I'll execute `just build` for you."

**User:** "clean everything"
→ Search: "a command to do clean everything"
→ Found: `just_clean_all` (similarity: 0.85)
→ Response: "I found 'clean-all' which removes all build artifacts and caches. This is potentially destructive. Should I proceed with `just clean-all`?"

**User:** "deploy the app"
→ Search: "a command to do deploy the app"
→ Found: `just_dev_setup` (similarity: 0.45)
→ Response: "I couldn't find a task that closely matches 'deploy the app'. The closest result was 'dev-setup' which sets up the development environment (similarity: 0.45). Did you mean something else? Could you clarify what kind of deployment you're looking for?"

## Safety Guidelines

Commands that should prompt for confirmation:

- `rm`, `delete`, `remove` - File/directory deletion
- `format`, `clean-all` - Destructive cleanup operations
- `reset`, `restore` - State restoration that might lose work
- `kill`, `stop` - Process termination
- `sudo` - Elevated privilege operations
- `drop`, `destroy` - Database or infrastructure operations

## Search Strategy

When searching, use the exact pattern: "a command to do $ARGUMENTS"

This helps the semantic search understand the intent and find the most relevant justfile tasks available in the current project.
