use apchat_toolcore::{param, Tool, ToolParameters, ToolResult, ParameterDefinition};
use apchat_toolcore::tool_context::ToolContext;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use rustyline::DefaultEditor;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditOperation {
    file_path: String,
    old_content: String,
    new_content: String,
    description: String,
}

/// Tool for switching between AI models
pub struct SwitchModelTool {
    // This will need to be connected to the main application state
    // For now, it's a placeholder that returns information about the switch
}

impl SwitchModelTool {
    pub fn new() -> Self {
        Self { }
    }
}

#[async_trait]
impl Tool for SwitchModelTool {
    fn name(&self) -> &str {
        "switch_model"
    }

    fn description(&self) -> &str {
        "Switch to a different AI model for better task handling"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("model", "string", "Model to switch to (kimi or gpt_oss)", required),
            param!("reason", "string", "Reason for switching models", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, _context: &ToolContext) -> ToolResult {
        let model = match params.get_required::<String>("model") {
            Ok(model) => model,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let reason = match params.get_required::<String>("reason") {
            Ok(reason) => reason,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        // Validate model name
        if !["kimi", "gpt_oss", "blu_model", "grn_model", "anthropic"].contains(&model.as_str()) {
            return ToolResult::error("Invalid model. Available models: kimi, gpt_oss, blu_model, grn_model, anthropic".to_string());
        }

        // For now, return information about the requested switch
        // In the full implementation, this would trigger an actual model switch
        let message = format!(
            "Model switch requested:\n  Target model: {}\n  Reason: {}\n  Note: Actual model switching will be implemented in the main application layer",
            model, reason
        );

        ToolResult::success(message)
    }
}

// Helper function to get the edit plan file path
fn get_plan_file_path(work_dir: &PathBuf) -> PathBuf {
    work_dir.join(".apchat_edit_plan.json")
}

// Helper function to save edit plan
fn save_edit_plan(work_dir: &PathBuf, edits: &[EditOperation]) -> Result<(), String> {
    let plan_path = get_plan_file_path(work_dir);
    let json = serde_json::to_string_pretty(edits)
        .map_err(|e| format!("Failed to serialize edit plan: {}", e))?;
    fs::write(&plan_path, json)
        .map_err(|e| format!("Failed to write edit plan: {}", e))?;
    Ok(())
}

// Helper function to load edit plan
fn load_edit_plan(work_dir: &PathBuf) -> Result<Vec<EditOperation>, String> {
    let plan_path = get_plan_file_path(work_dir);
    if !plan_path.exists() {
        return Err("No edit plan exists. Create one first using plan_edits.".to_string());
    }
    let json = fs::read_to_string(&plan_path)
        .map_err(|e| format!("Failed to read edit plan: {}", e))?;
    let edits: Vec<EditOperation> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse edit plan: {}", e))?;
    Ok(edits)
}

// Helper function to clear edit plan
fn clear_edit_plan(work_dir: &PathBuf) {
    let plan_path = get_plan_file_path(work_dir);
    let _ = fs::remove_file(&plan_path); // Ignore errors if file doesn't exist
}

// Helper function to show unified diff using the similar crate
fn show_unified_diff(old_content: &str, new_content: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut output = String::new();

    for (idx, group) in diff.grouped_ops(2).iter().enumerate() {
        if idx > 0 {
            output.push_str(&format!("{}\n", "---".bright_black()));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, color_fn): (&str, fn(&str) -> colored::ColoredString) = match change.tag() {
                    ChangeTag::Delete => ("-", |s: &str| s.red()),
                    ChangeTag::Insert => ("+", |s: &str| s.green()),
                    ChangeTag::Equal => (" ", |s: &str| s.normal()),
                };
                output.push_str(&format!("{}{}", sign, color_fn(&change.to_string())));
            }
        }
    }
    output
}

// Helper function to visualize whitespace characters for better debugging
fn show_whitespace(s: &str) -> String {
    s.replace('\t', "⇥")
     .replace('\n', "↵\n")
     .replace('\r', "⏎")
     .replace(' ', "·")
}

// Helper function to normalize whitespace for fuzzy matching
fn normalize_whitespace(s: &str) -> String {
    // Convert tabs to spaces
    let with_spaces = s.replace('\t', "    ");

    // Normalize line endings to \n
    let normalized_lines = with_spaces.replace("\r\n", "\n").replace('\r', "\n");

    // Trim each line and rejoin
    normalized_lines
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

// Helper function to find similar content in file and provide helpful suggestions
fn find_similar_content(file_content: &str, search_for: &str, max_context_lines: usize) -> Option<String> {
    // Try to find the first few words/lines of search_for in the file
    let search_lines: Vec<&str> = search_for.lines().collect();
    if search_lines.is_empty() {
        return None;
    }

    let first_search_line = search_lines[0].trim();
    if first_search_line.is_empty() && search_lines.len() > 1 {
        // Try second line if first is empty
        let second_line = search_lines[1].trim();
        if !second_line.is_empty() {
            return find_content_near(file_content, second_line, max_context_lines);
        }
    }

    if !first_search_line.is_empty() {
        return find_content_near(file_content, first_search_line, max_context_lines);
    }

    None
}

// Helper to find content near a pattern and return context
fn find_content_near(file_content: &str, pattern: &str, context_lines: usize) -> Option<String> {
    let file_lines: Vec<&str> = file_content.lines().collect();

    // Try to find the pattern (or part of it) in the file
    for (idx, line) in file_lines.iter().enumerate() {
        if line.contains(pattern) || pattern.contains(line.trim()) {
            // Found something similar, return context around it
            let start = idx.saturating_sub(context_lines);
            let end = (idx + context_lines + 1).min(file_lines.len());

            let context: Vec<String> = file_lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let line_num = start + i + 1;
                    let marker = if start + i == idx { ">>>" } else { "   " };
                    format!("{} {:4} | {}", marker, line_num, l)
                })
                .collect();

            return Some(context.join("\n"));
        }
    }

