//! Drawing Markup Language (DML) XML elements
//!
//! Core DrawingML types used across OOXML documents.

use super::xmlchemy::XmlElement;

/// Color types in DrawingML
#[derive(Debug, Clone)]
pub enum Color {
    /// RGB color (e.g., "FF0000" for red)
    Rgb(String),
    /// Scheme color (e.g., "accent1", "dk1")
    Scheme(String),
    /// System color (e.g., "windowText")
    System(String),
}

impl Color {
    pub fn rgb(hex: &str) -> Self {
        Color::Rgb(hex.trim_start_matches('#').to_uppercase())
    }

    pub fn scheme(name: &str) -> Self {
        Color::Scheme(name.to_string())
    }

    pub fn parse(elem: &XmlElement) -> Option<Self> {
        if let Some(srgb) = elem.find("srgbClr") {
            return srgb.attr("val").map(|v| Color::Rgb(v.to_string()));
        }
        if let Some(scheme) = elem.find("schemeClr") {
            return scheme.attr("val").map(|v| Color::Scheme(v.to_string()));
        }
        if let Some(sys) = elem.find("sysClr") {
            return sys.attr("val").map(|v| Color::System(v.to_string()));
        }
        None
    }

    pub fn to_xml(&self) -> String {
        match self {
            Color::Rgb(hex) => format!(r#"<a:srgbClr val="{hex}"/>"#),
            Color::Scheme(name) => format!(r#"<a:schemeClr val="{name}"/>"#),
            Color::System(name) => format!(r#"<a:sysClr val="{name}"/>"#),
        }
    }
}

/// Effect extent (a:effectExtent)
#[derive(Debug, Clone, Default)]
pub struct EffectExtent {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl EffectExtent {
    pub fn parse(elem: &XmlElement) -> Self {
        EffectExtent {
            left: elem.attr("l").and_then(|v| v.parse().ok()).unwrap_or(0),
            top: elem.attr("t").and_then(|v| v.parse().ok()).unwrap_or(0),
            right: elem.attr("r").and_then(|v| v.parse().ok()).unwrap_or(0),
            bottom: elem.attr("b").and_then(|v| v.parse().ok()).unwrap_or(0),
        }
    }

    pub fn to_xml(&self) -> String {
        let left = self.left;
        let top = self.top;
        let right = self.right;
        let bottom = self.bottom;
        format!(
            r#"<a:effectExtent l="{left}" t="{top}" r="{right}" b="{bottom}"/>"#
        )
    }
}

/// Line cap style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineCap {
    Round,
    Square,
    Flat,
}

impl LineCap {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineCap::Round => "rnd",
            LineCap::Square => "sq",
            LineCap::Flat => "flat",
        }
    }
}

/// Line join style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineJoin {
    Round,
    Bevel,
    Miter,
}

impl LineJoin {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineJoin::Round => "round",
            LineJoin::Bevel => "bevel",
            LineJoin::Miter => "miter",
        }
    }
}

/// Preset dash pattern
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DashPattern {
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
    LongDash,
    LongDashDot,
    LongDashDotDot,
    SystemDash,
    SystemDot,
    SystemDashDot,
    SystemDashDotDot,
}

impl DashPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            DashPattern::Solid => "solid",
            DashPattern::Dash => "dash",
            DashPattern::Dot => "dot",
            DashPattern::DashDot => "dashDot",
            DashPattern::DashDotDot => "dashDotDot",
            DashPattern::LongDash => "lgDash",
            DashPattern::LongDashDot => "lgDashDot",
            DashPattern::LongDashDotDot => "lgDashDotDot",
            DashPattern::SystemDash => "sysDash",
            DashPattern::SystemDot => "sysDot",
            DashPattern::SystemDashDot => "sysDashDot",
            DashPattern::SystemDashDotDot => "sysDashDotDot",
        }
    }
}

/// Outline (a:ln) - line/border properties
#[derive(Debug, Clone, Default)]
pub struct Outline {
    pub width: Option<u32>,
    pub cap: Option<LineCap>,
    pub compound: Option<String>,
    pub color: Option<Color>,
    pub dash: Option<DashPattern>,
    pub join: Option<LineJoin>,
    pub miter_limit: Option<u32>,
}

impl Outline {
    pub fn new() -> Self {
        Outline::default()
    }

