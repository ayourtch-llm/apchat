//! SmartArt part
//!
//! Represents SmartArt diagrams in presentations.
//! SmartArt provides visual representations of information and ideas.

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;
use crate::core::escape_xml;

/// SmartArt layout type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmartArtLayout {
    // List layouts
    #[default]
    BasicBlockList,
    VerticalBlockList,
    HorizontalBulletList,
    SquareAccentList,
    PictureAccentList,
    // Process layouts
    BasicProcess,
    AccentProcess,
    AlternatingFlow,
    ContinuousBlockProcess,
    // Cycle layouts
    BasicCycle,
    TextCycle,
    BlockCycle,
    // Hierarchy layouts
    OrgChart,
    Hierarchy,
    HorizontalHierarchy,
    // Relationship layouts
    BasicVenn,
    LinearVenn,
    StackedVenn,
    BasicRadial,
    // Matrix layouts
    BasicMatrix,
    TitledMatrix,
    // Pyramid layouts
    BasicPyramid,
    InvertedPyramid,
    // Picture layouts
    PictureStrips,
    PictureGrid,
}

impl SmartArtLayout {
    /// Get the layout GUID
    pub fn layout_id(&self) -> &'static str {
        match self {
            SmartArtLayout::BasicBlockList => "urn:microsoft.com/office/officeart/2005/8/layout/vList1",
            SmartArtLayout::VerticalBlockList => "urn:microsoft.com/office/officeart/2005/8/layout/vList2",
            SmartArtLayout::HorizontalBulletList => "urn:microsoft.com/office/officeart/2005/8/layout/hList1",
            SmartArtLayout::SquareAccentList => "urn:microsoft.com/office/officeart/2005/8/layout/vList3",
            SmartArtLayout::PictureAccentList => "urn:microsoft.com/office/officeart/2005/8/layout/vList5",
            SmartArtLayout::BasicProcess => "urn:microsoft.com/office/officeart/2005/8/layout/process1",
            SmartArtLayout::AccentProcess => "urn:microsoft.com/office/officeart/2005/8/layout/process2",
            SmartArtLayout::AlternatingFlow => "urn:microsoft.com/office/officeart/2005/8/layout/process3",
            SmartArtLayout::ContinuousBlockProcess => "urn:microsoft.com/office/officeart/2005/8/layout/process4",
            SmartArtLayout::BasicCycle => "urn:microsoft.com/office/officeart/2005/8/layout/cycle1",
            SmartArtLayout::TextCycle => "urn:microsoft.com/office/officeart/2005/8/layout/cycle2",
            SmartArtLayout::BlockCycle => "urn:microsoft.com/office/officeart/2005/8/layout/cycle3",
            SmartArtLayout::OrgChart => "urn:microsoft.com/office/officeart/2005/8/layout/orgChart1",
            SmartArtLayout::Hierarchy => "urn:microsoft.com/office/officeart/2005/8/layout/hierarchy1",
            SmartArtLayout::HorizontalHierarchy => "urn:microsoft.com/office/officeart/2005/8/layout/hierarchy2",
            SmartArtLayout::BasicVenn => "urn:microsoft.com/office/officeart/2005/8/layout/venn1",
            SmartArtLayout::LinearVenn => "urn:microsoft.com/office/officeart/2005/8/layout/venn2",
            SmartArtLayout::StackedVenn => "urn:microsoft.com/office/officeart/2005/8/layout/venn3",
            SmartArtLayout::BasicRadial => "urn:microsoft.com/office/officeart/2005/8/layout/radial1",
            SmartArtLayout::BasicMatrix => "urn:microsoft.com/office/officeart/2005/8/layout/matrix1",
            SmartArtLayout::TitledMatrix => "urn:microsoft.com/office/officeart/2005/8/layout/matrix2",
            SmartArtLayout::BasicPyramid => "urn:microsoft.com/office/officeart/2005/8/layout/pyramid1",
            SmartArtLayout::InvertedPyramid => "urn:microsoft.com/office/officeart/2005/8/layout/pyramid2",
            SmartArtLayout::PictureStrips => "urn:microsoft.com/office/officeart/2005/8/layout/picture1",
            SmartArtLayout::PictureGrid => "urn:microsoft.com/office/officeart/2005/8/layout/picture2",
        }
    }

    /// Get layout name
    pub fn name(&self) -> &'static str {
        match self {
            SmartArtLayout::BasicBlockList => "Basic Block List",
            SmartArtLayout::VerticalBlockList => "Vertical Block List",
            SmartArtLayout::HorizontalBulletList => "Horizontal Bullet List",
            SmartArtLayout::SquareAccentList => "Square Accent List",
            SmartArtLayout::PictureAccentList => "Picture Accent List",
            SmartArtLayout::BasicProcess => "Basic Process",
            SmartArtLayout::AccentProcess => "Accent Process",
            SmartArtLayout::AlternatingFlow => "Alternating Flow",
            SmartArtLayout::ContinuousBlockProcess => "Continuous Block Process",
            SmartArtLayout::BasicCycle => "Basic Cycle",
            SmartArtLayout::TextCycle => "Text Cycle",
            SmartArtLayout::BlockCycle => "Block Cycle",
            SmartArtLayout::OrgChart => "Organization Chart",
            SmartArtLayout::Hierarchy => "Hierarchy",
            SmartArtLayout::HorizontalHierarchy => "Horizontal Hierarchy",
            SmartArtLayout::BasicVenn => "Basic Venn",
            SmartArtLayout::LinearVenn => "Linear Venn",
            SmartArtLayout::StackedVenn => "Stacked Venn",
            SmartArtLayout::BasicRadial => "Basic Radial",
            SmartArtLayout::BasicMatrix => "Basic Matrix",
            SmartArtLayout::TitledMatrix => "Titled Matrix",
            SmartArtLayout::BasicPyramid => "Basic Pyramid",
            SmartArtLayout::InvertedPyramid => "Inverted Pyramid",
            SmartArtLayout::PictureStrips => "Picture Strips",
            SmartArtLayout::PictureGrid => "Picture Grid",
        }
    }
}