    None
}

// Enum to represent different matching strategies
#[derive(Debug, Clone, Copy)]
enum MatchStrategy {
    Exact,
    NormalizedWhitespace,
}

// Helper function to check if content matches using the specified strategy
fn content_matches(haystack: &str, needle: &str, strategy: MatchStrategy) -> bool {
    match strategy {
        MatchStrategy::Exact => haystack.contains(needle),
        MatchStrategy::NormalizedWhitespace => {
            let normalized_haystack = normalize_whitespace(haystack);
            let normalized_needle = normalize_whitespace(needle);
            normalized_haystack.contains(&normalized_needle)
        }
    }
}

/// Tool for planning multiple file edits
pub struct PlanEditsTool;

#[async_trait]
impl Tool for PlanEditsTool {
    fn name(&self) -> &str {
        "plan_edits"
    }

    fn description(&self) -> &str {
        "Plan multiple file edits to apply atomically. Validates all edits before storing the plan."
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        HashMap::from([
            param!("edits", "string", "JSON array of edit operations with file_path, old_content, new_content, and description fields", required),
        ])
    }

    async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Parse edits from parameters
        let edits_str = match params.get_required::<String>("edits") {
            Ok(edits) => edits,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        let edits_value: serde_json::Value = match serde_json::from_str(&edits_str) {
            Ok(value) => value,
            Err(e) => return ToolResult::error(format!("Failed to parse edits JSON: {}", e)),
        };

        let edits: Vec<EditOperation> = match serde_json::from_value(edits_value) {
            Ok(edits) => edits,
            Err(e) => return ToolResult::error(format!("Failed to parse edits: {}", e)),
        };

        if edits.is_empty() {
            return ToolResult::error("Cannot create empty edit plan. Provide at least one edit operation.".to_string());
        }

        println!("\n{}", "📋 Edit Plan Created".bright_cyan().bold());
        println!("{}", "═".repeat(60).bright_black());

        // Validate and preview each edit
        let mut validated_edits = Vec::new();
        for (idx, edit) in edits.iter().enumerate() {
            println!("\n{} {} - {}",
                format!("Edit #{}", idx + 1).bright_yellow(),
                edit.file_path.cyan(),
                edit.description.bright_white()
            );

            // Read current file to validate old_content exists
            let full_path = context.work_dir.join(&edit.file_path);
            let current_content = match fs::read_to_string(&full_path) {
                Ok(content) => content,
                Err(_) => return ToolResult::error(format!("Edit #{}: File not found: {}", idx + 1, edit.file_path)),
            };

            if edit.old_content.is_empty() {
                return ToolResult::error(format!("Edit #{}: old_content cannot be empty for file {}", idx + 1, edit.file_path));
            }

            if edit.old_content == edit.new_content {
                return ToolResult::error(format!(
                    "Edit #{}: old_content and new_content are identical for file {}. No change would be made.",
                    idx + 1, edit.file_path
                ));
            }

            // Try exact match first
            if !content_matches(&current_content, &edit.old_content, MatchStrategy::Exact) {
                // Exact match failed - try normalized whitespace match
                if content_matches(&current_content, &edit.old_content, MatchStrategy::NormalizedWhitespace) {
                    // Found with normalized whitespace - warn but allow
                    println!("  {} Whitespace differences detected - using normalized matching", "⚠".yellow());
                    println!("  {} Make sure tabs/spaces match exactly in the future", "ℹ".bright_blue());
                } else {
                    // Neither exact nor normalized match worked - provide detailed error
                    let mut error_msg = format!(
                        "Edit #{}: old_content not found in file {}\n\n",
                        idx + 1, edit.file_path
                    );

                    // Show what we're looking for with visible whitespace
                    error_msg.push_str(&format!("{}:\n", "Looking for (whitespace shown)".bright_yellow()));
                    let visible_search = show_whitespace(&edit.old_content);
                    for line in visible_search.lines().take(20) {
                        error_msg.push_str(&format!("  {}\n", line.bright_white()));
                    }
                    if edit.old_content.lines().count() > 20 {
                        error_msg.push_str(&format!("  {}\n", "... (truncated)".bright_black()));
                    }

                    // Try to find similar content
                    if let Some(similar) = find_similar_content(&current_content, &edit.old_content, 3) {
                        error_msg.push_str(&format!("\n{}:\n", "Similar content found in file".bright_cyan()));
                        error_msg.push_str(&format!("{}\n", similar));
                        error_msg.push_str(&format!("\n{}\n", "Hint: Check for differences in:".bright_yellow()));
                        error_msg.push_str(&format!("  {} Tabs vs spaces (⇥ = tab, · = space)\n", "•".bright_blue()));
                        error_msg.push_str(&format!("  {} Line endings (↵ = LF, ⏎ = CR)\n", "•".bright_blue()));
                        error_msg.push_str(&format!("  {} Leading/trailing whitespace\n", "•".bright_blue()));
                    } else {
                        error_msg.push_str(&format!("\n{}\n", "No similar content found in file.".bright_red()));
                        error_msg.push_str(&format!("{}\n", "The content you're trying to match may not exist in the file.".bright_black()));
                    }

                    return ToolResult::error(error_msg);
                }
            }

            // Show unified diff preview
            let diff_output = show_unified_diff(&edit.old_content, &edit.new_content);
            if !diff_output.is_empty() {
                for line in diff_output.lines() {
                    println!("  {}", line);
                }
            } else {
                println!("  {}", "(No changes)".bright_black());
            }

            validated_edits.push(edit.clone());
        }

        println!("\n{}", "═".repeat(60).bright_black());
        println!("{} {} edits planned", "✓".green(), validated_edits.len());
        println!("\n{}", "Use apply_edit_plan to execute all edits atomically.".bright_yellow());
        println!("{}", "The plan will be cleared after application or if you create a new plan.".bright_black());

        // Store the plan
        if let Err(e) = save_edit_plan(&context.work_dir, &validated_edits) {
            return ToolResult::error(e);
        }

        ToolResult::success(format!(
            "Edit plan created successfully with {} operation(s). All edits have been validated. \
            Use apply_edit_plan to execute all changes atomically.",
            validated_edits.len()
        ))
    }
}

