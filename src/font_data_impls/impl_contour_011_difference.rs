use super::*;

impl Contour {
    /// Subtracts `other` from this contour while preserving Bézier segments.
    pub fn difference(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        subject
            .overlay(
                &clip,
                i_curve::OverlayRule::Difference,
                i_curve::FillRule::NonZero,
            )
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }
}
