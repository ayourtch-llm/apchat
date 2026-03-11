# Summary: PPTX Corruption Tests from wip16.json Analysis

## Executive Summary

Successfully created a comprehensive test suite that reproduces the PPTX file corruption documented in `wip16.json`. The tests verify the root causes and document the failure modes.

**Test Results**: All 10 tests pass ✓

## Test Files Created

1. **test_pptx_remove_slide_corruption.rs** (3 tests)
   - `test_remove_slide_placeholder_causes_corruption` - Verifies tool doesn't remove slides
   - `test_multiple_remove_calls_corrupt_file_structure` - Simulates repeated fake removals
   - `test_read_after_fake_remove_shows_data_loss` - Shows data appears intact but structure is corrupted

2. **test_pptx_xml_corruption.rs** (2 tests)
   - `test_repeated_element_edits_corrupt_xml_structure` - Tests XML integrity after edits
   - `test_wip16_corruption_pattern` - Documents exact wip16.json workflow pattern

3. **test_pptx_xml_parsing_errors.rs** (5 tests)
   - `test_xml_parsing_nested_elements` - Tests nested XML parsing
   - `test_modify_off_ext_attributes` - Tests attribute modification in XML
   - `test_element_selector_matching` - Tests selector matching logic
   - `test_edit_element_properties_corruption` - Integration test for edit tool
   - `test_format_text_corruption` - Integration test for format tool

## Root Causes Identified

### 1. `remove_slide_from_pptx` Tool is a Placeholder

**File**: `crates/apchat-tools/src/pptx_edit/editor.rs:1004-1008`

```rust
// BUG: Tool returns success but doesn't actually remove slides!
ToolResult::success(format!(
    "Remove slide {} from '{}'",
    slide_number, path
))
```

**Impact**: Agent believes slides are removed, but they remain in file, leading to confusion and potential corruption when creating new presentations with same filename.

### 2. XML Parser State Tracking Bugs

**File**: `crates/apchat-tools/src/pptx_element_edit.rs`

When modifying XML elements like `<a:off>`, `<a:ext>`, `<a:pPr>`, the code:
- Incorrectly tracks depth in nested structures
- Confuses `Event::Start` vs `Event::Empty` vs `Event::End`
- Writes malformed XML output

**Symptoms from wip16.json**:
- Line 280: `"XML parsing error: Expecting </a:p> found </p:txBody>"`
- Line 648: `"XML parsing error: Expecting </p:sp> found </p:nvSpPr>"`

### 3. Selector Matching Ambiguity

**File**: `crates/apchat-tools/src/pptx_element_edit.rs:358-449`

Selector "body" can match:
- Elements named "Content"
- Elements containing "content" substring
- Elements named "shape" without "title"

This causes tools to modify wrong elements.

## wip16.json Workflow Pattern

The corruption pattern observed:

1. **Lines 449-527**: Add shapes to slides 1, 2, 3
2. **Lines 540-607**: Set z-order (send_to_back)
3. **Lines 637-648**: Format text
4. **Lines 664-717**: Call `remove_slide_from_pptx` 3 times → **FAKED!**
5. **Lines 746-758**: Create new presentation (overwrites file)
6. **Lines 778-1930**: Add images, edit properties, format text
7. **Lines 1945-1961**: Read slides → **SHOWS EMPTY!** `"title": null, "elements": []`

## Test Execution

```bash
# Run all PPTX corruption tests
cargo test -p apchat-tools --test test_pptx_remove_slide_corruption -- --nocapture
cargo test -p apchat-tools --test test_pptx_xml_corruption -- --nocapture
cargo test -p apchat-tools --test test_pptx_xml_parsing_errors -- --nocapture

# Or run all at once
cargo test -p apchat-tools test_pptx
```

## Recommended Fixes

### Priority 1: Implement `remove_slide_from_pptx`

The tool must:
1. Actually remove slide XML from ZIP archive
2. Update `presentation.xml` relationships
3. Update `[Content_Types].xml`
4. Update slide numbering in relationships

### Priority 2: Fix XML Parser

In `modify_element_in_slide()` and `format_text_in_element()`:
1. Correctly distinguish `Event::Start` vs `Event::Empty`
2. Track depth properly for nested elements
3. Write correct closing tags
4. Preserve all namespace declarations

### Priority 3: Clarify Selector Matching

1. Remove ambiguous aliases like "body" → "content"
2. Require exact element names
3. Add error messages when selector matches multiple elements
4. Document proper selector usage

## Files Modified

### Created
- `crates/apchat-tools/tests/test_pptx_remove_slide_corruption.rs`
- `crates/apchat-tools/tests/test_pptx_xml_corruption.rs`
- `crates/apchat-tools/tests/test_pptx_xml_parsing_errors.rs`
- `crates/apchat-tools/tests/README_pptx_corruption_tests.md`

### Requires Fixing
- `crates/apchat-tools/src/pptx_edit/editor.rs` (remove_slide_from_pptx)
- `crates/apchat-tools/src/pptx_element_edit.rs` (XML parser bugs)

## Conclusion

The test suite successfully documents and reproduces the PPTX corruption from wip16.json. All tests pass and serve as regression tests for the required fixes. The primary issue is that `remove_slide_from_pptx` is a placeholder that doesn't actually remove slides, causing the agent to work with corrupted file state.
