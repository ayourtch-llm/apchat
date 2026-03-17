# PPTX Tools

The `--pptx-tools` flag enables a suite of PowerPoint presentation tools for creating, reading, and editing PPTX files.

## Available Tools

### 1. `create_presentation`
Create a new PPTX presentation from scratch.

**Parameters:**
- `path`: Output file path (e.g., 'presentation.pptx')
- `title`: Presentation title
- `author`: Author name
- `slides`: Array of slide objects with:
  - `type`: 'title' or 'content'
  - `title`: Slide title
  - `subtitle`: Subtitle (for title slides, optional)
  - `bullets`: Array of bullet points (for content slides)
  - `background_color`: Optional hex color (e.g., '1A1A2E')

**Example:**
```json
{
  "path": "my_presentation.pptx",
  "title": "My Presentation",
  "author": "John Doe",
  "slides": [
    {
      "type": "title",
      "title": "Welcome",
      "subtitle": "My Amazing Presentation",
      "background_color": "1A1A2E"
    },
    {
      "type": "content",
      "title": "Key Points",
      "bullets": ["Point 1", "Point 2", "Point 3"],
      "background_color": "16213E"
    }
  ]
}
```

### 2. `read_pptx`
Read and extract information from an existing PPTX file.

**Parameters:**
- `path`: Path to the PPTX file to read

**Returns:** JSON with:
- Presentation metadata (title, author)
- Slide count
- Each slide's:
  - Slide number
  - Title
  - Bullet count
  - Has image/chart/table flags
  - Notes (if present)

**Example:**
```json
{
  "path": "existing_presentation.pptx"
}
```

### 3. `set_slide_background`
Set the background color of a specific slide.

**Parameters:**
- `path`: Path to the PPTX file to modify
- `slide_number`: 1-based slide number to modify
- `color`: Hex color code (e.g., '1A1A2E' or '#1A1A2E')

**Example:**
```json
{
  "path": "presentation.pptx",
  "slide_number": 1,
  "color": "1A1A2E"
}
```

### 4. `edit_pptx_slide`
Edit the content of a specific slide (title, bullets).

**Parameters:**
- `path`: Path to the PPTX file to modify
- `slide_number`: 1-based slide number to modify
- `title`: New title (optional)
- `bullets`: New bullet points array (optional)

**Example:**
```json
{
  "path": "presentation.pptx",
  "slide_number": 2,
  "title": "Updated Title",
  "bullets": ["New point 1", "New point 2"]
}
```

### 5. `add_slide_to_pptx`
Add a new slide to an existing presentation.

**Parameters:**
- `path`: Path to the PPTX file to modify
- `title`: Title for the new slide
- `bullets`: Bullet points for the new slide (optional)
- `after_slide`: Insert after this slide number (optional, appends if not specified)

**Example:**
```json
{
  "path": "presentation.pptx",
  "title": "New Slide",
  "bullets": ["Point 1", "Point 2"],
  "after_slide": 2
}
```

### 6. `remove_slide_from_pptx`
Remove a slide from an existing presentation.

**Parameters:**
- `path`: Path to the PPTX file to modify
- `slide_number`: 1-based slide number to remove

**Example:**
```json
{
  "path": "presentation.pptx",
  "slide_number": 3
}
```

## Usage Pattern

The recommended workflow is:

1. **Create** a basic presentation with `create_presentation`
2. **Read** it with `read_pptx` to verify structure
3. **Edit** slides individually with `set_slide_background` and `edit_pptx_slide`
4. **Add/Remove** slides as needed
5. **Read** again to verify final result

This incremental approach allows for better control over the final presentation appearance.

## Enabling the Tools

Add the `--pptx-tools` flag when running apchat:

```bash
apchat --pptx-tools
```

Or with a local model:

```bash
apchat --pptx-tools --llama-cpp-url http://localhost:8080
```

## Implementation Notes

- The `create_presentation` tool uses the `ppt-rs` library (Apache-2.0 licensed)
- The `set_slide_background` tool directly modifies the slide XML within the ZIP archive
- All tools preserve the original PPTX structure and relationships
- Background colors are stored as sRGB hex values in the slide XML