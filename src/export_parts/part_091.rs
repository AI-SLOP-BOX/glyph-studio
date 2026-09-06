
fn project_axis_bounds(project: &FontProject, axis_index: usize) -> (f64, f64, f64) {
    let axes = variable_master_axis_values(project);
    let Some((tag, values)) = axes.get(axis_index) else {
        return (-1.0, 0.0, 1.0);
    };
    let mut coordinates = values.values().copied().collect::<Vec<_>>();
    coordinates.sort_by(f64::total_cmp);
    let min = coordinates.first().copied().unwrap_or(0.0);
    let max = coordinates.last().copied().unwrap_or(0.0);
    let default_id = &project.default_master_id;
    let default = values.get(default_id).copied().unwrap_or_else(|| {
        if tag == "wght" {
            project
                .masters
                .first()
                .map(|master| master.weight)
                .unwrap_or(0.0)
        } else {
            0.0
        }
    });
    (min, default, max)
}
