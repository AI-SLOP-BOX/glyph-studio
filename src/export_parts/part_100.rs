
fn parse_feature_value_records(text: &str) -> Vec<ParsedGposValueRecord> {
    top_level_angle_groups(text)
        .into_iter()
        .filter_map(|group| {
            let nested_devices = top_level_angle_groups(&group);
            let mut stripped = String::with_capacity(group.len());
            let mut depth = 0_i32;
            for character in group.chars() {
                match character {
                    '<' => depth += 1,
                    '>' => depth = (depth - 1).max(0),
                    _ if depth == 0 => stripped.push(character),
                    _ => {}
                }
            }
            let values = stripped
                .split_whitespace()
                .filter_map(|value| value.parse::<i16>().ok())
                .collect::<Vec<_>>();
            if values.is_empty() && nested_devices.is_empty() {
                return None;
            }
            let mut parsed = ParsedGposValueRecord {
                values,
                ..Default::default()
            };
            for (index, device) in nested_devices.into_iter().enumerate().take(4) {
                parsed.devices[index] = parse_feature_device(&device)?;
            }
            Some(parsed)
        })
        .collect()
}
