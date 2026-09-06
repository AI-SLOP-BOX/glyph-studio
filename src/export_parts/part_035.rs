
/// Builds the CFF vertical-origin table from the active outlines and vertical
/// side bearings. The default origin is used whenever a glyph agrees with it,
/// keeping the table compact while preserving per-glyph Japanese vertical
/// metrics where they differ.
fn build_vorg(project: &FontProject, master_id: &str) -> Result<Vec<u8>, String> {
    let default_origin = checked_i16(
        project.master_metrics_for(master_id).ascender,
        "VORGデフォルト原点",
    )?;
    let mut records = Vec::new();
    for (glyph_id, name) in project.glyph_names_sorted().iter().enumerate() {
        let Some((_, _, _, max_y)) = project.outline_bounds_for_glyph(name) else {
            continue;
        };
        let metric = project.vertical_metrics_for_glyph_in_master(name, master_id);
        let origin = checked_i16(max_y + metric.top_side_bearing, "VORG原点")?;
        if origin != default_origin {
            records.push((
                u16::try_from(glyph_id + 1).map_err(|_| "VORGのグリフ数が多すぎます")?,
                origin,
            ));
        }
    }
    let mut bytes = Vec::with_capacity(8 + records.len() * 4);
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&default_origin.to_be_bytes());
    bytes.extend_from_slice(
        &u16::try_from(records.len())
            .map_err(|_| "VORGレコード数が多すぎます")?
            .to_be_bytes(),
    );
    for (glyph_id, origin) in records {
        bytes.extend_from_slice(&glyph_id.to_be_bytes());
        bytes.extend_from_slice(&origin.to_be_bytes());
    }
    Ok(bytes)
}
