
pub fn export_interpolation_set(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
    factors: &[f64],
    directory: &Path,
) -> Result<usize, String> {
    if factors.is_empty() {
        return Err("補間率を1つ以上指定してください".to_string());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut seen = std::collections::HashSet::new();
    let mut count = 0;
    for &factor in factors {
        if !factor.is_finite() || !(0.0..=1.0).contains(&factor) {
            return Err("補間率は0〜1の範囲で指定してください".to_string());
        }
        let key = (factor * 1000.0).round() as i64;
        if !seen.insert(key) {
            return Err(format!("補間率 {:.1}% が重複しています", factor * 100.0));
        }
        let filename = format!("instance-{:.0}.ttf", factor * 100.0);
        export_ttf_at_interpolation(
            project,
            from_master_id,
            to_master_id,
            factor,
            &directory.join(filename),
        )?;
        count += 1;
    }
    Ok(count)
}
