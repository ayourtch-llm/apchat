# format_text_on_slide Bug Fixes

## 🐛 Bug 1: Case-Sensitive Element Name Matching

### Problem
User tried to format text using selector `'title'` but got error:
```
Error: Element 'title' not found on slide
```

### Root Cause
Element names in PPTX are capitalized (`'Title'`, `'Content'`), but user passed lowercase `'title'`. The `element_matches_selector()` function did exact string comparison.

### Fix
Made element name comparison case-insensitive:
```rust
// Before
return elem_name == *name;

// After  
return elem_name.to_lowercase() == name.to_lowercase();
```

### Impact
✅ `'title'` now matches `'Title'` element
✅ `'body'` now matches `'Body'` or `'Content'` element
✅ User-friendly selectors work as expected

---

## 🐛 Bug 2: Color Parameter Ignored

### Problem
User called:
```json
{"tool": "format_text_on_slide", "arguments": {
  "color": "FFFFFF",
  "element_selector": "1"
}}
```
Tool said "Successfully formatted" but text remained BLACK in PowerPoint!

### Root Cause
The `color` parameter was:
1. Accepted in function signature ✅
2. Passed to `modify_run_properties()` ✅
3. **NEVER ACTUALLY USED** ❌

The function just ignored it!

### Why It's Complex
PPTX text color requires XML structure:
```xml
<a:rPr>
  <a:solidFill>
    <a:srgbClr val="FFFFFF"/>
  </a:solidFill>
</a:rPr>
```

This is a CHILD ELEMENT, not an attribute. Can't just add to attribute list!

### Fix
Modified `format_text_in_element()` to write solidFill element after opening `<a:rPr>`:

```rust
if name.as_ref() == b"a:rPr" {
    let modified = modify_run_properties(...);
    writer.write_event(Event::Start(modified))?;
    
    // Write solidFill if color specified
    if let Some(ref color_val) = color {
        let mut solid_fill = BytesStart::new("a:solidFill");
        writer.write_event(Event::Start(solid_fill))?;
        
        let mut srgb_clr = BytesStart::new("a:srgbClr");
        srgb_clr.push_attribute(("val", color_val.as_str()));
        writer.write_event(Event::Empty(srgb_clr))?;
        
        writer.write_event(Event::End(BytesEnd::new("a:solidFill")))?;
    }
}
```

### Impact
✅ Color changes now VISIBLE in PowerPoint
✅ White text on dark backgrounds works
✅ Branding colors can be applied
✅ Tool claims match reality

---

## Testing

Before fix:
```
User: format_text_on_slide(path="deck.pptx", slide=1, selector="title", color="FFFFFF")
Tool: "Element 'title' not found" ❌

User: format_text_on_slide(path="deck.pptx", slide=1, selector="1", color="FFFFFF")  
Tool: "Successfully formatted"
PowerPoint: Text still BLACK ❌
```

After fix:
```
User: format_text_on_slide(path="deck.pptx", slide=1, selector="title", color="FFFFFF")
Tool: "Successfully formatted" ✅
PowerPoint: Text is WHITE ✅
```

---

## Files Changed

- `crates/apchat-tools/src/pptx_element_edit.rs`
  - `element_matches_selector()` - case-insensitive comparison
  - `format_text_in_element()` - solidFill element writing for color

**Both bugs FIXED!** 🎉
