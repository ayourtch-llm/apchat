# read_slide_detailed Color Extraction Fix

## 🐛 Bug: Text Colors Not Returned

### Symptoms
`read_slide_detailed` returned elements without color information:

```json
{
  "elements": [{
    "name": "Title",
    "content": "Who Am I?",
    "position": {"x": 457200, "y": 274638},
    "properties": {}  ← Always empty!
  }]
}
```

### Root Cause

PPTX stores text colors in `<a:srgbClr val="FFFFFF"/>` elements, but the parser wasn't extracting them!

**PPTX Color Structure:**
```xml
<a:rPr>
  <a:solidFill>
    <a:srgbClr val="EEEEEE"/>  ← Color value here!
  </a:solidFill>
</a:rPr>
<a:t>Who Am I?</a:t>
```

The parser extracted text content but ignored the color attribute.

### The Fix

Added `a:srgbClr` event handling in the XML parser:

```rust
b"a:srgbClr" => {
    // Extract color from val attribute
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"val" {
            let color_val = String::from_utf8_lossy(&attr.value).to_string();
            if let Some(ref mut elem) = current_element {
                elem.properties.insert("color".to_string(), color_val);
            }
            break;
        }
    }
}
```

Also added `properties: HashMap<String, String>` field to `SlideElementBuilder` to store the color.

### Impact

**BEFORE:**
```json
{
  "elements": [{
    "name": "Title",
    "properties": {}  ← No color info
  }]
}
```

**AFTER:**
```json
{
  "elements": [{
    "name": "Title",
    "properties": {
      "color": "EEEEEE"  ← White text!
    }
  }]
}
```

### Use Cases

1. **Color Debugging**: Verify text colors match design
2. **Format Validation**: Check if colors are consistent
3. **Template Analysis**: Understand color schemes
4. **Accessibility**: Verify contrast ratios

### Example

```bash
read_slide_detailed(path="presentation.pptx", slide_number=1)
```

Returns:
```json
{
  "slide_number": 1,
  "title": "Who Am I?",
  "elements": [
    {
      "element_type": "text",
      "name": "Title",
      "content": "Who Am I?",
      "position": {"x": 457200, "y": 274638},
      "size": {"width": 8230200, "height": 1143000},
      "properties": {
        "color": "EEEEEE"
      }
    }
  ]
}
```

### Color Format

Colors are returned as hex strings without `#`:
- `"FFFFFF"` = White
- `"000000"` = Black
- `"EEEEEE"` = Light gray
- `"E94560"` = Red/pink accent

### Files Changed

- `crates/apchat-tools/src/pptx_advanced_reader.rs`
  - Added `a:srgbClr` event handling
  - Added `properties` field to `SlideElementBuilder`
  - Colors now extracted and stored in element properties

**Bug FIXED!** 🎉
