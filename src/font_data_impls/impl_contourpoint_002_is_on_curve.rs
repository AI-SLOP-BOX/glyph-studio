use super::*;

impl ContourPoint {
    pub fn is_on_curve(&self) -> bool {
        self.point_type == PointType::OnCurve
    }
}
