//! Package URI handling

/// Represents a URI within a package
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackUri {
    uri: String,
}

impl PackUri {
    /// Create a new PackUri
    pub fn new(uri: &str) -> Self {
        PackUri {
            uri: uri.to_string(),
        }
    }

    /// Get the URI as a string
    pub fn as_str(&self) -> &str {
        &self.uri
    }

    /// Get the base URI (directory part)
    pub fn base_uri(&self) -> PackUri {
        if let Some(pos) = self.uri.rfind('/') {
            PackUri {
                uri: self.uri[..=pos].to_string(),
            }
        } else {
            PackUri {
                uri: "/".to_string(),
            }
        }
    }

    /// Get the filename part
    pub fn filename(&self) -> &str {
        if let Some(pos) = self.uri.rfind('/') {
            &self.uri[pos + 1..]
        } else {
            &self.uri
        }
    }

    /// Resolve a relative URI against this URI
    pub fn resolve(&self, relative: &str) -> PackUri {
        let base = self.base_uri();
        PackUri {
            uri: format!("{}{}", base.uri, relative),
        }
    }
}

impl std::fmt::Display for PackUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uri)
    }
}

impl From<&str> for PackUri {
    fn from(uri: &str) -> Self {
        PackUri::new(uri)
    }
}

impl From<String> for PackUri {
    fn from(uri: String) -> Self {
        PackUri { uri }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packuri_creation() {
        let uri = PackUri::new("/ppt/slides/slide1.xml");
        assert_eq!(uri.as_str(), "/ppt/slides/slide1.xml");
    }

    #[test]
    fn test_packuri_filename() {
        let uri = PackUri::new("/ppt/slides/slide1.xml");
        assert_eq!(uri.filename(), "slide1.xml");
    }

    #[test]
    fn test_packuri_base_uri() {
        let uri = PackUri::new("/ppt/slides/slide1.xml");
        assert_eq!(uri.base_uri().as_str(), "/ppt/slides/");
    }

    #[test]
    fn test_packuri_resolve() {
        let uri = PackUri::new("/ppt/slides/slide1.xml");
        let resolved = uri.resolve("../theme/theme1.xml");
        assert_eq!(resolved.as_str(), "/ppt/slides/../theme/theme1.xml");
    }

    #[test]
    fn test_packuri_from_str() {
        let uri: PackUri = "/ppt/presentation.xml".into();
        assert_eq!(uri.as_str(), "/ppt/presentation.xml");
    }

    #[test]
    fn test_packuri_from_string() {
        let uri: PackUri = String::from("/ppt/presentation.xml").into();
        assert_eq!(uri.as_str(), "/ppt/presentation.xml");
    }

    #[test]
    fn test_packuri_display() {
        let uri = PackUri::new("/ppt/slides/slide1.xml");
        assert_eq!(format!("{}", uri), "/ppt/slides/slide1.xml");
    }

    #[test]
    fn test_packuri_equality() {
        let uri1 = PackUri::new("/ppt/slides/slide1.xml");
        let uri2 = PackUri::new("/ppt/slides/slide1.xml");
        let uri3 = PackUri::new("/ppt/slides/slide2.xml");
        assert_eq!(uri1, uri2);
        assert_ne!(uri1, uri3);
    }

    #[test]
    fn test_packuri_clone() {
        let uri1 = PackUri::new("/ppt/slides/slide1.xml");
        let uri2 = uri1.clone();
        assert_eq!(uri1, uri2);
    }

    #[test]
    fn test_packuri_root() {
        let uri = PackUri::new("/");
        assert_eq!(uri.as_str(), "/");
        assert_eq!(uri.filename(), "");
        assert_eq!(uri.base_uri().as_str(), "/");
    }

    #[test]
    fn test_packuri_no_slash() {
        let uri = PackUri::new("file.xml");
        assert_eq!(uri.filename(), "file.xml");
        assert_eq!(uri.base_uri().as_str(), "/");
    }

    #[test]
    fn test_packuri_common_paths() {
        // Presentation
        let pres = PackUri::new("/ppt/presentation.xml");
        assert_eq!(pres.filename(), "presentation.xml");
        assert_eq!(pres.base_uri().as_str(), "/ppt/");

        // Slide
        let slide = PackUri::new("/ppt/slides/slide1.xml");
        assert_eq!(slide.filename(), "slide1.xml");
        assert_eq!(slide.base_uri().as_str(), "/ppt/slides/");

        // Theme
        let theme = PackUri::new("/ppt/theme/theme1.xml");
        assert_eq!(theme.filename(), "theme1.xml");
        assert_eq!(theme.base_uri().as_str(), "/ppt/theme/");

        // Relationships
        let rels = PackUri::new("/ppt/_rels/presentation.xml.rels");
        assert_eq!(rels.filename(), "presentation.xml.rels");
        assert_eq!(rels.base_uri().as_str(), "/ppt/_rels/");
    }

    #[test]
    fn test_packuri_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PackUri::new("/ppt/slides/slide1.xml"));
        set.insert(PackUri::new("/ppt/slides/slide2.xml"));
        set.insert(PackUri::new("/ppt/slides/slide1.xml")); // duplicate
        assert_eq!(set.len(), 2);
    }
}
