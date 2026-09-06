use super::*;

impl ContourPoint {
    pub fn off_curve(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            point_type: PointType::OffCurve,
            smooth: false,
        }
    }
}
