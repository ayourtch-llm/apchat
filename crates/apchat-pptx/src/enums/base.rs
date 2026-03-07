//! Base enumeration types

use std::collections::HashMap;

/// Base enumeration type with MS API values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseEnum {
    pub name: &'static str,
    pub value: i32,
    pub doc: &'static str,
}

impl BaseEnum {
    /// Create a new BaseEnum
    pub const fn new(name: &'static str, value: i32, doc: &'static str) -> Self {
        BaseEnum { name, value, doc }
    }
}

impl std::fmt::Display for BaseEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.value)
    }
}

/// Enumeration type that maps to XML attribute values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseXmlEnum {
    pub name: &'static str,
    pub value: i32,
    pub xml_value: Option<&'static str>,
    pub doc: &'static str,
}

impl BaseXmlEnum {
    /// Create a new BaseXmlEnum
    pub const fn new(
        name: &'static str,
        value: i32,
        xml_value: Option<&'static str>,
        doc: &'static str,
    ) -> Self {
        BaseXmlEnum {
            name,
            value,
            xml_value,
            doc,
        }
    }

    /// Get enumeration member from XML value
    pub fn from_xml(xml_value: &str, members: &[BaseXmlEnum]) -> Result<BaseXmlEnum, String> {
        if xml_value.is_empty() {
            return Err("Empty XML value".to_string());
        }

        members
            .iter()
            .find(|m| m.xml_value == Some(xml_value))
            .copied()
            .ok_or_else(|| format!("No XML mapping for {}", xml_value))
    }

    /// Get XML value for enumeration member
    pub fn to_xml(&self) -> Result<&'static str, String> {
        self.xml_value
            .ok_or_else(|| format!("{} has no XML representation", self.name))
    }
}

impl std::fmt::Display for BaseXmlEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.value)
    }
}

/// Registry for enum members
pub struct EnumRegistry {
    members: HashMap<String, BaseXmlEnum>,
}

impl EnumRegistry {
    /// Create a new EnumRegistry
    pub fn new() -> Self {
        EnumRegistry {
            members: HashMap::new(),
        }
    }

    /// Register an enum member
    pub fn register(&mut self, name: String, member: BaseXmlEnum) {
        self.members.insert(name, member);
    }

    /// Get an enum member by name
    pub fn get(&self, name: &str) -> Option<BaseXmlEnum> {
        self.members.get(name).copied()
    }
}

impl Default for EnumRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_enum_creation() {
        let e = BaseEnum::new("TEST", 1, "Test enum");
        assert_eq!(e.name, "TEST");
        assert_eq!(e.value, 1);
        assert_eq!(e.doc, "Test enum");
    }

    #[test]
    fn test_base_enum_display() {
        let e = BaseEnum::new("TEST", 42, "Test");
        assert_eq!(format!("{}", e), "TEST (42)");
    }

    #[test]
    fn test_base_enum_equality() {
        let e1 = BaseEnum::new("TEST", 1, "Test");
        let e2 = BaseEnum::new("TEST", 1, "Test");
        let e3 = BaseEnum::new("OTHER", 2, "Other");
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_base_enum_clone() {
        let e1 = BaseEnum::new("TEST", 1, "Test");
        let e2 = e1;
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_base_xml_enum_creation() {
        let e = BaseXmlEnum::new("CENTER", 1, Some("ctr"), "Center alignment");
        assert_eq!(e.name, "CENTER");
        assert_eq!(e.value, 1);
        assert_eq!(e.xml_value, Some("ctr"));
        assert_eq!(e.doc, "Center alignment");
    }

    #[test]
    fn test_base_xml_enum_to_xml() {
        let e = BaseXmlEnum::new("CENTER", 1, Some("ctr"), "Center");
        assert_eq!(e.to_xml(), Ok("ctr"));
    }

    #[test]
    fn test_base_xml_enum_to_xml_none() {
        let e = BaseXmlEnum::new("NONE", 0, None, "No XML");
        assert!(e.to_xml().is_err());
    }

    #[test]
    fn test_base_xml_enum_from_xml() {
        let members = [
            BaseXmlEnum::new("LEFT", 0, Some("l"), "Left"),
            BaseXmlEnum::new("CENTER", 1, Some("ctr"), "Center"),
            BaseXmlEnum::new("RIGHT", 2, Some("r"), "Right"),
        ];
        
        let result = BaseXmlEnum::from_xml("ctr", &members);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "CENTER");
    }

    #[test]
    fn test_base_xml_enum_from_xml_not_found() {
        let members = [
            BaseXmlEnum::new("LEFT", 0, Some("l"), "Left"),
        ];
        
        let result = BaseXmlEnum::from_xml("unknown", &members);
        assert!(result.is_err());
    }

    #[test]
    fn test_base_xml_enum_from_xml_empty() {
        let members = [
            BaseXmlEnum::new("LEFT", 0, Some("l"), "Left"),
        ];
        
        let result = BaseXmlEnum::from_xml("", &members);
        assert!(result.is_err());
    }

    #[test]
    fn test_base_xml_enum_display() {
        let e = BaseXmlEnum::new("CENTER", 1, Some("ctr"), "Center");
        assert_eq!(format!("{}", e), "CENTER (1)");
    }

    #[test]
    fn test_enum_registry_new() {
        let registry = EnumRegistry::new();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_enum_registry_default() {
        let registry = EnumRegistry::default();
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_enum_registry_register_and_get() {
        let mut registry = EnumRegistry::new();
        let member = BaseXmlEnum::new("CENTER", 1, Some("ctr"), "Center");
        registry.register("CENTER".to_string(), member);
        
        let result = registry.get("CENTER");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "CENTER");
    }

    #[test]
    fn test_enum_registry_get_not_found() {
        let registry = EnumRegistry::new();
        assert!(registry.get("NOT_FOUND").is_none());
    }

    #[test]
    fn test_base_enum_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BaseEnum::new("A", 1, "A"));
        set.insert(BaseEnum::new("B", 2, "B"));
        set.insert(BaseEnum::new("A", 1, "A")); // duplicate
        assert_eq!(set.len(), 2);
    }
}
