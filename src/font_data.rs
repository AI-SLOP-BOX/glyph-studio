use kurbo::{flatten, BezPath, PathEl, Point};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontProject {
    pub metadata: FontMetadata,
    pub glyphs: HashMap<String, GlyphData>,
    pub kerning: HashMap<(String, String), f64>,
    /// Per-master kerning. `kerning` remains the active-master view for
    /// backwards compatibility with existing callers and project files.
    #[serde(default)]
    pub kerning_by_master: HashMap<String, HashMap<(String, String), f64>>,
    #[serde(default)]
    pub vertical_metrics: HashMap<String, VerticalMetrics>,
    #[serde(default)]
    pub vertical_metrics_by_master: HashMap<String, HashMap<String, VerticalMetrics>>,
    /// Optional global vertical metrics per master. Missing entries inherit
    /// the project-wide metadata values.
    #[serde(default)]
    pub metrics_by_master: HashMap<String, MasterMetrics>,
    #[serde(default)]
    pub color_layers: HashMap<String, Vec<ColorLayer>>,
    /// Optional COLR v1 affine transforms, parallel to each base glyph's
    /// color-layer list.
    #[serde(default)]
    pub color_layer_transforms: HashMap<String, Vec<Option<ColorLayerTransform>>>,
    #[serde(default)]
    pub color_palettes: Vec<Vec<[u8; 4]>>,
    /// Optional human-readable labels for CPAL v1 palettes.
    #[serde(default)]
    pub color_palette_names: Vec<String>,
    /// CPAL v1 palette type flags (bit 0: light background, bit 1: dark background).
    #[serde(default)]
    pub color_palette_types: Vec<u32>,
    /// Optional CPAL v1 labels shared by the color entries in every palette.
    #[serde(default)]
    pub color_palette_entry_names: Vec<String>,
    #[serde(default)]
    pub glyph_order: Vec<String>,
    #[serde(default = "default_masters")]
    pub masters: Vec<FontMaster>,
    #[serde(default)]
    pub instances: Vec<FontInstance>,
    #[serde(default)]
    pub default_master_id: String,
    #[serde(default)]
    pub axis_names: HashMap<String, String>,
    /// Raw fvar axis flags, including the Hidden Axis bit.
    #[serde(default)]
    pub axis_flags: HashMap<String, u16>,
    /// Optional normalized axis remapping for the OpenType `avar` table.
    #[serde(default)]
    pub axis_mappings: HashMap<String, Vec<AxisMappingPoint>>,
    #[serde(default)]
    pub opentype_features: String,
    #[serde(default)]
    pub opentype_classes: String,
    #[serde(default)]
    pub background_images: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub background_opacities: HashMap<String, HashMap<String, f32>>,
    #[serde(default)]
    pub background_transforms: HashMap<String, HashMap<String, BackgroundImageTransform>>,
    #[serde(default)]
    pub guidelines: Vec<Guideline>,
    /// Per-master global guides. `guidelines` is the active-master view.
    #[serde(default)]
    pub guidelines_by_master: HashMap<String, Vec<Guideline>>,
    #[serde(default)]
    pub conditional_layers: HashMap<String, Vec<ConditionalLayer>>,
    /// Unicode Variation Sequence mappings (base scalar + selector -> glyph).
    #[serde(default)]
    pub unicode_variation_sequences: Vec<UnicodeVariationSequence>,
    /// Raw OpenType tables that Glyph Studio does not edit yet. Keeping these
    /// bytes makes importing and re-exporting fonts lossless for advanced
    /// tables such as MATH, JSTF, bitmap strikes, AAT, and vendor metadata.
    #[serde(default)]
    pub preserved_tables: HashMap<String, Vec<u8>>,
    /// Feature source captured when importing a font. If unchanged, raw
    /// GSUB/GPOS/GDEF tables can be reused without re-compiling them.
    #[serde(default)]
    pub preserved_layout_source: Option<String>,
    #[serde(default)]
    pub preserved_layout_fingerprint: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicodeVariationSequence {
    pub base: u32,
    pub selector: u32,
    pub glyph: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontMaster {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub width: f64,
    pub is_bracket: bool,
    #[serde(default)]
    pub axes: HashMap<String, f64>,
}

/// A named location in the variable-font design space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontInstance {
    pub name: String,
    #[serde(default)]
    pub axes: HashMap<String, f64>,
    #[serde(default = "default_instance_weight")]
    pub weight: f64,
    #[serde(default = "default_instance_width")]
    pub width: f64,
}

fn default_instance_weight() -> f64 {
    400.0
}

fn default_instance_width() -> f64 {
    100.0
}

/// Finds the four masters of a rectangular two-axis design space.
/// Returns bottom-left, bottom-right, top-left, top-right indices and the
/// normalized target factors. Irregular or incomplete layouts are rejected.
pub fn find_bilinear_masters(
    masters: &[FontMaster],
    axis_x: &str,
    axis_y: &str,
    target_x: f64,
    target_y: f64,
) -> Option<([usize; 4], (f64, f64))> {
    if masters.len() < 4 || axis_x == axis_y {
        return None;
    }
    let coordinates: Vec<(usize, f64, f64)> = masters
        .iter()
        .enumerate()
        .filter_map(|(index, master)| {
            Some((index, *master.axes.get(axis_x)?, *master.axes.get(axis_y)?))
        })
        .collect();
    if coordinates.len() < 4 {
        return None;
    }
    let min_x = coordinates
        .iter()
        .map(|(_, x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let max_x = coordinates
        .iter()
        .map(|(_, x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = coordinates
        .iter()
        .map(|(_, _, y)| *y)
        .fold(f64::INFINITY, f64::min);
    let max_y = coordinates
        .iter()
        .map(|(_, _, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    if (max_x - min_x).abs() < f64::EPSILON || (max_y - min_y).abs() < f64::EPSILON {
        return None;
    }
    let corner = |x: f64, y: f64| {
        coordinates
            .iter()
            .find(|(_, cx, cy)| (*cx - x).abs() < 1e-9 && (*cy - y).abs() < 1e-9)
            .map(|(index, _, _)| *index)
    };
    let indices = [
        corner(min_x, min_y)?,
        corner(max_x, min_y)?,
        corner(min_x, max_y)?,
        corner(max_x, max_y)?,
    ];
    Some((
        indices,
        (
            ((target_x - min_x) / (max_x - min_x)).clamp(0.0, 1.0),
            ((target_y - min_y) / (max_y - min_y)).clamp(0.0, 1.0),
        ),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VerticalMetrics {
    pub advance_height: f64,
    pub top_side_bearing: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MasterMetrics {
    pub ascender: f64,
    pub descender: f64,
    pub line_gap: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorGradientKind {
    #[default]
    Linear,
    Radial,
    Sweep,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorGradientExtend {
    #[default]
    Pad,
    Repeat,
    Reflect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradientStop {
    pub offset: f64,
    pub palette_index: u16,
    #[serde(default = "default_gradient_alpha")]
    pub alpha: f64,
}

fn default_gradient_alpha() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradient {
    pub start_palette_index: u16,
    pub end_palette_index: u16,
    #[serde(default)]
    pub kind: ColorGradientKind,
    #[serde(default)]
    pub extend: ColorGradientExtend,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    #[serde(default)]
    pub x2: f64,
    #[serde(default)]
    pub y2: f64,
    #[serde(default)]
    pub stops: Vec<ColorGradientStop>,
    #[serde(default = "default_gradient_radius")]
    pub radius0: f64,
    #[serde(default = "default_gradient_radius")]
    pub radius1: f64,
    #[serde(default)]
    pub start_angle: f64,
    #[serde(default = "default_gradient_end_angle")]
    pub end_angle: f64,
}

impl ColorGradient {
    pub fn effective_stops(&self) -> Vec<ColorGradientStop> {
        if self.stops.is_empty() {
            vec![
                ColorGradientStop {
                    offset: 0.0,
                    palette_index: self.start_palette_index,
                    alpha: 1.0,
                },
                ColorGradientStop {
                    offset: 1.0,
                    palette_index: self.end_palette_index,
                    alpha: 1.0,
                },
            ]
        } else {
            self.stops.clone()
        }
    }
}

fn default_gradient_radius() -> f64 {
    0.0
}

fn default_gradient_end_angle() -> f64 {
    360.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorLayer {
    pub glyph: String,
    pub palette_index: u16,
    #[serde(default)]
    pub gradient: Option<ColorGradient>,
    #[serde(default = "default_color_layer_alpha")]
    pub alpha: f64,
}

fn default_color_layer_alpha() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorLayerTransform {
    #[serde(default = "default_color_transform_scale")]
    pub xx: f64,
    #[serde(default)]
    pub yx: f64,
    #[serde(default)]
    pub xy: f64,
    #[serde(default = "default_color_transform_scale")]
    pub yy: f64,
    #[serde(default)]
    pub dx: f64,
    #[serde(default)]
    pub dy: f64,
}

fn default_color_transform_scale() -> f64 {
    1.0
}

impl Default for ColorLayerTransform {
    fn default() -> Self {
        Self {
            xx: 1.0,
            yx: 0.0,
            xy: 0.0,
            yy: 1.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

impl ColorLayerTransform {
    pub fn is_identity(self) -> bool {
        self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BackgroundImageTransform {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisMappingPoint {
    /// Input and output are normalized coordinates in the -1.0..=1.0 range.
    pub input: f64,
    pub output: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalLayer {
    pub id: String,
    pub conditions: HashMap<String, AxisRange>,
    pub layer: GlyphLayer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guideline {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub angle: f64,
    #[serde(default)]
    pub name: String,
}

fn default_masters() -> Vec<FontMaster> {
    vec![FontMaster::default()]
}

impl Default for FontMaster {
    fn default() -> Self {
        Self {
            id: "regular".to_string(),
            name: "Regular".to_string(),
            weight: 400.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontMetadata {
    pub family_name: String,
    pub style_name: String,
    #[serde(default)]
    pub copyright: String,
    #[serde(default)]
    pub designer: String,
    #[serde(default)]
    pub manufacturer: String,
    #[serde(default = "default_vendor_id")]
    pub vendor_id: String,
    #[serde(default = "default_font_revision")]
    pub font_revision: f64,
    pub units_per_em: f64,
    pub ascender: f64,
    pub descender: f64,
    pub line_gap: f64,
    #[serde(default)]
    pub italic_angle: f64,
    #[serde(default)]
    pub underline_position: f64,
    #[serde(default)]
    pub underline_thickness: f64,
    #[serde(default)]
    pub is_fixed_pitch: bool,
    #[serde(default)]
    pub x_height: f64,
    #[serde(default)]
    pub cap_height: f64,
    #[serde(default = "default_weight_class")]
    pub weight_class: u16,
    #[serde(default = "default_width_class")]
    pub width_class: u16,
    #[serde(default)]
    pub fs_type: u16,
    /// Raw OS/2 fsSelection flags from an imported font. Zero means that a
    /// new project should use the style-derived defaults at export time.
    #[serde(default)]
    pub fs_selection: u16,
    /// Raw `head.flags`; zero uses the standard export defaults.
    #[serde(default)]
    pub head_flags: u16,
    /// Raw `head.macStyle`; zero derives Bold/Italic from metadata.
    #[serde(default)]
    pub head_mac_style: u16,
    /// Raw `head.lowestRecPPEM`; zero uses the standard export default.
    #[serde(default)]
    pub lowest_rec_ppem: u16,
    /// `head.fontDirectionHint` used by legacy rasterizers.
    #[serde(default = "default_font_direction_hint")]
    pub font_direction_hint: i16,
    /// OS/2 PANOSE classification bytes.
    #[serde(default = "default_panose")]
    pub panose: [u8; 10],
    /// OS/2 fallback character codepoints. Zero uses automatic values.
    #[serde(default)]
    pub default_char: u16,
    #[serde(default)]
    pub break_char: u16,
    /// OS/2 maximum shaping context. Zero derives it from the Feature source.
    #[serde(default)]
    pub max_context: u16,
    /// hhea caret slope rise. Zero uses the OpenType default of 1.
    #[serde(default)]
    pub caret_slope_rise: i16,
    /// hhea caret slope run.
    #[serde(default)]
    pub caret_slope_run: i16,
    /// hhea caret offset.
    #[serde(default)]
    pub caret_offset: i16,
    /// OS/2 subscript and superscript metrics. Zero uses calculated defaults.
    #[serde(default)]
    pub subscript_x_size: i16,
    #[serde(default)]
    pub subscript_y_size: i16,
    #[serde(default)]
    pub subscript_x_offset: i16,
    #[serde(default)]
    pub subscript_y_offset: i16,
    #[serde(default)]
    pub superscript_x_size: i16,
    #[serde(default)]
    pub superscript_y_size: i16,
    #[serde(default)]
    pub superscript_x_offset: i16,
    #[serde(default)]
    pub superscript_y_offset: i16,
    #[serde(default)]
    pub strikeout_size: i16,
    #[serde(default)]
    pub strikeout_position: i16,
    /// OS/2 sFamilyClass. Zero is the neutral class.
    #[serde(default)]
    pub family_class: i16,
    /// OS/2 v5 optical point-size bounds. Zero omits the legacy bounds.
    #[serde(default)]
    pub lower_optical_point_size: u16,
    #[serde(default)]
    pub upper_optical_point_size: u16,
    /// vhea caret metrics. A zero rise uses the default of 1.
    #[serde(default)]
    pub vertical_caret_slope_rise: i16,
    #[serde(default)]
    pub vertical_caret_slope_run: i16,
    #[serde(default)]
    pub vertical_caret_offset: i16,
    /// OS/2 Windows ascent/descent. Zero derives values from the outline.
    #[serde(default)]
    pub win_ascent: u16,
    #[serde(default)]
    pub win_descent: u16,
}

fn default_font_revision() -> f64 {
    1.0
}

fn default_font_direction_hint() -> i16 {
    2
}

fn default_panose() -> [u8; 10] {
    [2, 0, 0, 0, 0, 0, 0, 0, 0, 0]
}

fn default_vendor_id() -> String {
    "GLYP".to_string()
}

fn default_weight_class() -> u16 {
    400
}
fn default_width_class() -> u16 {
    5
}

impl Default for FontMetadata {
    fn default() -> Self {
        Self {
            family_name: "My Font".to_string(),
            style_name: "Regular".to_string(),
            copyright: String::new(),
            designer: String::new(),
            manufacturer: String::new(),
            vendor_id: default_vendor_id(),
            font_revision: 1.0,
            units_per_em: 1000.0,
            ascender: 800.0,
            descender: -200.0,
            line_gap: 0.0,
            italic_angle: 0.0,
            underline_position: -100.0,
            underline_thickness: 50.0,
            is_fixed_pitch: false,
            x_height: 0.0,
            cap_height: 0.0,
            weight_class: 400,
            width_class: 5,
            fs_type: 0,
            fs_selection: 0,
            head_flags: 0,
            head_mac_style: 0,
            lowest_rec_ppem: 0,
            font_direction_hint: default_font_direction_hint(),
            panose: default_panose(),
            default_char: 0,
            break_char: 0,
            max_context: 0,
            caret_slope_rise: 0,
            caret_slope_run: 0,
            caret_offset: 0,
            subscript_x_size: 0,
            subscript_y_size: 0,
            subscript_x_offset: 0,
            subscript_y_offset: 0,
            superscript_x_size: 0,
            superscript_y_size: 0,
            superscript_x_offset: 0,
            superscript_y_offset: 0,
            strikeout_size: 0,
            strikeout_position: 0,
            family_class: 0,
            lower_optical_point_size: 0,
            upper_optical_point_size: 0,
            vertical_caret_slope_rise: 0,
            vertical_caret_slope_run: 0,
            vertical_caret_offset: 0,
            win_ascent: 0,
            win_descent: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rectangular_two_axis_masters_and_factors() {
        let master = |id: &str, x: f64, y: f64| FontMaster {
            id: id.into(),
            name: id.into(),
            weight: x,
            width: y,
            is_bracket: false,
            axes: [("wght".into(), x), ("wdth".into(), y)]
                .into_iter()
                .collect(),
        };
        let masters = vec![
            master("bl", 100.0, 75.0),
            master("br", 900.0, 75.0),
            master("tl", 100.0, 125.0),
            master("tr", 900.0, 125.0),
        ];
        let (indices, factors) = find_bilinear_masters(&masters, "wght", "wdth", 500.0, 100.0)
            .expect("complete rectangle");
        assert_eq!(indices, [0, 1, 2, 3]);
        assert_eq!(factors, (0.5, 0.5));
    }

    #[test]
    fn rejects_incomplete_two_axis_master_rectangle() {
        let masters = vec![
            FontMaster {
                id: "bl".into(),
                name: "bl".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 100.0), ("wdth".into(), 75.0)]
                    .into_iter()
                    .collect(),
            },
            FontMaster {
                id: "br".into(),
                name: "br".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 900.0), ("wdth".into(), 75.0)]
                    .into_iter()
                    .collect(),
            },
            FontMaster {
                id: "tl".into(),
                name: "tl".into(),
                weight: 0.0,
                width: 0.0,
                is_bracket: false,
                axes: [("wght".into(), 100.0), ("wdth".into(), 125.0)]
                    .into_iter()
                    .collect(),
            },
        ];
        assert!(find_bilinear_masters(&masters, "wght", "wdth", 500.0, 100.0).is_none());
    }

    #[test]
    fn set_width_for_glyphs_updates_existing_names_only() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let names = vec!["A".into(), "missing".into(), "B".into()];
        assert_eq!(project.set_width_for_glyphs(&names, 720.0), 2);
        assert_eq!(project.glyphs["A"].width, 720.0);
        assert_eq!(project.glyphs["A"].layers["regular"].width, 720.0);
        assert_eq!(project.glyphs["B"].width, 720.0);
        assert_eq!(project.set_width_for_glyphs(&names, -1.0), 0);
    }

    #[test]
    fn unicode_assignments_remove_conflicting_primary_and_alias_values() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), None);
        project.glyphs.get_mut("B").unwrap().unicodes = vec![65, 66];
        assert_eq!(project.set_unicode_assignments(&[("B".into(), 65)]), 2);
        assert_eq!(project.glyphs["A"].unicode, None);
        assert_eq!(project.glyphs["B"].unicode, Some(65));
        assert_eq!(project.glyphs["B"].unicodes, vec![66]);
    }

    #[test]
    fn vertical_metrics_are_stored_per_glyph_with_defaults() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        let defaults = project.vertical_metrics_for_glyph("A");
        assert_eq!(defaults.advance_height, 1000.0);
        assert_eq!(defaults.top_side_bearing, 800.0);
        project.set_vertical_metrics("A", 1200.0, 700.0).unwrap();
        assert_eq!(
            project.vertical_metrics_for_glyph("A").advance_height,
            1200.0
        );
        assert!(project
            .set_vertical_metrics("missing", 1000.0, 0.0)
            .is_err());
    }

    #[test]
    fn master_vertical_metrics_override_legacy_glyph_value() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.set_vertical_metrics("A", 1000.0, 800.0).unwrap();
        project
            .set_vertical_metrics_for_master("A", "bold", 1200.0, 640.0)
            .unwrap();
        assert_eq!(
            project
                .vertical_metrics_for_glyph_in_master("A", "regular")
                .advance_height,
            1000.0
        );
        assert_eq!(
            project
                .vertical_metrics_for_glyph_in_master("A", "bold")
                .top_side_bearing,
            640.0
        );
    }

    #[test]
    fn translate_glyphs_moves_outline_and_anchors() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(10.0, 20.0)],
        });
        glyph.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 5.0,
            y: 6.0,
        });
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.translate_glyphs(&["A".into()], 12.0, -3.0), 1);
        assert_eq!(project.glyphs["A"].contours[0].points[0].x, 22.0);
        assert_eq!(project.glyphs["A"].anchors[0].y, 3.0);
    }

    #[test]
    fn fit_widths_to_outlines_uses_each_glyph_bounds() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 900.0;
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(430.0, 20.0)],
        });
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.fit_widths_to_outlines(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].width, 430.0);
    }

    #[test]
    fn fit_widths_to_outlines_updates_every_master_layer() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 900.0;
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(430.0, 20.0)],
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 1100.0,
                contours: vec![Contour {
                    points: vec![ContourPoint::on_curve(520.0, 20.0)],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);

        assert_eq!(project.fit_widths_to_outlines(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].width, 430.0);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 520.0);
    }

    #[test]
    fn center_glyphs_in_width_centers_each_master_independently() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 200.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(10.0, 0.0),
                ContourPoint::on_curve(110.0, 0.0),
                ContourPoint::on_curve(110.0, 100.0),
            ],
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 400.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(50.0, 0.0),
                        ContourPoint::on_curve(150.0, 0.0),
                        ContourPoint::on_curve(150.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);

        assert_eq!(project.center_glyphs_in_width(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].contours[0].points[0].x, 50.0);
        assert_eq!(
            project.glyphs["A"].layers["bold"].contours[0].points[0].x,
            150.0
        );
    }

    #[test]
    fn split_quadratic_preserves_curve_shape() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
            ],
        };
        let index = contour.split_segment(0, 0.5).unwrap();
        assert_eq!(index, 2);
        assert_eq!(contour.points.len(), 5);
        assert!((contour.points[index].x - 50.0).abs() < 1e-9);
        assert!((contour.points[index].y - 50.0).abs() < 1e-9);
        assert!(!contour.points[1].is_on_curve());
        assert!(!contour.points[3].is_on_curve());
    }

    #[test]
    fn reverse_direction_preserves_node_data_in_reverse_order() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(1.0, 2.0),
                ContourPoint::off_curve(3.0, 4.0),
                ContourPoint::on_curve(5.0, 6.0),
            ],
        };
        let original = contour.points.clone();
        contour.reverse_direction();
        assert_eq!(
            contour.points,
            original.into_iter().rev().collect::<Vec<_>>()
        );
    }

    #[test]
    fn reverse_contour_updates_authored_geometry_and_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour.clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.reverse_contour_all_layers(0).unwrap();
        let expected: Vec<_> = contour.points.into_iter().rev().collect();
        assert_eq!(glyph.contours[0].points, expected);
        assert_eq!(glyph.layers["regular"].contours[0].points, expected);
    }

    #[test]
    fn normalize_contour_directions_updates_every_layer() {
        let clockwise = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![clockwise.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![clockwise],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let mut project = FontProject::new();
        project.glyphs.insert(glyph.name.clone(), glyph);
        assert_eq!(project.normalize_glyph_winding(&["A".into()]), 1);
        let glyph = &project.glyphs["A"];
        assert!(glyph.contours[0].signed_area() <= 0.0);
        assert!(glyph.layers["regular"].contours[0].signed_area() <= 0.0);
    }

    #[test]
    fn normalize_glyph_winding_keeps_counter_direction_opposite() {
        let triangle = |offset: f64, size: f64| Contour {
            points: vec![
                ContourPoint::on_curve(offset, offset),
                ContourPoint::on_curve(offset + size, offset),
                ContourPoint::on_curve(offset, offset + size),
            ],
        };
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![triangle(0.0, 200.0), triangle(50.0, 50.0)];
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.normalize_glyph_winding(&["A".into()]), 1);
        let contours = &project.glyphs["A"].contours;
        assert!(contours[0].signed_area() * contours[1].signed_area() < 0.0);
    }

    #[test]
    fn toggle_curve_nodes_updates_authored_geometry_and_all_layers() {
        let point = ContourPoint::off_curve(0.0, 0.0);
        let contour = Contour {
            points: vec![
                point,
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).unwrap();
        assert!(glyph.contours[0].points[0].is_on_curve());
        assert!(glyph.layers["regular"].contours[0].points[0].is_on_curve());
    }

    #[test]
    fn toggle_curve_nodes_rejects_invalid_topology_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour.clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.clone();
        assert!(glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).is_err());
        assert_eq!(glyph, before);
    }

    #[test]
    fn toggle_curve_nodes_ignores_unselected_invalid_contours() {
        let valid = Contour {
            points: vec![
                ContourPoint::off_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let invalid = Contour {
            points: vec![ContourPoint::on_curve(10.0, 10.0)],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![valid, invalid];
        glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).unwrap();
        assert!(glyph.contours[0].points[0].is_on_curve());
        assert_eq!(glyph.contours[1].points.len(), 1);
    }

    #[test]
    fn set_smooth_nodes_updates_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.set_smooth_nodes_all_layers(&[(0, 0)], true).unwrap();
        assert!(glyph.contours[0].points[0].smooth);
        assert!(glyph.layers["regular"].contours[0].points[0].smooth);
    }

    #[test]
    fn set_smooth_nodes_rejects_layer_structure_mismatch_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![Contour { points: vec![] }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.clone();
        assert!(glyph.set_smooth_nodes_all_layers(&[(0, 0)], true).is_err());
        assert_eq!(glyph, before);
    }

    #[test]
    fn knife_split_is_applied_to_all_layers_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.split_segment_all_layers(0, 0, 0.5).unwrap();
        glyph.split_segment_all_layers(0, 2, 0.5).unwrap();
        glyph.cut_contour_all_layers(0, 1, 3).unwrap();
        assert_eq!(glyph.contours.len(), 2);
        assert_eq!(glyph.layers["bold"].contours.len(), 2);
    }

    #[test]
    fn translate_nodes_and_components_can_sync_all_layers_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.components.push(GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: glyph.components.clone(),
                anchors: Vec::new(),
            },
        );
        glyph
            .translate_nodes_all_layers(&[(0, 1)], 12.0, -4.0)
            .unwrap();
        glyph.translate_component_all_layers(0, 5.0, 7.0).unwrap();
        assert_eq!(glyph.contours[0].points[1].x, 112.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 112.0);
        assert_eq!(glyph.components[0].x_offset, 15.0);
        assert_eq!(glyph.layers["bold"].components[0].y_offset, 27.0);
    }

    #[test]
    fn transform_nodes_syncs_each_master_around_its_own_selection_center() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph
            .transform_nodes_all_layers(&[(0, 0), (0, 1)], 2.0, 0.0)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, -50.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 150.0);
    }

    #[test]
    fn reflect_nodes_syncs_bounds_to_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph
            .reflect_nodes_all_layers(&[(0, 0), (0, 1)], true)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, 100.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 0.0);
    }

    #[test]
    fn align_and_distribute_nodes_sync_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 10.0),
                ContourPoint::on_curve(40.0, 30.0),
                ContourPoint::on_curve(100.0, 50.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph
            .align_nodes_all_layers(&[(0, 0), (0, 1)], true)
            .unwrap();
        glyph
            .distribute_nodes_all_layers(&[(0, 0), (0, 1), (0, 2)], true)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, 0.0);
        assert_eq!(glyph.contours[0].points[1].x, 50.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 50.0);
    }

    #[test]
    fn component_transform_and_reflection_sync_all_layers() {
        let component = GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.components = vec![component.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: Vec::new(),
                components: vec![component],
                anchors: Vec::new(),
            },
        );
        glyph.transform_component_all_layers(0, 2.0, 0.0).unwrap();
        glyph.reflect_component_all_layers(0, true).unwrap();
        assert_eq!(glyph.components[0].x_scale, -2.0);
        assert_eq!(glyph.layers["bold"].components[0].x_scale, -2.0);
    }

    #[test]
    fn remove_contour_updates_authored_geometry_and_all_layers() {
        let contour = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 10.0, 0.0),
                ContourPoint::on_curve(x, 10.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour(0.0), contour(100.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.remove_contour_all_layers(0).unwrap();
        assert_eq!(glyph.contours.len(), 1);
        assert_eq!(glyph.layers["regular"].contours.len(), 1);
        assert_eq!(glyph.contours[0].points[0].x, 100.0);
    }

    #[test]
    fn remove_contour_rejects_layer_structure_mismatch_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(10.0, 0.0),
                ContourPoint::on_curve(0.0, 10.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.clone();
        assert!(glyph.remove_contour_all_layers(0).is_err());
        assert_eq!(glyph, before);
    }

    #[test]
    fn set_smooth_aligns_adjacent_handles() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(40.0, 30.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::off_curve(60.0, 130.0),
            ],
        };
        contour.set_smooth(2, true);
        let before = contour.points[1];
        let after = contour.points[3];
        let incoming = (before.x - 100.0, before.y - 100.0);
        let outgoing = (after.x - 100.0, after.y - 100.0);
        assert!((incoming.0 * outgoing.1 - incoming.1 * outgoing.0).abs() < 1e-9);
        assert!(incoming.0 * outgoing.0 + incoming.1 * outgoing.1 < 0.0);
    }

    #[test]
    fn translate_smooth_point_moves_adjacent_handles_together() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        contour.set_smooth(2, true);
        let before = contour.points[1];
        let after = contour.points[3];
        contour.translate_point(2, 12.0, -7.0);
        assert_eq!(contour.points[2].x, 112.0);
        assert_eq!(contour.points[2].y, -7.0);
        assert_eq!(contour.points[1].x, before.x + 12.0);
        assert_eq!(contour.points[1].y, before.y - 7.0);
        assert_eq!(contour.points[3].x, after.x + 12.0);
        assert_eq!(contour.points[3].y, after.y - 7.0);
    }

    #[test]
    fn translate_points_does_not_move_selected_smooth_handles_twice() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        contour.set_smooth(2, true);
        let before: Vec<(f64, f64)> = contour
            .points
            .iter()
            .map(|point| (point.x, point.y))
            .collect();
        contour.translate_points(&[0, 1, 2, 3], 12.0, -7.0);
        for (point, (x, y)) in contour.points.iter().zip(before) {
            assert_eq!(point.x, x + 12.0);
            assert_eq!(point.y, y - 7.0);
        }
    }

    #[test]
    fn translate_points_repairs_a_moved_smooth_handle() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 20.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::off_curve(60.0, 130.0),
            ],
        };
        contour.set_smooth(2, true);
        contour.translate_points(&[3], 10.0, -20.0);
        let incoming = (contour.points[1].x - 100.0, contour.points[1].y - 100.0);
        let outgoing = (contour.points[3].x - 100.0, contour.points[3].y - 100.0);
        assert!((incoming.0 * outgoing.1 - incoming.1 * outgoing.0).abs() < 1e-9);
        assert!(incoming.0 * outgoing.0 + incoming.1 * outgoing.1 < 0.0);
    }

    #[test]
    fn translate_nodes_all_layers_moves_selected_smooth_geometry_once() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.contours[0].set_smooth(2, true);
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![glyph.contours[0].clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.contours[0].points.clone();
        glyph
            .translate_nodes_all_layers(&[(0, 0), (0, 1), (0, 2), (0, 3)], 12.0, -7.0)
            .unwrap();
        for points in [
            &glyph.contours[0].points,
            &glyph.layers["bold"].contours[0].points,
        ] {
            for (point, original) in points.iter().zip(&before) {
                assert_eq!(point.x, original.x + 12.0);
                assert_eq!(point.y, original.y - 7.0);
            }
        }
    }

    #[test]
    fn translate_smooth_handle_keeps_opposite_handle_collinear() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 20.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::off_curve(60.0, 130.0),
            ],
        };
        contour.set_smooth(2, true);
        let opposite_length = {
            let point = contour.points[1];
            ((point.x - 100.0).powi(2) + (point.y - 100.0).powi(2)).sqrt()
        };
        contour.translate_point(3, 10.0, -20.0);
        let incoming = (contour.points[1].x - 100.0, contour.points[1].y - 100.0);
        let outgoing = (contour.points[3].x - 100.0, contour.points[3].y - 100.0);
        assert!((incoming.0 * outgoing.1 - incoming.1 * outgoing.0).abs() < 1e-9);
        assert!(incoming.0 * outgoing.0 + incoming.1 * outgoing.1 < 0.0);
        let actual_length = (incoming.0.powi(2) + incoming.1.powi(2)).sqrt();
        assert!((actual_length - opposite_length).abs() < 1e-9);
    }

    #[test]
    fn cut_at_nodes_preserves_both_closed_paths() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let (first, second) = contour.cut_at_nodes(0, 2).unwrap();
        assert_eq!(first.points.len(), 3);
        assert_eq!(second.points.len(), 3);
        assert_eq!(first.points.first().unwrap().x, 0.0);
        assert_eq!(first.points.last().unwrap().x, 100.0);
        assert_eq!(second.points.first().unwrap().x, 100.0);
        assert_eq!(second.points.last().unwrap().x, 0.0);
    }

    #[test]
    fn signed_area_uses_the_flattened_bezier_shape() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 120.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, -100.0),
                ContourPoint::on_curve(0.0, -100.0),
            ],
        };
        assert!(contour.signed_area() < 0.0);
    }

    #[test]
    fn glyph_order_is_preserved_and_recovers_missing_entries() {
        let mut project = FontProject::new();
        project.add_glyph("z".into(), None);
        project.add_glyph("a".into(), None);
        project.add_glyph("m".into(), None);
        project.move_glyph("m", -2);
        assert_eq!(project.glyph_names_sorted(), vec!["m", "z", "a"]);
        project.remove_glyph("z");
        assert_eq!(project.glyph_names_sorted(), vec!["m", "a"]);
        project
            .glyphs
            .insert("b".into(), GlyphData::new("b".into(), None));
        assert_eq!(project.glyph_names_sorted(), vec!["m", "a", "b"]);
    }

    #[test]
    fn duplicate_glyph_copies_layers_and_clears_unicode() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.vertical_metrics.insert(
            "A".into(),
            VerticalMetrics {
                advance_height: 1100.0,
                top_side_bearing: 700.0,
            },
        );
        project.color_layers.insert(
            "A".into(),
            vec![ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .background_images
            .entry("A".into())
            .or_default()
            .insert("regular".into(), "/tmp/A.png".into());
        project
            .background_opacities
            .entry("A".into())
            .or_default()
            .insert("regular".into(), 0.5);
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour::new()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let name = project.duplicate_glyph("A").unwrap();
        assert_eq!(name, "A.copy1");
        assert_eq!(project.glyphs[&name].unicode, None);
        assert!(project.glyphs[&name].layers.contains_key("regular"));
        assert!(project.vertical_metrics.contains_key(&name));
        assert!(project.color_layers.contains_key(&name));
        assert_eq!(project.background_images[&name]["regular"], "/tmp/A.png");
        assert_eq!(project.background_opacities[&name]["regular"], 0.5);
        assert!(project.duplicate_glyph("missing").is_none());
    }

    #[test]
    fn rename_glyph_updates_references_and_kerning() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("mark".into(), None);
        project
            .glyphs
            .get_mut("mark")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        project
            .kerning
            .insert(("base".into(), "mark".into()), -40.0);
        project.vertical_metrics.insert(
            "base".into(),
            VerticalMetrics {
                advance_height: 1100.0,
                top_side_bearing: 700.0,
            },
        );
        project.color_layers.insert(
            "mark".into(),
            vec![ColorLayer {
                glyph: "base".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .background_images
            .entry("base".into())
            .or_default()
            .insert("regular".into(), "/tmp/base.png".into());
        project
            .unicode_variation_sequences
            .push(UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "base".into(),
            });
        project.opentype_features = "feature liga { sub base by mark; } liga;".into();
        assert!(project.rename_glyph("base", "renamed".into()));
        assert!(project.glyphs.contains_key("renamed"));
        assert_eq!(project.glyphs["mark"].components[0].base, "renamed");
        assert_eq!(
            project.kerning.get(&("renamed".into(), "mark".into())),
            Some(&-40.0)
        );
        assert!(project.opentype_features.contains("sub renamed by mark"));
        assert!(project.vertical_metrics.contains_key("renamed"));
        assert_eq!(
            project.background_images["renamed"]["regular"],
            "/tmp/base.png"
        );
        assert_eq!(project.unicode_variation_sequences[0].glyph, "renamed");
        assert_eq!(project.color_layers["mark"][0].glyph, "renamed");
        project.add_glyph("liga".into(), None);
        project.opentype_features = "feature liga { sub liga by mark; } liga;".into();
        assert!(project.rename_glyph("liga", "ligature".into()));
        assert_eq!(
            project.opentype_features,
            "feature liga { sub ligature by mark; } liga;"
        );
    }

    #[test]
    fn remove_glyph_cleans_references_and_kerning() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("mark".into(), None);
        project
            .glyphs
            .get_mut("mark")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        project
            .kerning
            .insert(("base".into(), "mark".into()), -40.0);
        project.vertical_metrics.insert(
            "base".into(),
            VerticalMetrics {
                advance_height: 1100.0,
                top_side_bearing: 700.0,
            },
        );
        project.color_layers.insert(
            "mark".into(),
            vec![ColorLayer {
                glyph: "base".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .background_images
            .entry("base".into())
            .or_default()
            .insert("regular".into(), "/tmp/base.png".into());
        project
            .unicode_variation_sequences
            .push(UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "base".into(),
            });
        project.opentype_features = "feature liga { sub base by mark; } liga;".into();
        project.remove_glyph("base");
        assert!(project.kerning.is_empty());
        assert!(project.glyphs["mark"].components.is_empty());
        assert!(!project.vertical_metrics.contains_key("base"));
        assert!(!project.background_images.contains_key("base"));
        assert!(!project.color_layers.contains_key("mark"));
        assert!(project.unicode_variation_sequences.is_empty());
        assert!(project.opentype_features.contains("sub .notdef by mark"));
    }

    #[test]
    fn kerning_source_prefers_exception_over_group_value() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.left_kerning_group = "latin-left".into();
        let mut a_alt = GlyphData::new("A.alt".into(), None);
        a_alt.left_kerning_group = "latin-left".into();
        let mut v = GlyphData::new("V".into(), None);
        v.right_kerning_group = "latin-right".into();
        let mut v_alt = GlyphData::new("V.alt".into(), None);
        v_alt.right_kerning_group = "latin-right".into();
        let mut a_z = GlyphData::new("A.z".into(), None);
        a_z.left_kerning_group = "latin-left".into();
        let mut v_z = GlyphData::new("V.z".into(), None);
        v_z.right_kerning_group = "latin-right".into();
        project.glyphs.extend([
            ("A".into(), a),
            ("A.alt".into(), a_alt),
            ("V".into(), v),
            ("V.alt".into(), v_alt),
            ("A.z".into(), a_z),
            ("V.z".into(), v_z),
        ]);
        project.kerning.insert(("A".into(), "V".into()), -80.0);
        project.kerning.insert(("A.z".into(), "V.z".into()), -60.0);
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(-80.0));
        assert_eq!(
            project.kerning_source_for_glyphs("A.alt", "V.alt"),
            Some((("A".into(), "V".into()), -80.0))
        );
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), -120.0);
        assert_eq!(
            project.kerning_source_for_glyphs("A.alt", "V.alt"),
            Some((("A.alt".into(), "V.alt".into()), -120.0))
        );
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), 0.0);
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(0.0));
        assert_eq!(project.kerning_for_glyphs("A", "missing"), None);
        project
            .glyphs
            .get_mut("V.alt")
            .unwrap()
            .right_kerning_group
            .clear();
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(0.0));
        project.kerning.remove(&("A.alt".into(), "V.alt".into()));
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), None);
    }

    #[test]
    fn kerning_is_independent_per_master_and_survives_switching() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("V".into(), Some(86));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project.set_kerning_pair("A", "V", -60.0).unwrap();
        project.sync_active_layer("regular");
        project.switch_master("regular", "bold");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-60.0));
        project.set_kerning_pair("A", "V", -120.0).unwrap();
        project.sync_active_layer("bold");
        project.switch_master("bold", "regular");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-60.0));
        project.switch_master("regular", "bold");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-120.0));
    }

    #[test]
    fn master_kerning_api_updates_only_requested_master() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("V".into(), Some(86));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project
            .set_kerning_pair_for_master("bold", "A", "V", -100.0)
            .unwrap();
        assert!(!project.kerning.contains_key(&("A".into(), "V".into())));
        assert_eq!(
            project.kerning_by_master["bold"][&("A".into(), "V".into())],
            -100.0
        );
        assert!(project
            .set_kerning_pair_for_master("missing", "A", "V", -20.0)
            .is_err());
    }

    #[test]
    fn feature_source_merges_classes_before_features() {
        let mut project = FontProject::new();
        project.opentype_classes = "@Upper = [A B];".into();
        project.opentype_features = "feature ccmp { sub @Upper by A; } ccmp;".into();
        assert_eq!(
            project.feature_source(),
            "@Upper = [A B];\n\nfeature ccmp { sub @Upper by A; } ccmp;"
        );
    }

    #[test]
    fn batch_spacing_and_kerning_reject_invalid_rows_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.add_glyph("B".into(), None);

        assert!(project
            .set_kerning_pairs([("A", "B", -40.0), ("A", "missing", f64::NAN)])
            .is_err());
        assert!(project.kerning.is_empty());

        assert!(project
            .set_side_bearings_batch([("A", 20.0, 20.0), ("missing", 20.0, 20.0)])
            .is_err());
        assert_eq!(project.glyphs["A"].width, 600.0);

        assert!(project
            .set_widths_batch([("A", 500.0), ("missing", 600.0)])
            .is_err());
        assert_eq!(project.glyphs["A"].width, 600.0);

        assert!(project
            .set_unicode_assignments_strict(&[("A".into(), 0x41), ("B".into(), 0x41)])
            .is_err());
        assert_eq!(project.glyphs["A"].unicode, None);
    }

    #[test]
    fn set_kerning_groups_updates_multiple_glyphs_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.add_glyph("A.alt".into(), None);
        assert_eq!(
            project
                .set_kerning_groups(&["A".into(), "A.alt".into()], "upper-left", "upper-right",)
                .unwrap(),
            2
        );
        assert_eq!(project.glyphs["A"].left_kerning_group, "upper-left");
        assert!(project
            .set_kerning_groups(&["A".into(), "missing".into()], "x", "y")
            .is_err());
        assert_eq!(project.glyphs["A"].left_kerning_group, "upper-left");
    }

    #[test]
    fn conditional_layer_selection_prefers_most_specific_match() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        let layer = |id: &str, conditions| ConditionalLayer {
            id: id.into(),
            conditions,
            layer: GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![
                layer(
                    "weight",
                    HashMap::from([(
                        "wght".into(),
                        AxisRange {
                            min: Some(700.0),
                            max: None,
                        },
                    )]),
                ),
                layer(
                    "weight-width",
                    HashMap::from([
                        (
                            "wght".into(),
                            AxisRange {
                                min: Some(700.0),
                                max: None,
                            },
                        ),
                        (
                            "wdth".into(),
                            AxisRange {
                                min: Some(90.0),
                                max: Some(110.0),
                            },
                        ),
                        (
                            "opsz".into(),
                            AxisRange {
                                min: Some(12.0),
                                max: Some(18.0),
                            },
                        ),
                        (
                            "GRAD".into(),
                            AxisRange {
                                min: Some(0.0),
                                max: Some(100.0),
                            },
                        ),
                        (
                            "slnt".into(),
                            AxisRange {
                                min: Some(-15.0),
                                max: Some(0.0),
                            },
                        ),
                    ]),
                ),
            ],
        );
        let coordinates = HashMap::from([
            ("wght".into(), 750.0),
            ("wdth".into(), 100.0),
            ("opsz".into(), 14.0),
            ("GRAD".into(), 50.0),
            ("slnt".into(), -10.0),
        ]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "weight-width"
        );
    }

    #[test]
    fn conditional_layer_selection_prefers_narrower_overlapping_range() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        let layer = |id: &str, min: f64, max: f64| ConditionalLayer {
            id: id.into(),
            conditions: HashMap::from([(
                "wght".into(),
                AxisRange {
                    min: Some(min),
                    max: Some(max),
                },
            )]),
            layer: GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        };
        project.conditional_layers.insert(
            "A".into(),
            vec![layer("wide", 600.0, 900.0), layer("narrow", 700.0, 800.0)],
        );
        let coordinates = HashMap::from([("wght".into(), 750.0)]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "narrow"
        );
    }

    #[test]
    fn conditional_layer_selection_matches_axis_tags_case_insensitively() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.conditional_layers.insert(
            "A".into(),
            vec![ConditionalLayer {
                id: "uppercase-axis".into(),
                conditions: HashMap::from([(
                    "WGHT".into(),
                    AxisRange {
                        min: Some(700.0),
                        max: None,
                    },
                )]),
                layer: GlyphLayer {
                    width: 600.0,
                    contours: Vec::new(),
                    components: Vec::new(),
                    anchors: Vec::new(),
                },
            }],
        );
        let coordinates = HashMap::from([("wght".into(), 750.0)]);
        assert_eq!(
            project
                .conditional_layer_for_glyph("A", &coordinates)
                .unwrap()
                .id,
            "uppercase-axis"
        );
    }

    #[test]
    fn component_anchors_are_transformed_when_inherited() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 100.0,
            y: 200.0,
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.5,
            yx_scale: 0.0,
            y_scale: 3.0,
            x_offset: 10.0,
            y_offset: -20.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.anchors_for_glyph("composite"),
            vec![GlyphAnchor {
                name: "top".into(),
                x: 310.0,
                y: 580.0,
            }]
        );
        let mut accent = GlyphData::new("accent".into(), None);
        accent.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 0.0,
            y: 0.0,
        });
        project.glyphs.insert("accent".into(), accent);
        let mut accented = GlyphData::new("accented".into(), None);
        accented.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        accented.components.push(GlyphComponent {
            base: "accent".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 200.0,
        });
        project.glyphs.insert("accented".into(), accented);
        assert_eq!(project.anchors_for_glyph("accented").len(), 1);
        assert_eq!(project.anchors_for_glyph("accented")[0].name, "top");
    }

    #[test]
    fn outline_bounds_include_transformed_component_geometry() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 200.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 3.0,
            x_offset: 50.0,
            y_offset: -20.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.outline_bounds_for_glyph("composite"),
            Some((50.0, -20.0, 250.0, 580.0))
        );
    }

    #[test]
    fn explicit_component_anchor_name_wins_over_inherited_duplicate() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 10.0,
            y: 20.0,
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 100.0,
            y: 200.0,
        });
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.anchors_for_glyph("composite"),
            vec![GlyphAnchor {
                name: "top".into(),
                x: 100.0,
                y: 200.0
            }]
        );
    }

    #[test]
    fn sync_active_layer_persists_geometry_for_export() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().width = 777.0;
        project.sync_active_layer("regular");
        assert_eq!(project.glyphs["A"].layers["regular"].width, 777.0);
    }

    #[test]
    fn copy_master_to_all_copies_layer_geometry_without_metadata_changes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.glyphs.get_mut("A").unwrap().width = 812.0;
        project.sync_active_layer("regular");
        let copied = project.copy_master_to_all("regular");
        assert_eq!(copied, 1);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert_eq!(project.glyphs["A"].unicode, Some('A' as u32));
    }

    #[test]
    fn master_guidelines_follow_switch_and_remain_independent() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 100.0,
                y: 200.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].guidelines_for_master("bold").len(), 1);
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("bold")
            .push(Guideline {
                x: 120.0,
                y: 300.0,
                angle: 90.0,
                name: "bold-cap".into(),
            });
        project.sync_active_layer("bold");
        project.switch_master("bold", "regular");
        assert_eq!(project.glyphs["A"].guidelines.len(), 1);
        assert_eq!(project.glyphs["A"].guidelines[0].x, 100.0);
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].guidelines.len(), 2);
        assert_eq!(project.glyphs["A"].guidelines[1].x, 120.0);
    }

    #[test]
    fn master_guidelines_translate_with_geometry() {
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.guidelines.push(Guideline {
            x: 10.0,
            y: 20.0,
            angle: 0.0,
            name: String::new(),
        });
        glyph.master_guidelines.insert(
            "bold".into(),
            vec![Guideline {
                x: 30.0,
                y: 40.0,
                angle: 90.0,
                name: String::new(),
            }],
        );
        glyph.translate_geometry(5.0, -7.0);
        assert_eq!(glyph.guidelines[0].x, 15.0);
        assert_eq!(glyph.guidelines[0].y, 13.0);
        assert_eq!(glyph.guidelines_for_master("bold")[0].x, 35.0);
        assert_eq!(glyph.guidelines_for_master("bold")[0].y, 33.0);
    }

    #[test]
    fn sync_active_layer_preserves_global_guides_edited_through_master_view() {
        let mut project = FontProject::new();
        project
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 0.0,
                y: 680.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        assert_eq!(project.guidelines.len(), 1);
        assert_eq!(project.guidelines_for_master("regular")[0].y, 680.0);
    }

    #[test]
    fn sync_active_layer_preserves_guides_edited_through_master_view() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 80.0,
                y: 650.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.guidelines.len(), 1);
        assert_eq!(glyph.guidelines_for_master("regular")[0].y, 650.0);
    }

    #[test]
    fn copy_master_to_all_for_glyphs_only_copies_requested_names() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.glyphs.get_mut("A").unwrap().width = 812.0;
        project.glyphs.get_mut("B").unwrap().width = 913.0;
        project.sync_active_layer("regular");
        let copied = project.copy_master_to_all_for_glyphs("regular", ["A"]);
        assert_eq!(copied, 1);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert!(!project.glyphs["B"].layers.contains_key("bold"));
    }

    #[test]
    fn subset_master_copy_does_not_overwrite_global_master_data() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), Some(66));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project.kerning_by_master.insert(
            "regular".into(),
            HashMap::from([(("A".into(), "V".into()), -80.0)]),
        );
        project.kerning_by_master.insert(
            "bold".into(),
            HashMap::from([(("A".into(), "V".into()), -120.0)]),
        );
        project.guidelines_by_master.insert(
            "regular".into(),
            vec![Guideline {
                x: 0.0,
                y: 700.0,
                angle: 0.0,
                name: "regular".into(),
            }],
        );
        project.guidelines_by_master.insert(
            "bold".into(),
            vec![Guideline {
                x: 0.0,
                y: 720.0,
                angle: 0.0,
                name: "bold".into(),
            }],
        );
        project.copy_master_to_all_for_glyphs("regular", ["A"]);
        assert_eq!(
            project.kerning_by_master["bold"][&("A".into(), "V".into())],
            -120.0
        );
        assert_eq!(project.guidelines_by_master["bold"][0].y, 720.0);
    }

    #[test]
    fn switch_master_initializes_new_layer_for_every_glyph() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.glyphs.get_mut("A").unwrap().width = 812.0;
        project.glyphs.get_mut("B").unwrap().width = 913.0;
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert_eq!(project.glyphs["B"].layers["bold"].width, 913.0);
    }

    #[test]
    fn remove_duplicate_nodes_cleans_authored_and_master_geometry() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
                ContourPoint::on_curve(0.0, 0.0),
            ],
        };
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.remove_duplicate_nodes(&["A".into()]), 4);
        assert_eq!(project.glyphs["A"].contours[0].points.len(), 3);
        assert_eq!(
            project.glyphs["A"].layers["regular"].contours[0]
                .points
                .len(),
            3
        );
        let mut fragile = GlyphData::new("fragile".into(), None);
        fragile.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(10.0, 10.0),
            ],
        });
        project.glyphs.insert("fragile".into(), fragile);
        assert_eq!(project.remove_duplicate_nodes(&["fragile".into()]), 0);
        assert_eq!(project.glyphs["fragile"].contours[0].points.len(), 3);
    }

    #[test]
    fn bezier_contour_converts_to_curve_boolean_shape() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let shape = contour.to_curve_shape().unwrap();
        assert_eq!(shape.len(), 1);
        assert_eq!(shape.segment_count(), 3);
        assert!(contour.difference(&contour).unwrap().is_empty());
    }

    #[test]
    fn quadratic_and_cubic_nodes_survive_boolean_conversion() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(125.0, -50.0),
                ContourPoint::off_curve(175.0, -50.0),
                ContourPoint::on_curve(200.0, 0.0),
            ],
        };
        let shape = contour.to_curve_shape().unwrap();
        assert_eq!(shape.segment_count(), 4);
        let shifted = Contour {
            points: contour
                .points
                .iter()
                .map(|point| ContourPoint {
                    x: point.x + 50.0,
                    y: point.y,
                    ..*point
                })
                .collect(),
        };
        let union = contour.union(&shifted).unwrap();
        assert!(!union.is_empty());
    }

    #[test]
    fn rational_arc_boolean_result_is_imported_as_cubic() {
        let arc = i_curve::float::arc::EllipticArc {
            ellipse: i_curve::float::arc::Ellipse {
                center: [0.0, 0.0],
                radius_x: 100.0,
                radius_y: 100.0,
                rotation: 0.0,
            },
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        };
        let mut builder = i_curve::CurveBuilder::new();
        builder.move_to([100.0, 0.0]).unwrap();
        builder.arc_to(arc).unwrap();
        builder.line_to([100.0, 0.0]).unwrap();
        builder.close_contour().unwrap();
        let path = builder.build().unwrap().into_contours().remove(0);
        let contour = Contour::from_curve_path(path).unwrap();

        assert_eq!(contour.points.len(), 4);
        assert!(contour.points[0].is_on_curve());
        assert_eq!(contour.points[1].point_type, PointType::OffCurve);
        assert_eq!(contour.points[2].point_type, PointType::OffCurve);
        assert!(contour.points[3].is_on_curve());
        assert!((contour.points[3].x - 0.0).abs() < 1.0e-9);
        assert!((contour.points[3].y - 100.0).abs() < 1.0e-9);
    }

    #[test]
    fn boolean_operations_preserve_overlapping_outline_results() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let left = rectangle(0.0);
        let right = rectangle(50.0);
        let union = left.union(&right).unwrap();
        assert!(!union.is_empty());
        assert!(union.iter().all(|contour| contour.points.len() >= 3));
        let difference = left.difference(&right).unwrap();
        assert!(!difference.is_empty());
        assert!(difference.iter().all(|contour| contour.points.len() >= 3));
        let intersection = left.intersection(&right).unwrap();
        assert!(!intersection.is_empty());
        assert!(intersection.iter().all(|contour| contour.points.len() >= 3));
        let xor = left.xor(&right).unwrap();
        assert!(!xor.is_empty());
        assert!(xor.iter().all(|contour| contour.points.len() >= 3));
    }

    #[test]
    fn union_all_contours_updates_authored_and_layers() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0), rectangle(1000.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.union_all_contours_all_layers().unwrap();
        assert_eq!(glyph.contours.len(), 2);
        assert_eq!(glyph.layers["regular"].contours.len(), 2);
    }

    #[test]
    fn boolean_layer_operations_update_every_master_atomically() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.union_contours_all_layers(0).unwrap();
        assert_eq!(glyph.contours.len(), glyph.layers["regular"].contours.len());
        let mut difference_glyph = glyph.clone();
        difference_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        difference_glyph.layers.get_mut("regular").unwrap().contours =
            difference_glyph.contours.clone();
        difference_glyph.difference_contours_all_layers(0).unwrap();
        assert_eq!(
            difference_glyph.contours.len(),
            difference_glyph.layers["regular"].contours.len()
        );
        let mut intersection_glyph = GlyphData::new("intersection".into(), None);
        intersection_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        intersection_glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: intersection_glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        intersection_glyph
            .intersection_contours_all_layers(0)
            .unwrap();
        assert_eq!(
            intersection_glyph.contours.len(),
            intersection_glyph.layers["regular"].contours.len()
        );
        let mut xor_glyph = GlyphData::new("xor".into(), None);
        xor_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        xor_glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: xor_glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        xor_glyph.xor_contours_all_layers(0).unwrap();
        assert_eq!(
            xor_glyph.contours.len(),
            xor_glyph.layers["regular"].contours.len()
        );
    }

    #[test]
    fn boolean_operations_do_not_partially_update_when_a_layer_fails() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![rectangle(0.0)],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.contours.clone();
        assert!(glyph.union_contours_all_layers(0).is_err());
        assert_eq!(glyph.contours, before);
    }

    #[test]
    fn normalize_glyph_winding_preserves_alternating_counter_direction() {
        let mut project = FontProject::new();
        project.add_glyph("O".into(), Some('O' as u32));
        let glyph = project.glyphs.get_mut("O").unwrap();
        glyph.contours = vec![
            Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            },
            Contour {
                points: vec![
                    ContourPoint::on_curve(25.0, 25.0),
                    ContourPoint::on_curve(25.0, 75.0),
                    ContourPoint::on_curve(75.0, 75.0),
                    ContourPoint::on_curve(75.0, 25.0),
                ],
            },
        ];
        project.normalize_glyph_winding(&["O".into()]);
        let contours = &project.glyphs["O"].contours;
        assert!(contours[0].signed_area() < 0.0);
        assert!(contours[1].signed_area() > 0.0);
    }

    #[test]
    fn set_side_bearings_updates_active_and_master_geometry() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(-20.0, 0.0),
                ContourPoint::on_curve(80.0, 0.0),
                ContourPoint::on_curve(80.0, 100.0),
            ],
        });
        glyph.width = 600.0;
        project.sync_active_layer("regular");
        assert_eq!(project.set_side_bearings(&["A".into()], 30.0, 40.0), 1);
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.width, 170.0);
        assert_eq!(glyph.contours[0].points[0].x, 30.0);
        assert_eq!(glyph.layers["regular"].width, 170.0);
        assert_eq!(glyph.layers["regular"].contours[0].points[0].x, 30.0);
    }

    #[test]
    fn duplicate_component_is_copied_to_every_master_layer() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.sync_active_layer("regular");
        let component = GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 40.0,
            y_offset: 0.0,
        };
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.components.push(component.clone());
        glyph
            .layers
            .get_mut("regular")
            .unwrap()
            .components
            .push(component.clone());
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: vec![component],
                anchors: Vec::new(),
            },
        );
        assert!(project.duplicate_component_all_layers("A", 0));
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.components.len(), 2);
        assert_eq!(glyph.layers["regular"].components.len(), 2);
        assert_eq!(glyph.layers["bold"].components.len(), 2);
        assert_eq!(glyph.layers["bold"].components[1].x_offset, 40.0);
        assert!(!project.duplicate_component_all_layers("A", 99));
        assert!(!project.duplicate_component_all_layers("missing", 0));
        assert_eq!(project.glyphs["A"].components.len(), 2);
        assert_eq!(project.glyphs["A"].layers["regular"].components.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].components.len(), 2);
    }

    #[test]
    fn component_lifecycle_keeps_master_indices_aligned() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let component = GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 12.0,
            y_offset: 24.0,
        };
        assert_eq!(
            project.add_component_all_layers("A", component.clone()),
            Some(0)
        );
        assert_eq!(
            project.glyphs["A"].layers["bold"].components,
            vec![component]
        );
        assert!(project.move_component_all_layers("A", 0, 1).is_err());
        assert!(project.remove_component_all_layers("A", 0).is_ok());
        assert!(project.glyphs["A"].components.is_empty());
        assert!(project.glyphs["A"].layers["bold"].components.is_empty());
    }

    #[test]
    fn contour_add_and_duplicate_keep_master_indices_aligned() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(10.0, 20.0),
                ContourPoint::on_curve(110.0, 20.0),
                ContourPoint::on_curve(10.0, 120.0),
            ],
        };
        assert_eq!(
            project.add_contour_all_layers("A", contour.clone()),
            Some(0)
        );
        assert_eq!(project.duplicate_contour_all_layers("A", 0), Some(1));
        assert_eq!(project.glyphs["A"].contours.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].contours.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].contours[0], contour);
        assert!(project.duplicate_contour_all_layers("A", 99).is_none());
    }

    #[test]
    fn remove_nodes_updates_every_master_layer_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .remove_nodes_all_layers(&[(0, 0)])
            .unwrap();
        assert_eq!(project.glyphs["A"].contours[0].points.len(), 3);
        assert_eq!(
            project.glyphs["A"].layers["bold"].contours[0].points.len(),
            3
        );
        assert!(project
            .glyphs
            .get_mut("A")
            .unwrap()
            .remove_nodes_all_layers(&[(0, 99)])
            .is_err());
    }

    #[test]
    fn set_side_bearings_uses_recursive_component_bounds() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("acute".into(), Some('Á' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        project
            .glyphs
            .get_mut("acute")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 200.0,
                y_offset: 0.0,
            });
        assert_eq!(project.set_side_bearings(&["acute".into()], 20.0, 30.0), 1);
        let glyph = &project.glyphs["acute"];
        assert_eq!(glyph.width, 150.0);
        assert_eq!(glyph.components[0].x_offset, 20.0);
    }

    #[test]
    fn set_side_bearings_uses_component_bounds_per_master() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("acute".into(), Some('Á' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        project.glyphs.get_mut("base").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 400.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(200.0, 0.0),
                        ContourPoint::on_curve(200.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project
            .glyphs
            .get_mut("acute")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        project.glyphs.get_mut("acute").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 400.0,
                contours: Vec::new(),
                components: vec![GlyphComponent {
                    base: "base".into(),
                    x_scale: 1.0,
                    xy_scale: 0.0,
                    yx_scale: 0.0,
                    y_scale: 1.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                }],
                anchors: Vec::new(),
            },
        );
        assert_eq!(project.set_side_bearings(&["acute".into()], 20.0, 30.0), 1);
        let glyph = &project.glyphs["acute"];
        assert_eq!(glyph.width, 150.0);
        assert_eq!(glyph.layers["bold"].width, 250.0);
    }

    #[test]
    fn set_side_bearings_rejects_invalid_values_without_changes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().width = 600.0;
        let before = project.clone();
        assert_eq!(project.set_side_bearings(&["A".into()], -1.0, 20.0), 0);
        assert_eq!(project, before);
        assert_eq!(project.set_side_bearings(&["A".into()], f64::NAN, 20.0), 0);
        assert_eq!(project, before);
    }

    #[test]
    fn metrics_keys_apply_reference_side_bearing() {
        let mut project = FontProject::new();
        let mut reference = GlyphData::new("H".into(), Some('H' as u32));
        reference.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(50.0, 0.0),
                ContourPoint::on_curve(300.0, 0.0),
                ContourPoint::on_curve(300.0, 700.0),
            ],
        });
        project.glyphs.insert("H".into(), reference);
        let mut target = GlyphData::new("A".into(), Some('A' as u32));
        target.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(200.0, 0.0),
                ContourPoint::on_curve(200.0, 700.0),
            ],
        });
        target.left_metrics_key = "=H".into();
        project.glyphs.insert("A".into(), target);
        assert_eq!(project.apply_metrics_keys(&["A".into()]).unwrap(), 1);
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.contours[0].points[0].x, 50.0);
        assert_eq!(glyph.width, 550.0);
        assert!(project.apply_metrics_keys(&["missing".into()]).is_err());
    }

    #[test]
    fn align_component_anchors_matches_transformed_mark_anchor() {
        let mut project = FontProject::new();
        let mut mark = GlyphData::new("mark".into(), None);
        mark.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 20.0,
            y: 30.0,
        });
        project.glyphs.insert("mark".into(), mark);
        let mut accented = GlyphData::new("accented".into(), None);
        accented.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 300.0,
            y: 700.0,
        });
        accented.components.push(GlyphComponent {
            base: "mark".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 2.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        project.glyphs.insert("accented".into(), accented);
        assert!(project.align_component_anchors("accented", 0));
        let aligned = &project.glyphs["accented"].components[0];
        assert_eq!((aligned.x_offset, aligned.y_offset), (260.0, 640.0));
    }

    #[test]
    fn align_component_anchors_leaves_unmatched_component_unchanged() {
        let mut project = FontProject::new();
        project
            .glyphs
            .insert("mark".into(), GlyphData::new("mark".into(), None));
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "mark".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 12.0,
            y_offset: 34.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert!(!project.align_component_anchors("composite", 0));
        let component = &project.glyphs["composite"].components[0];
        assert_eq!((component.x_offset, component.y_offset), (12.0, 34.0));
    }

    #[test]
    fn remove_orphaned_layers_keeps_valid_master_layers() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "deleted".into(),
            GlyphLayer {
                width: 700.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.remove_orphaned_layers(), 1);
        assert!(project.glyphs["A"].layers.contains_key("regular"));
        assert!(!project.glyphs["A"].layers.contains_key("deleted"));
    }

    #[test]
    fn masters_normalize_empty_and_duplicate_ids() {
        let mut project = FontProject::new();
        project.masters = vec![
            FontMaster {
                id: "regular".into(),
                name: "Regular".into(),
                weight: 400.0,
                width: 100.0,
                is_bracket: false,
                axes: [("wght".into(), 400.0)].into_iter().collect(),
            },
            FontMaster {
                id: "regular".into(),
                name: "Duplicate".into(),
                ..FontMaster::default()
            },
            FontMaster {
                id: " ".into(),
                name: "Invalid".into(),
                ..FontMaster::default()
            },
        ];
        project.normalize_masters();
        assert_eq!(project.masters.len(), 1);
        assert_eq!(project.masters[0].id, "regular");
    }

    #[test]
    fn remove_master_cleans_layers_metrics_and_background_data() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..FontMaster::default()
        });
        project.axis_names.insert("oldx".into(), "Old Axis".into());
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project
            .vertical_metrics_by_master
            .insert("bold".into(), HashMap::new());
        project.background_images.insert(
            "A".into(),
            HashMap::from([("bold".into(), "/tmp/A.png".into())]),
        );
        project
            .background_opacities
            .insert("A".into(), HashMap::from([("bold".into(), 0.5)]));
        project.background_transforms.insert(
            "A".into(),
            HashMap::from([(
                "bold".into(),
                BackgroundImageTransform {
                    x: 10.0,
                    y: 20.0,
                    scale: 1.0,
                    rotation: 5.0,
                    flip_x: false,
                    flip_y: false,
                },
            )]),
        );
        assert!(project.remove_master("bold"));
        assert_eq!(project.masters.len(), 1);
        assert_eq!(project.default_master_id, "regular");
        assert!(!project.glyphs["A"].layers.contains_key("bold"));
        assert!(project.vertical_metrics_by_master.is_empty());
        assert!(project.background_images.is_empty());
        assert!(project.background_opacities.is_empty());
        assert!(project.background_transforms.is_empty());
        assert!(project.axis_names.is_empty());
        project.switch_master("bold", "regular");
        assert!(!project.kerning_by_master.contains_key("bold"));
        assert!(!project.guidelines_by_master.contains_key("bold"));
        assert!(!project.glyphs["A"].layers.contains_key("bold"));
        assert!(!project.remove_master("regular"));
    }

    #[test]
    fn move_master_changes_order_without_touching_layer_data() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..FontMaster::default()
        });
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 777.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );

        assert!(project.move_master("bold", -1));
        assert_eq!(project.masters[0].id, "bold");
        assert_eq!(project.glyphs["A"].layers["bold"].width, 777.0);
        assert!(!project.move_master("bold", -1));
        assert!(!project.move_master("missing", 1));
    }

    #[test]
    fn duplicate_master_copies_metadata_and_every_glyph_layer() {
        let mut project = FontProject::new();
        project.masters[0].name = "Regular".into();
        project.masters[0].axes.insert("wght".into(), 400.0);
        project
            .glyphs
            .insert("A".into(), GlyphData::new("A".into(), None));
        project.kerning_by_master.insert(
            "regular".into(),
            HashMap::from([(("A".into(), "A".into()), -70.0)]),
        );
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 612.0,
                contours: vec![Contour {
                    points: vec![ContourPoint::on_curve(10.0, 20.0)],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );

        let new_id = project.duplicate_master("regular").unwrap();
        assert_eq!(new_id, "regular.copy1");
        assert_eq!(project.masters[1].id, new_id);
        assert_eq!(project.masters[1].name, "Regular Copy");
        assert_eq!(project.masters[1].axes["wght"], 400.0);
        assert_eq!(project.glyphs["A"].layers[&new_id].width, 612.0);
        assert_eq!(
            project.glyphs["A"].layers[&new_id].contours[0].points[0].x,
            10.0
        );
        assert_eq!(
            project.kerning_by_master[&new_id][&("A".into(), "A".into())],
            -70.0
        );
        assert_eq!(
            project.duplicate_master("regular").unwrap(),
            "regular.copy2"
        );
    }

    #[test]
    fn glyph_layers_interpolate_geometry_and_width() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: vec![GlyphAnchor {
                name: "top".into(),
                x: 100.0,
                y: 200.0,
            }],
        };
        let b = GlyphLayer {
            width: 700.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(100.0, 200.0)],
            }],
            components: Vec::new(),
            anchors: vec![GlyphAnchor {
                name: "top".into(),
                x: 300.0,
                y: 400.0,
            }],
        };
        let middle = a.interpolate(&b, 0.5).unwrap();
        assert_eq!(middle.width, 600.0);
        assert_eq!(middle.contours[0].points[0].x, 50.0);
        assert_eq!(middle.contours[0].points[0].y, 100.0);
        assert_eq!(middle.anchors.len(), 1);
        assert_eq!(middle.anchors[0].x, 200.0);
        assert_eq!(middle.anchors[0].y, 300.0);
    }

    #[test]
    fn glyph_layers_interpolate_bilinear_uses_both_axes() {
        let base = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let mut right = base.clone();
        right.width = 700.0;
        let mut top = base.clone();
        top.width = 900.0;
        let mut top_right = base.clone();
        top_right.width = 1100.0;
        let middle = base
            .interpolate_bilinear(&right, &top, &top_right, 0.5, 0.5)
            .unwrap();
        assert_eq!(middle.width, 800.0);
    }

    #[test]
    fn glyph_layers_match_anchors_by_name_not_storage_order() {
        let layer = |anchors: Vec<GlyphAnchor>| GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors,
        };
        let a = layer(vec![
            GlyphAnchor {
                name: "top".into(),
                x: 0.0,
                y: 100.0,
            },
            GlyphAnchor {
                name: "bottom".into(),
                x: 0.0,
                y: -100.0,
            },
        ]);
        let b = layer(vec![
            GlyphAnchor {
                name: "bottom".into(),
                x: 20.0,
                y: -80.0,
            },
            GlyphAnchor {
                name: "top".into(),
                x: 20.0,
                y: 120.0,
            },
        ]);
        let middle = a.interpolate(&b, 0.5).unwrap();
        assert_eq!(middle.anchors[0].name, "top");
        assert_eq!(middle.anchors[0].x, 10.0);
        assert_eq!(middle.anchors[1].name, "bottom");
    }

    #[test]
    fn glyph_layers_reject_topology_mismatch() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour::new()],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let b = GlyphLayer {
            width: 500.0,
            contours: Vec::new(),
            components: Vec::new(),
            anchors: Vec::new(),
        };
        assert!(a.interpolate(&b, 0.5).is_none());
    }

    #[test]
    fn glyph_layers_reject_on_off_curve_mismatch() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let b = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::off_curve(10.0, 10.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        assert!(a.interpolate(&b, 0.5).is_none());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphData {
    pub name: String,
    pub unicode: Option<u32>,
    #[serde(default)]
    pub unicodes: Vec<u32>,
    pub width: f64,
    #[serde(default)]
    pub left_kerning_group: String,
    #[serde(default)]
    pub right_kerning_group: String,
    /// Optional metric-key references such as `=H`. The left/right key uses
    /// the corresponding side bearing of the referenced glyph when applied.
    #[serde(default)]
    pub left_metrics_key: String,
    #[serde(default)]
    pub right_metrics_key: String,
    #[serde(default)]
    pub anchors: Vec<GlyphAnchor>,
    pub contours: Vec<Contour>,
    pub components: Vec<GlyphComponent>,
    #[serde(default)]
    pub layers: HashMap<String, GlyphLayer>,
    #[serde(default)]
    pub guidelines: Vec<Guideline>,
    /// Per-master glyph guides. `guidelines` remains the active-layer view
    /// for backwards-compatible callers and file formats.
    #[serde(default)]
    pub master_guidelines: HashMap<String, Vec<Guideline>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphAnchor {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphLayer {
    pub width: f64,
    pub contours: Vec<Contour>,
    pub components: Vec<GlyphComponent>,
    #[serde(default)]
    pub anchors: Vec<GlyphAnchor>,
}

impl GlyphLayer {
    pub fn translate_geometry(&mut self, dx: f64, dy: f64) {
        for contour in &mut self.contours {
            for point in &mut contour.points {
                point.x += dx;
                point.y += dy;
            }
        }
        for component in &mut self.components {
            component.x_offset += dx;
            component.y_offset += dy;
        }
        for anchor in &mut self.anchors {
            anchor.x += dx;
            anchor.y += dy;
        }
    }

    #[allow(dead_code)]
    pub fn interpolate(&self, other: &Self, factor: f64) -> Option<Self> {
        if self.contours.len() != other.contours.len()
            || self.components.len() != other.components.len()
            || self.anchors.len() != other.anchors.len()
            || self
                .anchors
                .iter()
                .any(|anchor| !other.anchors.iter().any(|item| item.name == anchor.name))
            || self.contours.iter().zip(&other.contours).any(|(a, b)| {
                a.points.len() != b.points.len()
                    || a.points
                        .iter()
                        .zip(&b.points)
                        .any(|(a, b)| a.point_type != b.point_type)
            })
        {
            return None;
        }
        let t = factor.clamp(0.0, 1.0);
        let mix = |a: f64, b: f64| a + (b - a) * t;
        let contours = self
            .contours
            .iter()
            .zip(&other.contours)
            .map(|(a, b)| Contour {
                points: a
                    .points
                    .iter()
                    .zip(&b.points)
                    .map(|(a, b)| ContourPoint {
                        x: mix(a.x, b.x),
                        y: mix(a.y, b.y),
                        point_type: a.point_type,
                        smooth: if t < 0.5 { a.smooth } else { b.smooth },
                    })
                    .collect(),
            })
            .collect();
        let components = self
            .components
            .iter()
            .zip(&other.components)
            .map(|(a, b)| {
                (a.base == b.base).then(|| GlyphComponent {
                    base: a.base.clone(),
                    x_scale: mix(a.x_scale, b.x_scale),
                    xy_scale: mix(a.xy_scale, b.xy_scale),
                    yx_scale: mix(a.yx_scale, b.yx_scale),
                    y_scale: mix(a.y_scale, b.y_scale),
                    x_offset: mix(a.x_offset, b.x_offset),
                    y_offset: mix(a.y_offset, b.y_offset),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let anchors = self
            .anchors
            .iter()
            .map(|anchor| {
                let other_anchor = other
                    .anchors
                    .iter()
                    .find(|item| item.name == anchor.name)
                    .expect("anchor topology was validated above");
                GlyphAnchor {
                    name: anchor.name.clone(),
                    x: mix(anchor.x, other_anchor.x),
                    y: mix(anchor.y, other_anchor.y),
                }
            })
            .collect();
        Some(Self {
            width: mix(self.width, other.width),
            contours,
            components,
            anchors,
        })
    }

    /// Bilinearly interpolates a rectangular four-master design space.
    ///
    /// The layers are ordered as bottom-left, bottom-right, top-left,
    /// top-right. Keeping this operation explicit avoids treating sparse or
    /// irregular master layouts as if they were rectangular.
    pub fn interpolate_bilinear(
        &self,
        bottom_right: &Self,
        top_left: &Self,
        top_right: &Self,
        x_factor: f64,
        y_factor: f64,
    ) -> Option<Self> {
        let bottom = self.interpolate(bottom_right, x_factor)?;
        let top = top_left.interpolate(top_right, x_factor)?;
        bottom.interpolate(&top, y_factor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphComponent {
    pub base: String,
    pub x_scale: f64,
    pub xy_scale: f64,
    pub yx_scale: f64,
    pub y_scale: f64,
    pub x_offset: f64,
    pub y_offset: f64,
}

impl GlyphData {
    /// Removes one contour from authored geometry and every master layer.
    pub fn remove_contour_all_layers(&mut self, index: usize) -> Result<(), String> {
        if index >= self.contours.len()
            || self
                .layers
                .values()
                .any(|layer| index >= layer.contours.len())
        {
            return Err("マスター間で削除対象の輪郭構造が一致しません".to_string());
        }
        self.contours.remove(index);
        for layer in self.layers.values_mut() {
            layer.contours.remove(index);
        }
        Ok(())
    }

    /// Toggles on/off-curve state for matching nodes in every layer.
    pub fn toggle_curve_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
    ) -> Result<(), String> {
        let toggle = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let mut result = contours.to_vec();
            for &(contour_index, point_index) in nodes {
                if let Some(point) = result
                    .get_mut(contour_index)
                    .and_then(|contour| contour.points.get_mut(point_index))
                {
                    point.point_type = match point.point_type {
                        PointType::OnCurve => PointType::OffCurve,
                        PointType::OffCurve => PointType::OnCurve,
                    };
                    if point.is_on_curve() {
                        point.smooth = false;
                    }
                }
            }
            for &contour_index in nodes.iter().map(|(contour_index, _)| contour_index) {
                let Some(contour) = result.get(contour_index) else {
                    continue;
                };
                if contour
                    .points
                    .iter()
                    .filter(|point| point.is_on_curve())
                    .count()
                    < 3
                {
                    return Err("オンカーブ点が3つ未満になるため変更できません".to_string());
                }
            }
            Ok(result)
        };
        let authored = toggle(&self.contours)?;
        let layers = self
            .layers
            .iter()
            .map(|(id, layer)| toggle(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<Vec<_>, _>>()?;
        self.contours = authored;
        for (id, contours) in layers {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Applies smooth/corner state to the same node positions in all layers.
    pub fn set_smooth_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        smooth: bool,
    ) -> Result<(), String> {
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".to_string());
        }
        let apply = |contours: &mut [Contour]| {
            for &(contour_index, point_index) in nodes {
                if let Some(contour) = contours.get_mut(contour_index) {
                    if smooth {
                        contour.set_smooth(point_index, true);
                    } else if let Some(point) = contour.points.get_mut(point_index) {
                        point.smooth = false;
                    }
                }
            }
        };
        apply(&mut self.contours);
        for layer in self.layers.values_mut() {
            apply(&mut layer.contours);
        }
        Ok(())
    }

    /// Removes selected nodes from the authored outline and every saved
    /// master layer atomically. Fragments left without enough authored data
    /// are discarded consistently in every layer.
    pub fn remove_nodes_all_layers(&mut self, nodes: &[(usize, usize)]) -> Result<(), String> {
        if nodes.is_empty() {
            return Ok(());
        }
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".to_string());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut Vec<Contour>| {
            let mut grouped: std::collections::BTreeMap<usize, Vec<usize>> =
                std::collections::BTreeMap::new();
            for &(contour_index, point_index) in nodes {
                grouped.entry(contour_index).or_default().push(point_index);
            }
            for (contour_index, mut point_indices) in grouped {
                if let Some(contour) = contours.get_mut(contour_index) {
                    point_indices.sort_unstable();
                    point_indices.dedup();
                    for point_index in point_indices.into_iter().rev() {
                        if point_index < contour.points.len() {
                            contour.points.remove(point_index);
                        }
                    }
                }
            }
            contours.retain(|contour| {
                contour.points.len() >= 3 && contour.points.iter().any(|point| point.is_on_curve())
            });
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }

    /// Inserts a node at the same segment position in the authored outline
    /// and every master layer, without leaving a partially edited glyph.
    pub fn split_segment_all_layers(
        &mut self,
        contour_index: usize,
        start_index: usize,
        factor: f64,
    ) -> Result<usize, String> {
        let mut candidate = self.clone();
        let inserted = candidate
            .contours
            .get_mut(contour_index)
            .and_then(|contour| contour.split_segment(start_index, factor))
            .ok_or_else(|| "輪郭を分割できません".to_string())?;
        for layer in candidate.layers.values_mut() {
            layer
                .contours
                .get_mut(contour_index)
                .and_then(|contour| contour.split_segment(start_index, factor))
                .ok_or_else(|| "マスター間で輪郭構造が一致しません".to_string())?;
        }
        *self = candidate;
        Ok(inserted)
    }

    /// Cuts a contour at two nodes in the authored outline and every layer.
    pub fn cut_contour_all_layers(
        &mut self,
        contour_index: usize,
        first: usize,
        second: usize,
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        let replace = |contours: &mut Vec<Contour>| -> Result<(), String> {
            let (first_contour, second_contour) = contours
                .get(contour_index)
                .and_then(|contour| contour.cut_at_nodes(first, second))
                .ok_or_else(|| "2点の間隔が短すぎるため輪郭を分割できません".to_string())?;
            contours.remove(contour_index);
            contours.insert(contour_index, second_contour);
            contours.insert(contour_index, first_contour);
            Ok(())
        };
        replace(&mut candidate.contours)?;
        for layer in candidate.layers.values_mut() {
            replace(&mut layer.contours)?;
        }
        *self = candidate;
        Ok(())
    }

    /// Moves the same authored nodes in every master layer atomically.
    pub fn translate_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        dx: f64,
        dy: f64,
    ) -> Result<(), String> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err("移動量が不正です".into());
        }
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour]| {
            for (contour_index, contour) in contours.iter_mut().enumerate() {
                let indices: Vec<usize> = nodes
                    .iter()
                    .filter_map(|&(selected_contour, point_index)| {
                        (selected_contour == contour_index).then_some(point_index)
                    })
                    .collect();
                if !indices.is_empty() {
                    contour.translate_points(&indices, dx, dy);
                }
            }
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }

    /// Moves one component in every master layer atomically.
    pub fn translate_component_all_layers(
        &mut self,
        component_index: usize,
        dx: f64,
        dy: f64,
    ) -> Result<(), String> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err("移動量が不正です".into());
        }
        if component_index >= self.components.len()
            || self
                .layers
                .values()
                .any(|layer| component_index >= layer.components.len())
        {
            return Err("マスター間でコンポーネント構造が一致しません".into());
        }
        let mut candidate = self.clone();
        candidate.components[component_index].x_offset += dx;
        candidate.components[component_index].y_offset += dy;
        for layer in candidate.layers.values_mut() {
            layer.components[component_index].x_offset += dx;
            layer.components[component_index].y_offset += dy;
        }
        *self = candidate;
        Ok(())
    }

    /// Applies a scale/rotation to one component in every master layer.
    pub fn transform_component_all_layers(
        &mut self,
        component_index: usize,
        scale: f64,
        angle_radians: f64,
    ) -> Result<(), String> {
        if !scale.is_finite() || !angle_radians.is_finite() || scale == 0.0 {
            return Err("変形値が不正です".into());
        }
        let base = self
            .components
            .get(component_index)
            .map(|component| component.base.as_str())
            .ok_or_else(|| "コンポーネントがありません".to_string())?;
        if self.layers.values().any(|layer| {
            layer
                .components
                .get(component_index)
                .is_none_or(|component| component.base != base)
        }) {
            return Err("マスター間でコンポーネント構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let (sin, cos) = angle_radians.sin_cos();
        let apply = |component: &mut GlyphComponent| {
            let a = component.x_scale;
            let b = component.xy_scale;
            let c = component.yx_scale;
            let d = component.y_scale;
            component.x_scale = scale * (cos * a - sin * b);
            component.xy_scale = scale * (sin * a + cos * b);
            component.yx_scale = scale * (cos * c - sin * d);
            component.y_scale = scale * (sin * c + cos * d);
        };
        apply(&mut candidate.components[component_index]);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.components[component_index]);
        }
        *self = candidate;
        Ok(())
    }

    /// Reflects one component in every master layer.
    pub fn reflect_component_all_layers(
        &mut self,
        component_index: usize,
        horizontal: bool,
    ) -> Result<(), String> {
        let base = self
            .components
            .get(component_index)
            .map(|component| component.base.as_str())
            .ok_or_else(|| "コンポーネントがありません".to_string())?;
        if self.layers.values().any(|layer| {
            layer
                .components
                .get(component_index)
                .is_none_or(|component| component.base != base)
        }) {
            return Err("マスター間でコンポーネント構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |component: &mut GlyphComponent| {
            if horizontal {
                component.x_scale = -component.x_scale;
                component.xy_scale = -component.xy_scale;
            } else {
                component.yx_scale = -component.yx_scale;
                component.y_scale = -component.y_scale;
            }
        };
        apply(&mut candidate.components[component_index]);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.components[component_index]);
        }
        *self = candidate;
        Ok(())
    }

    /// Applies a scale/rotation to the same nodes in every master layer.
    pub fn transform_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        scale: f64,
        angle_radians: f64,
    ) -> Result<(), String> {
        if !scale.is_finite() || !angle_radians.is_finite() || scale == 0.0 {
            return Err("変形値が不正です".into());
        }
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour]| {
            let (cx, cy) = nodes.iter().fold((0.0, 0.0), |(x, y), &(ci, pi)| {
                let point = contours[ci].points[pi];
                (x + point.x, y + point.y)
            });
            let center = (cx / nodes.len() as f64, cy / nodes.len() as f64);
            let (sin, cos) = angle_radians.sin_cos();
            for &(ci, pi) in nodes {
                let point = &mut contours[ci].points[pi];
                let x = (point.x - center.0) * scale;
                let y = (point.y - center.1) * scale;
                point.x = center.0 + x * cos - y * sin;
                point.y = center.1 + x * sin + y * cos;
            }
            for contour in contours {
                contour.repair_smooth_handles();
            }
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }

    /// Reflects the same nodes around their selection bounds in every layer.
    pub fn reflect_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if nodes.is_empty()
            || !valid(&self.contours)
            || self.layers.values().any(|layer| !valid(&layer.contours))
        {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour]| {
            let points = nodes.iter().map(|&(ci, pi)| contours[ci].points[pi]);
            let (min_x, max_x, min_y, max_y) = points.fold(
                (
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ),
                |(min_x, max_x, min_y, max_y), point| {
                    (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_y.min(point.y),
                        max_y.max(point.y),
                    )
                },
            );
            let center = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
            for &(ci, pi) in nodes {
                let point = &mut contours[ci].points[pi];
                if horizontal {
                    point.x = center.0 - (point.x - center.0);
                } else {
                    point.y = center.1 - (point.y - center.1);
                }
            }
            for contour in contours {
                contour.repair_smooth_handles();
            }
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }

    /// Aligns the selected nodes on one axis in every master layer.
    pub fn align_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        self.map_nodes_all_layers(nodes, |points| {
            let target = points
                .iter()
                .map(|point| if horizontal { point.y } else { point.x })
                .sum::<f64>()
                / points.len() as f64;
            for point in points {
                if horizontal {
                    point.y = target;
                } else {
                    point.x = target;
                }
            }
        })
    }

    /// Distributes selected nodes evenly on one axis in every master layer.
    pub fn distribute_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        if nodes.len() < 3 {
            return Err("分布には3つ以上のノードが必要です".into());
        }
        self.map_nodes_all_layers(nodes, |points| {
            let mut ordered: Vec<(f64, usize)> = points
                .iter()
                .enumerate()
                .map(|(index, point)| (if horizontal { point.x } else { point.y }, index))
                .collect();
            ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
            let first = ordered.first().map(|item| item.0).unwrap_or_default();
            let last = ordered.last().map(|item| item.0).unwrap_or(first);
            let step = (last - first) / (ordered.len() - 1) as f64;
            for (index, (_, point_index)) in ordered.into_iter().enumerate() {
                if horizontal {
                    points[point_index].x = first + step * index as f64;
                } else {
                    points[point_index].y = first + step * index as f64;
                }
            }
        })
    }

    fn map_nodes_all_layers<F>(
        &mut self,
        nodes: &[(usize, usize)],
        mut operation: F,
    ) -> Result<(), String>
    where
        F: FnMut(&mut Vec<ContourPoint>),
    {
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if nodes.is_empty()
            || !valid(&self.contours)
            || self.layers.values().any(|layer| !valid(&layer.contours))
        {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour], operation: &mut F| {
            let mut points: Vec<ContourPoint> = nodes
                .iter()
                .map(|&(contour_index, point_index)| contours[contour_index].points[point_index])
                .collect();
            operation(&mut points);
            for (&(contour_index, point_index), point) in nodes.iter().zip(points) {
                contours[contour_index].points[point_index] = point;
            }
            for contour in contours {
                contour.repair_smooth_handles();
            }
        };
        apply(&mut candidate.contours, &mut operation);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours, &mut operation);
        }
        *self = candidate;
        Ok(())
    }

    /// Reverses one contour in the authored geometry and every master layer.
    pub fn reverse_contour_all_layers(&mut self, index: usize) -> Result<(), String> {
        let reverse = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let mut result = contours.to_vec();
            let contour = result
                .get_mut(index)
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            contour.reverse_direction();
            Ok(result)
        };
        let authored = reverse(&self.contours)?;
        let layers = self
            .layers
            .iter()
            .map(|(id, layer)| reverse(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<Vec<_>, _>>()?;
        self.contours = authored;
        for (id, contours) in layers {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Unions every authored contour into one Boolean result in every layer.
    pub fn union_all_contours_all_layers(&mut self) -> Result<(), String> {
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            if contours.len() < 2 {
                return Err("統合する輪郭が2つ以上必要です".to_string());
            }
            let mut result = vec![contours[0].clone()];
            for contour in &contours[1..] {
                let mut merged = Vec::new();
                for current in result {
                    merged.extend(current.union(contour)?);
                }
                result = merged;
            }
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Unions a contour with its following contour in the authored geometry
    /// and every available master layer.
    pub fn union_contours_all_layers(&mut self, index: usize) -> Result<(), String> {
        let next = index
            .checked_add(1)
            .ok_or_else(|| "輪郭番号が大きすぎます".to_string())?;
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let first = contours
                .get(index)
                .cloned()
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            let second = contours
                .get(next)
                .cloned()
                .ok_or_else(|| "統合対象の次の輪郭が存在しません".to_string())?;
            let mut result = contours.to_vec();
            let union = first.union(&second)?;
            result.remove(next);
            result.remove(index);
            result.splice(index..index, union);
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Subtracts a contour from its following contour in every layer.
    pub fn difference_contours_all_layers(&mut self, index: usize) -> Result<(), String> {
        let next = index
            .checked_add(1)
            .ok_or_else(|| "輪郭番号が大きすぎます".to_string())?;
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let first = contours
                .get(index)
                .cloned()
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            let second = contours
                .get(next)
                .cloned()
                .ok_or_else(|| "差分対象の次の輪郭が存在しません".to_string())?;
            let mut result = contours.to_vec();
            let difference = first.difference(&second)?;
            result.remove(next);
            result.remove(index);
            result.splice(index..index, difference);
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Keeps the intersection of adjacent contours in every layer.
    pub fn intersection_contours_all_layers(&mut self, index: usize) -> Result<(), String> {
        let next = index
            .checked_add(1)
            .ok_or_else(|| "輪郭番号が大きすぎます".to_string())?;
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let first = contours
                .get(index)
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            let second = contours
                .get(next)
                .ok_or_else(|| "交差対象の次の輪郭が存在しません".to_string())?;
            let mut result = contours.to_vec();
            let intersection = first.intersection(second)?;
            result.remove(next);
            result.remove(index);
            result.splice(index..index, intersection);
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    /// Keeps the exclusive area of adjacent contours in every layer.
    pub fn xor_contours_all_layers(&mut self, index: usize) -> Result<(), String> {
        let next = index
            .checked_add(1)
            .ok_or_else(|| "輪郭番号が大きすぎます".to_string())?;
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let first = contours
                .get(index)
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            let second = contours
                .get(next)
                .ok_or_else(|| "XOR対象の次の輪郭が存在しません".to_string())?;
            let mut result = contours.to_vec();
            let xor = first.xor(second)?;
            result.remove(next);
            result.remove(index);
            result.splice(index..index, xor);
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }

    pub fn translate_geometry(&mut self, dx: f64, dy: f64) {
        self.translate_authored_geometry(dx, dy);
        for layer in self.layers.values_mut() {
            layer.translate_geometry(dx, dy);
        }
        for guides in self.master_guidelines.values_mut() {
            for guide in guides {
                guide.x += dx;
                guide.y += dy;
            }
        }
    }

    fn translate_authored_geometry(&mut self, dx: f64, dy: f64) {
        for contour in &mut self.contours {
            for point in &mut contour.points {
                point.x += dx;
                point.y += dy;
            }
        }
        for component in &mut self.components {
            component.x_offset += dx;
            component.y_offset += dy;
        }
        for anchor in &mut self.anchors {
            anchor.x += dx;
            anchor.y += dy;
        }
        for guide in &mut self.guidelines {
            guide.x += dx;
            guide.y += dy;
        }
    }

    pub fn new(name: String, unicode: Option<u32>) -> Self {
        Self {
            name,
            unicode,
            unicodes: Vec::new(),
            width: 600.0,
            left_kerning_group: String::new(),
            right_kerning_group: String::new(),
            left_metrics_key: String::new(),
            right_metrics_key: String::new(),
            anchors: Vec::new(),
            contours: Vec::new(),
            components: Vec::new(),
            layers: HashMap::new(),
            guidelines: Vec::new(),
            master_guidelines: HashMap::new(),
        }
    }

    fn layer_snapshot(&self) -> GlyphLayer {
        GlyphLayer {
            width: self.width,
            contours: self.contours.clone(),
            components: self.components.clone(),
            anchors: self.anchors.clone(),
        }
    }

    pub fn ensure_layer(&mut self, master_id: &str) {
        if self.layers.is_empty() {
            self.layers
                .insert(master_id.to_string(), self.layer_snapshot());
        }
        self.master_guidelines
            .entry(master_id.to_string())
            .or_insert_with(|| self.guidelines.clone());
    }

    /// Returns the guides belonging to a master, falling back to the legacy
    /// active-layer field for projects created before per-master guides.
    pub fn guidelines_for_master(&self, master_id: &str) -> &[Guideline] {
        self.master_guidelines
            .get(master_id)
            .map(Vec::as_slice)
            .unwrap_or(&self.guidelines)
    }

    /// Returns the editable guide list for a master, upgrading legacy data
    /// lazily when that master has no explicit guide list yet.
    pub fn guidelines_for_master_mut(&mut self, master_id: &str) -> &mut Vec<Guideline> {
        if !self.master_guidelines.contains_key(master_id) {
            self.master_guidelines
                .insert(master_id.to_string(), self.guidelines.clone());
        }
        self.master_guidelines
            .get_mut(master_id)
            .expect("guide entry inserted")
    }

    pub fn switch_layer(&mut self, from_id: &str, to_id: &str) {
        self.ensure_layer(from_id);
        self.layers
            .insert(from_id.to_string(), self.layer_snapshot());
        self.master_guidelines
            .insert(from_id.to_string(), self.guidelines.clone());
        if let Some(layer) = self.layers.get(to_id).cloned() {
            self.width = layer.width;
            self.contours = layer.contours;
            self.components = layer.components;
            self.anchors = layer.anchors;
            self.guidelines = self
                .master_guidelines
                .get(to_id)
                .cloned()
                .unwrap_or_default();
            self.master_guidelines
                .entry(to_id.to_string())
                .or_insert_with(|| self.guidelines.clone());
        } else {
            let layer = self.layer_snapshot();
            self.layers.insert(to_id.to_string(), layer);
            self.master_guidelines
                .insert(to_id.to_string(), self.guidelines.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    pub points: Vec<ContourPoint>,
}

impl Contour {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Reverses the winding direction while preserving the authored nodes.
    pub fn reverse_direction(&mut self) {
        self.points.reverse();
    }

    /// Marks an on-curve node smooth and aligns its adjacent handles.
    pub fn set_smooth(&mut self, index: usize, smooth: bool) {
        let len = self.points.len();
        if index >= len || !self.points[index].is_on_curve() {
            return;
        }
        self.points[index].smooth = smooth;
        if !smooth || len < 3 {
            return;
        }
        let previous = (index + len - 1) % len;
        let next = (index + 1) % len;
        if self.points[previous].is_on_curve() || self.points[next].is_on_curve() {
            return;
        }
        let point = self.points[index];
        let before = self.points[previous];
        let after = self.points[next];
        let dx = after.x - before.x;
        let dy = after.y - before.y;
        let direction_length = (dx * dx + dy * dy).sqrt();
        if direction_length <= f64::EPSILON {
            return;
        }
        let before_length = ((before.x - point.x).powi(2) + (before.y - point.y).powi(2)).sqrt();
        let after_length = ((after.x - point.x).powi(2) + (after.y - point.y).powi(2)).sqrt();
        let ux = dx / direction_length;
        let uy = dy / direction_length;
        self.points[previous].x = point.x - ux * before_length;
        self.points[previous].y = point.y - uy * before_length;
        self.points[next].x = point.x + ux * after_length;
        self.points[next].y = point.y + uy * after_length;
    }

    /// Re-aligns every smooth node after a bulk transform.
    pub fn repair_smooth_handles(&mut self) {
        let smooth_nodes: Vec<usize> = self
            .points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| (point.is_on_curve() && point.smooth).then_some(index))
            .collect();
        for index in smooth_nodes {
            self.set_smooth(index, true);
        }
    }

    pub fn translate_point(&mut self, index: usize, dx: f64, dy: f64) {
        let len = self.points.len();
        if index >= len {
            return;
        }
        let was_smooth = self.points[index].is_on_curve() && self.points[index].smooth;
        self.points[index].x += dx;
        self.points[index].y += dy;
        if was_smooth {
            for neighbor in [(index + len - 1) % len, (index + 1) % len] {
                if !self.points[neighbor].is_on_curve() {
                    self.points[neighbor].x += dx;
                    self.points[neighbor].y += dy;
                }
            }
        } else if !self.points[index].is_on_curve() {
            self.align_opposite_smooth_handle(index);
        }
    }

    /// Moves a selection of points without moving any point twice. Smooth
    /// handles follow their on-curve point only when they are not themselves
    /// part of the selection.
    pub fn translate_points(&mut self, indices: &[usize], dx: f64, dy: f64) {
        let selected: std::collections::HashSet<usize> = indices
            .iter()
            .copied()
            .filter(|&index| index < self.points.len())
            .collect();
        let mut move_indices = selected.clone();
        let len = self.points.len();
        for &index in &selected {
            if self.points[index].is_on_curve() && self.points[index].smooth {
                for neighbor in [(index + len - 1) % len, (index + 1) % len] {
                    if !self.points[neighbor].is_on_curve() && !selected.contains(&neighbor) {
                        move_indices.insert(neighbor);
                    }
                }
            }
        }
        for index in move_indices {
            self.points[index].x += dx;
            self.points[index].y += dy;
        }
        self.repair_smooth_handles();
    }

    fn align_opposite_smooth_handle(&mut self, handle_index: usize) {
        let len = self.points.len();
        if len < 3 || handle_index >= len {
            return;
        }
        let previous = (handle_index + len - 1) % len;
        let next = (handle_index + 1) % len;
        let (anchor_index, opposite_index) =
            if self.points[previous].is_on_curve() && self.points[previous].smooth {
                let opposite = (previous + len - 1) % len;
                (previous, opposite)
            } else if self.points[next].is_on_curve() && self.points[next].smooth {
                let opposite = (next + 1) % len;
                (next, opposite)
            } else {
                return;
            };
        if self.points[opposite_index].is_on_curve() {
            return;
        }
        let anchor = self.points[anchor_index];
        let handle = self.points[handle_index];
        let dx = handle.x - anchor.x;
        let dy = handle.y - anchor.y;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f64::EPSILON {
            return;
        }
        let opposite_length = {
            let point = self.points[opposite_index];
            ((point.x - anchor.x).powi(2) + (point.y - anchor.y).powi(2)).sqrt()
        };
        if opposite_length <= f64::EPSILON {
            return;
        }
        let ux = dx / length;
        let uy = dy / length;
        self.points[opposite_index].x = anchor.x - ux * opposite_length;
        self.points[opposite_index].y = anchor.y - uy * opposite_length;
    }

    pub fn to_bezpath(&self) -> BezPath {
        let mut path = BezPath::new();
        if self.points.is_empty() {
            return path;
        }

        let points = &self.points;
        let start_idx = points.iter().position(|p| p.is_on_curve()).unwrap_or(0);
        let first = points[start_idx];

        path.move_to(Point::new(first.x, first.y));

        let mut i = (start_idx + 1) % points.len();
        while i != start_idx {
            let p = points[i];
            if p.is_on_curve() {
                path.line_to(Point::new(p.x, p.y));
            } else {
                let mut off_curve = vec![p];
                let next_i = (i + 1) % points.len();
                let next = points[next_i];

                if !next.is_on_curve() {
                    off_curve.push(next);
                    let after_next = points[(next_i + 1) % points.len()];
                    if off_curve.len() == 2 {
                        path.curve_to(
                            Point::new(off_curve[0].x, off_curve[0].y),
                            Point::new(off_curve[1].x, off_curve[1].y),
                            Point::new(after_next.x, after_next.y),
                        );
                        i = (next_i + 1) % points.len();
                        continue;
                    }
                }

                path.quad_to(Point::new(p.x, p.y), Point::new(next.x, next.y));
                i = next_i;
            }
            i = (i + 1) % points.len();
        }

        path.close_path();
        path
    }

    /// Converts this contour to curve-preserving Boolean geometry.
    #[allow(dead_code)]
    pub(crate) fn to_curve_shape(&self) -> Result<i_curve::FloatCurveShape<[f64; 2]>, String> {
        let mut builder = i_curve::CurveBuilder::new();
        for element in self.to_bezpath().elements() {
            match element {
                PathEl::MoveTo(point) => {
                    builder
                        .move_to([point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::LineTo(point) => {
                    builder
                        .line_to([point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::QuadTo(control, point) => {
                    builder
                        .quad_to([control.x, control.y], [point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::CurveTo(control0, control1, point) => {
                    builder
                        .cubic_to(
                            [control0.x, control0.y],
                            [control1.x, control1.y],
                            [point.x, point.y],
                        )
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::ClosePath => {}
            }
        }
        builder
            .close_contour()
            .map_err(|error| format!("輪郭を閉じられません: {error}"))?;
        builder
            .build()
            .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))
    }

    fn from_curve_path(path: i_curve::FloatCurvePath<[f64; 2]>) -> Result<Self, String> {
        let (start, segments) = path.into_parts();
        let mut points = vec![ContourPoint::on_curve(start[0], start[1])];
        let mut current = start;
        for segment in segments {
            match segment {
                i_curve::FloatCurveSegment::Line { to } => {
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Quad { ctrl, to } => {
                    points.push(ContourPoint::off_curve(ctrl[0], ctrl[1]));
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Cubic { ctrl0, ctrl1, to } => {
                    points.push(ContourPoint::off_curve(ctrl0[0], ctrl0[1]));
                    points.push(ContourPoint::off_curve(ctrl1[0], ctrl1[1]));
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Arc { arc } => {
                    // i_curve returns rational quadratic arcs. Our editable
                    // contour format has no rational weights, so degree-
                    // approximate each (already monotone) arc piece as a
                    // cubic. The endpoint tangent is exact; the remaining
                    // error is bounded by the small pieces produced by
                    // i_curve's arc materializer.
                    let p0 = current;
                    let p1 = arc.control_points[1];
                    let p2 = arc.end_point();
                    let factor = (2.0 * arc.weights[1]) / 3.0;
                    let ctrl0 = [
                        p0[0] + factor * (p1[0] - p0[0]),
                        p0[1] + factor * (p1[1] - p0[1]),
                    ];
                    let ctrl1 = [
                        p2[0] - factor * (p2[0] - p1[0]),
                        p2[1] - factor * (p2[1] - p1[1]),
                    ];
                    points.push(ContourPoint::off_curve(ctrl0[0], ctrl0[1]));
                    points.push(ContourPoint::off_curve(ctrl1[0], ctrl1[1]));
                    if p2 != start {
                        points.push(ContourPoint::on_curve(p2[0], p2[1]));
                    }
                    current = p2;
                }
            }
        }
        if points.len() < 3 {
            return Err("Boolean結果が不完全な輪郭です".into());
        }
        Ok(Self { points })
    }

    /// Unions two contours while preserving supported Bézier segments.
    pub fn union(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        let shapes = subject.overlay(
            &clip,
            i_curve::OverlayRule::Union,
            i_curve::FillRule::NonZero,
        );
        shapes
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }

    /// Subtracts `other` from this contour while preserving Bézier segments.
    pub fn difference(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        subject
            .overlay(
                &clip,
                i_curve::OverlayRule::Difference,
                i_curve::FillRule::NonZero,
            )
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }

    /// Keeps only the intersection of two contours.
    pub fn intersection(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        subject
            .overlay(
                &clip,
                i_curve::OverlayRule::Intersect,
                i_curve::FillRule::NonZero,
            )
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }

    /// Keeps the exclusive area of two contours.
    pub fn xor(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        subject
            .overlay(&clip, i_curve::OverlayRule::Xor, i_curve::FillRule::NonZero)
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }

    pub fn signed_area(&self) -> f64 {
        let mut outline = Vec::new();
        flatten(self.to_bezpath(), 0.25, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                outline.push(point);
            }
        });
        outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .take(outline.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f64>()
            * 0.5
    }

    /// Splits a line, quadratic, or cubic segment whose start point is
    /// `start_index`. The segment must not wrap around the point vector.
    /// Returns the inserted on-curve point index.
    pub fn split_segment(&mut self, start_index: usize, t: f64) -> Option<usize> {
        if self.points.len() < 2 || start_index >= self.points.len() {
            return None;
        }
        let start = self.points[start_index];
        if !start.is_on_curve() {
            return None;
        }
        let mut end = start_index + 1;
        while end < self.points.len() && !self.points[end].is_on_curve() {
            end += 1;
        }
        if end >= self.points.len() {
            // The segment wraps around the cyclic contour. Rotate it so the
            // existing non-wrapping implementation can preserve its controls.
            if start_index == 0 {
                return None;
            }
            let rotation = start_index;
            self.points.rotate_left(rotation);
            let inserted = self.split_segment(0, t);
            let new_len = self.points.len();
            self.points.rotate_right(rotation % new_len);
            return inserted.map(|index| (index + rotation) % new_len);
        }
        if end - start_index > 3 {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: ContourPoint, b: ContourPoint| {
            ContourPoint::on_curve(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
        };
        let mut replacement = Vec::new();
        match end - start_index {
            1 => replacement.push(lerp(start, self.points[end])),
            2 => {
                let c = self.points[start_index + 1];
                let mut left = lerp(start, c);
                let mut right = lerp(c, self.points[end]);
                left.point_type = PointType::OffCurve;
                right.point_type = PointType::OffCurve;
                let q = lerp(left, right);
                replacement.extend([left, q, right]);
            }
            3 => {
                let c1 = self.points[start_index + 1];
                let c2 = self.points[start_index + 2];
                let mut a = lerp(start, c1);
                let mut b = lerp(c1, c2);
                let mut c = lerp(c2, self.points[end]);
                let mut d = lerp(a, b);
                let mut e = lerp(b, c);
                for point in [&mut a, &mut b, &mut c, &mut d, &mut e] {
                    point.point_type = PointType::OffCurve;
                }
                let q = lerp(d, e);
                replacement.extend([a, d, q, e, c]);
            }
            _ => return None,
        }
        self.points.splice((start_index + 1)..end, replacement);
        Some(start_index + 1 + if end - start_index == 1 { 0 } else { 1 })
    }

    /// Cuts a closed contour at two on-curve node indices.
    pub fn cut_at_nodes(&self, first: usize, second: usize) -> Option<(Self, Self)> {
        let len = self.points.len();
        if len < 4 || first >= len || second >= len || first == second {
            return None;
        }
        if !self.points[first].is_on_curve() || !self.points[second].is_on_curve() {
            return None;
        }
        let (start, end) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        if end - start < 2 || len - (end - start) < 2 {
            return None;
        }
        let first_path = self.points[start..=end].to_vec();
        let mut second_path = self.points[end..].to_vec();
        second_path.extend_from_slice(&self.points[..=start]);
        if first_path.len() < 3 || second_path.len() < 3 {
            return None;
        }
        Some((
            Self { points: first_path },
            Self {
                points: second_path,
            },
        ))
    }
}

impl Default for Contour {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContourPoint {
    pub x: f64,
    pub y: f64,
    pub point_type: PointType,
    pub smooth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointType {
    OnCurve,
    OffCurve,
}

impl ContourPoint {
    pub fn on_curve(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            point_type: PointType::OnCurve,
            smooth: false,
        }
    }

    pub fn off_curve(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            point_type: PointType::OffCurve,
            smooth: false,
        }
    }

    pub fn is_on_curve(&self) -> bool {
        self.point_type == PointType::OnCurve
    }
}

impl FontProject {
    /// Interpolates a glyph in a complete rectangular two-axis master set.
    pub fn interpolate_glyph_bilinear(
        &self,
        glyph_name: &str,
        axis_x: &str,
        axis_y: &str,
        target_x: f64,
        target_y: f64,
    ) -> Option<GlyphLayer> {
        let (indices, (x_factor, y_factor)) =
            find_bilinear_masters(&self.masters, axis_x, axis_y, target_x, target_y)?;
        let glyph = self.glyphs.get(glyph_name)?;
        let layers = indices.map(|index| glyph.layers.get(&self.masters[index].id));
        layers[0]?.interpolate_bilinear(layers[1]?, layers[2]?, layers[3]?, x_factor, y_factor)
    }

    #[allow(dead_code)]
    pub fn vertical_metrics_for_glyph(&self, name: &str) -> VerticalMetrics {
        self.vertical_metrics
            .get(name)
            .copied()
            .unwrap_or(VerticalMetrics {
                advance_height: self.metadata.units_per_em,
                top_side_bearing: self.metadata.ascender,
            })
    }

    pub fn vertical_metrics_for_glyph_in_master(
        &self,
        name: &str,
        master_id: &str,
    ) -> VerticalMetrics {
        self.vertical_metrics_by_master
            .get(master_id)
            .and_then(|metrics| metrics.get(name).copied())
            .unwrap_or_else(|| self.vertical_metrics_for_glyph(name))
    }

    pub fn master_metrics_for(&self, master_id: &str) -> MasterMetrics {
        self.metrics_by_master
            .get(master_id)
            .copied()
            .unwrap_or(MasterMetrics {
                ascender: self.metadata.ascender,
                descender: self.metadata.descender,
                line_gap: self.metadata.line_gap,
            })
    }

    pub fn set_master_metrics(
        &mut self,
        master_id: &str,
        metrics: MasterMetrics,
    ) -> Result<(), String> {
        if !self.masters.iter().any(|master| master.id == master_id) {
            return Err(format!("マスター '{}' がありません", master_id));
        }
        if !metrics.ascender.is_finite()
            || !metrics.descender.is_finite()
            || !metrics.line_gap.is_finite()
        {
            return Err("マスターメトリクスは有限値で指定してください".into());
        }
        self.metrics_by_master
            .insert(master_id.to_string(), metrics);
        Ok(())
    }

    pub fn clear_master_metrics(&mut self, master_id: &str) -> bool {
        self.metrics_by_master.remove(master_id).is_some()
    }

    pub fn set_vertical_metrics_for_master(
        &mut self,
        name: &str,
        master_id: &str,
        advance_height: f64,
        top_side_bearing: f64,
    ) -> Result<(), String> {
        if !self.glyphs.contains_key(name) {
            return Err(format!("グリフ '{}' がありません", name));
        }
        if !advance_height.is_finite() || advance_height < 0.0 {
            return Err("縦アドバンスは0以上の有限値で指定してください".into());
        }
        if !top_side_bearing.is_finite() {
            return Err("縦TSBは有限値で指定してください".into());
        }
        self.vertical_metrics_by_master
            .entry(master_id.to_string())
            .or_default()
            .insert(
                name.to_string(),
                VerticalMetrics {
                    advance_height,
                    top_side_bearing,
                },
            );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_vertical_metrics(
        &mut self,
        name: &str,
        advance_height: f64,
        top_side_bearing: f64,
    ) -> Result<(), String> {
        if !self.glyphs.contains_key(name) {
            return Err(format!("グリフ '{}' がありません", name));
        }
        if !advance_height.is_finite() || advance_height < 0.0 || !top_side_bearing.is_finite() {
            return Err("縦メトリクスが不正です".into());
        }
        self.vertical_metrics.insert(
            name.to_string(),
            VerticalMetrics {
                advance_height,
                top_side_bearing,
            },
        );
        Ok(())
    }

    /// Sets the advance width for each existing glyph in `names`.
    /// Returns the number of glyphs whose value changed.
    pub fn set_width_for_glyphs(&mut self, names: &[String], width: f64) -> usize {
        if !width.is_finite() || width < 0.0 {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                if (glyph.width - width).abs() > f64::EPSILON {
                    glyph.width = width;
                    for layer in glyph.layers.values_mut() {
                        layer.width = width;
                    }
                    changed += 1;
                }
            }
        }
        changed
    }

    /// 複数グリフの字幅を検証してからまとめて設定する。
    pub fn set_widths_batch<I, N>(&mut self, rows: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (N, f64)>,
        N: AsRef<str>,
    {
        let rows: Vec<(String, f64)> = rows
            .into_iter()
            .map(|(name, width)| (name.as_ref().to_string(), width))
            .collect();
        for (name, width) in &rows {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if !width.is_finite() || *width < 0.0 {
                return Err(format!("グリフ '{}' の字幅が不正です", name));
            }
        }
        let count = rows.len();
        for (name, width) in rows {
            self.set_width_for_glyphs(&[name], width);
        }
        Ok(count)
    }

    /// Assigns primary Unicode values, removing conflicts from other glyphs.
    /// Returns the number of glyph records changed.
    pub fn set_unicode_assignments(&mut self, assignments: &[(String, u32)]) -> usize {
        let mut changed = 0;
        for (name, codepoint) in assignments {
            if char::from_u32(*codepoint).is_none() || !self.glyphs.contains_key(name) {
                continue;
            }
            for (other_name, glyph) in &mut self.glyphs {
                if other_name == name {
                    continue;
                }
                let mut glyph_changed = false;
                if glyph.unicode == Some(*codepoint) {
                    glyph.unicode = None;
                    glyph_changed = true;
                }
                let aliases_before = glyph.unicodes.len();
                glyph.unicodes.retain(|alias| *alias != *codepoint);
                glyph_changed |= aliases_before != glyph.unicodes.len();
                changed += usize::from(glyph_changed);
            }
            if let Some(glyph) = self.glyphs.get_mut(name) {
                if glyph.unicode != Some(*codepoint) {
                    glyph.unicode = Some(*codepoint);
                    changed += 1;
                }
                glyph.unicodes.retain(|alias| *alias != *codepoint);
            }
        }
        changed
    }

    /// Unicode一括設定の厳格版。全グリフとコードポイントを検証してから適用する。
    pub fn set_unicode_assignments_strict(
        &mut self,
        assignments: &[(String, u32)],
    ) -> Result<usize, String> {
        let mut seen = std::collections::HashSet::new();
        for (name, codepoint) in assignments {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if char::from_u32(*codepoint).is_none() {
                return Err(format!("グリフ '{}' のUnicodeが不正です", name));
            }
            if !seen.insert(*codepoint) {
                return Err(format!("Unicode U+{:04X} が重複しています", codepoint));
            }
        }
        Ok(self.set_unicode_assignments(assignments))
    }

    /// Sets left and right side bearings by translating authored geometry and
    /// recalculating advance width for every available master layer.
    /// Sets left and right kerning groups for existing glyphs.
    pub fn set_kerning_groups(
        &mut self,
        names: &[String],
        left_group: &str,
        right_group: &str,
    ) -> Result<usize, String> {
        let left_group = left_group.trim();
        let right_group = right_group.trim();
        if left_group.chars().any(char::is_whitespace)
            || right_group.chars().any(char::is_whitespace)
        {
            return Err("カーニンググループ名に空白は使えません".into());
        }
        for name in names {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
        }
        let mut changed = 0;
        for name in names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            if glyph.left_kerning_group != left_group || glyph.right_kerning_group != right_group {
                glyph.left_kerning_group = left_group.to_string();
                glyph.right_kerning_group = right_group.to_string();
                changed += 1;
            }
        }
        Ok(changed)
    }

    pub fn set_side_bearings(&mut self, names: &[String], left: f64, right: f64) -> usize {
        if !left.is_finite() || !right.is_finite() || left < 0.0 || right < 0.0 {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            let active_bounds = self.outline_bounds_for_glyph(name);
            let layer_bounds: HashMap<String, (f64, f64, f64, f64)> = self
                .glyphs
                .get(name)
                .into_iter()
                .flat_map(|glyph| glyph.layers.keys())
                .filter_map(|master_id| {
                    let mut snapshot = self.clone();
                    for other in snapshot.glyphs.values_mut() {
                        if let Some(layer) = other.layers.get(master_id).cloned() {
                            other.width = layer.width;
                            other.contours = layer.contours;
                            other.components = layer.components;
                            other.anchors = layer.anchors;
                        }
                    }
                    snapshot
                        .outline_bounds_for_glyph(name)
                        .map(|bounds| (master_id.clone(), bounds))
                })
                .collect();
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let mut glyph_changed = false;
            let adjust = |layer: &mut GlyphLayer, bounds: Option<(f64, f64)>| {
                let contour_min = layer
                    .contours
                    .iter()
                    .flat_map(|contour| contour.points.iter())
                    .map(|point| point.x)
                    .min_by(f64::total_cmp);
                let contour_max = layer
                    .contours
                    .iter()
                    .flat_map(|contour| contour.points.iter())
                    .map(|point| point.x)
                    .max_by(f64::total_cmp);
                let Some((min_x, max_x)) = bounds.or_else(|| contour_min.zip(contour_max)) else {
                    return false;
                };
                let dx = left - min_x;
                if dx.abs() > f64::EPSILON {
                    for contour in &mut layer.contours {
                        for point in &mut contour.points {
                            point.x += dx;
                        }
                    }
                    for component in &mut layer.components {
                        component.x_offset += dx;
                    }
                    for anchor in &mut layer.anchors {
                        anchor.x += dx;
                    }
                }
                let width = (max_x - min_x) + left + right;
                let width_changed = (layer.width - width).abs() > f64::EPSILON;
                layer.width = width.max(0.0);
                dx.abs() > f64::EPSILON || width_changed
            };
            let mut active = glyph.layer_snapshot();
            glyph_changed |= adjust(
                &mut active,
                active_bounds.map(|(min, _, max, _)| (min, max)),
            );
            glyph.width = active.width;
            glyph.contours = active.contours;
            glyph.components = active.components;
            glyph.anchors = active.anchors;
            for (master_id, layer) in glyph.layers.iter_mut() {
                glyph_changed |= adjust(
                    layer,
                    layer_bounds
                        .get(master_id)
                        .map(|(min, _, max, _)| (*min, *max)),
                );
            }
            if glyph_changed {
                changed += 1;
            }
        }
        changed
    }

    /// Applies metric-key references (for example `=H`) to the selected
    /// glyphs. Each side can reference a different glyph; empty keys leave
    /// that side unchanged. Reference bearings are resolved independently for
    /// every available master layer.
    pub fn apply_metrics_keys(&mut self, names: &[String]) -> Result<usize, String> {
        let parse_key = |key: &str| {
            let key = key.trim();
            if key.is_empty() {
                None
            } else {
                key.strip_prefix('=')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            }
        };
        for name in names {
            let glyph = self
                .glyphs
                .get(name)
                .ok_or_else(|| format!("グリフ '{}' がありません", name))?;
            for key in [&glyph.left_metrics_key, &glyph.right_metrics_key] {
                if let Some(reference) = parse_key(key) {
                    if reference == *name {
                        return Err(format!("グリフ '{}' は自分自身を基準にできません", name));
                    }
                    if !self.glyphs.contains_key(&reference) {
                        return Err(format!(
                            "メトリクスキーの基準グリフ '{}' がありません",
                            reference
                        ));
                    }
                } else if !key.trim().is_empty() {
                    return Err(format!(
                        "グリフ '{}' のメトリクスキーは =グリフ名 形式です",
                        name
                    ));
                }
            }
        }

        let bounds_to_bearings = |project: &FontProject, name: &str| {
            let (min_x, _, max_x, _) = project.outline_bounds_for_glyph(name)?;
            let width = project.glyphs.get(name)?.width;
            Some((min_x, width - max_x))
        };
        let mut changed = 0;
        for name in names {
            let (left_key, right_key) = {
                let glyph = self.glyphs.get(name).expect("validated glyph");
                (
                    parse_key(&glyph.left_metrics_key),
                    parse_key(&glyph.right_metrics_key),
                )
            };
            if left_key.is_none() && right_key.is_none() {
                continue;
            }

            // Apply the active view first, preserving any side without a key.
            let current = bounds_to_bearings(self, name);
            let left = left_key
                .as_deref()
                .and_then(|reference| bounds_to_bearings(self, reference))
                .map(|value| value.0)
                .or_else(|| current.map(|value| value.0));
            let right = right_key
                .as_deref()
                .and_then(|reference| bounds_to_bearings(self, reference))
                .map(|value| value.1)
                .or_else(|| current.map(|value| value.1));
            if let (Some(left), Some(right)) = (left, right) {
                self.set_side_bearings(std::slice::from_ref(name), left.max(0.0), right.max(0.0));
            }

            let master_ids: Vec<String> = self
                .glyphs
                .get(name)
                .map(|glyph| glyph.layers.keys().cloned().collect())
                .unwrap_or_default();
            for master_id in master_ids {
                let mut snapshot = self.clone();
                for glyph in snapshot.glyphs.values_mut() {
                    if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                        glyph.width = layer.width;
                        glyph.contours = layer.contours;
                        glyph.components = layer.components;
                        glyph.anchors = layer.anchors;
                    }
                }
                let target_bounds = bounds_to_bearings(&snapshot, name);
                let reference_left = left_key
                    .as_deref()
                    .and_then(|reference| bounds_to_bearings(&snapshot, reference))
                    .map(|value| value.0);
                let reference_right = right_key
                    .as_deref()
                    .and_then(|reference| bounds_to_bearings(&snapshot, reference))
                    .map(|value| value.1);
                let Some((current_left, current_right)) = target_bounds else {
                    continue;
                };
                let desired_left = reference_left.unwrap_or(current_left).max(0.0);
                let desired_right = reference_right.unwrap_or(current_right).max(0.0);
                if let Some(glyph) = self.glyphs.get_mut(name) {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        let min_x = current_left;
                        let max_x = layer.width - current_right;
                        let dx = desired_left - min_x;
                        for contour in &mut layer.contours {
                            for point in &mut contour.points {
                                point.x += dx;
                            }
                        }
                        for component in &mut layer.components {
                            component.x_offset += dx;
                        }
                        for anchor in &mut layer.anchors {
                            anchor.x += dx;
                        }
                        layer.width = (max_x - min_x + desired_left + desired_right).max(0.0);
                    }
                }
            }
            changed += 1;
        }
        Ok(changed)
    }

    /// 複数グリフの左右余白を検証してからまとめて設定する。
    pub fn set_side_bearings_batch<I, N>(&mut self, rows: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (N, f64, f64)>,
        N: AsRef<str>,
    {
        let rows: Vec<(String, f64, f64)> = rows
            .into_iter()
            .map(|(name, left, right)| (name.as_ref().to_string(), left, right))
            .collect();
        for (name, left, right) in &rows {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if !left.is_finite() || !right.is_finite() || *left < 0.0 || *right < 0.0 {
                return Err(format!("グリフ '{}' の余白が不正です", name));
            }
        }
        let count = rows.len();
        for (name, left, right) in rows {
            self.set_side_bearings(&[name], left, right);
        }
        Ok(count)
    }

    pub fn translate_glyphs(&mut self, names: &[String], dx: f64, dy: f64) -> usize {
        if !dx.is_finite() || !dy.is_finite() {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                glyph.translate_geometry(dx, dy);
                changed += 1;
            }
        }
        changed
    }

    pub fn fit_widths_to_outlines(&mut self, names: &[String]) -> usize {
        let widths: Vec<(String, f64, HashMap<String, f64>)> = names
            .iter()
            .filter_map(|name| {
                let (_, _, max_x, _) = self.outline_bounds_for_glyph(name)?;
                let glyph = self.glyphs.get(name)?;
                let mut layer_widths = HashMap::new();
                for master_id in glyph.layers.keys() {
                    let mut snapshot = self.clone();
                    for other in snapshot.glyphs.values_mut() {
                        if let Some(layer) = other.layers.get(master_id).cloned() {
                            other.width = layer.width;
                            other.contours = layer.contours;
                            other.components = layer.components;
                            other.anchors = layer.anchors;
                        }
                    }
                    if let Some((_, _, layer_max_x, _)) = snapshot.outline_bounds_for_glyph(name) {
                        layer_widths.insert(master_id.clone(), layer_max_x.max(0.0));
                    }
                }
                Some((name.clone(), max_x.max(0.0), layer_widths))
            })
            .collect();
        let mut changed = 0;
        for (name, width, layer_widths) in widths {
            if let Some(glyph) = self.glyphs.get_mut(&name) {
                let mut glyph_changed = (glyph.width - width).abs() > f64::EPSILON;
                glyph.width = width;
                for (master_id, layer_width) in layer_widths {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        glyph_changed |= (layer.width - layer_width).abs() > f64::EPSILON;
                        layer.width = layer_width;
                    }
                }
                if glyph_changed {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn center_glyphs_in_width(&mut self, names: &[String]) -> usize {
        let shifts: Vec<(String, f64, HashMap<String, f64>)> = names
            .iter()
            .filter_map(|name| {
                let (min_x, _, max_x, _) = self.outline_bounds_for_glyph(name)?;
                let glyph = self.glyphs.get(name)?;
                let active_shift = (glyph.width - (max_x - min_x)) * 0.5 - min_x;
                let mut layer_shifts = HashMap::new();
                for master_id in glyph.layers.keys() {
                    let mut snapshot = self.clone();
                    for other in snapshot.glyphs.values_mut() {
                        if let Some(layer) = other.layers.get(master_id).cloned() {
                            other.width = layer.width;
                            other.contours = layer.contours;
                            other.components = layer.components;
                            other.anchors = layer.anchors;
                        }
                    }
                    if let Some((layer_min_x, _, layer_max_x, _)) =
                        snapshot.outline_bounds_for_glyph(name)
                    {
                        let layer_width = glyph.layers[master_id].width;
                        layer_shifts.insert(
                            master_id.clone(),
                            (layer_width - (layer_max_x - layer_min_x)) * 0.5 - layer_min_x,
                        );
                    }
                }
                Some((name.clone(), active_shift, layer_shifts))
            })
            .collect();
        let mut changed = 0;
        for (name, active_shift, layer_shifts) in shifts {
            if let Some(glyph) = self.glyphs.get_mut(&name) {
                let mut glyph_changed = active_shift.abs() > f64::EPSILON;
                glyph.translate_authored_geometry(active_shift, 0.0);
                for (master_id, shift) in layer_shifts {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        glyph_changed |= shift.abs() > f64::EPSILON;
                        layer.translate_geometry(shift, 0.0);
                    }
                }
                if glyph_changed {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn align_all_component_anchors(&mut self, names: &[String]) -> usize {
        let targets: Vec<(String, usize)> = names
            .iter()
            .flat_map(|name| {
                let count = self
                    .glyphs
                    .get(name)
                    .map(|glyph| glyph.components.len())
                    .unwrap_or(0);
                (0..count).map(|index| (name.clone(), index))
            })
            .collect();
        let mut changed = 0;
        for (name, index) in targets {
            if self.align_component_anchors_all_layers(&name, index) {
                changed += 1;
            }
        }
        changed
    }

    /// Duplicates one component in the glyph and every saved master layer.
    pub fn duplicate_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> bool {
        let Some(component) = self
            .glyphs
            .get(glyph_name)
            .and_then(|glyph| glyph.components.get(component_index))
            .cloned()
        else {
            return false;
        };
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return false;
        };
        glyph.components.push(component.clone());
        for layer in glyph.layers.values_mut() {
            layer.components.push(component.clone());
        }
        true
    }

    /// Adds one component to the authored geometry and every saved master
    /// layer, keeping composite component indices aligned for interpolation.
    pub fn add_component_all_layers(
        &mut self,
        glyph_name: &str,
        component: GlyphComponent,
    ) -> Option<usize> {
        let glyph = self.glyphs.get_mut(glyph_name)?;
        let index = glyph.components.len();
        glyph.components.push(component.clone());
        for layer in glyph.layers.values_mut() {
            layer.components.push(component.clone());
        }
        Some(index)
    }

    /// Removes one component from the authored geometry and every saved master
    /// layer. Refuses partial removal so master component indices stay aligned.
    pub fn remove_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> Result<(), String> {
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return Err("対象グリフがありません".to_string());
        };
        if component_index >= glyph.components.len()
            || glyph
                .layers
                .values()
                .any(|layer| component_index >= layer.components.len())
        {
            return Err("マスター間で部品構造が一致しません".to_string());
        }
        glyph.components.remove(component_index);
        for layer in glyph.layers.values_mut() {
            layer.components.remove(component_index);
        }
        Ok(())
    }

    /// Reorders one component in the authored geometry and every saved master
    /// layer by one position.
    pub fn move_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
        delta: isize,
    ) -> Result<(), String> {
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return Err("対象グリフがありません".to_string());
        };
        let target = component_index as isize + delta;
        if component_index >= glyph.components.len()
            || target < 0
            || target as usize >= glyph.components.len()
            || glyph.layers.values().any(|layer| {
                component_index >= layer.components.len()
                    || target as usize >= layer.components.len()
            })
        {
            return Err("マスター間で部品構造が一致しません".to_string());
        }
        let target = target as usize;
        glyph.components.swap(component_index, target);
        for layer in glyph.layers.values_mut() {
            layer.components.swap(component_index, target);
        }
        Ok(())
    }

    /// Appends one contour to the authored geometry and every saved master
    /// layer, preserving contour indices for interpolation.
    pub fn add_contour_all_layers(&mut self, glyph_name: &str, contour: Contour) -> Option<usize> {
        let glyph = self.glyphs.get_mut(glyph_name)?;
        let index = glyph.contours.len();
        glyph.contours.push(contour.clone());
        for layer in glyph.layers.values_mut() {
            layer.contours.push(contour.clone());
        }
        Some(index)
    }

    /// Duplicates one contour in the authored geometry and every saved master
    /// layer. The copy uses the authored contour, matching the active editor.
    pub fn duplicate_contour_all_layers(
        &mut self,
        glyph_name: &str,
        contour_index: usize,
    ) -> Option<usize> {
        let contour = self
            .glyphs
            .get(glyph_name)
            .and_then(|glyph| glyph.contours.get(contour_index))
            .cloned()?;
        self.add_contour_all_layers(glyph_name, contour)
    }

    /// Removes one contour from the authored geometry and every saved master
    /// layer through the project-level API used by clipboard actions.
    pub fn remove_contour_all_layers(
        &mut self,
        glyph_name: &str,
        contour_index: usize,
    ) -> Result<(), String> {
        self.glyphs
            .get_mut(glyph_name)
            .ok_or_else(|| "対象グリフがありません".to_string())?
            .remove_contour_all_layers(contour_index)
    }

    /// Aligns a component against matching anchors in the active geometry and
    /// in every saved master layer.
    pub fn align_component_anchors_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> bool {
        let mut changed = self.align_component_anchors(glyph_name, component_index);
        let master_ids: Vec<String> = self
            .glyphs
            .get(glyph_name)
            .into_iter()
            .flat_map(|glyph| glyph.layers.keys().cloned())
            .collect();
        for master_id in master_ids {
            let mut snapshot = self.clone();
            for glyph in snapshot.glyphs.values_mut() {
                if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                    glyph.width = layer.width;
                    glyph.contours = layer.contours;
                    glyph.components = layer.components;
                    glyph.anchors = layer.anchors;
                }
            }
            if snapshot.align_component_anchors(glyph_name, component_index) {
                if let Some(component) = snapshot
                    .glyphs
                    .get(glyph_name)
                    .and_then(|glyph| glyph.components.get(component_index))
                    .cloned()
                {
                    if let Some(layer) = self
                        .glyphs
                        .get_mut(glyph_name)
                        .and_then(|glyph| glyph.layers.get_mut(&master_id))
                    {
                        if let Some(target) = layer.components.get_mut(component_index) {
                            target.x_offset = component.x_offset;
                            target.y_offset = component.y_offset;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }

    pub fn reverse_glyph_contours(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                for contour in &mut glyph.contours {
                    contour.reverse_direction();
                }
                for layer in glyph.layers.values_mut() {
                    for contour in &mut layer.contours {
                        contour.reverse_direction();
                    }
                }
                changed += 1;
            }
        }
        changed
    }

    /// Removes consecutive duplicate nodes from selected glyphs and all layers.
    /// Contours that would become invalid are left untouched.
    pub fn remove_duplicate_nodes(&mut self, names: &[String]) -> usize {
        let mut removed = 0;
        let mut clean = |contours: &mut Vec<Contour>| {
            for contour in contours {
                if contour.points.len() < 3 {
                    continue;
                }
                let original_points = std::mem::take(&mut contour.points);
                let original = original_points.len();
                let mut points = Vec::with_capacity(original);
                for point in original_points.iter().copied() {
                    if points
                        .last()
                        .is_some_and(|last: &ContourPoint| last.x == point.x && last.y == point.y)
                    {
                        continue;
                    }
                    points.push(point);
                }
                if points.len() > 1
                    && points.first().is_some_and(|first| {
                        points
                            .last()
                            .is_some_and(|last| first.x == last.x && first.y == last.y)
                    })
                {
                    points.pop();
                }
                if points.len() >= 3 {
                    removed += original - points.len();
                    contour.points = points;
                } else {
                    contour.points = original_points;
                }
            }
        };
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                clean(&mut glyph.contours);
                for layer in glyph.layers.values_mut() {
                    clean(&mut layer.contours);
                }
            }
        }
        removed
    }

    fn normalize_contour_list(contours: &mut [Contour]) -> bool {
        let polygons: Vec<Vec<Point>> = contours
            .iter()
            .map(|contour| {
                let mut polygon = Vec::new();
                flatten(contour.to_bezpath(), 0.25, |element| {
                    if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                        polygon.push(point);
                    }
                });
                polygon
            })
            .collect();
        let contains = |polygon: &[Point], point: Point| {
            if polygon.len() < 3 {
                return false;
            }
            let mut inside = false;
            for (a, b) in polygon
                .iter()
                .zip(polygon.iter().cycle().skip(1))
                .take(polygon.len())
            {
                if (a.y > point.y) != (b.y > point.y)
                    && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
                {
                    inside = !inside;
                }
            }
            inside
        };
        let mut changed = false;
        for index in 0..contours.len() {
            let Some(&probe) = polygons[index].first() else {
                continue;
            };
            let depth = polygons
                .iter()
                .enumerate()
                .filter(|(other, polygon)| *other != index && contains(polygon, probe))
                .count();
            let area = contours[index].signed_area();
            let wants_positive = depth % 2 == 1;
            if (wants_positive && area < 0.0) || (!wants_positive && area > 0.0) {
                contours[index].reverse_direction();
                changed = true;
            }
        }
        changed
    }

    /// Normalizes outer contours and nested counters in every layer.
    pub fn normalize_glyph_winding(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let mut glyph_changed = Self::normalize_contour_list(&mut glyph.contours);
            for layer in glyph.layers.values_mut() {
                glyph_changed |= Self::normalize_contour_list(&mut layer.contours);
            }
            if glyph_changed {
                changed += 1;
            }
        }
        changed
    }

    /// Returns the bounds of authored outlines and recursively referenced
    /// components in glyph coordinates.
    pub fn outline_bounds_for_glyph(&self, name: &str) -> Option<(f64, f64, f64, f64)> {
        fn collect(
            project: &FontProject,
            name: &str,
            transform: (f64, f64, f64, f64, f64, f64),
            stack: &mut std::collections::HashSet<String>,
            bounds: &mut Option<(f64, f64, f64, f64)>,
        ) {
            let Some(glyph) = project.glyphs.get(name) else {
                return;
            };
            if !stack.insert(name.to_string()) {
                return;
            }
            for point in glyph
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter())
            {
                let x = transform.0 * point.x + transform.1 * point.y + transform.4;
                let y = transform.2 * point.x + transform.3 * point.y + transform.5;
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                *bounds = Some(match *bounds {
                    Some((min_x, min_y, max_x, max_y)) => {
                        (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                    }
                    None => (x, y, x, y),
                });
            }
            for component in &glyph.components {
                let next = (
                    transform.0 * component.x_scale + transform.1 * component.yx_scale,
                    transform.0 * component.xy_scale + transform.1 * component.y_scale,
                    transform.2 * component.x_scale + transform.3 * component.yx_scale,
                    transform.2 * component.xy_scale + transform.3 * component.y_scale,
                    transform.0 * component.x_offset
                        + transform.1 * component.y_offset
                        + transform.4,
                    transform.2 * component.x_offset
                        + transform.3 * component.y_offset
                        + transform.5,
                );
                collect(project, &component.base, next, stack, bounds);
            }
            stack.remove(name);
        }
        let mut bounds = None;
        collect(
            self,
            name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
            &mut bounds,
        );
        bounds
    }

    pub fn anchors_for_glyph(&self, name: &str) -> Vec<GlyphAnchor> {
        fn collect(
            project: &FontProject,
            name: &str,
            transform: (f64, f64, f64, f64, f64, f64),
            stack: &mut std::collections::HashSet<String>,
            output: &mut Vec<GlyphAnchor>,
        ) {
            let Some(glyph) = project.glyphs.get(name) else {
                return;
            };
            if !stack.insert(name.to_string()) {
                return;
            }
            let map = |x: f64, y: f64| {
                (
                    transform.0 * x + transform.1 * y + transform.4,
                    transform.2 * x + transform.3 * y + transform.5,
                )
            };
            for anchor in &glyph.anchors {
                let (x, y) = map(anchor.x, anchor.y);
                output.push(GlyphAnchor {
                    name: anchor.name.clone(),
                    x,
                    y,
                });
            }
            for component in &glyph.components {
                let next = (
                    transform.0 * component.x_scale + transform.1 * component.yx_scale,
                    transform.0 * component.xy_scale + transform.1 * component.y_scale,
                    transform.2 * component.x_scale + transform.3 * component.yx_scale,
                    transform.2 * component.xy_scale + transform.3 * component.y_scale,
                    transform.0 * component.x_offset
                        + transform.1 * component.y_offset
                        + transform.4,
                    transform.2 * component.x_offset
                        + transform.3 * component.y_offset
                        + transform.5,
                );
                collect(project, &component.base, next, stack, output);
            }
            stack.remove(name);
        }
        let mut output = Vec::new();
        collect(
            self,
            name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
            &mut output,
        );
        let base_names: std::collections::HashSet<String> = output
            .iter()
            .filter(|anchor| !anchor.name.starts_with('_'))
            .map(|anchor| anchor.name.clone())
            .collect();
        output.retain(|anchor| {
            !anchor.name.starts_with('_')
                || !base_names.contains(anchor.name.trim_start_matches('_'))
        });
        let mut seen = std::collections::HashSet::new();
        output.retain(|anchor| seen.insert(anchor.name.clone()));
        output
    }

    pub fn kerning_for_glyphs(&self, left: &str, right: &str) -> Option<f64> {
        self.kerning_source_for_glyphs(left, right)
            .map(|(_, value)| value)
    }

    /// グリフの実効カーニング値と、その値を提供している保存キーを返す。
    pub fn kerning_source_for_glyphs(
        &self,
        left: &str,
        right: &str,
    ) -> Option<((String, String), f64)> {
        if let Some(value) = self.kerning.get(&(left.to_string(), right.to_string())) {
            return Some(((left.to_string(), right.to_string()), *value));
        }
        let left_group = self.glyphs.get(left)?.left_kerning_group.trim();
        let right_group = self.glyphs.get(right)?.right_kerning_group.trim();
        if left_group.is_empty() || right_group.is_empty() {
            return None;
        }
        self.kerning
            .iter()
            .filter_map(|((pair_left, pair_right), value)| {
                let pair_left_group = self.glyphs.get(pair_left)?.left_kerning_group.trim();
                let pair_right_group = self.glyphs.get(pair_right)?.right_kerning_group.trim();
                (pair_left_group == left_group && pair_right_group == right_group)
                    .then_some(((pair_left.clone(), pair_right.clone()), *value))
            })
            .min_by(|(a, _), (b, _)| a.cmp(b))
    }

    /// グリフまたはグループのカーニングペアを設定する。
    pub fn set_kerning_pair(
        &mut self,
        left: impl Into<String>,
        right: impl Into<String>,
        value: f64,
    ) -> Result<(), String> {
        if !value.is_finite() {
            return Err("カーニング値が不正です".into());
        }
        let left = left.into();
        let right = right.into();
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err("カーニングの左右キーを空にできません".into());
        }
        self.kerning.insert((left, right), value);
        Ok(())
    }

    pub fn set_kerning_pair_for_master(
        &mut self,
        master_id: &str,
        left: impl Into<String>,
        right: impl Into<String>,
        value: f64,
    ) -> Result<(), String> {
        if !self.masters.iter().any(|master| master.id == master_id) {
            return Err(format!("マスター '{}' がありません", master_id));
        }
        if !value.is_finite() {
            return Err("カーニング値が不正です".into());
        }
        let left = left.into();
        let right = right.into();
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err("カーニングの左右キーを空にできません".into());
        }
        let fallback = self.kerning.clone();
        let pairs = self
            .kerning_by_master
            .entry(master_id.to_string())
            .or_insert(fallback);
        pairs.insert((left.clone(), right.clone()), value);
        if master_id == self.default_master_id {
            self.kerning.insert((left, right), value);
        }
        Ok(())
    }

    /// 指定したカーニングペアを削除し、削除できたかを返す。
    pub fn remove_kerning_pair(&mut self, left: &str, right: &str) -> bool {
        self.kerning
            .remove(&(left.to_string(), right.to_string()))
            .is_some()
    }

    /// 複数のカーニングペアを検証してからまとめて設定する。
    pub fn set_kerning_pairs<I, L, R>(&mut self, pairs: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (L, R, f64)>,
        L: AsRef<str>,
        R: AsRef<str>,
    {
        let pairs: Vec<(String, String, f64)> = pairs
            .into_iter()
            .map(|(left, right, value)| {
                (left.as_ref().to_string(), right.as_ref().to_string(), value)
            })
            .collect();
        for (left, right, value) in &pairs {
            if !value.is_finite() {
                return Err("カーニング値が不正です".into());
            }
            if left.trim().is_empty() || right.trim().is_empty() {
                return Err("カーニングの左右キーを空にできません".into());
            }
        }
        let count = pairs.len();
        for (left, right, value) in pairs {
            self.kerning.insert((left, right), value);
        }
        Ok(count)
    }

    pub fn feature_source(&self) -> String {
        match (self.opentype_classes.trim(), self.opentype_features.trim()) {
            ("", features) => features.to_string(),
            (classes, "") => classes.to_string(),
            (classes, features) => format!("{classes}\n\n{features}"),
        }
    }

    pub fn conditional_layer_for_glyph(
        &self,
        glyph_name: &str,
        axis_values: &HashMap<String, f64>,
    ) -> Option<&ConditionalLayer> {
        self.conditional_layers
            .get(glyph_name)?
            .iter()
            .filter(|layer| {
                layer.conditions.iter().all(|(tag, range)| {
                    let value = axis_values.get(tag).or_else(|| {
                        axis_values
                            .iter()
                            .find(|(axis, _)| axis.eq_ignore_ascii_case(tag))
                            .map(|(_, value)| value)
                    });
                    let Some(value) = value else {
                        return false;
                    };
                    range.min.is_none_or(|min| *value >= min)
                        && range.max.is_none_or(|max| *value <= max)
                })
            })
            .max_by(|left, right| {
                let condition_order = left.conditions.len().cmp(&right.conditions.len());
                if condition_order != std::cmp::Ordering::Equal {
                    return condition_order;
                }
                let span = |layer: &ConditionalLayer| {
                    layer.conditions.values().fold(0.0, |total, range| {
                        total
                            + match (range.min, range.max) {
                                (Some(min), Some(max)) => (max - min).max(0.0),
                                _ => f64::INFINITY,
                            }
                    })
                };
                span(right)
                    .partial_cmp(&span(left))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn new() -> Self {
        Self {
            metadata: FontMetadata::default(),
            glyphs: HashMap::new(),
            kerning: HashMap::new(),
            kerning_by_master: HashMap::new(),
            vertical_metrics: HashMap::new(),
            vertical_metrics_by_master: HashMap::new(),
            metrics_by_master: HashMap::new(),
            color_layers: HashMap::new(),
            color_layer_transforms: HashMap::new(),
            color_palettes: Vec::new(),
            color_palette_names: Vec::new(),
            color_palette_types: Vec::new(),
            color_palette_entry_names: Vec::new(),
            glyph_order: Vec::new(),
            masters: default_masters(),
            instances: Vec::new(),
            default_master_id: "regular".to_string(),
            axis_names: HashMap::new(),
            axis_flags: HashMap::new(),
            axis_mappings: HashMap::new(),
            opentype_features: String::new(),
            opentype_classes: String::new(),
            background_images: HashMap::new(),
            background_opacities: HashMap::new(),
            background_transforms: HashMap::new(),
            guidelines: Vec::new(),
            guidelines_by_master: HashMap::new(),
            conditional_layers: HashMap::new(),
            unicode_variation_sequences: Vec::new(),
            preserved_tables: HashMap::new(),
            preserved_layout_source: None,
            preserved_layout_fingerprint: None,
        }
    }

    pub fn add_glyph(&mut self, name: String, unicode: Option<u32>) {
        if !self.glyphs.contains_key(&name) {
            self.glyphs
                .insert(name.clone(), GlyphData::new(name.clone(), unicode));
            self.glyph_order.push(name);
        }
    }

    pub fn guidelines_for_master(&self, master_id: &str) -> &[Guideline] {
        self.guidelines_by_master
            .get(master_id)
            .map(Vec::as_slice)
            .unwrap_or(&self.guidelines)
    }

    pub fn guidelines_for_master_mut(&mut self, master_id: &str) -> &mut Vec<Guideline> {
        if !self.guidelines_by_master.contains_key(master_id) {
            self.guidelines_by_master
                .insert(master_id.to_string(), self.guidelines.clone());
        }
        self.guidelines_by_master
            .get_mut(master_id)
            .expect("global guide entry inserted")
    }

    pub fn remove_master(&mut self, master_id: &str) -> bool {
        if self.masters.len() <= 1 || !self.masters.iter().any(|master| master.id == master_id) {
            return false;
        }
        let fallback = self
            .masters
            .iter()
            .find(|master| master.id != master_id)
            .map(|master| master.id.clone())
            .expect("at least one master remains");
        self.masters.retain(|master| master.id != master_id);
        let remaining_axis_tags: std::collections::HashSet<String> = self
            .masters
            .iter()
            .flat_map(|master| master.axes.keys().cloned())
            .collect();
        self.axis_names
            .retain(|tag, _| remaining_axis_tags.contains(tag));
        for glyph in self.glyphs.values_mut() {
            glyph.layers.remove(master_id);
            glyph.master_guidelines.remove(master_id);
        }
        self.vertical_metrics_by_master.remove(master_id);
        self.metrics_by_master.remove(master_id);
        self.kerning_by_master.remove(master_id);
        self.guidelines_by_master.remove(master_id);
        for masters in self.background_images.values_mut() {
            masters.remove(master_id);
        }
        self.background_images
            .retain(|_, masters| !masters.is_empty());
        for masters in self.background_opacities.values_mut() {
            masters.remove(master_id);
        }
        self.background_opacities
            .retain(|_, masters| !masters.is_empty());
        for masters in self.background_transforms.values_mut() {
            masters.remove(master_id);
        }
        self.background_transforms
            .retain(|_, masters| !masters.is_empty());
        if self.default_master_id == master_id {
            self.default_master_id = fallback;
        }
        true
    }

    /// Moves a master in the project order without changing its identity or
    /// any glyph layer data. The order is also used when presenting and
    /// exporting master-specific results.
    pub fn move_master(&mut self, master_id: &str, delta: isize) -> bool {
        let Some(index) = self
            .masters
            .iter()
            .position(|master| master.id == master_id)
        else {
            return false;
        };
        let target = index as isize + delta;
        if !(0..self.masters.len() as isize).contains(&target) || target == index as isize {
            return false;
        }
        self.masters.swap(index, target as usize);
        true
    }

    /// Duplicates a master, including its design-space metadata and every
    /// glyph layer. The new master is inserted immediately after its source.
    pub fn duplicate_master(&mut self, source_master_id: &str) -> Option<String> {
        let source_index = self
            .masters
            .iter()
            .position(|master| master.id == source_master_id)?;
        let source = self.masters[source_index].clone();
        let mut suffix = 1;
        let new_id = loop {
            let candidate = format!("{}.copy{}", source.id, suffix);
            if !self.masters.iter().any(|master| master.id == candidate) {
                break candidate;
            }
            suffix += 1;
        };
        let mut duplicate = source.clone();
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} Copy", source.name);
        self.masters.insert(source_index + 1, duplicate);
        let source_is_default = source_master_id == self.default_master_id;

        if let Some(pairs) = self.kerning_by_master.get(source_master_id).cloned() {
            self.kerning_by_master.insert(new_id.clone(), pairs);
        } else if source_is_default {
            self.kerning_by_master
                .insert(new_id.clone(), self.kerning.clone());
        }
        if let Some(guides) = self.guidelines_by_master.get(source_master_id).cloned() {
            self.guidelines_by_master.insert(new_id.clone(), guides);
        } else if source_is_default {
            self.guidelines_by_master
                .insert(new_id.clone(), self.guidelines.clone());
        }

        for glyph in self.glyphs.values_mut() {
            if let Some(layer) = glyph
                .layers
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.layer_snapshot()))
            {
                glyph.layers.insert(new_id.clone(), layer);
            }
            let source_guidelines = glyph
                .master_guidelines
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.guidelines.clone()))
                .unwrap_or_default();
            glyph
                .master_guidelines
                .insert(new_id.clone(), source_guidelines);
        }
        Some(new_id)
    }

    pub fn duplicate_glyph(&mut self, source_name: &str) -> Option<String> {
        let mut duplicate = self.glyphs.get(source_name)?.clone();
        let mut suffix = 1;
        let new_name = loop {
            let candidate = format!("{source_name}.copy{suffix}");
            if !self.glyphs.contains_key(&candidate) {
                break candidate;
            }
            suffix += 1;
        };
        duplicate.name = new_name.clone();
        duplicate.unicode = None;
        duplicate.unicodes.clear();
        self.glyph_order.push(new_name.clone());
        self.glyphs.insert(new_name.clone(), duplicate);
        if let Some(layers) = self.color_layers.get(source_name).cloned() {
            self.color_layers.insert(new_name.clone(), layers);
        }
        if let Some(transforms) = self.color_layer_transforms.get(source_name).cloned() {
            self.color_layer_transforms
                .insert(new_name.clone(), transforms);
        }
        if let Some(layers) = self.conditional_layers.get(source_name).cloned() {
            self.conditional_layers.insert(new_name.clone(), layers);
        }
        if let Some(metrics) = self.vertical_metrics.get(source_name).copied() {
            self.vertical_metrics.insert(new_name.clone(), metrics);
        }
        if let Some(images) = self.background_images.get(source_name).cloned() {
            self.background_images.insert(new_name.clone(), images);
        }
        if let Some(opacities) = self.background_opacities.get(source_name).cloned() {
            self.background_opacities
                .insert(new_name.clone(), opacities);
        }
        if let Some(transforms) = self.background_transforms.get(source_name).cloned() {
            self.background_transforms
                .insert(new_name.clone(), transforms);
        }
        for metrics in self.vertical_metrics_by_master.values_mut() {
            if let Some(value) = metrics.get(source_name).copied() {
                metrics.insert(new_name.clone(), value);
            }
        }
        Some(new_name)
    }

    pub fn remove_glyph(&mut self, name: &str) {
        self.glyphs.remove(name);
        self.glyph_order.retain(|item| item != name);
        self.vertical_metrics.remove(name);
        for metrics in self.vertical_metrics_by_master.values_mut() {
            metrics.remove(name);
        }
        self.vertical_metrics_by_master
            .retain(|_, metrics| !metrics.is_empty());
        self.color_layers.remove(name);
        self.color_layer_transforms.remove(name);
        self.conditional_layers.remove(name);
        self.background_images.remove(name);
        self.background_opacities.remove(name);
        self.background_transforms.remove(name);
        for layers in self.color_layers.values_mut() {
            layers.retain(|layer| layer.glyph != name);
        }
        self.color_layers.retain(|_, layers| !layers.is_empty());
        self.kerning
            .retain(|(left, right), _| left != name && right != name);
        for pairs in self.kerning_by_master.values_mut() {
            pairs.retain(|(left, right), _| left != name && right != name);
        }
        self.kerning_by_master.retain(|_, pairs| !pairs.is_empty());
        for glyph in self.glyphs.values_mut() {
            glyph.components.retain(|component| component.base != name);
            for layer in glyph.layers.values_mut() {
                layer.components.retain(|component| component.base != name);
            }
        }
        self.opentype_features =
            rewrite_feature_glyph_name(&self.opentype_features, name, ".notdef");
        self.opentype_classes = rewrite_feature_glyph_name(&self.opentype_classes, name, ".notdef");
        self.unicode_variation_sequences
            .retain(|sequence| sequence.glyph != name);
    }

    pub fn rename_glyph(&mut self, old_name: &str, new_name: String) -> bool {
        if old_name == new_name || new_name.trim().is_empty() || self.glyphs.contains_key(&new_name)
        {
            return false;
        }
        let Some(mut glyph) = self.glyphs.remove(old_name) else {
            return false;
        };
        glyph.name = new_name.clone();
        self.glyphs.insert(new_name.clone(), glyph);
        for name in &mut self.glyph_order {
            if name == old_name {
                *name = new_name.clone();
            }
        }
        let kerning = std::mem::take(&mut self.kerning);
        self.kerning = kerning
            .into_iter()
            .map(|((left, right), value)| {
                (
                    (
                        if left == old_name {
                            new_name.clone()
                        } else {
                            left
                        },
                        if right == old_name {
                            new_name.clone()
                        } else {
                            right
                        },
                    ),
                    value,
                )
            })
            .collect();
        for pairs in self.kerning_by_master.values_mut() {
            let old_pairs = std::mem::take(pairs);
            *pairs = old_pairs
                .into_iter()
                .map(|((left, right), value)| {
                    (
                        (
                            if left == old_name {
                                new_name.clone()
                            } else {
                                left
                            },
                            if right == old_name {
                                new_name.clone()
                            } else {
                                right
                            },
                        ),
                        value,
                    )
                })
                .collect();
        }
        for other in self.glyphs.values_mut() {
            for component in &mut other.components {
                if component.base == old_name {
                    component.base = new_name.clone();
                }
            }
            for layer in other.layers.values_mut() {
                for component in &mut layer.components {
                    if component.base == old_name {
                        component.base = new_name.clone();
                    }
                }
            }
        }
        if let Some(metrics) = self.vertical_metrics.remove(old_name) {
            self.vertical_metrics.insert(new_name.clone(), metrics);
        }
        for metrics in self.vertical_metrics_by_master.values_mut() {
            if let Some(value) = metrics.remove(old_name) {
                metrics.insert(new_name.clone(), value);
            }
        }
        if let Some(layers) = self.color_layers.remove(old_name) {
            self.color_layers.insert(new_name.clone(), layers);
        }
        if let Some(transforms) = self.color_layer_transforms.remove(old_name) {
            self.color_layer_transforms
                .insert(new_name.clone(), transforms);
        }
        if let Some(layers) = self.conditional_layers.remove(old_name) {
            self.conditional_layers.insert(new_name.clone(), layers);
        }
        if let Some(images) = self.background_images.remove(old_name) {
            self.background_images.insert(new_name.clone(), images);
        }
        if let Some(opacities) = self.background_opacities.remove(old_name) {
            self.background_opacities
                .insert(new_name.clone(), opacities);
        }
        if let Some(transforms) = self.background_transforms.remove(old_name) {
            self.background_transforms
                .insert(new_name.clone(), transforms);
        }
        for layers in self.color_layers.values_mut() {
            for layer in layers {
                if layer.glyph == old_name {
                    layer.glyph = new_name.clone();
                }
            }
        }
        self.opentype_features =
            rewrite_feature_glyph_name(&self.opentype_features, old_name, &new_name);
        self.opentype_classes =
            rewrite_feature_glyph_name(&self.opentype_classes, old_name, &new_name);
        for sequence in &mut self.unicode_variation_sequences {
            if sequence.glyph == old_name {
                sequence.glyph = new_name.clone();
            }
        }
        true
    }

    pub fn move_glyph(&mut self, name: &str, delta: isize) {
        let Some(index) = self.glyph_order.iter().position(|item| item == name) else {
            return;
        };
        let target = index as isize + delta;
        if target >= 0 && target < self.glyph_order.len() as isize {
            let item = self.glyph_order.remove(index);
            self.glyph_order.insert(target as usize, item);
        }
    }

    pub fn normalize_glyph_order(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.glyph_order
            .retain(|name| self.glyphs.contains_key(name) && seen.insert(name.clone()));
        let mut missing: Vec<String> = self
            .glyphs
            .keys()
            .filter(|name| !seen.contains(*name))
            .cloned()
            .collect();
        missing.sort();
        self.glyph_order.extend(missing);
    }

    pub fn normalize_masters(&mut self) {
        if self.masters.is_empty() {
            self.masters = default_masters();
        }
        let mut seen = std::collections::HashSet::new();
        self.masters
            .retain(|master| !master.id.trim().is_empty() && seen.insert(master.id.clone()));
        if self.masters.is_empty() {
            self.masters = default_masters();
        }
        let default_id = self.masters[0].id.clone();
        if !self
            .masters
            .iter()
            .any(|master| master.id == self.default_master_id)
        {
            self.default_master_id = default_id.clone();
        }
        let axis_tags: std::collections::HashSet<String> = self
            .masters
            .iter()
            .flat_map(|master| master.axes.keys().cloned())
            .collect();
        let axis_defaults = self
            .masters
            .iter()
            .find(|master| master.id == self.default_master_id)
            .or_else(|| self.masters.first())
            .map(|master| master.axes.clone())
            .unwrap_or_default();
        for master in &mut self.masters {
            for tag in &axis_tags {
                let default = axis_defaults.get(tag).copied().unwrap_or(0.0);
                master.axes.entry(tag.clone()).or_insert(default);
            }
        }
        self.axis_names.retain(|tag, _| axis_tags.contains(tag));
        self.guidelines_by_master
            .entry(default_id.clone())
            .or_insert_with(|| self.guidelines.clone());
        for glyph in self.glyphs.values_mut() {
            glyph.ensure_layer(&default_id);
        }
    }

    pub fn switch_master(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }
        let from_exists = self.masters.iter().any(|master| master.id == from_id);
        if from_exists {
            self.kerning_by_master
                .insert(from_id.to_string(), self.kerning.clone());
            self.guidelines_by_master
                .insert(from_id.to_string(), self.guidelines.clone());
        }
        self.kerning = self
            .kerning_by_master
            .get(to_id)
            .cloned()
            .unwrap_or_else(|| self.kerning.clone());
        self.kerning_by_master
            .entry(to_id.to_string())
            .or_insert_with(|| self.kerning.clone());
        self.guidelines = self
            .guidelines_by_master
            .get(to_id)
            .cloned()
            .unwrap_or_else(|| self.guidelines.clone());
        self.guidelines_by_master
            .entry(to_id.to_string())
            .or_insert_with(|| self.guidelines.clone());
        for glyph in self.glyphs.values_mut() {
            if from_exists {
                glyph.switch_layer(from_id, to_id);
            } else {
                glyph.switch_layer(to_id, to_id);
            }
        }
    }

    /// Persists the currently active geometry into its selected master layer.
    /// This keeps edits made before a master switch or export from becoming stale.
    pub fn sync_active_layer(&mut self, master_id: &str) {
        for glyph in self.glyphs.values_mut() {
            glyph
                .layers
                .insert(master_id.to_string(), glyph.layer_snapshot());
            let guides = glyph
                .master_guidelines
                .get(master_id)
                .cloned()
                .unwrap_or_else(|| glyph.guidelines.clone());
            glyph.guidelines = guides.clone();
            glyph
                .master_guidelines
                .insert(master_id.to_string(), guides);
        }
        self.kerning_by_master
            .insert(master_id.to_string(), self.kerning.clone());
        let guides = self
            .guidelines_by_master
            .get(master_id)
            .cloned()
            .unwrap_or_else(|| self.guidelines.clone());
        self.guidelines = guides.clone();
        self.guidelines_by_master
            .insert(master_id.to_string(), guides);
    }

    /// Copies one master layer to every other master, preserving glyph metadata.
    /// This is useful when adding a new master that should initially match a
    /// finished master before making weight/width-specific edits.
    pub fn copy_master_to_all(&mut self, source_master_id: &str) -> usize {
        let target_ids: Vec<String> = self
            .masters
            .iter()
            .filter(|master| master.id != source_master_id)
            .map(|master| master.id.clone())
            .collect();
        let source_is_default = source_master_id == self.default_master_id;
        let source_kerning = self
            .kerning_by_master
            .get(source_master_id)
            .cloned()
            .or_else(|| source_is_default.then(|| self.kerning.clone()))
            .unwrap_or_default();
        let source_guidelines = self
            .guidelines_by_master
            .get(source_master_id)
            .cloned()
            .or_else(|| source_is_default.then(|| self.guidelines.clone()))
            .unwrap_or_default();
        for target_id in target_ids {
            self.kerning_by_master
                .insert(target_id.clone(), source_kerning.clone());
            self.guidelines_by_master
                .insert(target_id, source_guidelines.clone());
        }
        let names: Vec<String> = self.glyphs.keys().cloned().collect();
        self.copy_master_to_all_for_glyphs(source_master_id, names.iter().map(String::as_str))
    }

    /// Copies one master layer to every other master for only the supplied glyphs.
    pub fn copy_master_to_all_for_glyphs<'a, I>(
        &mut self,
        source_master_id: &str,
        glyph_names: I,
    ) -> usize
    where
        I: IntoIterator<Item = &'a str>,
    {
        let target_ids: Vec<String> = self
            .masters
            .iter()
            .filter(|master| master.id != source_master_id)
            .map(|master| master.id.clone())
            .collect();
        let source_is_default = source_master_id == self.default_master_id;
        let mut copied = 0;
        for name in glyph_names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let Some(source) = glyph
                .layers
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.layer_snapshot()))
            else {
                continue;
            };
            let source_guidelines = glyph
                .master_guidelines
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.guidelines.clone()))
                .unwrap_or_default();
            for target_id in &target_ids {
                glyph.layers.insert(target_id.clone(), source.clone());
                glyph
                    .master_guidelines
                    .insert(target_id.clone(), source_guidelines.clone());
                copied += 1;
            }
        }
        copied
    }

    /// Removes layers whose master no longer exists and returns the number removed.
    pub fn remove_orphaned_layers(&mut self) -> usize {
        let valid: std::collections::HashSet<String> = self
            .masters
            .iter()
            .map(|master| master.id.clone())
            .collect();
        let mut removed = 0;
        for glyph in self.glyphs.values_mut() {
            let before = glyph.layers.len();
            glyph
                .layers
                .retain(|master_id, _| valid.contains(master_id));
            glyph
                .master_guidelines
                .retain(|master_id, _| valid.contains(master_id));
            removed += before - glyph.layers.len();
        }
        let before = self.kerning_by_master.len();
        self.kerning_by_master
            .retain(|master_id, _| valid.contains(master_id));
        removed += before - self.kerning_by_master.len();
        let before = self.guidelines_by_master.len();
        self.guidelines_by_master
            .retain(|master_id, _| valid.contains(master_id));
        removed += before - self.guidelines_by_master.len();
        removed
    }

    /// Aligns a component's underscored anchors to matching parent anchors.
    pub fn align_component_anchors(&mut self, glyph_name: &str, component_index: usize) -> bool {
        let (base_name, component) = match self.glyphs.get(glyph_name).and_then(|glyph| {
            glyph
                .components
                .get(component_index)
                .map(|component| (component.base.clone(), component.clone()))
        }) {
            Some(value) => value,
            None => return false,
        };
        let parent_anchors = self
            .glyphs
            .get(glyph_name)
            .map(|glyph| glyph.anchors.clone())
            .unwrap_or_default();
        let child_anchors = self.anchors_for_glyph(&base_name);
        let mut delta = (0.0, 0.0);
        let mut matches = 0.0;
        for child in &child_anchors {
            let Some(name) = child.name.strip_prefix('_') else {
                continue;
            };
            let Some(parent) = parent_anchors.iter().find(|anchor| anchor.name == name) else {
                continue;
            };
            let x = component.x_scale * child.x + component.xy_scale * child.y + component.x_offset;
            let y = component.yx_scale * child.x + component.y_scale * child.y + component.y_offset;
            delta.0 += parent.x - x;
            delta.1 += parent.y - y;
            matches += 1.0;
        }
        if matches == 0.0 {
            return false;
        }
        let (dx, dy) = (delta.0 / matches, delta.1 / matches);
        if let Some(glyph) = self.glyphs.get_mut(glyph_name) {
            let component = &mut glyph.components[component_index];
            component.x_offset += dx;
            component.y_offset += dy;
            return true;
        }
        false
    }

    pub fn get_glyph(&self, name: &str) -> Option<&GlyphData> {
        self.glyphs.get(name)
    }

    pub fn glyph_names_sorted(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .glyph_order
            .iter()
            .filter_map(|name| self.glyphs.contains_key(name).then_some(name.as_str()))
            .collect();
        let mut remaining: Vec<&str> = self
            .glyphs
            .keys()
            .filter(|name| !self.glyph_order.iter().any(|ordered| ordered == *name))
            .map(String::as_str)
            .collect();
        remaining.sort();
        names.extend(remaining);
        names
    }

    pub fn to_norad(&self) -> Result<norad::Font, String> {
        if !self.metadata.units_per_em.is_finite()
            || !(16.0..=16384.0).contains(&self.metadata.units_per_em)
        {
            return Err("UPMは16〜16384の範囲で指定してください".to_string());
        }
        for glyph in self.glyphs.values() {
            if !glyph.width.is_finite() || glyph.width < 0.0 {
                return Err(format!("グリフ '{}' の幅が不正です", glyph.name));
            }
            for contour in &glyph.contours {
                if contour.points.len() < 3 {
                    return Err(format!("グリフ '{}' に不完全な輪郭があります", glyph.name));
                }
                if contour
                    .points
                    .iter()
                    .any(|p| !p.x.is_finite() || !p.y.is_finite())
                {
                    return Err(format!("グリフ '{}' に不正な座標があります", glyph.name));
                }
                if contour
                    .points
                    .windows(2)
                    .any(|pair| pair[0].x == pair[1].x && pair[0].y == pair[1].y)
                {
                    return Err(format!(
                        "グリフ '{}' に重複した連続ノードがあります",
                        glyph.name
                    ));
                }
            }
            for component in &glyph.components {
                if component.base == glyph.name {
                    return Err(format!(
                        "グリフ '{}' が自分自身を参照しています",
                        glyph.name
                    ));
                }
                if !self.glyphs.contains_key(&component.base) {
                    return Err(format!(
                        "グリフ '{}' が参照するコンポーネント '{}' がありません",
                        glyph.name, component.base
                    ));
                }
                let transform = [
                    component.x_scale,
                    component.xy_scale,
                    component.yx_scale,
                    component.y_scale,
                    component.x_offset,
                    component.y_offset,
                ];
                if transform.iter().any(|value| !value.is_finite()) {
                    return Err(format!(
                        "グリフ '{}' のコンポーネント変換が不正です",
                        glyph.name
                    ));
                }
            }
        }
        let mut font = norad::Font::new();

        font.font_info.family_name = Some(self.metadata.family_name.clone());
        font.font_info.style_name = Some(self.metadata.style_name.clone());
        font.font_info.copyright =
            (!self.metadata.copyright.trim().is_empty()).then(|| self.metadata.copyright.clone());
        font.font_info.open_type_name_designer =
            (!self.metadata.designer.trim().is_empty()).then(|| self.metadata.designer.clone());
        font.font_info.open_type_name_manufacturer =
            (!self.metadata.manufacturer.trim().is_empty())
                .then(|| self.metadata.manufacturer.clone());
        font.font_info.open_type_name_version =
            Some(format!("Version {:.3}", self.metadata.font_revision));
        let units_per_em = self
            .metadata
            .units_per_em
            .try_into()
            .map_err(|_| "UPMを整数に変換できません".to_string())?;
        font.font_info.units_per_em = Some(units_per_em);
        font.font_info.ascender = Some(self.metadata.ascender);
        font.font_info.descender = Some(self.metadata.descender);
        font.font_info.x_height = Some(self.metadata.x_height);
        font.font_info.cap_height = Some(self.metadata.cap_height);
        font.font_info.italic_angle = Some(self.metadata.italic_angle);
        font.font_info.postscript_underline_position = Some(self.metadata.underline_position);
        font.font_info.postscript_underline_thickness = Some(self.metadata.underline_thickness);
        font.font_info.postscript_is_fixed_pitch = Some(self.metadata.is_fixed_pitch);
        font.font_info.open_type_os2_typo_line_gap = Some(self.metadata.line_gap as i32);
        font.font_info.guidelines = Some(
            self.guidelines_for_master(&self.default_master_id)
                .iter()
                .map(|guide| {
                    let line = if guide.angle.abs() < f64::EPSILON {
                        norad::Line::Horizontal(guide.y)
                    } else if (guide.angle - 90.0).abs() < f64::EPSILON {
                        norad::Line::Vertical(guide.x)
                    } else {
                        norad::Line::Angle {
                            x: guide.x,
                            y: guide.y,
                            degrees: guide.angle,
                        }
                    };
                    let name = (!guide.name.is_empty())
                        .then(|| norad::Name::new(&guide.name))
                        .transpose()
                        .map_err(|error| format!("ガイド名が不正です: {error}"))?;
                    Ok(norad::Guideline::new(line, name, None, None, None))
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
        font.features = self.feature_source();
        font.lib.insert(
            "com.glyph-studio.opentypeClasses".into(),
            plist::Value::String(self.opentype_classes.clone()),
        );
        let unicode_variation_sequences = serde_json::to_string(&self.unicode_variation_sequences)
            .map_err(|error| format!("IVSのUFO保存に失敗しました: {error}"))?;
        font.lib.insert(
            "com.glyph-studio.unicodeVariationSequences".into(),
            plist::Value::String(unicode_variation_sequences),
        );
        let axis_mappings = serde_json::to_string(&self.axis_mappings)
            .map_err(|error| format!("avar軸マッピングのUFO保存に失敗しました: {error}"))?;
        font.lib.insert(
            "com.glyph-studio.axisMappings".into(),
            plist::Value::String(axis_mappings),
        );
        let preserved_tables = serde_json::to_string(&self.preserved_tables)
            .map_err(|error| format!("OpenTypeテーブルのUFO保存に失敗しました: {error}"))?;
        font.lib.insert(
            "com.glyph-studio.preservedTables".into(),
            plist::Value::String(preserved_tables),
        );
        if let Some(source) = &self.preserved_layout_source {
            font.lib.insert(
                "com.glyph-studio.preservedLayoutSource".into(),
                plist::Value::String(source.clone()),
            );
        }
        if let Some(fingerprint) = self.preserved_layout_fingerprint {
            font.lib.insert(
                "com.glyph-studio.preservedLayoutFingerprint".into(),
                plist::Value::String(fingerprint.to_string()),
            );
        }
        let conditional_layers = serde_json::to_string(&self.conditional_layers)
            .map_err(|error| format!("条件レイヤーのUFO保存に失敗しました: {error}"))?;
        font.lib.insert(
            "com.glyph-studio.conditionalLayers".into(),
            plist::Value::String(conditional_layers),
        );
        let axis_names = self
            .axis_names
            .iter()
            .map(|(tag, name)| (tag.clone(), plist::Value::String(name.clone())))
            .collect();
        font.lib.insert(
            "com.glyph-studio.axisNames".into(),
            plist::Value::Dictionary(axis_names),
        );
        let mut master_info = plist::Dictionary::new();
        for master in &self.masters {
            let mut info = plist::Dictionary::new();
            info.insert("id".into(), plist::Value::String(master.id.clone()));
            info.insert("weight".into(), plist::Value::Real(master.weight));
            info.insert("width".into(), plist::Value::Real(master.width));
            info.insert("bracket".into(), plist::Value::Boolean(master.is_bracket));
            let axes = master
                .axes
                .iter()
                .map(|(tag, value)| (tag.clone(), plist::Value::Real(*value)))
                .collect();
            info.insert("axes".into(), plist::Value::Dictionary(axes));
            master_info.insert(master.name.clone(), plist::Value::Dictionary(info));
        }
        font.lib.insert(
            "com.glyph-studio.masters".into(),
            plist::Value::Dictionary(master_info),
        );
        let mut metadata = plist::Dictionary::new();
        metadata.insert(
            "fontRevision".into(),
            plist::Value::Real(self.metadata.font_revision),
        );
        font.lib.insert(
            "com.glyph-studio.metadata".into(),
            plist::Value::Dictionary(metadata),
        );
        let guidelines = self
            .guidelines_for_master(&self.default_master_id)
            .iter()
            .map(|guide| {
                let mut value = plist::Dictionary::new();
                value.insert("x".into(), plist::Value::Real(guide.x));
                value.insert("y".into(), plist::Value::Real(guide.y));
                value.insert("angle".into(), plist::Value::Real(guide.angle));
                value.insert("name".into(), plist::Value::String(guide.name.clone()));
                plist::Value::Dictionary(value)
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.guidelines".into(),
            plist::Value::Array(guidelines),
        );
        if !self.guidelines_by_master.is_empty() {
            let by_master = self
                .guidelines_by_master
                .iter()
                .map(|(master_id, guides)| {
                    let values = guides
                        .iter()
                        .map(|guide| {
                            let mut value = plist::Dictionary::new();
                            value.insert("x".into(), plist::Value::Real(guide.x));
                            value.insert("y".into(), plist::Value::Real(guide.y));
                            value.insert("angle".into(), plist::Value::Real(guide.angle));
                            value.insert("name".into(), plist::Value::String(guide.name.clone()));
                            plist::Value::Dictionary(value)
                        })
                        .collect();
                    (master_id.clone(), plist::Value::Array(values))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.guidelinesByMaster".into(),
                plist::Value::Dictionary(by_master),
            );
        }
        let vertical_metrics = self
            .vertical_metrics
            .iter()
            .map(|(name, metrics)| {
                let mut value = plist::Dictionary::new();
                value.insert(
                    "advanceHeight".into(),
                    plist::Value::Real(metrics.advance_height),
                );
                value.insert(
                    "topSideBearing".into(),
                    plist::Value::Real(metrics.top_side_bearing),
                );
                (name.clone(), plist::Value::Dictionary(value))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.verticalMetrics".into(),
            plist::Value::Dictionary(vertical_metrics),
        );
        let vertical_metrics_by_master = self
            .vertical_metrics_by_master
            .iter()
            .map(|(master_id, metrics)| {
                let values = metrics
                    .iter()
                    .map(|(name, metric)| {
                        let mut value = plist::Dictionary::new();
                        value.insert(
                            "advanceHeight".into(),
                            plist::Value::Real(metric.advance_height),
                        );
                        value.insert(
                            "topSideBearing".into(),
                            plist::Value::Real(metric.top_side_bearing),
                        );
                        (name.clone(), plist::Value::Dictionary(value))
                    })
                    .collect();
                (master_id.clone(), plist::Value::Dictionary(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.verticalMetricsByMaster".into(),
            plist::Value::Dictionary(vertical_metrics_by_master),
        );
        let metrics_by_master = self
            .metrics_by_master
            .iter()
            .map(|(master_id, metrics)| {
                let mut value = plist::Dictionary::new();
                value.insert("ascender".into(), plist::Value::Real(metrics.ascender));
                value.insert("descender".into(), plist::Value::Real(metrics.descender));
                value.insert("lineGap".into(), plist::Value::Real(metrics.line_gap));
                (master_id.clone(), plist::Value::Dictionary(value))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.metricsByMaster".into(),
            plist::Value::Dictionary(metrics_by_master),
        );
        let background_images = self
            .background_images
            .iter()
            .map(|(glyph, masters)| {
                let values = masters
                    .iter()
                    .map(|(master, path)| (master.clone(), plist::Value::String(path.clone())))
                    .collect();
                (glyph.clone(), plist::Value::Dictionary(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.backgroundImages".into(),
            plist::Value::Dictionary(background_images),
        );
        let background_opacities = self
            .background_opacities
            .iter()
            .map(|(glyph, masters)| {
                let values = masters
                    .iter()
                    .map(|(master, opacity)| {
                        (master.clone(), plist::Value::Real(f64::from(*opacity)))
                    })
                    .collect();
                (glyph.clone(), plist::Value::Dictionary(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.backgroundOpacities".into(),
            plist::Value::Dictionary(background_opacities),
        );
        let background_transforms = self
            .background_transforms
            .iter()
            .map(|(glyph, masters)| {
                let values = masters
                    .iter()
                    .map(|(master, transform)| {
                        let mut value = plist::Dictionary::new();
                        value.insert("x".into(), plist::Value::Real(f64::from(transform.x)));
                        value.insert("y".into(), plist::Value::Real(f64::from(transform.y)));
                        value.insert(
                            "scale".into(),
                            plist::Value::Real(f64::from(transform.scale)),
                        );
                        value.insert(
                            "rotation".into(),
                            plist::Value::Real(f64::from(transform.rotation)),
                        );
                        value.insert("flipX".into(), plist::Value::Boolean(transform.flip_x));
                        value.insert("flipY".into(), plist::Value::Boolean(transform.flip_y));
                        (master.clone(), plist::Value::Dictionary(value))
                    })
                    .collect();
                (glyph.clone(), plist::Value::Dictionary(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.backgroundTransforms".into(),
            plist::Value::Dictionary(background_transforms),
        );
        let palettes = self
            .color_palettes
            .iter()
            .map(|palette| {
                plist::Value::Array(
                    palette
                        .iter()
                        .map(|&[r, g, b, a]| {
                            plist::Value::Array(
                                [r, g, b, a]
                                    .into_iter()
                                    .map(|value| plist::Value::Integer(i64::from(value).into()))
                                    .collect(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.colorPalettes".into(),
            plist::Value::Array(palettes),
        );
        font.lib.insert(
            "com.glyph-studio.colorPaletteNames".into(),
            plist::Value::Array(
                self.color_palette_names
                    .iter()
                    .map(|name| plist::Value::String(name.clone()))
                    .collect(),
            ),
        );
        font.lib.insert(
            "com.glyph-studio.colorPaletteTypes".into(),
            plist::Value::Array(
                self.color_palette_types
                    .iter()
                    .map(|value| plist::Value::Integer(i64::from(*value).into()))
                    .collect(),
            ),
        );
        font.lib.insert(
            "com.glyph-studio.colorPaletteEntryNames".into(),
            plist::Value::Array(
                self.color_palette_entry_names
                    .iter()
                    .map(|name| plist::Value::String(name.clone()))
                    .collect(),
            ),
        );
        let color_layers = self
            .color_layers
            .iter()
            .map(|(name, layers)| {
                let values = layers
                    .iter()
                    .map(|layer| {
                        let mut value = plist::Dictionary::new();
                        value.insert("glyph".into(), plist::Value::String(layer.glyph.clone()));
                        value.insert(
                            "paletteIndex".into(),
                            plist::Value::Integer(i64::from(layer.palette_index).into()),
                        );
                        if let Some(gradient) = &layer.gradient {
                            let mut info = plist::Dictionary::new();
                            info.insert(
                                "startPaletteIndex".into(),
                                plist::Value::Integer(
                                    i64::from(gradient.start_palette_index).into(),
                                ),
                            );
                            info.insert(
                                "endPaletteIndex".into(),
                                plist::Value::Integer(i64::from(gradient.end_palette_index).into()),
                            );
                            let kind = match gradient.kind {
                                ColorGradientKind::Linear => "linear",
                                ColorGradientKind::Radial => "radial",
                                ColorGradientKind::Sweep => "sweep",
                            };
                            info.insert("kind".into(), plist::Value::String(kind.into()));
                            let extend = match gradient.extend {
                                ColorGradientExtend::Pad => "pad",
                                ColorGradientExtend::Repeat => "repeat",
                                ColorGradientExtend::Reflect => "reflect",
                            };
                            info.insert("extend".into(), plist::Value::String(extend.into()));
                            for (key, value) in [
                                ("x0", gradient.x0),
                                ("y0", gradient.y0),
                                ("x1", gradient.x1),
                                ("y1", gradient.y1),
                                ("x2", gradient.x2),
                                ("y2", gradient.y2),
                                ("radius0", gradient.radius0),
                                ("radius1", gradient.radius1),
                                ("startAngle", gradient.start_angle),
                                ("endAngle", gradient.end_angle),
                            ] {
                                info.insert(key.into(), plist::Value::Real(value));
                            }
                            if !gradient.stops.is_empty() {
                                info.insert(
                                    "stops".into(),
                                    plist::Value::Array(
                                        gradient
                                            .stops
                                            .iter()
                                            .map(|stop| {
                                                let mut value = plist::Dictionary::new();
                                                value.insert(
                                                    "offset".into(),
                                                    plist::Value::Real(stop.offset),
                                                );
                                                value.insert(
                                                    "paletteIndex".into(),
                                                    plist::Value::Integer(
                                                        i64::from(stop.palette_index).into(),
                                                    ),
                                                );
                                                value.insert(
                                                    "alpha".into(),
                                                    plist::Value::Real(stop.alpha),
                                                );
                                                plist::Value::Dictionary(value)
                                            })
                                            .collect(),
                                    ),
                                );
                            }
                            value.insert("gradient".into(), plist::Value::Dictionary(info));
                        }
                        plist::Value::Dictionary(value)
                    })
                    .collect();
                (name.clone(), plist::Value::Array(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.colorLayers".into(),
            plist::Value::Dictionary(color_layers),
        );
        let color_layer_transforms = self
            .color_layer_transforms
            .iter()
            .map(|(name, transforms)| {
                let values = transforms
                    .iter()
                    .map(|transform| {
                        let Some(transform) = transform else {
                            return plist::Value::Dictionary(plist::Dictionary::new());
                        };
                        let mut value = plist::Dictionary::new();
                        for (key, number) in [
                            ("xx", transform.xx),
                            ("yx", transform.yx),
                            ("xy", transform.xy),
                            ("yy", transform.yy),
                            ("dx", transform.dx),
                            ("dy", transform.dy),
                        ] {
                            value.insert(key.into(), plist::Value::Real(number));
                        }
                        plist::Value::Dictionary(value)
                    })
                    .collect();
                (name.clone(), plist::Value::Array(values))
            })
            .collect();
        font.lib.insert(
            "com.glyph-studio.colorLayerTransforms".into(),
            plist::Value::Dictionary(color_layer_transforms),
        );
        let mut left_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
        let mut right_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
        for glyph in self.glyphs.values() {
            if !glyph.left_kerning_group.trim().is_empty() {
                left_groups
                    .entry(format!("public.kern1.{}", glyph.left_kerning_group.trim()))
                    .or_default()
                    .push(glyph.name.clone());
            }
            if !glyph.right_kerning_group.trim().is_empty() {
                right_groups
                    .entry(format!("public.kern2.{}", glyph.right_kerning_group.trim()))
                    .or_default()
                    .push(glyph.name.clone());
            }
        }
        for (group, members) in left_groups.into_iter().chain(right_groups) {
            let group_name = norad::Name::new(&group)
                .map_err(|error| format!("カーニンググループ名が不正です: {error}"))?;
            let member_names = members
                .into_iter()
                .map(|member| {
                    norad::Name::new(&member)
                        .map_err(|error| format!("グループ所属グリフ名が不正です: {error}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            font.groups.insert(group_name, member_names);
        }

        let layer = font.default_layer_mut();
        for glyph_data in self.glyphs.values() {
            let mut glyph = norad::Glyph::new(&glyph_data.name);
            glyph.width = glyph_data.width;
            if !glyph_data.left_kerning_group.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.leftKerningGroup".into(),
                    plist::Value::String(glyph_data.left_kerning_group.clone()),
                );
            }
            if !glyph_data.right_kerning_group.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.rightKerningGroup".into(),
                    plist::Value::String(glyph_data.right_kerning_group.clone()),
                );
            }
            if !glyph_data.left_metrics_key.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.leftMetricsKey".into(),
                    plist::Value::String(glyph_data.left_metrics_key.clone()),
                );
            }
            if !glyph_data.right_metrics_key.is_empty() {
                glyph.lib.insert(
                    "com.glyph-studio.rightMetricsKey".into(),
                    plist::Value::String(glyph_data.right_metrics_key.clone()),
                );
            }
            for anchor in &glyph_data.anchors {
                glyph.anchors.push(norad::Anchor::new(
                    anchor.x,
                    anchor.y,
                    Some(
                        norad::Name::new(&anchor.name)
                            .map_err(|error| format!("アンカー名が不正です: {error}"))?,
                    ),
                    None,
                    None,
                    None,
                ));
            }
            for guide in &glyph_data.guidelines {
                let line = if guide.angle.abs() < f64::EPSILON {
                    norad::Line::Horizontal(guide.y)
                } else if (guide.angle - 90.0).abs() < f64::EPSILON {
                    norad::Line::Vertical(guide.x)
                } else {
                    norad::Line::Angle {
                        x: guide.x,
                        y: guide.y,
                        degrees: guide.angle,
                    }
                };
                let name = (!guide.name.is_empty())
                    .then(|| norad::Name::new(&guide.name))
                    .transpose()
                    .map_err(|error| format!("ガイド名が不正です: {error}"))?;
                glyph
                    .guidelines
                    .push(norad::Guideline::new(line, name, None, None, None));
            }
            if !glyph_data.master_guidelines.is_empty() {
                let master_guidelines = glyph_data
                    .master_guidelines
                    .iter()
                    .map(|(master_id, guides)| {
                        let values = guides
                            .iter()
                            .map(|guide| {
                                let mut value = plist::Dictionary::new();
                                value.insert("x".into(), plist::Value::Real(guide.x));
                                value.insert("y".into(), plist::Value::Real(guide.y));
                                value.insert("angle".into(), plist::Value::Real(guide.angle));
                                value.insert(
                                    "name".into(),
                                    plist::Value::String(guide.name.clone()),
                                );
                                plist::Value::Dictionary(value)
                            })
                            .collect();
                        (master_id.clone(), plist::Value::Array(values))
                    })
                    .collect();
                glyph.lib.insert(
                    "com.glyph-studio.masterGuidelines".into(),
                    plist::Value::Dictionary(master_guidelines),
                );
            }
            let mut codepoints = glyph_data.unicodes.clone();
            if let Some(unicode) = glyph_data.unicode {
                if !codepoints.contains(&unicode) {
                    codepoints.push(unicode);
                }
            }
            glyph.codepoints = codepoints.into_iter().filter_map(char::from_u32).collect();

            for contour_data in &glyph_data.contours {
                let mut norad_points = Vec::new();
                for point in &contour_data.points {
                    let point_type = match point.point_type {
                        PointType::OnCurve => norad::PointType::Line,
                        PointType::OffCurve => norad::PointType::OffCurve,
                    };
                    norad_points.push(norad::ContourPoint::new(
                        point.x,
                        point.y,
                        point_type,
                        point.smooth,
                        None,
                        None,
                        None,
                    ));
                }
                let contour = norad::Contour::new(norad_points, None, None);
                glyph.contours.push(contour);
            }
            for component in &glyph_data.components {
                glyph.components.push(norad::Component::new(
                    norad::Name::new(&component.base)
                        .map_err(|e| format!("コンポーネント名が不正です: {e}"))?,
                    norad::AffineTransform {
                        x_scale: component.x_scale,
                        xy_scale: component.xy_scale,
                        yx_scale: component.yx_scale,
                        y_scale: component.y_scale,
                        x_offset: component.x_offset,
                        y_offset: component.y_offset,
                    },
                    None,
                    None,
                ));
            }

            layer.insert_glyph(glyph);
        }

        for master in self
            .masters
            .iter()
            .filter(|master| master.id != self.default_master_id)
        {
            let layer_name = if master.name == "public.default"
                || font
                    .layers
                    .names()
                    .any(|name| name.to_string() == master.name)
            {
                &master.id
            } else {
                &master.name
            };
            let layer = font
                .layers
                .new_layer(layer_name)
                .map_err(|e| format!("UFOレイヤー作成エラー: {e}"))?;
            for glyph_data in self.glyphs.values() {
                let Some(layer_data) = glyph_data.layers.get(&master.id) else {
                    continue;
                };
                let mut glyph = norad::Glyph::new(&glyph_data.name);
                glyph.width = layer_data.width;
                if !glyph_data.left_kerning_group.is_empty() {
                    glyph.lib.insert(
                        "com.glyph-studio.leftKerningGroup".into(),
                        plist::Value::String(glyph_data.left_kerning_group.clone()),
                    );
                }
                if !glyph_data.right_kerning_group.is_empty() {
                    glyph.lib.insert(
                        "com.glyph-studio.rightKerningGroup".into(),
                        plist::Value::String(glyph_data.right_kerning_group.clone()),
                    );
                }
                for anchor in &layer_data.anchors {
                    glyph.anchors.push(norad::Anchor::new(
                        anchor.x,
                        anchor.y,
                        Some(
                            norad::Name::new(&anchor.name)
                                .map_err(|error| format!("アンカー名が不正です: {error}"))?,
                        ),
                        None,
                        None,
                        None,
                    ));
                }
                let mut codepoints = glyph_data.unicodes.clone();
                if let Some(unicode) = glyph_data.unicode {
                    if !codepoints.contains(&unicode) {
                        codepoints.push(unicode);
                    }
                }
                glyph.codepoints = codepoints.into_iter().filter_map(char::from_u32).collect();
                for contour_data in &layer_data.contours {
                    let points = contour_data
                        .points
                        .iter()
                        .map(|point| {
                            let point_type = match point.point_type {
                                PointType::OnCurve => norad::PointType::Line,
                                PointType::OffCurve => norad::PointType::OffCurve,
                            };
                            norad::ContourPoint::new(
                                point.x,
                                point.y,
                                point_type,
                                point.smooth,
                                None,
                                None,
                                None,
                            )
                        })
                        .collect();
                    glyph.contours.push(norad::Contour::new(points, None, None));
                }
                for component in &layer_data.components {
                    glyph.components.push(norad::Component::new(
                        norad::Name::new(&component.base)
                            .map_err(|e| format!("コンポーネント名が不正です: {e}"))?,
                        norad::AffineTransform {
                            x_scale: component.x_scale,
                            xy_scale: component.xy_scale,
                            yx_scale: component.yx_scale,
                            y_scale: component.y_scale,
                            x_offset: component.x_offset,
                            y_offset: component.y_offset,
                        },
                        None,
                        None,
                    ));
                }
                layer.insert_glyph(glyph);
            }
        }

        for ((left, right), value) in &self.kerning {
            let left = norad::Name::new(left)
                .map_err(|e| format!("カーニング左グリフ名が不正です: {e}"))?;
            let right = norad::Name::new(right)
                .map_err(|e| format!("カーニング右グリフ名が不正です: {e}"))?;
            font.kerning.entry(left).or_default().insert(right, *value);
        }
        if !self.kerning_by_master.is_empty() {
            let by_master = self
                .kerning_by_master
                .iter()
                .map(|(master_id, pairs)| {
                    let entries = pairs
                        .iter()
                        .map(|((left, right), value)| {
                            let mut entry = plist::Dictionary::new();
                            entry.insert("left".into(), plist::Value::String(left.clone()));
                            entry.insert("right".into(), plist::Value::String(right.clone()));
                            entry.insert("value".into(), plist::Value::Real(*value));
                            plist::Value::Dictionary(entry)
                        })
                        .collect();
                    (master_id.clone(), plist::Value::Array(entries))
                })
                .collect();
            font.lib.insert(
                "com.glyph-studio.kerningByMaster".into(),
                plist::Value::Dictionary(by_master),
            );
        }

        Ok(font)
    }
}

impl Default for FontProject {
    fn default() -> Self {
        Self::new()
    }
}

fn rewrite_feature_glyph_name(source: &str, old_name: &str, new_name: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut token = String::new();
    let mut skip_next_tag = false;
    let mut previous_token = String::new();
    let flush = |output: &mut String,
                 token: &mut String,
                 skip_next_tag: &mut bool,
                 previous_token: &mut String| {
        if !token.is_empty() {
            let skip_rewrite = *skip_next_tag || previous_token == "feature";
            if token == old_name && !skip_rewrite {
                output.push_str(new_name);
            } else {
                output.push_str(token);
            }
            *skip_next_tag = false;
            *previous_token = token.clone();
            if token == "feature" {
                *skip_next_tag = true;
            }
            token.clear();
        }
    };
    for character in source.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            token.push(character);
        } else {
            flush(
                &mut output,
                &mut token,
                &mut skip_next_tag,
                &mut previous_token,
            );
            if character == '}' {
                skip_next_tag = true;
            }
            output.push(character);
        }
    }
    flush(
        &mut output,
        &mut token,
        &mut skip_next_tag,
        &mut previous_token,
    );
    output
}
