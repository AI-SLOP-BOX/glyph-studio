
fn build_gvar_variation(
    source: &GlyphData,
    project: &FontProject,
    base_id: &str,
    has_width_axis: bool,
    has_variation: &mut bool,
) -> Result<Option<fonttools::gvar::GlyphVariationData>, String> {
    let Some(base) = source.layers.get(base_id) else {
        return Ok(None);
    };
    // The static glyf table is built from GlyphData's active outline. Only
    // emit deltas when that outline is the same as the selected base layer;
    // otherwise the gvar point indices would describe a different shape.
    if source.width != base.width
        || source.contours != base.contours
        || source.components != base.components
    {
        return Ok(None);
    }
    let mut custom_axis_tags: Vec<String> = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    custom_axis_tags.retain(|tag| {
        let mut values = project
            .masters
            .iter()
            .map(|master| master.axes.get(tag).copied().unwrap_or(0.0));
        values
            .next()
            .is_some_and(|first| values.any(|value| (value - first).abs() > f64::EPSILON))
    });
    let axis_value = |master: &FontMaster, tag: &str| master.axes.get(tag).copied().unwrap_or(0.0);
    let base_master = project.masters.iter().find(|master| master.id == base_id);
    let widths: Vec<f64> = project.masters.iter().map(|master| master.width).collect();
    let min_width = widths.iter().copied().fold(f64::INFINITY, f64::min);
    let max_width = widths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let default_width = project
        .masters
        .iter()
        .find(|master| master.id == base_id)
        .map(|master| master.width)
        .unwrap_or_default();
    let mut deltasets = Vec::new();
    for master in project.masters.iter().filter(|master| master.id != base_id) {
        let Some(target) = source.layers.get(&master.id) else {
            continue;
        };
        let component_variation = !base.components.is_empty()
            && base.components.len() == target.components.len()
            && base
                .components
                .iter()
                .zip(&target.components)
                .all(|(a, b)| {
                    a.base == b.base
                        && a.x_scale == b.x_scale
                        && a.xy_scale == b.xy_scale
                        && a.yx_scale == b.yx_scale
                        && a.y_scale == b.y_scale
                });
        let contour_variation = base.components.is_empty()
            && target.components.is_empty()
            && base.contours.len() == target.contours.len()
            && base
                .contours
                .iter()
                .zip(&target.contours)
                .all(|(a, b)| a.points.len() == b.points.len());
        if !component_variation && !contour_variation {
            continue;
        }
        let mut deltas = Vec::new();
        if component_variation {
            for (a, b) in base.components.iter().zip(&target.components) {
                deltas.push((
                    checked_i16(b.x_offset, "gvar コンポーネントX")?
                        .checked_sub(checked_i16(a.x_offset, "gvar 基準コンポーネントX")?)
                        .ok_or_else(|| "gvar X差分が範囲外です".to_string())?,
                    checked_i16(b.y_offset, "gvar コンポーネントY")?
                        .checked_sub(checked_i16(a.y_offset, "gvar 基準コンポーネントY")?)
                        .ok_or_else(|| "gvar Y差分が範囲外です".to_string())?,
                ));
            }
        } else {
            for (a_contour, b_contour) in base.contours.iter().zip(&target.contours) {
                for (a, b) in a_contour.points.iter().zip(&b_contour.points) {
                    deltas.push((
                        checked_i16(b.x, "gvar ターゲットX")?
                            .checked_sub(checked_i16(a.x, "gvar 基準X")?)
                            .ok_or_else(|| "gvar X差分が範囲外です".to_string())?,
                        checked_i16(b.y, "gvar ターゲットY")?
                            .checked_sub(checked_i16(a.y, "gvar 基準Y")?)
                            .ok_or_else(|| "gvar Y差分が範囲外です".to_string())?,
                    ));
                }
            }
        }
        deltas.extend([
            (0, 0),
            (
                checked_i16(target.width, "gvar ターゲット幅")?
                    .checked_sub(checked_i16(base.width, "gvar 基準幅")?)
                    .ok_or_else(|| "gvar 幅差分が範囲外です".to_string())?,
                0,
            ),
            (0, 0),
            (0, 0),
        ]);
        if deltas.iter().any(|(x, y)| *x != 0 || *y != 0) {
            *has_variation = true;
            let custom_peaks = custom_axis_tags.iter().map(|tag| {
                let values: Vec<f64> = project.masters.iter().map(|m| axis_value(m, tag)).collect();
                normalize_axis(
                    axis_value(master, tag),
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    base_master.map(|m| axis_value(m, tag)).unwrap_or_default(),
                    values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            });
            let weight_peak = if custom_axis_tags.is_empty() {
                let values: Vec<f64> = project.masters.iter().map(|m| m.weight).collect();
                normalize_axis(
                    master.weight,
                    values.iter().copied().fold(f64::INFINITY, f64::min),
                    base_master.map(|m| m.weight).unwrap_or_default(),
                    values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            } else {
                0.0
            };
            let width_peak = normalize_axis(master.width, min_width, default_width, max_width);
            deltasets.push(fonttools::gvar::DeltaSet {
                peak: if has_width_axis {
                    if custom_axis_tags.is_empty() {
                        vec![weight_peak, width_peak]
                    } else {
                        custom_peaks.chain(std::iter::once(width_peak)).collect()
                    }
                } else {
                    if custom_axis_tags.is_empty() {
                        vec![weight_peak]
                    } else {
                        custom_peaks.collect()
                    }
                },
                start: if has_width_axis {
                    vec![
                        0.0;
                        custom_axis_tags.len()
                            + usize::from(has_width_axis || custom_axis_tags.is_empty())
                    ]
                } else {
                    vec![0.0; custom_axis_tags.len().max(1)]
                },
                end: if has_width_axis {
                    vec![
                        1.0;
                        custom_axis_tags.len()
                            + usize::from(has_width_axis || custom_axis_tags.is_empty())
                    ]
                } else {
                    vec![1.0; custom_axis_tags.len().max(1)]
                },
                deltas,
            });
        }
    }
    Ok((!deltasets.is_empty()).then_some(fonttools::gvar::GlyphVariationData { deltasets }))
}
