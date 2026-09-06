
fn build_mvar(
    project: &FontProject,
    base_master: &FontMaster,
    axis_tags: &[String],
) -> Option<Vec<u8>> {
    let metric_names = ["ascender", "descender", "lineGap"];
    let (store, mapping) = build_metric_variation_store(
        project,
        &metric_names,
        base_master,
        axis_tags,
        |name, master_id| {
            let metrics = project.master_metrics_for(master_id);
            match name {
                "ascender" => metrics.ascender,
                "descender" => metrics.descender,
                _ => metrics.line_gap,
            }
        },
    )?;
    let store_bytes = write_fonts::dump_table(&store).ok()?;
    let record_size = 8u16;
    let record_count = u16::try_from(metric_names.len()).ok()?;
    let store_offset = 12usize + usize::from(record_size) * metric_names.len();
    let mut bytes = Vec::with_capacity(store_offset + store_bytes.len());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&record_size.to_be_bytes());
    bytes.extend_from_slice(&record_count.to_be_bytes());
    bytes.extend_from_slice(&u16::try_from(store_offset).ok()?.to_be_bytes());
    for (tag, index) in [(*b"hasc", 1usize), (*b"hdsc", 2), (*b"hlgp", 3)] {
        let variation_index = mapping.get(index)?;
        bytes.extend_from_slice(&tag);
        bytes.extend_from_slice(&variation_index.delta_set_outer_index.to_be_bytes());
        bytes.extend_from_slice(&variation_index.delta_set_inner_index.to_be_bytes());
    }
    bytes.extend_from_slice(&store_bytes);
    Some(bytes)
}
