//! Custom XML part
//!
//! Represents custom XML data stored in the presentation.
//! Used for storing application-specific data, metadata, or integration with external systems.

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;

/// Custom XML part (customXml/itemN.xml)
#[derive(Debug, Clone)]
pub struct CustomXmlPart {
    path: String,
    item_number: usize,
    namespace: Option<String>,
    root_element: String,
    content: String,
    properties: Vec<(String, String)>,
}

impl CustomXmlPart {
    /// Create a new custom XML part
    pub fn new(item_number: usize, root_element: impl Into<String>) -> Self {
        CustomXmlPart {
            path: format!("customXml/item{}.xml", item_number),
            item_number,
            namespace: None,
            root_element: root_element.into(),
            content: String::new(),
            properties: vec![],
        }
    }

    /// Set namespace
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    /// Set content (inner XML)
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Add a property
    pub fn property(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push((name.into(), value.into()));
        self
    }

    /// Get item number
    pub fn item_number(&self) -> usize {
        self.item_number
    }

    /// Get properties path
    pub fn properties_path(&self) -> String {
        format!("customXml/itemProps{}.xml", self.item_number)
    }

    fn generate_xml(&self) -> String {
        let ns_attr = self.namespace.as_ref()
            .map(|ns| format!(r#" xmlns="{}""#, ns))
            .unwrap_or_default();

        let props_xml: String = self.properties.iter()
            .map(|(k, v)| format!("<{}>{}</{}>", k, v, k))
            .collect::<Vec<_>>()
            .join("\n  ");

        let inner = if !self.content.is_empty() {
            &self.content
        } else if !props_xml.is_empty() {
            &props_xml
        } else {
            ""
        };

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<{}{}>
  {}
</{}>"#,
            self.root_element,
            ns_attr,
            inner,
            self.root_element
        )
    }

    /// Generate properties XML
    pub fn generate_properties_xml(&self) -> String {
        let ns = self.namespace.as_ref()
            .map(|ns| format!(r#"<ds:schemaRef ds:uri="{}"/>"#, ns))
            .unwrap_or_default();

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ds:datastoreItem xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml" ds:itemID="{{{}}}">
  <ds:schemaRefs>
    {}
  </ds:schemaRefs>
</ds:datastoreItem>"#,
            uuid::Uuid::new_v4().to_string().to_uppercase(),
            ns
        )
    }
}

impl Part for CustomXmlPart {
    fn path(&self) -> &str {
        &self.path
    }

    fn part_type(&self) -> PartType {
        PartType::Relationships // Custom handling
    }

    fn content_type(&self) -> ContentType {
        ContentType::Xml
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        Ok(self.generate_xml())
    }

    fn from_xml(xml: &str) -> Result<Self, PptxError> {
        let mut part = CustomXmlPart::new(1, "root");
        part.content = xml.to_string();
        Ok(part)
    }
}

/// Custom XML data store for managing multiple custom XML parts
#[derive(Debug, Clone, Default)]
pub struct CustomXmlStore {
    items: Vec<CustomXmlPart>,
}

impl CustomXmlStore {
    pub fn new() -> Self {
        CustomXmlStore::default()
    }

    /// Add a custom XML item
    pub fn add(&mut self, root_element: impl Into<String>) -> &mut CustomXmlPart {
        let item_number = self.items.len() + 1;
        self.items.push(CustomXmlPart::new(item_number, root_element));
        self.items.last_mut().unwrap()
    }

    /// Get all items
    pub fn items(&self) -> &[CustomXmlPart] {
        &self.items
    }

    /// Get item count
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_xml_new() {
        let part = CustomXmlPart::new(1, "myData");
        assert_eq!(part.path(), "customXml/item1.xml");
        assert_eq!(part.item_number(), 1);
    }

    #[test]
    fn test_custom_xml_builder() {
        let part = CustomXmlPart::new(1, "config")
            .namespace("http://example.com/config")
            .property("version", "1.0")
            .property("author", "Test");
        assert!(part.namespace.is_some());
        assert_eq!(part.properties.len(), 2);
    }

    #[test]
    fn test_custom_xml_to_xml() {
        let part = CustomXmlPart::new(1, "data")
            .property("name", "Test")
            .property("value", "123");
        let xml = part.to_xml().unwrap();
        assert!(xml.contains("<data>"));
        assert!(xml.contains("<name>Test</name>"));
    }

    #[test]
    fn test_custom_xml_store() {
        let mut store = CustomXmlStore::new();
        store.add("config");
        store.add("metadata");
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_properties_path() {
        let part = CustomXmlPart::new(3, "data");
        assert_eq!(part.properties_path(), "customXml/itemProps3.xml");
    }
}
