//! VBA macro part
//!
//! Represents VBA macros embedded in the presentation (.pptm files).
//! Note: VBA macros require the presentation to be saved as .pptm format.

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;

/// VBA project part (ppt/vbaProject.bin)
#[derive(Debug, Clone)]
pub struct VbaProjectPart {
    path: String,
    data: Vec<u8>,
    modules: Vec<VbaModule>,
}

/// VBA module
#[derive(Debug, Clone)]
pub struct VbaModule {
    pub name: String,
    pub code: String,
    pub module_type: VbaModuleType,
}

/// VBA module type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaModuleType {
    Standard,
    Class,
    Form,
    Document,
}

impl VbaModule {
    /// Create a new standard module
    pub fn new(name: impl Into<String>, code: impl Into<String>) -> Self {
        VbaModule {
            name: name.into(),
            code: code.into(),
            module_type: VbaModuleType::Standard,
        }
    }

    /// Create a class module
    pub fn class(name: impl Into<String>, code: impl Into<String>) -> Self {
        VbaModule {
            name: name.into(),
            code: code.into(),
            module_type: VbaModuleType::Class,
        }
    }

    /// Set module type
    pub fn module_type(mut self, module_type: VbaModuleType) -> Self {
        self.module_type = module_type;
        self
    }
}

impl VbaProjectPart {
    /// Create a new VBA project part
    pub fn new() -> Self {
        VbaProjectPart {
            path: "ppt/vbaProject.bin".to_string(),
            data: vec![],
            modules: vec![],
        }
    }

    /// Create from binary data (existing vbaProject.bin)
    pub fn from_data(data: Vec<u8>) -> Self {
        VbaProjectPart {
            path: "ppt/vbaProject.bin".to_string(),
            data,
            modules: vec![],
        }
    }

    /// Add a module
    pub fn add_module(mut self, module: VbaModule) -> Self {
        self.modules.push(module);
        self
    }

    /// Get modules
    pub fn modules(&self) -> &[VbaModule] {
        &self.modules
    }

    /// Get binary data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Check if this is a macro-enabled presentation
    pub fn is_macro_enabled(&self) -> bool {
        !self.data.is_empty() || !self.modules.is_empty()
    }

    /// Get the content type for macro-enabled presentations
    pub fn macro_content_type() -> &'static str {
        "application/vnd.ms-office.vbaProject"
    }

    /// Get the file extension for macro-enabled presentations
    pub fn macro_extension() -> &'static str {
        "pptm"
    }
}

impl Default for VbaProjectPart {
    fn default() -> Self {
        Self::new()
    }
}

impl Part for VbaProjectPart {
    fn path(&self) -> &str {
        &self.path
    }

    fn part_type(&self) -> PartType {
        PartType::Relationships // Custom handling for binary
    }

    fn content_type(&self) -> ContentType {
        ContentType::Xml // Actually binary, but handled specially
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        // VBA projects are binary, not XML
        Err(PptxError::InvalidOperation("VBA projects are binary, not XML".to_string()))
    }

    fn from_xml(_xml: &str) -> Result<Self, PptxError> {
        Err(PptxError::InvalidOperation("VBA projects cannot be created from XML".to_string()))
    }
}

/// VBA macro security settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MacroSecurity {
    #[default]
    DisableAll,
    DisableWithNotification,
    DisableExceptDigitallySigned,
    EnableAll,
}

impl MacroSecurity {
    pub fn as_str(&self) -> &'static str {
        match self {
            MacroSecurity::DisableAll => "DisableAll",
            MacroSecurity::DisableWithNotification => "DisableWithNotification",
            MacroSecurity::DisableExceptDigitallySigned => "DisableExceptDigitallySigned",
            MacroSecurity::EnableAll => "EnableAll",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vba_project_new() {
        let project = VbaProjectPart::new();
        assert_eq!(project.path(), "ppt/vbaProject.bin");
        assert!(!project.is_macro_enabled());
    }

    #[test]
    fn test_vba_module_new() {
        let module = VbaModule::new("Module1", "Sub Test()\nEnd Sub");
        assert_eq!(module.name, "Module1");
        assert_eq!(module.module_type, VbaModuleType::Standard);
    }

    #[test]
    fn test_vba_module_class() {
        let module = VbaModule::class("MyClass", "Private x As Integer");
        assert_eq!(module.module_type, VbaModuleType::Class);
    }

    #[test]
    fn test_vba_project_add_module() {
        let project = VbaProjectPart::new()
            .add_module(VbaModule::new("Module1", "Sub Test()\nEnd Sub"))
            .add_module(VbaModule::class("Class1", ""));
        assert_eq!(project.modules().len(), 2);
        assert!(project.is_macro_enabled());
    }

    #[test]
    fn test_vba_from_data() {
        let project = VbaProjectPart::from_data(vec![0x00, 0x01, 0x02]);
        assert!(project.is_macro_enabled());
        assert_eq!(project.data().len(), 3);
    }

    #[test]
    fn test_macro_extension() {
        assert_eq!(VbaProjectPart::macro_extension(), "pptm");
    }
}