    pub fn with_width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = Some(cap);
        self
    }

    pub fn with_dash(mut self, dash: DashPattern) -> Self {
        self.dash = Some(dash);
        self
    }

    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = Some(join);
        self
    }

    pub fn with_miter_limit(mut self, limit: u32) -> Self {
        self.miter_limit = Some(limit);
        self
    }

    pub fn parse(elem: &XmlElement) -> Self {
        let mut outline = Outline::new();
        
        outline.width = elem.attr("w").and_then(|v| v.parse().ok());
        if let Some(cap_str) = elem.attr("cap") {
            outline.cap = match cap_str {
                "rnd" => Some(LineCap::Round),
                "sq" => Some(LineCap::Square),
                "flat" => Some(LineCap::Flat),
                _ => None,
            };
        }
        outline.compound = elem.attr("cmpd").map(|s| s.to_string());

        if let Some(solid_fill) = elem.find("solidFill") {
            outline.color = Color::parse(solid_fill);
        }

        if let Some(prst_dash) = elem.find("prstDash") {
            if let Some(val) = prst_dash.attr("val") {
                outline.dash = match val {
                    "solid" => Some(DashPattern::Solid),
                    "dash" => Some(DashPattern::Dash),
                    "dot" => Some(DashPattern::Dot),
                    "dashDot" => Some(DashPattern::DashDot),
                    "dashDotDot" => Some(DashPattern::DashDotDot),
                    "lgDash" => Some(DashPattern::LongDash),
                    "lgDashDot" => Some(DashPattern::LongDashDot),
                    "lgDashDotDot" => Some(DashPattern::LongDashDotDot),
                    "sysDash" => Some(DashPattern::SystemDash),
                    "sysDot" => Some(DashPattern::SystemDot),
                    "sysDashDot" => Some(DashPattern::SystemDashDot),
                    "sysDashDotDot" => Some(DashPattern::SystemDashDotDot),
                    _ => None,
                };
            }
        }

        outline
    }

    pub fn to_xml(&self) -> String {
        let width = self.width.unwrap_or(12700);
        let mut attrs = vec![format!(r#"w="{width}""#)];

        if let Some(cap) = &self.cap {
            attrs.push(format!(r#"cap="{}""#, cap.as_str()));
        }

        if let Some(join) = &self.join {
            attrs.push(format!(r#"join="{}""#, join.as_str()));
        }

        if let Some(miter) = &self.miter_limit {
            attrs.push(format!(r#"miterLim="{}""#, miter));
        }

        let attr_str = attrs.join(" ");
        let mut inner = String::new();

        if let Some(ref color) = self.color {
            inner.push_str("<a:solidFill>");
            inner.push_str(&color.to_xml());
            inner.push_str("</a:solidFill>");
        }

        if let Some(dash) = &self.dash {
            inner.push_str(&format!(r#"<a:prstDash val="{}"/>"#, dash.as_str()));
        }

        if inner.is_empty() {
            format!(r#"<a:ln {attr_str}/>"#)
        } else {
            format!(r#"<a:ln {attr_str}>{inner}</a:ln>"#)
        }
    }
}

/// Gradient stop
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub position: u32, // 0-100000 (percentage * 1000)
    pub color: Color,
}

impl GradientStop {
    pub fn new(position: u32, color: Color) -> Self {
        GradientStop { position, color }
    }

    pub fn to_xml(&self) -> String {
        format!(
            r#"<a:gs pos="{}">{}</a:gs>"#,
            self.position,
            self.color.to_xml()
        )
    }
}

/// Gradient fill
#[derive(Debug, Clone)]
pub struct GradientFill {
    pub stops: Vec<GradientStop>,
    pub angle: Option<i32>, // in 60000ths of a degree
}

impl GradientFill {
    pub fn new() -> Self {
        GradientFill {
            stops: Vec::new(),
            angle: None,
        }
    }

    pub fn add_stop(mut self, position: u32, color: Color) -> Self {
        self.stops.push(GradientStop::new(position, color));
        self
    }

    pub fn with_angle(mut self, degrees: i32) -> Self {
        self.angle = Some(degrees * 60000);
        self
    }

    pub fn to_xml(&self) -> String {
        let mut xml = String::from("<a:gradFill><a:gsLst>");
        for stop in &self.stops {
            xml.push_str(&stop.to_xml());
        }
        xml.push_str("</a:gsLst>");

        if let Some(angle) = self.angle {
            xml.push_str(&format!(r#"<a:lin ang="{angle}" scaled="1"/>"#));
        }

        xml.push_str("</a:gradFill>");
        xml
    }
}

impl Default for GradientFill {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern fill type
#[derive(Debug, Clone)]
pub struct PatternFill {
    pub preset: String,
    pub foreground: Color,
    pub background: Color,
}

impl PatternFill {
    pub fn new(preset: &str, fg: Color, bg: Color) -> Self {
        PatternFill {
            preset: preset.to_string(),
            foreground: fg,
            background: bg,
        }
    }

    pub fn to_xml(&self) -> String {
        format!(
            r#"<a:pattFill prst="{}"><a:fgClr>{}</a:fgClr><a:bgClr>{}</a:bgClr></a:pattFill>"#,
            self.preset,
            self.foreground.to_xml(),
            self.background.to_xml()
        )
    }
}

/// Picture fill
#[derive(Debug, Clone)]
pub struct PictureFill {
    pub r_embed: String, // Relationship ID to embedded image
    pub stretch: bool,  // Stretch or tile
}

impl PictureFill {
    pub fn new(r_embed: &str) -> Self {
        PictureFill {
            r_embed: r_embed.to_string(),
            stretch: true,
        }
    }

    pub fn with_stretch(mut self, stretch: bool) -> Self {
        self.stretch = stretch;
        self
    }

    pub fn to_xml(&self) -> String {
        if self.stretch {
            format!(
                r#"<a:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></a:blipFill>"#,
                self.r_embed
            )
        } else {
            format!(
                r#"<a:blipFill><a:blip r:embed="{}"/><a:tile/></a:blipFill>"#,
                self.r_embed
            )
        }
    }
}

/// Texture fill
#[derive(Debug, Clone)]
pub struct TextureFill {
    pub r_embed: String, // Relationship ID to texture image
    pub tile: bool,      // Tile or stretch
}

impl TextureFill {
    pub fn new(r_embed: &str) -> Self {
        TextureFill {
            r_embed: r_embed.to_string(),
            tile: true,
        }
    }

    pub fn with_tile(mut self, tile: bool) -> Self {
        self.tile = tile;
        self
    }

    pub fn to_xml(&self) -> String {
        if self.tile {
            format!(
                r#"<a:blipFill><a:blip r:embed="{}"/><a:tile/></a:blipFill>"#,
                self.r_embed
            )
        } else {
            format!(
                r#"<a:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></a:blipFill>"#,
                self.r_embed
            )
        }
    }
}

/// Fill types
#[derive(Debug, Clone)]
pub enum Fill {
    None,
    Solid(Color),
    Gradient(GradientFill),
    Pattern(PatternFill),
    Picture(PictureFill),
    Texture(TextureFill),
}

impl Fill {
    pub fn solid(color: Color) -> Self {
        Fill::Solid(color)
    }

    pub fn picture(r_embed: &str) -> Self {
        Fill::Picture(PictureFill::new(r_embed))
    }

    pub fn texture(r_embed: &str) -> Self {
        Fill::Texture(TextureFill::new(r_embed))
    }

    pub fn to_xml(&self) -> String {
        match self {
            Fill::None => "<a:noFill/>".to_string(),
            Fill::Solid(color) => format!("<a:solidFill>{}</a:solidFill>", color.to_xml()),
            Fill::Gradient(grad) => grad.to_xml(),
            Fill::Pattern(pat) => pat.to_xml(),
            Fill::Picture(pic) => pic.to_xml(),
            Fill::Texture(tex) => tex.to_xml(),
        }
    }
}

/// Point in EMUs
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub fn new(x: i64, y: i64) -> Self {
        Point { x, y }
    }

    pub fn from_inches(x: f64, y: f64) -> Self {
        Point {
            x: (x * 914400.0) as i64,
            y: (y * 914400.0) as i64,
        }
    }
}

/// Size in EMUs
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: i64,
    pub height: i64,
}

impl Size {
    pub fn new(width: i64, height: i64) -> Self {
        Size { width, height }
    }

    pub fn from_inches(width: f64, height: f64) -> Self {
        Size {
            width: (width * 914400.0) as i64,
            height: (height * 914400.0) as i64,
        }
    }
}

/// Shadow effect
#[derive(Debug, Clone)]
pub struct Shadow {
    pub color: Option<Color>,
    pub blur_radius: Option<u32>, // in EMU
    pub distance: Option<u32>,     // in EMU
    pub angle: Option<i32>,        // in 60000ths of a degree
    pub offset_x: Option<i64>,     // in EMU
    pub offset_y: Option<i64>,     // in EMU
}

impl Shadow {
    pub fn new() -> Self {
        Shadow {
            color: None,
            blur_radius: None,
            distance: None,
            angle: None,
            offset_x: None,
            offset_y: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_blur(mut self, radius: u32) -> Self {
        self.blur_radius = Some(radius);
        self
    }

    pub fn with_distance(mut self, distance: u32) -> Self {
        self.distance = Some(distance);
        self
    }

    pub fn with_angle(mut self, degrees: i32) -> Self {
        self.angle = Some(degrees * 60000);
        self
    }

    pub fn with_offset(mut self, x: i64, y: i64) -> Self {
        self.offset_x = Some(x);
        self.offset_y = Some(y);
        self
    }

    pub fn to_xml(&self) -> String {
        let mut attrs = Vec::new();
        
        if let Some(blur) = self.blur_radius {
            attrs.push(format!(r#"blurRad="{blur}""#));
        }
        if let Some(dist) = self.distance {
            attrs.push(format!(r#"dist="{dist}""#));
        }
        if let Some(angle) = self.angle {
            attrs.push(format!(r#"dir="{angle}""#));
        }

        let attr_str = if attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", attrs.join(" "))
        };

        let mut inner = String::new();
        if let Some(ref color) = self.color {
            inner.push_str("<a:srgbClr>");
            inner.push_str(&color.to_xml());
            inner.push_str("</a:srgbClr>");
        }

        if let (Some(x), Some(y)) = (self.offset_x, self.offset_y) {
            format!(
                r#"<a:outerShdw{attr_str}><a:off x="{x}" y="{y}"/>{inner}</a:outerShdw>"#
            )
        } else {
            format!(r#"<a:outerShdw{attr_str}>{inner}</a:outerShdw>"#)
        }
    }
}

/// Glow effect
#[derive(Debug, Clone)]
pub struct Glow {
    pub color: Option<Color>,
    pub radius: Option<u32>, // in EMU
}

impl Glow {
    pub fn new() -> Self {
        Glow {
            color: None,
            radius: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_radius(mut self, radius: u32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn to_xml(&self) -> String {
        let radius_attr = self.radius
            .map(|r| format!(r#" rad="{r}""#))
            .unwrap_or_default();

        let mut inner = String::new();
        if let Some(ref color) = self.color {
            inner.push_str("<a:srgbClr>");
            inner.push_str(&color.to_xml());
            inner.push_str("</a:srgbClr>");
        }

        format!(r#"<a:glow{radius_attr}>{inner}</a:glow>"#)
    }
}

/// Reflection effect
#[derive(Debug, Clone)]
pub struct Reflection {
    pub blur_radius: Option<u32>, // in EMU
    pub distance: Option<u32>,    // in EMU
    pub alpha: Option<u32>,        // 0-100000 (transparency)
}

impl Reflection {
    pub fn new() -> Self {
        Reflection {
            blur_radius: None,
            distance: None,
            alpha: None,
        }
    }

    pub fn with_blur(mut self, radius: u32) -> Self {
        self.blur_radius = Some(radius);
        self
    }

    pub fn with_distance(mut self, distance: u32) -> Self {
        self.distance = Some(distance);
        self
    }

    pub fn with_alpha(mut self, alpha: u32) -> Self {
        self.alpha = Some(alpha.min(100000));
        self
    }

    pub fn to_xml(&self) -> String {
        let mut attrs = Vec::new();
        
        if let Some(blur) = self.blur_radius {
            attrs.push(format!(r#"blurRad="{blur}""#));
        }
        if let Some(dist) = self.distance {
            attrs.push(format!(r#"dist="{dist}""#));
        }

        let attr_str = if attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", attrs.join(" "))
        };

        let mut inner = String::new();
        if let Some(alpha) = self.alpha {
            inner.push_str(&format!(r#"<a:alpha val="{alpha}"/>"#));
        }

        if inner.is_empty() {
            format!(r#"<a:reflection{attr_str}/>"#)
        } else {
            format!(r#"<a:reflection{attr_str}>{inner}</a:reflection>"#)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgb() {
        let color = Color::rgb("FF0000");
        let xml = color.to_xml();
        assert!(xml.contains("srgbClr"));
        assert!(xml.contains("FF0000"));
    }

    #[test]
    fn test_color_scheme() {
        let color = Color::scheme("accent1");
        let xml = color.to_xml();
        assert!(xml.contains("schemeClr"));
        assert!(xml.contains("accent1"));
    }

    #[test]
    fn test_outline_to_xml() {
        let outline = Outline::new()
            .with_width(25400)
            .with_color(Color::rgb("0000FF"));
        let xml = outline.to_xml();
        
        assert!(xml.contains("w=\"25400\""));
        assert!(xml.contains("0000FF"));
    }

    #[test]
    fn test_gradient_fill() {
        let grad = GradientFill::new()
            .add_stop(0, Color::rgb("FF0000"))
            .add_stop(100000, Color::rgb("0000FF"))
            .with_angle(90);
        
        let xml = grad.to_xml();
        assert!(xml.contains("gradFill"));
        assert!(xml.contains("FF0000"));
        assert!(xml.contains("0000FF"));
    }

    #[test]
    fn test_fill_solid() {
        let fill = Fill::solid(Color::rgb("00FF00"));
        let xml = fill.to_xml();
        assert!(xml.contains("solidFill"));
        assert!(xml.contains("00FF00"));
    }
}