/// SmartArt node (data point)
#[derive(Debug, Clone)]
pub struct SmartArtNode {
    pub text: String,
    pub children: Vec<SmartArtNode>,
    pub color: Option<String>,
}

impl SmartArtNode {
    /// Create a new node
    pub fn new(text: impl Into<String>) -> Self {
        SmartArtNode {
            text: text.into(),
            children: vec![],
            color: None,
        }
    }

    /// Add a child node
    pub fn child(mut self, node: SmartArtNode) -> Self {
        self.children.push(node);
        self
    }

    /// Set color
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Generate data XML for this node
    pub fn to_data_xml(&self, depth: usize) -> String {
        let children_xml: String = self.children.iter()
            .map(|c| c.to_data_xml(depth + 1))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<dgm:pt modelId="{}" type="node">
  <dgm:prSet/>
  <dgm:spPr/>
  <dgm:t>
    <a:bodyPr/>
    <a:lstStyle/>
    <a:p><a:r><a:t>{}</a:t></a:r></a:p>
  </dgm:t>
</dgm:pt>
{}"#,
            depth * 100 + 1,
            escape_xml(&self.text),
            children_xml
        )
    }
}

/// SmartArt diagram part
#[derive(Debug, Clone)]
pub struct SmartArtPart {
    diagram_number: usize,
    layout: SmartArtLayout,
    nodes: Vec<SmartArtNode>,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    color_style: Option<String>,
}

impl SmartArtPart {
    /// Create a new SmartArt part
    pub fn new(diagram_number: usize, layout: SmartArtLayout) -> Self {
        SmartArtPart {
            diagram_number,
            layout,
            nodes: vec![],
            x: 914400,      // 1 inch
            y: 1828800,     // 2 inches
            width: 7315200, // 8 inches
            height: 3657600, // 4 inches
            color_style: None,
        }
    }

