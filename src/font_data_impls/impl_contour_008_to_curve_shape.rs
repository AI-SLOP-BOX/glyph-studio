use super::*;

impl Contour {
    /// Converts this contour to curve-preserving Boolean geometry.
    #[allow(dead_code)]
    pub(crate) fn to_curve_shape(&self) -> Result<i_curve::FloatCurveShape<[f64; 2]>, String> {
        let mut builder = i_curve::CurveBuilder::new();
        for element in self.to_bezpath().elements() {
            match element {
                PathEl::MoveTo(point) => {
                    builder
                        .move_to([point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::LineTo(point) => {
                    builder
                        .line_to([point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::QuadTo(control, point) => {
                    builder
                        .quad_to([control.x, control.y], [point.x, point.y])
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::CurveTo(control0, control1, point) => {
                    builder
                        .cubic_to(
                            [control0.x, control0.y],
                            [control1.x, control1.y],
                            [point.x, point.y],
                        )
                        .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))?;
                }
                PathEl::ClosePath => {}
            }
        }
        builder
            .close_contour()
            .map_err(|error| format!("輪郭を閉じられません: {error}"))?;
        builder
            .build()
            .map_err(|error| format!("輪郭のBoolean変換に失敗しました: {error}"))
    }
}
