# apchat-pptx

Pure Rust library for creating PPTX presentations.

## Features

- Create presentations from scratch
- Title and content slides
- Theme support (corporate blue, etc.)
- Builder pattern API

## Usage

```rust
use apchat_pptx::{Presentation, Theme};

let mut ppt = Presentation::new()
    .title("My Presentation")
    .author("My Name")
    .theme(Theme::corporate_blue());

ppt.add_title_slide("Welcome", "Subtitle");
ppt.add_content_slide("Content", vec!["Bullet 1", "Bullet 2"]);

ppt.save("output.pptx")?;
```

## APChat Integration

Enable with `--pptx-tools` flag to use PPTX creation tools in APChat.

## Example

Run the example:
```bash
cargo run --example create_basic
```

This will create `examples/output.pptx` with a sample presentation.

## License

MIT