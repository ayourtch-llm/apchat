use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// RLM Context Chunking tool.
///
/// Based on the Recursive Language Models paper (arXiv:2512.24601).
/// Splits large inputs into manageable chunks stored on disk,
/// enabling agents to process inputs that exceed context window limits.
pub struct RlmContextChunkTool;

#[derive(Debug, Clone)]
struct ChunkManifest {
    input_path: String,
    strategy: String,
    chunk_size: usize,
    total_chunks: usize,
    chunk_paths: Vec<String>,
    input_bytes: u64,
    input_lines: usize,
}

impl ChunkManifest {
    fn to_display(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("[RLM CHUNK MANIFEST]\n"));
        output.push_str(&format!("Input: {} ({} bytes, {} lines)\n",
            self.input_path, self.input_bytes, self.input_lines));
        output.push_str(&format!("Strategy: {}, chunk_size: {}\n",
            self.strategy, self.chunk_size));
        output.push_str(&format!("Total chunks: {}\n\n", self.total_chunks));
        for (i, path) in self.chunk_paths.iter().enumerate() {
            output.push_str(&format!("  chunk_{:03}: {}\n", i + 1, path));
        }
        output.push_str(&format!("\nUse launch_subagent to process each chunk, writing results to files.\n"));
        output.push_str(&format!("Then aggregate results with run_command."));
        output
    }
}

fn chunk_by_lines(content: &str, chunk_size: usize, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunk_paths = Vec::new();

    for (i, chunk_lines) in lines.chunks(chunk_size).enumerate() {
        let chunk_path = output_dir.join(format!("chunk_{:03}.txt", i + 1));
        let chunk_content = chunk_lines.join("\n");
        fs::write(&chunk_path, &chunk_content)
            .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
        chunk_paths.push(chunk_path);
    }

    Ok(chunk_paths)
}

fn chunk_by_chars(content: &str, chunk_size: usize, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut chunk_paths = Vec::new();
    let chars: Vec<char> = content.chars().collect();

    for (i, chunk_chars) in chars.chunks(chunk_size).enumerate() {
        let chunk_path = output_dir.join(format!("chunk_{:03}.txt", i + 1));
        let chunk_content: String = chunk_chars.iter().collect();
        fs::write(&chunk_path, &chunk_content)
            .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
        chunk_paths.push(chunk_path);
    }

    Ok(chunk_paths)
}

fn chunk_by_separator(content: &str, chunk_size: usize, separator: &str, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let sections: Vec<&str> = content.split(separator).collect();
    let mut chunk_paths = Vec::new();
    let mut current_chunk = String::new();
    let mut chunk_index = 0;

    for section in sections {
        if !current_chunk.is_empty() && current_chunk.len() + separator.len() + section.len() > chunk_size {
            chunk_index += 1;
            let chunk_path = output_dir.join(format!("chunk_{:03}.txt", chunk_index));
            fs::write(&chunk_path, &current_chunk)
                .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
            chunk_paths.push(chunk_path);
            current_chunk = String::new();
        }
        if !current_chunk.is_empty() {
            current_chunk.push_str(separator);
        }
        current_chunk.push_str(section);
    }

    if !current_chunk.is_empty() {
        chunk_index += 1;
        let chunk_path = output_dir.join(format!("chunk_{:03}.txt", chunk_index));
        fs::write(&chunk_path, &current_chunk)
            .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
        chunk_paths.push(chunk_path);
    }

    Ok(chunk_paths)
}

