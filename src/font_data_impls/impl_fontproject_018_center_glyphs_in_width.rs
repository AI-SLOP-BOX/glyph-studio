use super::*;

impl FontProject {
    pub fn center_glyphs_in_width(&mut self, names: &[String]) -> usize {
        let shifts: Vec<(String, f64, HashMap<String, f64>)> = names
            .iter()
            .filter_map(|name| {
                let (min_x, _, max_x, _) = self.outline_bounds_for_glyph(name)?;
                let glyph = self.glyphs.get(name)?;
                let active_shift = (glyph.width - (max_x - min_x)) * 0.5 - min_x;
                let mut layer_shifts = HashMap::new();
                for master_id in glyph.layers.keys() {
                    let mut snapshot = self.clone();
                    for other in snapshot.glyphs.values_mut() {
                        if let Some(layer) = other.layers.get(master_id).cloned() {
                            other.width = layer.width;
                            other.contours = layer.contours;
                            other.components = layer.components;
                            other.anchors = layer.anchors;
                        }
                    }
                    if let Some((layer_min_x, _, layer_max_x, _)) =
                        snapshot.outline_bounds_for_glyph(name)
                    {
                        let layer_width = glyph.layers[master_id].width;
                        layer_shifts.insert(
                            master_id.clone(),
                            (layer_width - (layer_max_x - layer_min_x)) * 0.5 - layer_min_x,
                        );
                    }
                }
                Some((name.clone(), active_shift, layer_shifts))
            })
            .collect();
        let mut changed = 0;
        for (name, active_shift, layer_shifts) in shifts {
            if let Some(glyph) = self.glyphs.get_mut(&name) {
                let mut glyph_changed = active_shift.abs() > f64::EPSILON;
                glyph.translate_authored_geometry(active_shift, 0.0);
                for (master_id, shift) in layer_shifts {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        glyph_changed |= shift.abs() > f64::EPSILON;
                        layer.translate_geometry(shift, 0.0);
                    }
                }
                if glyph_changed {
                    changed += 1;
                }
            }
        }
        changed
    }
}
