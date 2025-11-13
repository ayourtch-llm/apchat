use crate::{param, core::tool::{Tool, ToolParameters, ToolResult, ParameterDefinition}};
use crate::core::tool_context::ToolContext;
use crate::tools::helpers::build_glob_pattern;
use crate::open_file;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use colored::Colorize;
use rustyline::DefaultEditor;

/// Tool for opening files with optional line range
pub struct OpenFileTool;

#[async_trait]
impl Tool for OpenFileTool {
    fn name(&self) -> &str {
        "open_file"
    }

    fn description(&self) -> &str {
        "Open a file and display its contents with optional line range"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file relative to the work directory", required),
            param!("start_line", "integer", "Starting line number (1-based)", optional),
            param!("end_line", "integer", "Ending line number (1-based)", optional),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let start_line = params.get_optional::<i32>("start_line").unwrap_or(None);
        let end_line = params.get_optional::<i32>("end_line").unwrap_or(None);

        let line_range = if let (Some(start), Some(end)) = (start_line, end_line) {
            if start > 0 && end > 0 {
                Some((start as usize)..=(end as usize))
            } else {
                None
            }
        } else {
            None
        };

        match open_file::open_file(&context.work_dir, &file_path, line_range).await {
            Ok(content) => ToolResult::success(content),
            Err(e) => ToolResult::error(format!("Failed to open file: {}", e)),
        }
    }
}

/// Tool for reading file previews (first 10 lines)
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read a preview of a file (first 10 lines) with total line count. For reading specific line ranges, use open_file instead."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file relative to the work directory", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Use the same logic as the original read_file method
        let full_path = context.work_dir.join(&file_path);
        if !full_path.exists() {
            // Check for directory with similar name
            if let Some(stem) = full_path.file_stem().and_then(|s| s.to_str()) {
                let parent = full_path.parent().unwrap_or(&context.work_dir);
                let possible_dir = parent.join(stem);

                if possible_dir.exists() && possible_dir.is_dir() {
                    return ToolResult::error(format!(
                        "File not found: {} (Note: Found a directory named '{}' at this location. Did you mean to list files in that directory instead?)",
                        file_path, stem
                    ));
                }
            }

            return ToolResult::error(format!("File not found: {}", file_path));
        }

        // Check if it's a directory
        if full_path.is_dir() {
            return ToolResult::error(format!(
                "Path '{}' is a directory, not a file. Use list_files to see its contents.",
                file_path
            ));
        }

        match fs::read_to_string(&full_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total_lines = lines.len();

                let preview = if total_lines <= 10 {
                    content
                } else {
                    let preview_lines = lines.iter().take(10).cloned().collect::<Vec<_>>().join("\n");
                    format!("{}\n[{} more lines]", preview_lines, total_lines - 10)
                };

                ToolResult::success(preview)
            }
            Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
        }
    }
}

/// Tool for writing content to files
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file in the work directory"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file relative to the work directory", required),
            param!("content", "string", "Content to write to the file", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let content = match params.get_required::<String>("content") {
            Ok(content) => content,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let full_path = context.work_dir.join(&file_path);

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::error(format!("Failed to create directories: {}", e));
            }
        }

        match fs::write(&full_path, content) {
            Ok(_) => ToolResult::success(format!("Successfully wrote to file: {}", file_path)),
            Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
        }
    }
}

