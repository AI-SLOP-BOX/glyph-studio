use super::*;

impl FontProject {
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
}
