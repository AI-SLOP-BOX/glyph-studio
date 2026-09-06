
fn build_metric_variation_store<F>(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
    metric: F,
) -> Option<(
    write_fonts::tables::variations::ItemVariationStore,
    Vec<write_fonts::tables::layout::VariationIndex>,
)>
where
    F: Fn(&str, &str) -> f64,
{
    if project.masters.len() < 2 || axis_tags.is_empty() {
        return None;
    }
    let axis_value = |master: &FontMaster, tag: &str| match tag {
        "wght" => master.weight,
        "wdth" => master.width,
        _ => master.axes.get(tag).copied().unwrap_or(0.0),
    };
    let bounds = axis_tags
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
            let axes = bounds
                .iter()
                .map(|(tag, min, default, max)| {
                    let peak = normalize_axis(axis_value(master, tag), *min, *default, *max);
                    write_fonts::tables::variations::RegionAxisCoordinates::new(
                        write_fonts::types::F2Dot14::from_f32(peak.min(0.0)),
                        write_fonts::types::F2Dot14::from_f32(peak),
                        write_fonts::types::F2Dot14::from_f32(peak.max(0.0)),
                    )
                })
                .collect();
            (
                master.id.clone(),
                write_fonts::tables::variations::VariationRegion::new(axes),
            )
        })
        .collect::<Vec<_>>();
    if regions.is_empty() {
        return None;
    }
    let mut builder = write_fonts::tables::variations::ivs_builder::VariationStoreBuilder::new(
        axis_tags.len() as u16,
    );
    let mut temporary = vec![builder.add_deltas::<i32>(Vec::new())];
    let mut changed = false;
    for name in names {
        let base = metric(name, &base_master.id);
        let deltas = regions
            .iter()
            .map(|(master_id, region)| {
                let delta = (metric(name, master_id) - base).round() as i32;
                changed |= delta != 0;
                (region.clone(), delta)
            })
            .collect();
        temporary.push(builder.add_deltas(deltas));
    }
    if !changed {
        return None;
    }
    let (store, remapping) = builder.build();
    let mapping = temporary
        .into_iter()
        .map(|index| remapping.get(index).unwrap())
        .collect();
    Some((store, mapping))
}
