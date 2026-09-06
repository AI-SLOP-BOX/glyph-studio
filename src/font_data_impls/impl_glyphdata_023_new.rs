use super::*;

impl GlyphData {
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
}
