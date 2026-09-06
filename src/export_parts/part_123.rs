
fn build_hvar(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    if project.masters.len() < 2 || axis_tags.is_empty() {
        return None;
    }
    let axis_value = |master: &FontMaster, tag: &str| match tag {
        "wght" => master.weight,
        "wdth" => master.width,
        _ => master.axes.get(tag).copied().unwrap_or(0.0),
    };
    let axis_bounds = axis_tags
        .iter()
        .map(|tag| {
            let values = project.masters.iter().map(|master| axis_value(master, tag));
            (
                tag,
                values.clone().fold(f64::INFINITY, f64::min),
                axis_value(base_master, tag),
                values.fold(f64::NEG_INFINITY, f64::max),
            )
        })
        .collect::<Vec<_>>();
    let regions = project
        .masters
        .iter()
        .filter(|master| master.id != base_master.id)
        .map(|master| {
            let coords = axis_bounds
                .iter()
                .map(|(tag, min, default, max)| {
                    let peak = normalize_axis(axis_value(master, tag), *min, *default, *max);
                    let start = peak.min(0.0);
                    let end = peak.max(0.0);
                    write_fonts::tables::variations::RegionAxisCoordinates::new(
                        write_fonts::types::F2Dot14::from_f32(start),
                        write_fonts::types::F2Dot14::from_f32(peak),
                        write_fonts::types::F2Dot14::from_f32(end),
                    )
                })
                .collect::<Vec<_>>();
            (
                master.id.clone(),
                write_fonts::tables::variations::VariationRegion::new(coords),
            )
        })
        .collect::<Vec<_>>();
    if regions.is_empty() {
        return None;
    }
    let base_width = |name: &str| {
        project
            .glyphs
            .get(name)
            .and_then(|glyph| glyph.layers.get(&base_master.id).map(|layer| layer.width))
            .or_else(|| project.glyphs.get(name).map(|glyph| glyph.width))
            .unwrap_or(0.0)
    };
    let mut builder = write_fonts::tables::variations::ivs_builder::VariationStoreBuilder::new(
        axis_tags.len() as u16,
    );
    let mut temporary_indices = Vec::with_capacity(names.len() + 1);
    let mut has_delta = false;
    temporary_indices.push(builder.add_deltas::<i32>(Vec::new()));
    for name in names {
        let base = base_width(name);
        let deltas = regions
            .iter()
            .map(|(master_id, region)| {
                let target = project
                    .glyphs
                    .get(*name)
                    .and_then(|glyph| glyph.layers.get(master_id))
                    .map(|layer| layer.width)
                    .unwrap_or(base);
                let delta = (target - base).round() as i32;
                has_delta |= delta != 0;
                (region.clone(), delta)
            })
            .collect::<Vec<_>>();
        temporary_indices.push(builder.add_deltas(deltas));
    }
    if !has_delta {
        return None;
    }
    let (store, remapping) = builder.build();
    let mapping: write_fonts::tables::variations::DeltaSetIndexMap = temporary_indices
        .into_iter()
        .map(|index| remapping.get(index).unwrap())
        .collect();
    write_fonts::dump_table(&write_fonts::tables::hvar::Hvar::new(
        store,
        Some(mapping),
        None,
        None,
    ))
    .ok()
}
