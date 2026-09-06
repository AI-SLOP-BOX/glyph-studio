
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
