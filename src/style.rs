//! YAML-based style templates for HWPX output customisation.

use crate::error::Hwp2MdError;
use serde::Deserialize;
use std::path::Path;

/// User-supplied style template loaded from a YAML file.
///
/// All fields are optional — unspecified values fall back to the writer's
/// built-in defaults (A4 portrait, 바탕 font, standard margins).
///
/// # Example YAML
///
/// ```yaml
/// page:
///   width: 59528
///   height: 84188
///   landscape: false
///   margin:
///     left: 5670
///     right: 5670
///     top: 4252
///     bottom: 4252
/// font:
///   default: "맑은 고딕"
///   code: "D2Coding"
/// heading:
///   line_spacing: 180
/// ```
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct StyleTemplate {
    /// Page dimensions and margins (HWP units: 1 unit = 0.01 mm).
    pub page: PageStyle,
    /// Font overrides for body and code text.
    pub font: FontStyle,
    /// Heading formatting overrides.
    pub heading: HeadingStyle,
}

/// Page dimensions and margins in HWP units.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct PageStyle {
    /// Page width in HWP units (A4 default: 59528).
    pub width: Option<u32>,
    /// Page height in HWP units (A4 default: 84188).
    pub height: Option<u32>,
    /// Landscape orientation.
    pub landscape: Option<bool>,
    /// Page margins.
    pub margin: MarginStyle,
}

/// Page margins in HWP units.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MarginStyle {
    /// Left margin (default: 5670).
    pub left: Option<u32>,
    /// Right margin (default: 5670).
    pub right: Option<u32>,
    /// Top margin (default: 4252).
    pub top: Option<u32>,
    /// Bottom margin (default: 4252).
    pub bottom: Option<u32>,
}

/// Font name overrides.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct FontStyle {
    /// Default body font name (default: "바탕").
    pub default: Option<String>,
    /// Code block / inline code font name (default: "Courier New").
    pub code: Option<String>,
}

/// Heading formatting overrides.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct HeadingStyle {
    /// Heading line spacing percentage (default: 180).
    pub line_spacing: Option<u32>,
}

impl StyleTemplate {
    /// Load a style template from a YAML file.
    pub fn from_file(path: &Path) -> Result<Self, Hwp2MdError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Hwp2MdError::StyleLoad(format!("failed to read style file {}: {e}", path.display()))
        })?;
        Self::from_yaml(&content)
    }

    /// Parse a style template from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, Hwp2MdError> {
        serde_yaml::from_str(yaml)
            .map_err(|e| Hwp2MdError::StyleLoad(format!("invalid style YAML: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_yaml_produces_defaults() {
        let t = StyleTemplate::from_yaml("{}").unwrap();
        assert!(t.page.width.is_none());
        assert!(t.font.default.is_none());
        assert!(t.heading.line_spacing.is_none());
    }

    #[test]
    fn partial_yaml_parsed_correctly() {
        let yaml = r#"
page:
  width: 59528
  margin:
    left: 8000
font:
  default: "맑은 고딕"
"#;
        let t = StyleTemplate::from_yaml(yaml).unwrap();
        assert_eq!(t.page.width, Some(59528));
        assert!(t.page.height.is_none());
        assert_eq!(t.page.margin.left, Some(8000));
        assert!(t.page.margin.right.is_none());
        assert_eq!(t.font.default.as_deref(), Some("맑은 고딕"));
    }

    #[test]
    fn full_yaml_parsed() {
        let yaml = r#"
page:
  width: 59528
  height: 84188
  landscape: true
  margin:
    left: 5670
    right: 5670
    top: 4252
    bottom: 4252
font:
  default: "바탕"
  code: "D2Coding"
heading:
  line_spacing: 200
"#;
        let t = StyleTemplate::from_yaml(yaml).unwrap();
        assert_eq!(t.page.width, Some(59528));
        assert_eq!(t.page.height, Some(84188));
        assert_eq!(t.page.landscape, Some(true));
        assert_eq!(t.page.margin.left, Some(5670));
        assert_eq!(t.page.margin.right, Some(5670));
        assert_eq!(t.page.margin.top, Some(4252));
        assert_eq!(t.page.margin.bottom, Some(4252));
        assert_eq!(t.font.default.as_deref(), Some("바탕"));
        assert_eq!(t.font.code.as_deref(), Some("D2Coding"));
        assert_eq!(t.heading.line_spacing, Some(200));
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let result = StyleTemplate::from_yaml("not: [valid: yaml:");
        assert!(result.is_err());
    }

    #[test]
    fn missing_file_returns_error() {
        let result = StyleTemplate::from_file(Path::new("/nonexistent/style.yaml"));
        assert!(result.is_err());
    }
}
