# PPTX Corruption Test Suite

## Overview

This test suite reproduces the PPTX file corruption that occurred during the workflow documented in `wip16.json`. The tests identify specific bugs in the PPTX editing tools that lead to corrupted presentation files.

## Root Cause Analysis

Based on careful study of `wip16.json` and the source code, the following issues were identified:

### Issue 1: `remove_slide_from_pptx` Tool is a Placeholder

**Location**: `crates/apchat-tools/src/pptx_edit/editor.rs:1004-1008`

**Problem**: The `remove_slide_from_pptx` tool returns success but **does not actually remove slides**:

```rust
// Lines 1004-1008 in editor.rs
ToolResult::success(format!(
    "Remove slide {} from '{}'",
    slide_number, path
))
```

**Impact**: When the agent calls this tool thinking slides are removed, subsequent operations on the "modified" file can corrupt the structure because:
1. Agent believes slide was removed
2. Agent may create new presentation with same filename (overwriting)
3. Or agent continues editing a file with unexpected structure

**Workflow Evidence from wip16.json**:
- Lines 338-407: Agent calls `remove_slide_from_pptx` 3 times
- Lines 664-717: Agent calls `remove_slide_from_pptx` 3 times again  
- Lines 1976-2050: Agent calls `remove_slide_from_pptx` 3 times again
- Lines 3143-3217: Agent calls `remove_slide_from_pptx` 3 times again
- After each sequence, `read_slide_detailed` shows `"title": null, "elements": []` (line 1956)

### Issue 2: XML Parsing Bugs in Element Editing

**Location**: `crates/apchat-tools/src/pptx_element_edit.rs`

**Problem**: The XML parser in `modify_element_in_slide()` and `format_text_in_element()` has state tracking bugs that cause malformed XML output.

**Symptoms from wip16.json**:
- Line 280: `"XML parsing error: Expecting </a:p> found </p:txBody>"`
- Line 648: `"XML parsing error: Expecting </p:sp> found </p:nvSpPr>"`

**Root Cause**: When modifying nested XML elements like `<a:off>`, `<a:ext>`, `<a:pPr>`, or `<a:rPr>`, the code doesn't correctly track:
1. Depth levels in nested structures
2. Element boundaries (when one element ends and another begins)
3. Which element has been "matched" by a selector vs which is currently being processed

**Buggy Code Pattern**:
```rust
// In modify_element_in_slide(), lines 218-244
if in_target_element {
    target_element_depth += 1;
    
    if tag_name.as_ref() == b"a:off" {
        let modified = modify_off_event(e, new_x, new_y);
        writer.write_event(Event::Start(modified))?;  // BUG: writes Start instead of Empty!
        buf.clear();
        continue;
    }
}
```

The code writes `Event::Start` when it should write `Event::Empty` for self-closing tags, or doesn't properly handle the closing tag, leading to malformed XML.

### Issue 3: Selector Matching Ambiguity

**Location**: `crates/apchat-tools/src/pptx_element_edit.rs:358-449`

**Problem**: The `parse_selector()` and element matching logic can match wrong elements:

```rust
// Lines 390-404
let aliased_name = match selector_lower.as_str() {
    "body" => "content",
    "title" => "title",
    "textbox" | "text" => "content",  // BUG: "textbox" maps to "content"
    "shape" => "shape",
    "image" => "image",
    _ => selector_lower.as_str(),
};
```

When user specifies `"body"` selector, it may match:
- Elements named "Content"
- Elements named with "content" substring
- Elements named "shape" that don't contain "title"

This ambiguity causes tools to modify wrong elements, corrupting slide structure.

## Test Files

### 1. `test_pptx_remove_slide_corruption.rs`

Tests the placeholder bug in `remove_slide_from_pptx`:

```rust
#[tokio::test]
async fn test_remove_slide_placeholder_causes_corruption() {
    // Creates 3-slide presentation
    // Calls remove_slide_from_pptx
    // Verifies slide was NOT removed (tool is placeholder)
    // Documents how this leads to agent confusion
}
```

### 2. `test_pptx_xml_corruption.rs`

Tests XML corruption from repeated tool operations:

```rust
#[tokio::test]
async fn test_repeated_element_edits_corrupt_xml_structure() {
    // Simulates wip16.json workflow:
    // 1. Set background
    // 2. Format text  
    // 3. Edit element properties
    // 4. Set z-order
    // Verifies XML structure after each operation
}
```

### 3. `test_pptx_xml_parsing_errors.rs`

Tests specific XML parsing bugs:

```rust
#[test]
fn test_modify_off_ext_attributes() {
    // Tests attribute modification in XML parser
    // Verifies output is well-formed XML
}

#[tokio::test]
async fn test_edit_element_properties_corruption() {
    // Calls edit_element_properties with "body" selector
    // Checks for XML parsing errors like:
    // "Expecting </a:p> found </p:txBody>"
}
```

## How to Run Tests

```bash
cd /Users/ayourtch/llm-rust/a/apchat
cargo test -p apchat-tools test_pptx_remove_slide_corruption -- --nocapture
cargo test -p apchat-tools test_pptx_xml_corruption -- --nocapture
cargo test -p apchat-tools test_pptx_xml_parsing_errors -- --nocapture
```

## Expected Test Results

### Before Fix

Tests should fail with:
1. `test_remove_slide_placeholder_causes_corruption`: Slide count remains 3 (not removed)
2. `test_repeated_element_edits_corrupt_xml_structure`: May panic on malformed XML
3. `test_edit_element_properties_corruption`: Returns XML parsing error

### After Fix

All tests should pass:
1. `remove_slide_from_pptx` actually removes slides
2. XML structure remains valid after edits
3. No parsing errors

## Recommended Fixes

### Fix 1: Implement `remove_slide_from_pptx`

```rust
// In crates/apchat-tools/src/pptx_edit/editor.rs
async fn execute(&self, params: ToolParameters, context: &ToolContext) -> ToolResult {
    // ... parameter parsing ...
    
    let file_data = std::fs::read(&full_path)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(&file_data))?;
    
    // Actually remove slide XML file
    // Update presentation.xml relationships  
    // Update [Content_Types].xml
    // Re-write ZIP archive
    
    ToolResult::success(format!("Successfully removed slide {} from '{}'", slide_number, path))
}
```

### Fix 2: Fix XML Parser State Tracking

In `modify_element_in_slide()` and `format_text_in_element()`:
1. Correctly distinguish `Event::Start` vs `Event::Empty` vs `Event::End`
2. Track depth properly for nested elements
3. Write correct closing tags

### Fix 3: Clarify Selector Matching

Make selector matching more explicit:
- Require exact element names
- Remove ambiguous aliases like "body" → "content"
- Add error messages when selector matches multiple elements

## Files Modified by Tests

The tests create temporary files in `/tmp`:
- `test_remove.pptx`
- `corrupt_test.pptx`
- `apchat_test.pptx`
- `corruption_test.pptx`
- `zorder_test.pptx`
- `wip16_pattern.pptx`
- `edit_test.pptx`
- `format_test.pptx`

All are cleaned up after test completion via `tempdir`.

## Dependencies

Tests require:
- `tokio` with `test-util` feature
- `tempfile` for temp directories
- `zip` for PPTX manipulation
- `quick-xml` for XML parsing (already in workspace)
- `serde_json` for JSON serialization

All dependencies are already in the workspace.

## Related Documentation

- `wip16.json`: Original workflow showing corruption
- `docs/pptx_tools.md`: PPTX tool documentation
- `crates/apchat-pptx/`: PPTX library source code
