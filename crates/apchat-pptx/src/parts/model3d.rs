//! 3D Model part
//!
//! Represents 3D models embedded in presentations.
//! Supports GLB/GLTF format for 3D content.

use super::base::{Part, PartType, ContentType};
use crate::exc::PptxError;

/// 3D model format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Model3DFormat {
    #[default]
    Glb,
    Gltf,
    Obj,
    Fbx,
    Stl,
}

impl Model3DFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Model3DFormat::Glb => "glb",
            Model3DFormat::Gltf => "gltf",
            Model3DFormat::Obj => "obj",
            Model3DFormat::Fbx => "fbx",
            Model3DFormat::Stl => "stl",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Model3DFormat::Glb => "model/gltf-binary",
            Model3DFormat::Gltf => "model/gltf+json",
            Model3DFormat::Obj => "model/obj",
            Model3DFormat::Fbx => "application/octet-stream",
            Model3DFormat::Stl => "model/stl",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "glb" => Some(Model3DFormat::Glb),
            "gltf" => Some(Model3DFormat::Gltf),
            "obj" => Some(Model3DFormat::Obj),
            "fbx" => Some(Model3DFormat::Fbx),
            "stl" => Some(Model3DFormat::Stl),
            _ => None,
        }
    }
}

/// Camera preset for 3D view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraPreset {
    #[default]
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    IsometricTopUp,
    IsometricTopDown,
    IsometricBottomUp,
    IsometricBottomDown,
    IsometricLeftUp,
    IsometricLeftDown,
    IsometricRightUp,
    IsometricRightDown,
    IsometricOffAxis1Left,
    IsometricOffAxis1Right,
    IsometricOffAxis1Top,
    IsometricOffAxis2Left,
    IsometricOffAxis2Right,
    IsometricOffAxis2Top,
}

impl CameraPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            CameraPreset::Front => "front",
            CameraPreset::Back => "back",
            CameraPreset::Left => "left",
            CameraPreset::Right => "right",
            CameraPreset::Top => "top",
            CameraPreset::Bottom => "bottom",
            CameraPreset::IsometricTopUp => "isometricTopUp",
            CameraPreset::IsometricTopDown => "isometricTopDown",
            CameraPreset::IsometricBottomUp => "isometricBottomUp",
            CameraPreset::IsometricBottomDown => "isometricBottomDown",
            CameraPreset::IsometricLeftUp => "isometricLeftUp",
            CameraPreset::IsometricLeftDown => "isometricLeftDown",
            CameraPreset::IsometricRightUp => "isometricRightUp",
            CameraPreset::IsometricRightDown => "isometricRightDown",
            CameraPreset::IsometricOffAxis1Left => "isometricOffAxis1Left",
            CameraPreset::IsometricOffAxis1Right => "isometricOffAxis1Right",
            CameraPreset::IsometricOffAxis1Top => "isometricOffAxis1Top",
            CameraPreset::IsometricOffAxis2Left => "isometricOffAxis2Left",
            CameraPreset::IsometricOffAxis2Right => "isometricOffAxis2Right",
            CameraPreset::IsometricOffAxis2Top => "isometricOffAxis2Top",
        }
    }
}

/// 3D model rotation
#[derive(Debug, Clone, Copy, Default)]
pub struct Model3DRotation {
    pub x: f64, // degrees
    pub y: f64,
    pub z: f64,
}

impl Model3DRotation {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Model3DRotation { x, y, z }
    }

    /// Convert to EMU angles (60000ths of a degree)
    pub fn to_emu(&self) -> (i64, i64, i64) {
        (
            (self.x * 60000.0) as i64,
            (self.y * 60000.0) as i64,
            (self.z * 60000.0) as i64,
        )
    }
}

/// 3D model part (ppt/media/model3dN.glb)
#[derive(Debug, Clone)]
pub struct Model3DPart {
    path: String,
    model_number: usize,
    format: Model3DFormat,
    data: Vec<u8>,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    camera: CameraPreset,
    rotation: Model3DRotation,
    ambient_light: Option<String>,
    zoom: f64,
}

impl Model3DPart {
    /// Create a new 3D model part
    pub fn new(model_number: usize, format: Model3DFormat, data: Vec<u8>) -> Self {
        Model3DPart {
            path: format!("ppt/media/model3d{}.{}", model_number, format.extension()),
            model_number,
            format,
            data,
            x: 914400,      // 1 inch
            y: 1828800,     // 2 inches
            width: 4572000, // 5 inches
            height: 4572000, // 5 inches
            camera: CameraPreset::default(),
            rotation: Model3DRotation::default(),
            ambient_light: None,
            zoom: 1.0,
        }
    }

    /// Create from file
    pub fn from_file(model_number: usize, file_path: &str) -> Result<Self, PptxError> {
        let data = std::fs::read(file_path)?;
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| PptxError::InvalidValue("No file extension".to_string()))?;
        
        let format = Model3DFormat::from_extension(ext)
            .ok_or_else(|| PptxError::InvalidValue(format!("Unsupported 3D format: {}", ext)))?;
        
        Ok(Self::new(model_number, format, data))
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

    /// Set camera preset
    pub fn camera(mut self, camera: CameraPreset) -> Self {
        self.camera = camera;
        self
    }

    /// Set rotation
    pub fn rotation(mut self, x: f64, y: f64, z: f64) -> Self {
        self.rotation = Model3DRotation::new(x, y, z);
        self
    }