/// Tool for applying planned edits
pub struct ApplyEditPlanTool;

#[async_trait]
impl Tool for ApplyEditPlanTool {
    fn name(&self) -> &str {
        "apply_edit_plan"
    }

    fn description(&self) -> &str {
        "Apply a previously planned set of file edits atomically"
    }

    fn parameters(&self) -> HashMap<String, ParameterDefinition> {
        // No parameters needed - applies the stored plan
        HashMap::new()
    }

    async fn execute(&self, _params: ToolParameters, context: &ToolContext) -> ToolResult {
        // Load the plan
        let plan = match load_edit_plan(&context.work_dir) {
            Ok(plan) => plan,
            Err(e) => return ToolResult::error(e),
        };

        println!("\n{}", "🚀 Applying Edit Plan".bright_cyan().bold());
        println!("{}", "═".repeat(60).bright_black());
        println!("{} {} edit(s) will be applied:", "📋".cyan(), plan.len());

        // Show summary of all edits
        for (idx, edit) in plan.iter().enumerate() {
            println!("  {}. {} - {}",
                idx + 1,
                edit.file_path.bright_white(),
                edit.description.cyan()
            );
        }

        println!("{}", "═".repeat(60).bright_black());

        // Check if we need to ask for confirmation
        // In non-interactive mode (web/API), skip prompt - already confirmed via web UI
        if !context.non_interactive {
            // Ask for confirmation in interactive mode
            println!("\n{}", "Apply all these changes? [Y/n]".bright_green().bold());

            let mut rl = match DefaultEditor::new() {
                Ok(rl) => rl,
                Err(e) => return ToolResult::error(format!("Failed to create readline editor: {}", e)),
            };

            let response = match rl.readline(">>> ") {
                Ok(resp) => resp,
                Err(_) => {
                    clear_edit_plan(&context.work_dir);
                    return ToolResult::error("Edit plan application cancelled by user".to_string());
                }
            };

            let response = response.trim().to_lowercase();

            match response.as_str() {
                "" | "y" | "yes" => {
                    // Continue with applying edits
                    println!("\n{}", "Applying edits...".green());
                }
                _ => {
                    // Cancelled - ask for optional feedback
                    println!("{}", "Would you like to provide feedback to the model about why you rejected this? (optional)".bright_yellow());
                    println!("{}", "Press Enter to skip, or type your feedback:".bright_black());

                    let feedback = match rl.readline("") {
                        Ok(fb) if !fb.trim().is_empty() => format!(" - {}", fb.trim()),
                        _ => String::new(),
                    };

                    clear_edit_plan(&context.work_dir);
                    return ToolResult::error(format!("Edit plan application cancelled by user{}", feedback));
                }
            }
        } else {
            // Non-interactive mode - auto-confirm (already confirmed via web UI)
            println!("\n{}", "✓ Confirmed via web UI - Applying edits...".green());
        }

        // Apply all edits sequentially
        let mut results = Vec::new();
        for (idx, edit) in plan.iter().enumerate() {
            println!("\n{} {}", format!("Applying edit #{}", idx + 1).yellow(), edit.file_path.cyan());

            // Re-read file to get current state (in case previous edits affected it)
            let full_path = context.work_dir.join(&edit.file_path);
            let current_content = match fs::read_to_string(&full_path) {
                Ok(content) => content,
                Err(_) => {
                    clear_edit_plan(&context.work_dir);
                    return ToolResult::error(format!(
                        "Edit #{} failed: File not found: {}. Edit plan aborted and cleared.",
                        idx + 1, edit.file_path
                    ));
                }
            };

            // Check if content still exists (might have changed due to previous edits)
            // Try exact match first, fall back to normalized if needed
            let updated_content = if current_content.contains(&edit.old_content) {
                // Exact match - use normal replace
                current_content.replace(&edit.old_content, &edit.new_content)
            } else if content_matches(&current_content, &edit.old_content, MatchStrategy::NormalizedWhitespace) {
                // Found with normalized whitespace - need to find and replace with normalization
                println!("  {} Using normalized whitespace matching", "ℹ".bright_blue());

                // Find the actual content in the file that matches when normalized
                // This is more complex - we need to find the matching substring
                let normalized_old = normalize_whitespace(&edit.old_content);
                let normalized_current = normalize_whitespace(&current_content);

                if let Some(pos) = normalized_current.find(&normalized_old) {
                    // Found the normalized position - now map back to original
                    // For simplicity, use normalized content for the replacement
                    // This ensures the edit works even with whitespace differences
                    let before_normalized = &normalized_current[..pos];
                    let after_normalized = &normalized_current[pos + normalized_old.len()..];
                    let normalized_new = normalize_whitespace(&edit.new_content);

                    format!("{}{}{}", before_normalized, normalized_new, after_normalized)
                } else {
                    // This shouldn't happen since content_matches returned true
                    clear_edit_plan(&context.work_dir);
                    return ToolResult::error(format!(
                        "Edit #{} failed: Internal error in normalized matching for {}. \
                        Edit plan aborted and cleared.",
                        idx + 1, edit.file_path
                    ));
                }
            } else {
                // Neither exact nor normalized match worked
                clear_edit_plan(&context.work_dir);

                let mut error_msg = format!(
                    "Edit #{} failed: old_content no longer found in {}.\n\
                    A previous edit in this plan may have affected this file.\n\
                    Edit plan aborted at step {}. No further edits applied. Plan has been cleared.\n\n",
                    idx + 1, edit.file_path, idx + 1
                );

                // Show what we were looking for
                error_msg.push_str(&format!("{}:\n", "Was looking for (whitespace shown)".bright_yellow()));
                let visible_search = show_whitespace(&edit.old_content);
                for line in visible_search.lines().take(10) {
                    error_msg.push_str(&format!("  {}\n", line));
                }

                return ToolResult::error(error_msg);
            };

            // Write the updated content
            if let Err(e) = fs::write(&full_path, &updated_content) {
                clear_edit_plan(&context.work_dir);
                return ToolResult::error(format!(
                    "Edit #{} failed: Failed to write file {}: {}. Edit plan aborted and cleared.",
                    idx + 1, edit.file_path, e
                ));
            }

            results.push(format!("✓ {}", edit.file_path));
            println!("  {} {}", "✓".green(), edit.description);
        }

        println!("\n{}", "═".repeat(60).bright_black());
        println!("{} All {} edits applied successfully!", "✅".green(), plan.len());

        // Clear the plan after successful application
        clear_edit_plan(&context.work_dir);

        ToolResult::success(format!(
            "Successfully applied {} edit(s):\n{}",
            plan.len(),
            results.join("\n")
        ))
    }
}