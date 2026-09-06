
fn build_vertical_metrics_tables(
    project: &FontProject,
    names: &[&str],
    master_id: &str,
    upm: u16,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let master_metrics = project.master_metrics_for(master_id);
    let mut metrics = vec![(upm, checked_i16(master_metrics.ascender, "縦TSB")?)];
    metrics.extend(
        names
            .iter()
            .map(|name| project.vertical_metrics_for_glyph_in_master(name, master_id))
            .map(|metric| {
                Ok((
                    checked_u16(metric.advance_height, "縦アドバンス")?,
                    checked_i16(metric.top_side_bearing, "縦TSB")?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?,
    );
    let max_advance = metrics
        .iter()
        .map(|(advance, _)| *advance)
        .max()
        .unwrap_or(upm);
    let mut vhea = Vec::with_capacity(36);
    vhea.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.ascender, "縦アセンダ")?.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.descender, "縦ディセンダ")?.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.line_gap, "縦Line Gap")?.to_be_bytes());
    vhea.extend_from_slice(&max_advance.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&checked_i16(master_metrics.ascender, "縦Y最大")?.to_be_bytes());
    vhea.extend_from_slice(
        &if project.metadata.vertical_caret_slope_rise != 0 {
            project.metadata.vertical_caret_slope_rise
        } else {
            1
        }
        .to_be_bytes(),
    );
    vhea.extend_from_slice(&project.metadata.vertical_caret_slope_run.to_be_bytes());
    vhea.extend_from_slice(&project.metadata.vertical_caret_offset.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(&0_i16.to_be_bytes());
    vhea.extend_from_slice(
        &(u16::try_from(metrics.len()).map_err(|_| "縦メトリクスが多すぎます")?).to_be_bytes(),
    );
    let mut vmtx = Vec::with_capacity(metrics.len() * 4);
    for (advance, bearing) in metrics {
        vmtx.extend_from_slice(&advance.to_be_bytes());
        vmtx.extend_from_slice(&bearing.to_be_bytes());
    }
    Ok((vhea, vmtx))
}
