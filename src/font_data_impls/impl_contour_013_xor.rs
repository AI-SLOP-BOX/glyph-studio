use super::*;

impl Contour {
    /// Keeps the exclusive area of two contours.
    pub fn xor(&self, other: &Self) -> Result<Vec<Self>, String> {
        let subject = self.to_curve_shape()?;
        let clip = other.to_curve_shape()?;
        subject
            .overlay(&clip, i_curve::OverlayRule::Xor, i_curve::FillRule::NonZero)
            .into_iter()
            .flat_map(|shape| shape.into_contours())
            .map(Self::from_curve_path)
            .collect()
    }
}
