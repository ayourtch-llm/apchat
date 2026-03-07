//! PPTX editing tools - read, modify, and save PowerPoint presentations
//!
//! This module provides tools for:
//! - Reading existing PPTX files and extracting content
//! - Modifying slide properties (background colors, text, etc.)
//! - Adding/removing slides
//! - Saving modified presentations

mod reader;
mod editor;

pub use reader::{ReadPptxTool, PptxSlideInfo, PptxPresentationInfo};
pub use editor::{EditPptxSlideTool, SetSlideBackgroundTool, AddSlideToPptxTool, RemoveSlideFromPptxTool};