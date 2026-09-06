use super::*;

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
