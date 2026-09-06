
/// Finds the four masters of a rectangular two-axis design space.
/// Returns bottom-left, bottom-right, top-left, top-right indices and the
/// normalized target factors. Irregular or incomplete layouts are rejected.
pub fn find_bilinear_masters(
    masters: &[FontMaster],
    axis_x: &str,
    axis_y: &str,
    target_x: f64,
    target_y: f64,
) -> Option<([usize; 4], (f64, f64))> {
    if masters.len() < 4 || axis_x == axis_y {
        return None;
    }
    let coordinates: Vec<(usize, f64, f64)> = masters
        .iter()
        .enumerate()
        .filter_map(|(index, master)| {
            Some((index, *master.axes.get(axis_x)?, *master.axes.get(axis_y)?))
        })
        .collect();
    if coordinates.len() < 4 {
        return None;
    }
    let min_x = coordinates
        .iter()
        .map(|(_, x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let max_x = coordinates
        .iter()
        .map(|(_, x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = coordinates
        .iter()
        .map(|(_, _, y)| *y)
        .fold(f64::INFINITY, f64::min);
    let max_y = coordinates
        .iter()
        .map(|(_, _, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    if (max_x - min_x).abs() < f64::EPSILON || (max_y - min_y).abs() < f64::EPSILON {
        return None;
    }
    let corner = |x: f64, y: f64| {
        coordinates
            .iter()
            .find(|(_, cx, cy)| (*cx - x).abs() < 1e-9 && (*cy - y).abs() < 1e-9)
            .map(|(index, _, _)| *index)
    };
    let indices = [
        corner(min_x, min_y)?,
        corner(max_x, min_y)?,
        corner(min_x, max_y)?,
        corner(max_x, max_y)?,
    ];
    Some((
        indices,
        (
            ((target_x - min_x) / (max_x - min_x)).clamp(0.0, 1.0),
            ((target_y - min_y) / (max_y - min_y)).clamp(0.0, 1.0),
        ),
    ))
}