/// Tool for editing files by replacing content
pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old content with new content"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("file_path", "string", "Path to the file relative to the work directory", required),
            param!("old_content", "string", "Old content to find and replace (must not be empty)", required),
            param!("new_content", "string", "New content to replace with", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let file_path = match params.get_required::<String>("file_path") {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let old_content = match params.get_required::<String>("old_content") {
            Ok(content) => content,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let new_content = match params.get_required::<String>("new_content") {
            Ok(content) => content,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        if old_content.trim().is_empty() {
            return ToolResult::error("old_content must not be empty".to_string());
        }

        let full_path = context.work_dir.join(&file_path);

        if !full_path.exists() {
            return ToolResult::error(format!("File not found: {}", file_path));
        }

        // Read current content
        let current_content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => return ToolResult::error(format!("Failed to read file: {}", e)),
        };

        // Check if old content exists
        if !current_content.contains(&old_content) {
            return ToolResult::error(format!("Old content not found in file: {}", file_path));
        }

        // Calculate replacement
        let new_content_full = current_content.replace(&old_content, &new_content);
        let occurrences = current_content.matches(&old_content).count();

        // Show diff and ask for confirmation
        println!("{}", "═".repeat(60).bright_blue());
        println!("{} {}", "📝 Editing:".bright_cyan().bold(), file_path.bright_white());
        println!("{}", "═".repeat(60).bright_black());

        // Simple diff display
        println!("{}", "─ Old content:".red());
        for line in old_content.lines() {
            println!("{} {}", "-".red(), line);
        }
        println!();
        println!("{}", "+ New content:".green());
        for line in new_content.lines() {
            println!("{} {}", "+".green(), line);
        }
        println!("{}", "═".repeat(60).bright_black());

        if occurrences > 1 {
            println!("{}", format!("⚠️  Warning: {} occurrences will be replaced", occurrences).yellow());
        }

        // Check permission using policy system
        let approved = match context.check_permission(
            crate::policy::ActionType::FileEdit,
            &file_path,
            "Apply these changes? [Y/n]"
        ) {
            Ok(approved) => approved,
            Err(e) => return ToolResult::error(format!("Permission check failed: {}", e)),
        };

        if approved {
            // Write back to file
            match fs::write(&full_path, new_content_full) {
                Ok(_) => ToolResult::success(format!("✅ Successfully edited {} ({} replacement(s))", file_path, occurrences)),
                Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
            }
        } else {
            ToolResult::error("Edit cancelled by user or policy".to_string())
        }
    }
}

/// Tool for listing files with glob patterns
pub struct ListFilesTool;

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files matching a glob pattern. Automatically excludes build/cache directories (target/, .git/, node_modules/, etc.). Limited to 1000 results. Supports recursive search with **."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("pattern", "string", "Glob pattern (e.g., 'src/**/*.rs', '**/*.json'). Use ** for recursive search. Defaults to '*' (files in current directory). Automatically excludes build/cache directories and limits to 1000 results.", optional, "*"),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        let pattern = params.get_optional::<String>("pattern")
            .unwrap_or(Some("*".to_string()))
            .unwrap_or_else(|| "*".to_string());

        let glob_pattern = build_glob_pattern(&pattern, &context.work_dir);

        eprintln!("[DEBUG] list_files with pattern: '{}' in work_dir: {:?}", glob_pattern, context.work_dir);

        // Directories to exclude (common build/cache directories)
        const EXCLUDED_DIRS: &[&str] = &[
            "target",
            ".git",
            "node_modules",
            ".cache",
            "dist",
            "build",
            ".next",
            ".nuxt",
            "coverage",
            "__pycache__",
            ".pytest_cache",
            ".venv",
            "venv",
        ];

        const MAX_FILES: usize = 1000;

        match glob::glob(&glob_pattern) {
            Ok(paths) => {
                let mut files = Vec::new();
                let mut total_matched = 0;
                let mut excluded_count = 0;

                for path in paths {
                    match path {
                        Ok(path) => {
                            if let Some(relative_path) = path.strip_prefix(&context.work_dir).ok() {
                                // Check if path is in an excluded directory
                                let path_components: Vec<_> = relative_path.components().collect();
                                let should_exclude = path_components.iter().any(|comp| {
                                    if let std::path::Component::Normal(name) = comp {
                                        if let Some(name_str) = name.to_str() {
                                            return EXCLUDED_DIRS.contains(&name_str);
                                        }
                                    }
                                    false
                                });

                                if should_exclude {
                                    excluded_count += 1;
                                    continue;
                                }

                                total_matched += 1;
                                if files.len() < MAX_FILES {
                                    if let Some(path_str) = relative_path.to_str() {
                                        files.push(path_str.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            return ToolResult::error(format!("Error reading path: {}", e));
                        }
                    }
                }

                files.sort();
                let result = if files.is_empty() && total_matched == 0 {
                    format!(
                        "No files found matching pattern: '{}'\nSearched in: {:?}\nExcluded {} files in build/cache directories\nTip: Use ** for recursive search (e.g., 'src/**/*.rs')",
                        pattern, context.work_dir, excluded_count
                    )
                } else if total_matched > MAX_FILES {
                    format!(
                        "⚠️  Found {} matching file(s), but showing only first {} (excluded {} files in build/cache directories):\n{}\n\n\
                        Tip: Use a more specific pattern to reduce results (e.g., 'src/**/*.rs' instead of '**/*')",
                        total_matched,
                        MAX_FILES,
                        excluded_count,
                        files.join("\n")
                    )
                } else {
                    let exclusion_note = if excluded_count > 0 {
                        format!(" (excluded {} files in build/cache directories)", excluded_count)
                    } else {
                        String::new()
                    };
                    format!(
                        "Found {} file(s) matching '{}'{}:\n{}",
                        files.len(),
                        pattern,
                        exclusion_note,
                        files.join("\n")
                    )
                };

                ToolResult::success(result)
            }
            Err(e) => ToolResult::error(format!("Invalid glob pattern: {}", e)),
        }
    }
}