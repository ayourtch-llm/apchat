//! PowerPoint (.pptx) file manipulation library
//!
//! A comprehensive Rust library for creating, reading, and updating PowerPoint 2007+ (.pptx) files.
//!
//! # License
//!
//! This library is distributed under the Apache-2.0 license.
//! Original repository: https://github.com/yingkitw/ppt-rs
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use ppt_rs::{create_pptx_with_content, SlideContent};
//!
//! let slides = vec![
//!     SlideContent::new("Welcome")
//!         .add_bullet("First point")
//!         .add_bullet("Second point"),
//! ];
//! let pptx_data = create_pptx_with_content("My Presentation", slides).unwrap();
//! std::fs::write("output.pptx", pptx_data).unwrap();
//! ```
//!
//! # Module Organization
//!
//! - **core** - Core traits (`ToXml`, `Positioned`, `Styled`) and utilities
//! - **elements** - Unified element types (Color, Position, Size, Transform)
//! - **generator** - PPTX file generation with ZIP packaging and XML creation
//! - **parts** - Package parts (SlidePart, ImagePart, ChartPart)
//! - **integration** - High-level builders for presentations
//! - **opc** - Open Packaging Convention (ZIP) handling
//! - **oxml** - Office XML parsing and manipulation
//! - **exc** - Error types

// Core traits and utilities
pub mod core;

// Unified element types
pub mod elements;

// Main functionality
pub mod generator;
pub mod integration;

// Supporting modules
pub mod config;
pub mod constants;
pub mod enums;
pub mod exc;
pub mod util;
pub mod opc;
pub mod oxml;
pub mod parts;

// Public API
pub mod api;
pub mod types;
pub mod shared;

// Easy-to-use prelude
pub mod prelude;

// Re-exports for convenience
pub use api::Presentation;
pub use core::{ToXml, escape_xml};
pub use elements::{Color, RgbColor, SchemeColor, Position, Size, Transform};
pub use exc::{PptxError, Result};
pub use generator::{
    create_pptx, create_pptx_with_content, create_pptx_with_settings,
    create_pptx_to_writer, create_pptx_with_content_to_writer, create_pptx_lazy_to_writer,
    LazySlideSource,
    SlideContent, SlideLayout,
    TextFormat, FormattedText,
    Table, TableRow, TableCell, TableBuilder,
    Shape, ShapeType, ShapeFill, ShapeLine,
    Image, ImageBuilder, ImageSource,
    Chart, ChartType, ChartSeries, ChartBuilder,
    // Bullet styles
    BulletStyle, BulletPoint,
    // RTL text support
    TextDirection, RtlLanguage, RtlTextProps,
    // Comments and annotations
    Comment, CommentAuthor, CommentAuthorList, SlideComments,
    // Slide sections
    SlideSection, SectionManager,
    // Digital signatures
    DigitalSignature, SignerInfo, HashAlgorithm, SignatureCommitment,
    // Ink annotations
    InkAnnotations, InkStroke, InkPen, InkPoint, PenTip,
    // Slide show settings
    SlideShowSettings, ShowType, PenColor, SlideRange,
    // Print settings and handouts
    PrintSettings, HandoutLayout, PrintColorMode, PrintWhat, Orientation,
    // Advanced table merging
    TableMergeMap, MergeRegion, CellMergeState,
    // Embedded fonts
    EmbeddedFontList, EmbeddedFont, FontStyle, FontCharset,
    // Presentation-level settings
    PresentationSettings,
    // New element types
    Connector, ConnectorType, ConnectorLine, ArrowType, ArrowSize, ConnectionSite, LineDash,
    Hyperlink, HyperlinkAction,
    GradientFill, GradientType, GradientDirection, GradientStop, PresetGradients,
    Video, Audio, VideoFormat, AudioFormat, VideoOptions, AudioOptions,
};
pub use integration::{PresentationBuilder, SlideBuilder, PresentationMetadata};
pub use oxml::repair::{PptxRepair, RepairIssue, RepairResult};

// Parts re-exports
pub use parts::{
    Part, PartType, ContentType,
    PresentationPart, SlidePart, SlideLayoutPart, LayoutType,
    SlideMasterPart, ThemePart, NotesSlidePart,
    ImagePart, MediaPart, MediaFormat, ChartPart,
    TablePart, TableRowPart, TableCellPart,
    CorePropertiesPart, AppPropertiesPart,
    ContentTypesPart, Relationships,
};

pub const VERSION: &str = "0.2.6";