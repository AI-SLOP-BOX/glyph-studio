use super::*;

impl GlyphLayer {
    #[allow(dead_code)]
    pub fn interpolate(&self, other: &Self, factor: f64) -> Option<Self> {
        if self.contours.len() != other.contours.len()
            || self.components.len() != other.components.len()
            || self.anchors.len() != other.anchors.len()
            || self
                .anchors
                .iter()
                .any(|anchor| !other.anchors.iter().any(|item| item.name == anchor.name))
            || self.contours.iter().zip(&other.contours).any(|(a, b)| {
                a.points.len() != b.points.len()
                    || a.points
                        .iter()
                        .zip(&b.points)
                        .any(|(a, b)| a.point_type != b.point_type)
            })
        {
            return None;
        }
        let t = factor.clamp(0.0, 1.0);
        let mix = |a: f64, b: f64| a + (b - a) * t;
        let contours = self
            .contours
            .iter()
            .zip(&other.contours)
            .map(|(a, b)| Contour {
                points: a
                    .points
                    .iter()
                    .zip(&b.points)
                    .map(|(a, b)| ContourPoint {
                        x: mix(a.x, b.x),
                        y: mix(a.y, b.y),
                        point_type: a.point_type,
                        smooth: if t < 0.5 { a.smooth } else { b.smooth },
                    })
                    .collect(),
            })
            .collect();
        let components = self
            .components
            .iter()
            .zip(&other.components)
            .map(|(a, b)| {
                (a.base == b.base).then(|| GlyphComponent {
                    base: a.base.clone(),
                    x_scale: mix(a.x_scale, b.x_scale),
                    xy_scale: mix(a.xy_scale, b.xy_scale),
                    yx_scale: mix(a.yx_scale, b.yx_scale),
                    y_scale: mix(a.y_scale, b.y_scale),
                    x_offset: mix(a.x_offset, b.x_offset),
                    y_offset: mix(a.y_offset, b.y_offset),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let anchors = self
            .anchors
            .iter()
            .map(|anchor| {
                let other_anchor = other
                    .anchors
                    .iter()
                    .find(|item| item.name == anchor.name)
                    .expect("anchor topology was validated above");
                GlyphAnchor {
                    name: anchor.name.clone(),
                    x: mix(anchor.x, other_anchor.x),
                    y: mix(anchor.y, other_anchor.y),
                }
            })
            .collect();
        Some(Self {
            width: mix(self.width, other.width),
            contours,
            components,
            anchors,
        })
    }
}
