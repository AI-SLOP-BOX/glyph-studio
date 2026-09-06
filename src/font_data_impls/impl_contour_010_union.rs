use super::*;

impl Contour {
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
}
