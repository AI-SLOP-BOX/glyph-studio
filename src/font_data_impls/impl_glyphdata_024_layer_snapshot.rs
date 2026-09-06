use super::*;

impl GlyphData {
    pub(crate) fn layer_snapshot(&self) -> GlyphLayer {
        GlyphLayer {
            width: self.width,
            contours: self.contours.clone(),
            components: self.components.clone(),
            anchors: self.anchors.clone(),
        }
    }
}
