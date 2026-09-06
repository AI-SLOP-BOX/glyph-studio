
fn build_stat_table_with_values(
    axes: &[([u8; 4], u16)],
    values: &[Vec<f32>],
    value_name_ids: &[u16],
) -> Vec<u8> {
    let axis_value_count = values.len().min(value_name_ids.len());
    let offsets_start = 20 + axes.len() * 8;
    let values_start = offsets_start + axis_value_count * 2;
    let mut axis_value_tables = Vec::new();
    let mut offsets = Vec::with_capacity(axis_value_count);
    for (coordinates, value_name_id) in values.iter().zip(value_name_ids).take(axis_value_count) {
        let mut record = Vec::with_capacity(8 + axes.len() * 4);
        record.extend_from_slice(&4_u16.to_be_bytes());
        record.extend_from_slice(&(axes.len() as u16).to_be_bytes());
        record.extend_from_slice(&0_u16.to_be_bytes());
        record.extend_from_slice(&value_name_id.to_be_bytes());
        for (axis_index, coordinate) in coordinates.iter().enumerate().take(axes.len()) {
            record.extend_from_slice(&(axis_index as u16).to_be_bytes());
            let fixed = (*coordinate * 65536.0).round() as i32;
            record.extend_from_slice(&fixed.to_be_bytes());
        }
        offsets.push((values_start + axis_value_tables.len()) as u16);
        axis_value_tables.extend(record);
    }
    let mut table = Vec::with_capacity(values_start + axis_value_tables.len());
    table.extend_from_slice(&0x0001_0002_u32.to_be_bytes());
    table.extend_from_slice(&8_u16.to_be_bytes());
    table.extend_from_slice(&(axes.len() as u16).to_be_bytes());
    table.extend_from_slice(&20_u32.to_be_bytes());
    table.extend_from_slice(&(axis_value_count as u16).to_be_bytes());
    table.extend_from_slice(&(offsets_start as u32).to_be_bytes());
    table.extend_from_slice(&2_u16.to_be_bytes());
    for (tag, name_id) in axes {
        table.extend_from_slice(tag);
        table.extend_from_slice(&name_id.to_be_bytes());
        table.extend_from_slice(&0_u16.to_be_bytes());
    }
    for offset in offsets {
        table.extend_from_slice(&offset.to_be_bytes());
    }
    table.extend_from_slice(&axis_value_tables);
    table
}