    /// Set zoom level
    pub fn zoom(mut self, zoom: f64) -> Self {
        self.zoom = zoom;
        self
    }

    /// Set ambient light color
    pub fn ambient_light(mut self, color: impl Into<String>) -> Self {
        self.ambient_light = Some(color.into());
        self
    }

    /// Get model number
    pub fn model_number(&self) -> usize {
        self.model_number
    }

    /// Get format
    pub fn get_format(&self) -> Model3DFormat {
        self.format
    }

    /// Get data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get relative path for relationships
    pub fn rel_target(&self) -> String {
        format!("../media/model3d{}.{}", self.model_number, self.format.extension())
    }

    /// Generate shape XML for embedding in slide
    pub fn to_slide_xml(&self, shape_id: usize, rel_id: &str) -> String {
        let (rot_x, rot_y, rot_z) = self.rotation.to_emu();
        let ambient = self.ambient_light.as_ref()
            .map(|c| format!(r#"<am3d:ambientLight><a:srgbClr val="{}"/></am3d:ambientLight>"#, c.trim_start_matches('#')))
            .unwrap_or_default();

        format!(
            r#"<p:sp>
  <p:nvSpPr>
    <p:cNvPr id="{}" name="3D Model {}"/>
    <p:cNvSpPr/>
    <p:nvPr>
      <a:extLst>
        <a:ext uri="{{C183D7F6-B498-43B3-948B-1728B52AA6E4}}">
          <am3d:model3d xmlns:am3d="http://schemas.microsoft.com/office/drawing/2017/model3d" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
            <am3d:spPr>
              <a:xfrm>
                <a:off x="{}" y="{}"/>
                <a:ext cx="{}" cy="{}"/>
              </a:xfrm>
            </am3d:spPr>
            <am3d:model3DExtLst/>
            <am3d:model3DCamera prst="{}"/>
            <am3d:model3DRot ax="{}" ay="{}" az="{}"/>
            {}
            <am3d:model3DRaster r:embed="{}"/>
          </am3d:model3d>
        </a:ext>
      </a:extLst>
    </p:nvPr>
  </p:nvSpPr>
  <p:spPr>
    <a:xfrm>
      <a:off x="{}" y="{}"/>
      <a:ext cx="{}" cy="{}"/>
    </a:xfrm>
    <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
  </p:spPr>
</p:sp>"#,
            shape_id,
            shape_id,
            self.x,
            self.y,
            self.width,
            self.height,
            self.camera.as_str(),
            rot_x,
            rot_y,
            rot_z,
            ambient,
            rel_id,
            self.x,
            self.y,
            self.width,
            self.height
        )
    }
}

impl Part for Model3DPart {
    fn path(&self) -> &str {
        &self.path
    }

    fn part_type(&self) -> PartType {
        PartType::Media
    }

    fn content_type(&self) -> ContentType {
        ContentType::Media(self.format.extension().to_string())
    }

    fn to_xml(&self) -> Result<String, PptxError> {
        // 3D models are binary, not XML
        Err(PptxError::InvalidOperation("3D models are binary, not XML".to_string()))
    }

    fn from_xml(_xml: &str) -> Result<Self, PptxError> {
        Err(PptxError::InvalidOperation("3D models cannot be created from XML".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model3d_format() {
        assert_eq!(Model3DFormat::Glb.extension(), "glb");
        assert_eq!(Model3DFormat::Gltf.mime_type(), "model/gltf+json");
        assert_eq!(Model3DFormat::from_extension("glb"), Some(Model3DFormat::Glb));
    }

    #[test]
    fn test_camera_preset() {
        assert_eq!(CameraPreset::Front.as_str(), "front");
        assert_eq!(CameraPreset::IsometricTopUp.as_str(), "isometricTopUp");
    }

    #[test]
    fn test_model3d_rotation() {
        let rot = Model3DRotation::new(45.0, 90.0, 0.0);
        let (x, y, z) = rot.to_emu();
        assert_eq!(x, 2700000); // 45 * 60000
        assert_eq!(y, 5400000); // 90 * 60000
        assert_eq!(z, 0);
    }

    #[test]
    fn test_model3d_new() {
        let model = Model3DPart::new(1, Model3DFormat::Glb, vec![0, 1, 2]);
        assert_eq!(model.model_number(), 1);
        assert_eq!(model.path(), "ppt/media/model3d1.glb");
    }

    #[test]
    fn test_model3d_builder() {
        let model = Model3DPart::new(1, Model3DFormat::Glb, vec![])
            .position(914400, 914400)
            .size(4572000, 4572000)
            .camera(CameraPreset::IsometricTopUp)
            .rotation(30.0, 45.0, 0.0)
            .zoom(1.5);
        assert_eq!(model.camera, CameraPreset::IsometricTopUp);
        assert_eq!(model.zoom, 1.5);
    }

    #[test]
    fn test_model3d_rel_target() {
        let model = Model3DPart::new(2, Model3DFormat::Gltf, vec![]);
        assert_eq!(model.rel_target(), "../media/model3d2.gltf");
    }

    #[test]
    fn test_model3d_slide_xml() {
        let model = Model3DPart::new(1, Model3DFormat::Glb, vec![]);
        let xml = model.to_slide_xml(5, "rId10");
        assert!(xml.contains("p:sp"));
        assert!(xml.contains("am3d:model3d"));
        assert!(xml.contains("rId10"));
    }
}
