
fn validate_master_axes(project: &FontProject) -> Result<(), String> {
    for master in &project.masters {
        if !master.weight.is_finite() || !(1.0..=1000.0).contains(&master.weight) {
            return Err(format!("マスター '{}' のWeightが不正です", master.name));
        }
        if !master.width.is_finite() || !(1.0..=1000.0).contains(&master.width) {
            return Err(format!("マスター '{}' のWidthが不正です", master.name));
        }
        for (tag, value) in &master.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                return Err(format!(
                    "マスター '{}' の軸タグ '{}' が不正です",
                    master.name, tag
                ));
            }
            if tag == "wdth" {
                return Err("カスタム軸タグ 'wdth' はWidth属性と重複します".into());
            }
            if tag == "wght" {
                return Err("カスタム軸タグ 'wght' はWeight属性と重複します".into());
            }
            if !value.is_finite() || *value < f32::MIN as f64 || *value > f32::MAX as f64 {
                return Err(format!(
                    "マスター '{}' の軸 '{}' の値が不正です",
                    master.name, tag
                ));
            }
        }
    }
    for instance in &project.instances {
        if !instance.weight.is_finite() || !(1.0..=1000.0).contains(&instance.weight) {
            return Err(format!(
                "名前付きインスタンス '{}' のWeightが不正です",
                instance.name
            ));
        }
        if !instance.width.is_finite() || !(1.0..=1000.0).contains(&instance.width) {
            return Err(format!(
                "名前付きインスタンス '{}' のWidthが不正です",
                instance.name
            ));
        }
        for (tag, value) in &instance.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸タグ '{}' が不正です",
                    instance.name, tag
                ));
            }
            if tag.eq_ignore_ascii_case("wght") || tag.eq_ignore_ascii_case("wdth") {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸 '{}' はWeight/Width属性と重複します",
                    instance.name, tag
                ));
            }
            if !value.is_finite() || *value < f32::MIN as f64 || *value > f32::MAX as f64 {
                return Err(format!(
                    "名前付きインスタンス '{}' の軸 '{}' の値が不正です",
                    instance.name, tag
                ));
            }
        }
    }
    Ok(())
}
