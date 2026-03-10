# Element Name Matching Limitation

## Current Status

✅ **Case-insensitive matching**: IMPLEMENTED
- When element has `name` attribute directly on the tag, matching is case-insensitive
- `'title'` matches `'Title'` if the name is on the element itself

❌ **Child element names**: NOT YET IMPLEMENTED
- PPTX stores names on child `<p:cNvPr name="Title"/>` elements
- These are NOT accessible during the parent `<p:sp>` Start event
- quick-xml doesn't expose child attributes during parent events

## The Technical Challenge

**PPTX XML Structure:**
```xml
<p:sp>                    ← quick-xml Start event here (NO name attribute!)
  <p:nvSpPr>
    <p:cNvPr name="Title"/>  ← NAME IS HERE (child element)
  </p:nvSpPr>
  ...
</p:sp>
```

**The Problem:**
When quick-xml emits the `Start` event for `<p:sp>`, it only provides attributes on the `<p:sp>` tag itself. The `name` attribute is on a CHILD element (`<p:cNvPr>`), which hasn't been parsed yet!

**Borrow Checker Issue:**
To extract the name from the child element, we need to:
1. Read the XML buffer content
2. Search for `<p:cNvPr name="...">`
3. Extract the name value

BUT the buffer is mutably borrowed by the quick-xml reader, so we can't read from it while processing events.

## Workarounds

### Option 1: Use Numeric Selectors (Recommended)
Instead of `element_selector: "title"`, use `element_selector: "1"` (first element).

```json
// Works reliably
{"tool": "format_text_on_slide", "arguments": {
  "element_selector": "1",
  "color": "FFFFFF"
}}
```

### Option 2: Use read_slide_detailed First
Call `read_slide_detailed` to see actual element names and positions, then use those exact names.

```bash
read_slide_detailed(path="deck.pptx", slide_number=1)
# Returns element names like "Title", "Content", "roundedRectangle Shape"
# Then use those EXACT names (case-sensitive)
```

### Option 3: Pre-extract Names (Future Implementation)
A proper fix would:
1. Pre-extract all element names before quick-xml parsing
2. Store them in a vector: `["", "Title", "Content"]`
3. During parsing, use the pre-extracted name based on element index

This requires refactoring but would solve the issue completely.

## Current Implementation

```rust
fn element_matches_selector(
    event: &BytesStart,
    selector: &SelectorType,
    element_index: usize,
) -> bool {
    match selector {
        SelectorType::Index(idx) => element_index == *idx,
        SelectorType::Name(name) => {
            // Only works if name is on the element itself
            for attr in event.attributes().flatten() {
                if attr.key.as_ref() == b"name" {
                    let elem_name = String::from_utf8_lossy(&attr.value);
                    // Case-insensitive comparison
                    return elem_name.to_lowercase() == name.to_lowercase();
                }
            }
            false  // Name on child element - can't match yet
        }
    }
}
```

## Future Work

To fully fix this, we need to:
1. Add pre-extraction function to scan XML for all `<p:cNvPr name="...">` tags
2. Store names in order: `Vec<String>`
3. During quick-xml parsing, use element index to look up pre-extracted name
4. Compare selector against pre-extracted name

This is more invasive but would make name matching fully reliable.

## Impact

**Currently Works:**
- Numeric selectors: `"1"`, `"2"`, `"3"` ✅
- Case-insensitive matching when name is on parent element ✅

**Currently Limited:**
- Name selectors for elements with names on child `<p:cNvPr>` tags ❌

**Recommendation:** Use numeric selectors until buffer-based extraction is implemented.
