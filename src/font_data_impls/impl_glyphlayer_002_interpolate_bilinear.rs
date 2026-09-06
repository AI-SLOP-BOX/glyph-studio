use super::*;

impl GlyphLayer {
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