fn chunk_by_files(dir_path: &Path, chunk_size: usize, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    collect_files_recursive(dir_path, &mut all_files)?;
    all_files.sort();

    let mut chunk_paths = Vec::new();
    let mut current_chunk = String::new();
    let mut chunk_index = 0;

    for file_path in &all_files {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary files
        };

        let file_header = format!("=== {} ===\n", file_path.display());
        let file_entry = format!("{}{}\n", file_header, content);

        if !current_chunk.is_empty() && current_chunk.len() + file_entry.len() > chunk_size {
            chunk_index += 1;
            let chunk_path = output_dir.join(format!("chunk_{:03}.txt", chunk_index));
            fs::write(&chunk_path, &current_chunk)
                .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
            chunk_paths.push(chunk_path);
            current_chunk = String::new();
        }
        current_chunk.push_str(&file_entry);
    }

    if !current_chunk.is_empty() {
        chunk_index += 1;
        let chunk_path = output_dir.join(format!("chunk_{:03}.txt", chunk_index));
        fs::write(&chunk_path, &current_chunk)
            .map_err(|e| format!("Failed to write chunk {}: {}", chunk_path.display(), e))?;
        chunk_paths.push(chunk_path);
    }

    Ok(chunk_paths)
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            // Skip hidden dirs and common non-source dirs
            if !dir_name.starts_with('.') && dir_name != "node_modules" && dir_name != "target" {
                collect_files_recursive(&path, files)?;
            }
        } else if path.is_file() {
            files.push(path);
        }
    }

    Ok(())
}


#[async_trait]
impl Tool for RlmContextChunkTool {
    fn name(&self) -> &str {
        "rlm_context_chunk"
    }

