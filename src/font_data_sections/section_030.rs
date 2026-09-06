
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
