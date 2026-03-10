# format_text_on_slide Bug Fixes - COMPLETE

## 🐛 Bug #1: Element Name Case Sensitivity

### Problem
```
User: format_text_on_slide(selector="title")
Tool: "Element 'title' not found"
```
Element names in PPTX are capitalized: "Title", "Content"

### Root Cause
Exact string comparison: `elem_name == *name`

### Fix
Made comparison case-insensitive: `elem_name.to_lowercase() == name.to_lowercase()`

### Impact
✅ `'title'` matches `'Title'`
✅ `'content'` matches `'Content'`  
✅ Case-insensitive selectors work

---

## 🐛 Bug #2: Color Parameter Ignored

### Problem
```
User: format_text_on_slide(color="FFFFFF")
Tool: "Successfully formatted"
PowerPoint: Text still BLACK!
```

### Root Cause
Color parameter was accepted but NEVER APPLIED!

### Fix
Added solidFill element writing in format_text_in_element():
```rust
if let Some(ref color_val) = color {
    let mut solid_fill = BytesStart::new("a:solidFill");
    writer.write_event(Event::Start(solid_fill))?;
    
    let mut srgb_clr = BytesStart::new("a:srgbClr");
    srgb_clr.push_attribute(("val", color_val.as_str()));
    writer.write_event(Event::Empty(srgb_clr))?;
    
    writer.write_event(Event::End(BytesEnd::new("a:solidFill")))?;
}
```

### Impact
✅ Color changes now VISIBLE in PowerPoint
✅ White text on dark backgrounds works
✅ Branding colors can be applied

---

## Testing

### Before Fixes
```
format_text_on_slide(selector="title", color="FFFFFF")
→ "Element 'title' not found" ❌

format_text_on_slide(selector="1", color="FFFFFF")  
→ "Successfully formatted"
→ Text still BLACK ❌
```

### After Fixes
```
format_text_on_slide(selector="title", color="FFFFFF")
→ "Successfully formatted" ✅
→ Text is WHITE ✅

format_text_on_slide(selector="1", color="B3FFB3")
→ "Successfully formatted" ✅  
→ Text is light green ✅
```

---

## Files Changed

- `crates/apchat-tools/src/pptx_element_edit.rs`
  - `element_matches_selector()` - case-insensitive comparison
  - `format_text_in_element()` - solidFill element for color

**Both critical bugs FIXED!** 🎉

---

## Known Limitations

1. **Element names on cNvPr**: Some PPTX elements have names on child `<p:cNvPr>` elements, not on `<p:sp>` start tags. For these, use numeric indices (1, 2, 3).

2. **Complex selectors**: Selectors like "textbox:1" aren't supported yet. Use exact names ("Title", "Content") or indices.

**Workaround**: Use `read_slide_detailed` first to see actual element names, then use those exact names or indices.
