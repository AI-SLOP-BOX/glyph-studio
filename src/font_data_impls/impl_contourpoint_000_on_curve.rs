use super::*;

impl ContourPoint {
    pub fn on_curve(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            point_type: PointType::OnCurve,
            smooth: false,
        }
    }
}