    /// Add a node
    pub fn add_node(mut self, node: SmartArtNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add multiple nodes from text items
    pub fn add_items(mut self, items: Vec<&str>) -> Self {
        for item in items {
            self.nodes.push(SmartArtNode::new(item));
        }
        self
    }

    /// Set position
    pub fn position(mut self, x: i64, y: i64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Set size
    pub fn size(mut self, width: i64, height: i64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set color style
    pub fn color_style(mut self, style: impl Into<String>) -> Self {
        self.color_style = Some(style.into());
        self
    }

    /// Get diagram number
    pub fn diagram_number(&self) -> usize {
        self.diagram_number
    }

    /// Get layout
    pub fn get_layout(&self) -> SmartArtLayout {
        self.layout
    }

    /// Get nodes
    pub fn nodes(&self) -> &[SmartArtNode] {
        &self.nodes
    }

    /// Data path
    pub fn data_path(&self) -> String {
        format!("ppt/diagrams/data{}.xml", self.diagram_number)
    }

    /// Layout path
    pub fn layout_path(&self) -> String {
        format!("ppt/diagrams/layout{}.xml", self.diagram_number)
    }

    /// Colors path
    pub fn colors_path(&self) -> String {
        format!("ppt/diagrams/colors{}.xml", self.diagram_number)
    }

    /// Quick style path
    pub fn quick_style_path(&self) -> String {
        format!("ppt/diagrams/quickStyle{}.xml", self.diagram_number)
    }

    /// Drawing path
    pub fn drawing_path(&self) -> String {
        format!("ppt/diagrams/drawing{}.xml", self.diagram_number)
    }

    /// Generate data XML
    pub fn generate_data_xml(&self) -> String {
        let nodes_xml: String = self.nodes.iter()
            .enumerate()
            .map(|(i, n)| n.to_data_xml(i))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="0" type="doc"/>
    {}
  </dgm:ptLst>
  <dgm:cxnLst/>
  <dgm:bg/>
  <dgm:whole/>
</dgm:dataModel>"#,
            nodes_xml
        )
    }

    /// Generate shape XML for embedding in slide
    pub fn to_slide_xml(&self, shape_id: usize) -> String {
        format!(
            r#"<p:graphicFrame>
  <p:nvGraphicFramePr>
    <p:cNvPr id="{}" name="Diagram {}"/>
    <p:cNvGraphicFramePr/>
    <p:nvPr/>
  </p:nvGraphicFramePr>
  <p:xfrm>
    <a:off x="{}" y="{}"/>
    <a:ext cx="{}" cy="{}"/>
  </p:xfrm>
  <a:graphic>
    <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
      <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:dm="rId{}" r:lo="rId{}" r:qs="rId{}" r:cs="rId{}"/>
    </a:graphicData>
  </a:graphic>
</p:graphicFrame>"#,
            shape_id,
            shape_id,
            self.x,
            self.y,
            self.width,
            self.height,
            shape_id,
            shape_id + 1,
            shape_id + 2,
            shape_id + 3
        )
    }
}

impl Part for SmartArtPart {
    fn path(&self) -> &str {
        "" // SmartArt has multiple paths
    }

    fn part_type(&self) -> PartType {
        PartType::Chart // Similar handling
    }

    fn content_type(&self) -> ContentType {
        ContentType::Xml
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        Ok(self.generate_data_xml())
    }

    fn from_xml(_xml: &str) -> Result<Self, PptxError> {
        Ok(SmartArtPart::new(1, SmartArtLayout::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smartart_layout() {
        assert_eq!(SmartArtLayout::OrgChart.name(), "Organization Chart");
        assert!(SmartArtLayout::BasicProcess.layout_id().contains("process1"));
    }

    #[test]
    fn test_smartart_node() {
        let node = SmartArtNode::new("Root")
            .child(SmartArtNode::new("Child 1"))
            .child(SmartArtNode::new("Child 2"));
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn test_smartart_part_new() {
        let part = SmartArtPart::new(1, SmartArtLayout::BasicBlockList);
        assert_eq!(part.diagram_number(), 1);
        assert_eq!(part.get_layout(), SmartArtLayout::BasicBlockList);
    }

    #[test]
    fn test_smartart_add_items() {
        let part = SmartArtPart::new(1, SmartArtLayout::BasicProcess)
            .add_items(vec!["Step 1", "Step 2", "Step 3"]);
        assert_eq!(part.nodes().len(), 3);
    }

    #[test]
    fn test_smartart_paths() {
        let part = SmartArtPart::new(2, SmartArtLayout::OrgChart);
        assert_eq!(part.data_path(), "ppt/diagrams/data2.xml");
        assert_eq!(part.layout_path(), "ppt/diagrams/layout2.xml");
    }

    #[test]
    fn test_smartart_to_xml() {
        let part = SmartArtPart::new(1, SmartArtLayout::BasicBlockList)
            .add_items(vec!["Item 1", "Item 2"]);
        let xml = part.to_xml().unwrap();
        assert!(xml.contains("dgm:dataModel"));
        assert!(xml.contains("Item 1"));
    }

    #[test]
    fn test_smartart_slide_xml() {
        let part = SmartArtPart::new(1, SmartArtLayout::BasicCycle);
        let xml = part.to_slide_xml(5);
        assert!(xml.contains("p:graphicFrame"));
        assert!(xml.contains("dgm:relIds"));
    }
}
