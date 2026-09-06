
fn variable_master_axis_values(
    project: &FontProject,
) -> Vec<(String, std::collections::HashMap<String, f64>)> {
    let custom_tags = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut tags = custom_tags.iter().cloned().collect::<Vec<_>>();
    if custom_tags.is_empty() {
        tags.push("wght".into());
    }
    if project
        .masters
        .iter()
        .any(|master| (master.width - project.masters[0].width).abs() > f64::EPSILON)
    {
        tags.push("wdth".into());
    }
    tags.into_iter()
        .map(|tag| {
            let values = project
                .masters
                .iter()
                .map(|master| {
                    let value = match tag.as_str() {
                        "wght" if custom_tags.is_empty() => master.weight,
                        "wdth" => master.width,
                        _ => master.axes.get(&tag).copied().unwrap_or(0.0),
                    };
                    (master.id.clone(), value)
                })
                .collect();
            (tag, values)
        })
        .collect()
}