    fn description(&self) -> &str {
        "Split a large file or directory into manageable chunks for Recursive Language Model (RLM) processing. \
         Writes chunks to disk and returns a manifest of chunk file paths. Use this when input exceeds context \
         window limits. After chunking, use launch_subagent to process each chunk and run_command to aggregate results. \
         Load the 'recursive-context-processing' skill for the full RLM workflow."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("input_path", "string", "Path to the input file or directory to chunk", required),
            param!("chunk_strategy", "string", "Chunking strategy: 'lines' (split by line count), 'chars' (split by character count), 'separator' (split by delimiter), or 'files' (group directory files by total size). Default: 'lines'", optional),
            param!("chunk_size", "integer", "Size of each chunk. For 'lines': number of lines (default 500). For 'chars': number of characters (default 60000). For 'separator': max characters per chunk (default 60000). For 'files': max characters per chunk (default 60000).", optional),
            param!("output_dir", "string", "Directory to write chunk files. Default: /tmp/rlm_chunks_<timestamp>", optional),
            param!("separator", "string", "Delimiter for 'separator' strategy (e.g. '\\n---\\n' or '\\n## '). Required when chunk_strategy is 'separator'.", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let input_path_str = match params.get_required::<String>("input_path") {
            Ok(v) => v,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let strategy = params.get_optional::<String>("chunk_strategy")
            .unwrap_or(None)
            .unwrap_or_else(|| "lines".to_string());

        let separator = params.get_optional::<String>("separator")
            .unwrap_or(None);

        // Resolve input path relative to work_dir
        let input_path = if Path::new(&input_path_str).is_absolute() {
            PathBuf::from(&input_path_str)
        } else {
            context.work_dir.join(&input_path_str)
        };

        if !input_path.exists() {
            return ToolResult::error(format!("Input path does not exist: {}", input_path.display()));
        }

        // Validate strategy
        if !["lines", "chars", "separator", "files"].contains(&strategy.as_str()) {
            return ToolResult::error(
                "chunk_strategy must be one of: 'lines', 'chars', 'separator', 'files'".to_string()
            );
        }

        if strategy == "separator" && separator.is_none() {
            return ToolResult::error(
                "separator parameter is required when chunk_strategy is 'separator'".to_string()
            );
        }

        if strategy == "files" && !input_path.is_dir() {
            return ToolResult::error(
                "input_path must be a directory when chunk_strategy is 'files'".to_string()
            );
        }

        // Default chunk sizes by strategy
        let default_chunk_size: usize = match strategy.as_str() {
            "lines" => 500,
            _ => 60000,
        };

        let chunk_size = match params.get_optional::<i64>("chunk_size") {
            Ok(Some(v)) => {
                if v < 1 {
                    return ToolResult::error("chunk_size must be positive".to_string());
                }
                v as usize
            }
            Ok(None) => default_chunk_size,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Set up output directory
        let output_dir = match params.get_optional::<String>("output_dir") {
            Ok(Some(v)) => PathBuf::from(v),
            Ok(None) => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                PathBuf::from(format!("/tmp/rlm_chunks_{}", timestamp))
            }
            Err(e) => return ToolResult::error(e.to_string()),
        };

        if let Err(e) = fs::create_dir_all(&output_dir) {
            return ToolResult::error(format!("Failed to create output directory: {}", e));
        }

        // Execute chunking based on strategy
        let (chunk_paths, input_bytes, input_lines) = if strategy == "files" {
            // Directory mode
            let paths = match chunk_by_files(&input_path, chunk_size, &output_dir) {
                Ok(p) => p,
                Err(e) => return ToolResult::error(e),
            };
            let total_bytes: u64 = paths.iter()
                .filter_map(|p| fs::metadata(p).ok())
                .map(|m| m.len())
                .sum();
            (paths, total_bytes, 0)
        } else {
            // File mode - read content
            let content = match fs::read_to_string(&input_path) {
                Ok(c) => c,
                Err(e) => return ToolResult::error(format!("Failed to read input file: {}", e)),
            };

            let input_bytes = content.len() as u64;
            let input_lines = content.lines().count();

            let paths = match strategy.as_str() {
                "lines" => chunk_by_lines(&content, chunk_size, &output_dir),
                "chars" => chunk_by_chars(&content, chunk_size, &output_dir),
                "separator" => chunk_by_separator(
                    &content,
                    chunk_size,
                    separator.as_deref().unwrap_or("\n"),
                    &output_dir,
                ),
                _ => unreachable!(),
            };

            match paths {
                Ok(p) => (p, input_bytes, input_lines),
                Err(e) => return ToolResult::error(e),
            }
        };

        let manifest = ChunkManifest {
            input_path: input_path_str,
            strategy,
            chunk_size,
            total_chunks: chunk_paths.len(),
            chunk_paths: chunk_paths.iter().map(|p| p.display().to_string()).collect(),
            input_bytes,
            input_lines,
        };

        ToolResult::success(manifest.to_display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apchat_policy::PolicyManager;
    use std::path::PathBuf;

    fn test_context() -> ToolContext {
        ToolContext::new(
            PathBuf::from("/tmp"),
            "test_session".to_string(),
            PolicyManager::new(),
        )
    }

    fn create_test_file(content: &str) -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.into_path().join("test_input.txt");
        fs::write(&path, content).unwrap();
        path
    }

    fn create_test_dir() -> PathBuf {
        let dir = tempfile::tempdir().unwrap().into_path();
        fs::write(dir.join("file1.txt"), "Hello from file 1\nLine 2").unwrap();
        fs::write(dir.join("file2.txt"), "Hello from file 2\nLine 2\nLine 3").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/file3.txt"), "Nested file content").unwrap();
        dir
    }

    #[tokio::test]
    async fn test_chunk_by_lines() {
        let content = (1..=100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        let path = create_test_file(&content);
        let output_dir = tempfile::tempdir().unwrap();

        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "lines");
        params.set("chunk_size", 25i64);
        params.set("output_dir", output_dir.path().display().to_string());

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success, "Expected success, got error: {:?}", result.error);
        assert!(result.content.contains("Total chunks: 4"));
        assert!(result.content.contains("chunk_001"));
        assert!(result.content.contains("chunk_004"));

        // Verify chunk files exist and have content
        let chunk1 = fs::read_to_string(output_dir.path().join("chunk_001.txt")).unwrap();
        assert!(chunk1.contains("Line 1"));
        assert!(chunk1.contains("Line 25"));
        assert!(!chunk1.contains("Line 26"));
    }

    #[tokio::test]
    async fn test_chunk_by_chars() {
        let content = "A".repeat(1000);
        let path = create_test_file(&content);
        let output_dir = tempfile::tempdir().unwrap();

        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "chars");
        params.set("chunk_size", 300i64);
        params.set("output_dir", output_dir.path().display().to_string());

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success, "Expected success, got error: {:?}", result.error);
        assert!(result.content.contains("Total chunks: 4"));
    }

