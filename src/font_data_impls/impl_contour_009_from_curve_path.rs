use super::*;

impl Contour {
    pub(crate) fn from_curve_path(path: i_curve::FloatCurvePath<[f64; 2]>) -> Result<Self, String> {
        let (start, segments) = path.into_parts();
        let mut points = vec![ContourPoint::on_curve(start[0], start[1])];
        let mut current = start;
        for segment in segments {
            match segment {
                i_curve::FloatCurveSegment::Line { to } => {
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Quad { ctrl, to } => {
                    points.push(ContourPoint::off_curve(ctrl[0], ctrl[1]));
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Cubic { ctrl0, ctrl1, to } => {
                    points.push(ContourPoint::off_curve(ctrl0[0], ctrl0[1]));
                    points.push(ContourPoint::off_curve(ctrl1[0], ctrl1[1]));
                    if to != start {
                        points.push(ContourPoint::on_curve(to[0], to[1]));
                    }
                    current = to;
                }
                i_curve::FloatCurveSegment::Arc { arc } => {
                    // i_curve returns rational quadratic arcs. Our editable
                    // contour format has no rational weights, so degree-
                    // approximate each (already monotone) arc piece as a
                    // cubic. The endpoint tangent is exact; the remaining
                    // error is bounded by the small pieces produced by
                    // i_curve's arc materializer.
                    let p0 = current;
                    let p1 = arc.control_points[1];
                    let p2 = arc.end_point();
                    let factor = (2.0 * arc.weights[1]) / 3.0;
                    let ctrl0 = [
                        p0[0] + factor * (p1[0] - p0[0]),
                        p0[1] + factor * (p1[1] - p0[1]),
                    ];
                    let ctrl1 = [
                        p2[0] - factor * (p2[0] - p1[0]),
                        p2[1] - factor * (p2[1] - p1[1]),
                    ];
                    points.push(ContourPoint::off_curve(ctrl0[0], ctrl0[1]));
                    points.push(ContourPoint::off_curve(ctrl1[0], ctrl1[1]));
                    if p2 != start {
                        points.push(ContourPoint::on_curve(p2[0], p2[1]));
                    }
                    current = p2;
                }
            }
        }
        if points.len() < 3 {
            return Err("Boolean結果が不完全な輪郭です".into());
        }
        Ok(Self { points })
    }
}
