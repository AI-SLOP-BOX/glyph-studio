
#[derive(Default)]
struct ParsedGposValueRecord {
    values: Vec<i16>,
    devices: [Option<layout::DeviceOrVariationIndex>; 4],
}
