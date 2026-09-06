use super::*;

impl ColorLayerTransform {
    pub fn is_identity(self) -> bool {
        self == Self::default()
    }
}