    #[tokio::test]
    async fn test_chunk_by_separator() {
        let content = "Section 1 content\n---\nSection 2 content\n---\nSection 3 content";
        let path = create_test_file(content);
        let output_dir = tempfile::tempdir().unwrap();

        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "separator");
        params.set("separator", "\n---\n");
        params.set("chunk_size", 100i64);
        params.set("output_dir", output_dir.path().display().to_string());

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success, "Expected success, got error: {:?}", result.error);
        assert!(result.content.contains("chunk_001"));
    }

    #[tokio::test]
    async fn test_chunk_by_files() {
        let dir = create_test_dir();
        let output_dir = tempfile::tempdir().unwrap();

        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", dir.display().to_string());
        params.set("chunk_strategy", "files");
        params.set("chunk_size", 100i64);
        params.set("output_dir", output_dir.path().display().to_string());

        let result = tool.execute(params, &test_context()).await;
        assert!(result.success, "Expected success, got error: {:?}", result.error);
        assert!(result.content.contains("[RLM CHUNK MANIFEST]"));
        assert!(result.content.contains("Strategy: files"));
    }

    #[tokio::test]
    async fn test_missing_input_path() {
        let tool = RlmContextChunkTool;
        let params = ToolParameters::new();

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_nonexistent_input() {
        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", "/tmp/nonexistent_rlm_test_file_xyz.txt");

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_invalid_strategy() {
        let path = create_test_file("test content");
        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "invalid");

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("chunk_strategy must be one of"));
    }

    #[tokio::test]
    async fn test_separator_required_for_separator_strategy() {
        let path = create_test_file("test content");
        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "separator");

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("separator parameter is required"));
    }

    #[tokio::test]
    async fn test_files_strategy_requires_directory() {
        let path = create_test_file("test content");
        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_strategy", "files");

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("must be a directory"));
    }

    #[tokio::test]
    async fn test_invalid_chunk_size() {
        let path = create_test_file("test content");
        let tool = RlmContextChunkTool;
        let mut params = ToolParameters::new();
        params.set("input_path", path.display().to_string());
        params.set("chunk_size", 0i64);

        let result = tool.execute(params, &test_context()).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("chunk_size must be positive"));
    }

    #[test]
    fn test_tool_metadata() {
        let tool = RlmContextChunkTool;
        assert_eq!(tool.name(), "rlm_context_chunk");
        assert!(tool.description().contains("Recursive Language Model"));
        let params = tool.parameters();
        assert!(params.contains_key("input_path"));
        assert!(params.contains_key("chunk_strategy"));
        assert!(params.contains_key("chunk_size"));
        assert!(params.contains_key("output_dir"));
        assert!(params.contains_key("separator"));
        assert!(params["input_path"].required);
        assert!(!params["chunk_strategy"].required);
    }

    #[test]
    fn test_openai_definition() {
        let tool = RlmContextChunkTool;
        let def = tool.to_openai_definition();
        assert_eq!(def["function"]["name"], "rlm_context_chunk");
        assert_eq!(def["type"], "function");
    }

    #[test]
    fn test_chunk_by_lines_function() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let output_dir = tempfile::tempdir().unwrap();
        let paths = chunk_by_lines(content, 2, output_dir.path()).unwrap();
        assert_eq!(paths.len(), 3);

        let chunk1 = fs::read_to_string(&paths[0]).unwrap();
        assert_eq!(chunk1, "line1\nline2");

        let chunk3 = fs::read_to_string(&paths[2]).unwrap();
        assert_eq!(chunk3, "line5");
    }

    #[test]
    fn test_chunk_by_chars_function() {
        let content = "abcdefghij";
        let output_dir = tempfile::tempdir().unwrap();
        let paths = chunk_by_chars(content, 3, output_dir.path()).unwrap();
        assert_eq!(paths.len(), 4);

        let chunk1 = fs::read_to_string(&paths[0]).unwrap();
        assert_eq!(chunk1, "abc");

        let chunk4 = fs::read_to_string(&paths[3]).unwrap();
        assert_eq!(chunk4, "j");
    }
}
