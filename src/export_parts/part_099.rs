
fn parse_feature_device(value: &str) -> Option<Option<layout::DeviceOrVariationIndex>> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() == 1 && tokens.first()?.eq_ignore_ascii_case("NULL") {
        return Some(None);
    }
    if !tokens.first()?.eq_ignore_ascii_case("device") {
        return None;
    }
    if tokens
        .get(1)
        .is_some_and(|token| token.eq_ignore_ascii_case("NULL"))
    {
        return Some(None);
    }
    let numbers = tokens[1..]
        .iter()
        .flat_map(|token| token.split(','))
        .filter_map(|token| token.parse::<i16>().ok())
        .collect::<Vec<_>>();
    if numbers.len() < 2 || numbers.len() % 2 != 0 {
        return None;
    }
    let first_ppem = numbers[0];
    let last_ppem = numbers[numbers.len() - 2];
    if !(0..=255).contains(&first_ppem) || !(0..=255).contains(&last_ppem) || last_ppem < first_ppem
    {
        return None;
    }
    let mut values = vec![0_i8; (last_ppem - first_ppem + 1) as usize];
    for pair in numbers.chunks(2) {
        let ppem = pair[0];
        let delta = pair[1];
        if !(0..=255).contains(&ppem) || !(-128..=127).contains(&delta) {
            return None;
        }
        values[(ppem - first_ppem) as usize] = delta as i8;
    }
    Some(Some(layout::DeviceOrVariationIndex::device(
        first_ppem as u16,
        last_ppem as u16,
        &values,
    )))
}
