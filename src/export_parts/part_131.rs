
fn build_avar(
    axis_tags: &[String],
    mappings: &std::collections::HashMap<String, Vec<AxisMappingPoint>>,
) -> Option<Vec<u8>> {
    let mut axis_maps = Vec::new();
    let mut has_non_identity = false;
    for tag in axis_tags {
        let mut points = mappings
            .get(tag)
            .into_iter()
            .flatten()
            .copied()
            .filter(|point| point.input.is_finite() && point.output.is_finite())
            .map(|point| (point.input.clamp(-1.0, 1.0), point.output.clamp(-1.0, 1.0)))
            .collect::<Vec<_>>();
        points.sort_by(|left, right| left.0.total_cmp(&right.0));
        points.dedup_by(|left, right| (left.0 - right.0).abs() < f64::EPSILON);
        if points.is_empty() {
            points = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
        }
        if !points.iter().any(|(input, _)| input.abs() < f64::EPSILON) {
            points.push((0.0, 0.0));
            points.sort_by(|left, right| left.0.total_cmp(&right.0));
        }
        if !points
            .iter()
            .any(|(input, _)| (*input + 1.0).abs() < f64::EPSILON)
        {
            points.insert(0, (-1.0, -1.0));
        }
        if !points
            .iter()
            .any(|(input, _)| (*input - 1.0).abs() < f64::EPSILON)
        {
            points.push((1.0, 1.0));
        }
        if points
            .iter()
            .all(|(input, output)| (input - output).abs() < f64::EPSILON)
        {
            axis_maps.push(points);
        } else {
            has_non_identity = true;
            axis_maps.push(points);
        }
    }
    if axis_maps.len() != axis_tags.len() || !has_non_identity {
        return None;
    }
    let mut output = Vec::new();
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(axis_tags.len() as u16).to_be_bytes());
    for points in axis_maps {
        output.extend_from_slice(&(points.len() as u16).to_be_bytes());
        for (input, output_value) in points {
            output.extend_from_slice(&normalized_f2dot14(input).to_be_bytes());
            output.extend_from_slice(&normalized_f2dot14(output_value).to_be_bytes());
        }
    }
    Some(output)
}
