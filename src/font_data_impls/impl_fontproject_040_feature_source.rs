use super::*;

impl FontProject {
    pub fn feature_source(&self) -> String {
        match (self.opentype_classes.trim(), self.opentype_features.trim()) {
            ("", features) => features.to_string(),
            (classes, "") => classes.to_string(),
            (classes, features) => format!("{classes}\n\n{features}"),
        }
    }
}
