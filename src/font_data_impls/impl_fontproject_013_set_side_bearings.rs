use super::*;

impl FontProject {
    pub fn set_side_bearings(&mut self, names: &[String], left: f64, right: f64) -> usize {
        if !left.is_finite() || !right.is_finite() || left < 0.0 || right < 0.0 {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            let active_bounds = self.outline_bounds_for_glyph(name);
            let layer_bounds: HashMap<String, (f64, f64, f64, f64)> = self
                .glyphs
                .get(name)
                .into_iter()
                .flat_map(|glyph| glyph.layers.keys())
                .filter_map(|master_id| {
                    let mut snapshot = self.clone();
                    for other in snapshot.glyphs.values_mut() {
                        if let Some(layer) = other.layers.get(master_id).cloned() {
                            other.width = layer.width;
                            other.contours = layer.contours;
                            other.components = layer.components;
                            other.anchors = layer.anchors;
                        }
                    }
                    snapshot
                        .outline_bounds_for_glyph(name)
                        .map(|bounds| (master_id.clone(), bounds))
                })
                .collect();
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let mut glyph_changed = false;
            let adjust = |layer: &mut GlyphLayer, bounds: Option<(f64, f64)>| {
                let contour_min = layer
                    .contours
                    .iter()
                    .flat_map(|contour| contour.points.iter())
                    .map(|point| point.x)
                    .min_by(f64::total_cmp);
                let contour_max = layer
                    .contours
                    .iter()
                    .flat_map(|contour| contour.points.iter())
                    .map(|point| point.x)
                    .max_by(f64::total_cmp);
                let Some((min_x, max_x)) = bounds.or_else(|| contour_min.zip(contour_max)) else {
                    return false;
                };
                let dx = left - min_x;
                if dx.abs() > f64::EPSILON {
                    for contour in &mut layer.contours {
                        for point in &mut contour.points {
                            point.x += dx;
                        }
                    }
                    for component in &mut layer.components {
                        component.x_offset += dx;
                    }
                    for anchor in &mut layer.anchors {
                        anchor.x += dx;
                    }
                }
                let width = (max_x - min_x) + left + right;
                let width_changed = (layer.width - width).abs() > f64::EPSILON;
                layer.width = width.max(0.0);
                dx.abs() > f64::EPSILON || width_changed
            };
            let mut active = glyph.layer_snapshot();
            glyph_changed |= adjust(
                &mut active,
                active_bounds.map(|(min, _, max, _)| (min, max)),
            );
            glyph.width = active.width;
            glyph.contours = active.contours;
            glyph.components = active.components;
            glyph.anchors = active.anchors;
            for (master_id, layer) in glyph.layers.iter_mut() {
                glyph_changed |= adjust(
                    layer,
                    layer_bounds
                        .get(master_id)
                        .map(|(min, _, max, _)| (*min, *max)),
                );
            }
            if glyph_changed {
                changed += 1;
            }
        }
        changed
    }
}
