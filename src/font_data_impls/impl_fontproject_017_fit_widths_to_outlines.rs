use super::*;

impl FontProject {
    pub fn fit_widths_to_outlines(&mut self, names: &[String]) -> usize {
        let widths: Vec<(String, f64, HashMap<String, f64>)> = names
            .iter()
            .filter_map(|name| {
                let (_, _, max_x, _) = self.outline_bounds_for_glyph(name)?;
                let glyph = self.glyphs.get(name)?;
                let mut layer_widths = HashMap::new();
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
                    if let Some((_, _, layer_max_x, _)) = snapshot.outline_bounds_for_glyph(name) {
                        layer_widths.insert(master_id.clone(), layer_max_x.max(0.0));
                    }
                }
                Some((name.clone(), max_x.max(0.0), layer_widths))
            })
            .collect();
        let mut changed = 0;
        for (name, width, layer_widths) in widths {
            if let Some(glyph) = self.glyphs.get_mut(&name) {
                let mut glyph_changed = (glyph.width - width).abs() > f64::EPSILON;
                glyph.width = width;
                for (master_id, layer_width) in layer_widths {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        glyph_changed |= (layer.width - layer_width).abs() > f64::EPSILON;
                        layer.width = layer_width;
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
