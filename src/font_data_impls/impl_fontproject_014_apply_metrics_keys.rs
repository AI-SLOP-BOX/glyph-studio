use super::*;

impl FontProject {
    /// Applies metric-key references (for example `=H`) to the selected
    /// glyphs. Each side can reference a different glyph; empty keys leave
    /// that side unchanged. Reference bearings are resolved independently for
    /// every available master layer.
    pub fn apply_metrics_keys(&mut self, names: &[String]) -> Result<usize, String> {
        let parse_key = |key: &str| {
            let key = key.trim();
            if key.is_empty() {
                None
            } else {
                key.strip_prefix('=')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            }
        };
        for name in names {
            let glyph = self
                .glyphs
                .get(name)
                .ok_or_else(|| format!("グリフ '{}' がありません", name))?;
            for key in [&glyph.left_metrics_key, &glyph.right_metrics_key] {
                if let Some(reference) = parse_key(key) {
                    if reference == *name {
                        return Err(format!("グリフ '{}' は自分自身を基準にできません", name));
                    }
                    if !self.glyphs.contains_key(&reference) {
                        return Err(format!(
                            "メトリクスキーの基準グリフ '{}' がありません",
                            reference
                        ));
                    }
                } else if !key.trim().is_empty() {
                    return Err(format!(
                        "グリフ '{}' のメトリクスキーは =グリフ名 形式です",
                        name
                    ));
                }
            }
        }

        let bounds_to_bearings = |project: &FontProject, name: &str| {
            let (min_x, _, max_x, _) = project.outline_bounds_for_glyph(name)?;
            let width = project.glyphs.get(name)?.width;
            Some((min_x, width - max_x))
        };
        let mut changed = 0;
        for name in names {
            let (left_key, right_key) = {
                let glyph = self.glyphs.get(name).expect("validated glyph");
                (
                    parse_key(&glyph.left_metrics_key),
                    parse_key(&glyph.right_metrics_key),
                )
            };
            if left_key.is_none() && right_key.is_none() {
                continue;
            }

            // Apply the active view first, preserving any side without a key.
            let current = bounds_to_bearings(self, name);
            let left = left_key
                .as_deref()
                .and_then(|reference| bounds_to_bearings(self, reference))
                .map(|value| value.0)
                .or_else(|| current.map(|value| value.0));
            let right = right_key
                .as_deref()
                .and_then(|reference| bounds_to_bearings(self, reference))
                .map(|value| value.1)
                .or_else(|| current.map(|value| value.1));
            if let (Some(left), Some(right)) = (left, right) {
                self.set_side_bearings(std::slice::from_ref(name), left.max(0.0), right.max(0.0));
            }

            let master_ids: Vec<String> = self
                .glyphs
                .get(name)
                .map(|glyph| glyph.layers.keys().cloned().collect())
                .unwrap_or_default();
            for master_id in master_ids {
                let mut snapshot = self.clone();
                for glyph in snapshot.glyphs.values_mut() {
                    if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                        glyph.width = layer.width;
                        glyph.contours = layer.contours;
                        glyph.components = layer.components;
                        glyph.anchors = layer.anchors;
                    }
                }
                let target_bounds = bounds_to_bearings(&snapshot, name);
                let reference_left = left_key
                    .as_deref()
                    .and_then(|reference| bounds_to_bearings(&snapshot, reference))
                    .map(|value| value.0);
                let reference_right = right_key
                    .as_deref()
                    .and_then(|reference| bounds_to_bearings(&snapshot, reference))
                    .map(|value| value.1);
                let Some((current_left, current_right)) = target_bounds else {
                    continue;
                };
                let desired_left = reference_left.unwrap_or(current_left).max(0.0);
                let desired_right = reference_right.unwrap_or(current_right).max(0.0);
                if let Some(glyph) = self.glyphs.get_mut(name) {
                    if let Some(layer) = glyph.layers.get_mut(&master_id) {
                        let min_x = current_left;
                        let max_x = layer.width - current_right;
                        let dx = desired_left - min_x;
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
                        layer.width = (max_x - min_x + desired_left + desired_right).max(0.0);
                    }
                }
            }
            changed += 1;
        }
        Ok(changed)
    }
}
