
fn build_vvar(
    project: &FontProject,
    names: &[&str],
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    let (store, mapping) =
        build_metric_variation_store(project, names, base_master, axis_tags, |name, master_id| {
            project
                .vertical_metrics_for_glyph_in_master(name, master_id)
                .advance_height
        })?;
    let mapping: write_fonts::tables::variations::DeltaSetIndexMap = mapping.into_iter().collect();
    write_fonts::dump_table(&write_fonts::tables::vvar::Vvar::new(
        store,
        Some(mapping),
        None,
        None,
        None,
    ))
    .ok()
}
