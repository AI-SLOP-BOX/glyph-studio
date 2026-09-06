#[rustfmt::skip]
pub fn validate_project(project: &FontProject) -> Vec<String> {
    let mut issues = Vec::new();
    let is_noncharacter = |unicode: u32| (0xFDD0..=0xFDEF).contains(&unicode) || (unicode & 0xFFFF) >= 0xFFFE;
    let mut variation_keys = std::collections::HashSet::new();
    for variation in &project.unicode_variation_sequences {
        let valid_scalar = variation.base <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&variation.base) && variation.selector <= 0xFFFFFF;
        let valid_selector = (0xFE00..=0xFE0F).contains(&variation.selector) || (0xE0100..=0xE01EF).contains(&variation.selector);
        if !valid_scalar || !valid_selector {
            issues.push(format!("IVSのUnicodeまたはセレクタが不正です: U+{:X} U+{:X}", variation.base, variation.selector));
        }
        if !project.glyphs.contains_key(&variation.glyph) {
            issues.push(format!("IVSが存在しないグリフ '{}' を参照しています", variation.glyph));
        }
        if !variation_keys.insert((variation.base, variation.selector)) {
            issues.push(format!("IVSのUnicode／セレクタが重複しています: U+{:X} U+{:X}", variation.base, variation.selector));
        }
    }
    for (tag, points) in &project.axis_mappings {
        if tag.len() != 4 || !tag.is_ascii() {
            issues.push(format!("avar軸タグ '{}' はASCII 4文字で指定してください", tag));
        }
        let mut inputs = std::collections::HashSet::new();
        for point in points {
            if !point.input.is_finite() || !point.output.is_finite() || !(-1.0..=1.0).contains(&point.input) || !(-1.0..=1.0).contains(&point.output) {
                issues.push(format!("avar軸 '{}' のマッピング値は-1.0〜1.0の有限値で指定してください", tag));
            }
            if !inputs.insert(point.input.to_bits()) {
                issues.push(format!("avar軸 '{}' の入力座標が重複しています", tag));
            }
        }
    }
    if project.metadata.family_name.trim().is_empty() {
        issues.push("ファミリー名が空です".into());
    }
    if project.metadata.style_name.trim().is_empty() {
        issues.push("スタイル名が空です".into());
    }
    if !project.metadata.font_revision.is_finite() || !(0.0..=65535.0).contains(&project.metadata.font_revision) {
        issues.push("フォントバージョンが0〜65535の範囲外です".into());
    }
    if !project.metadata.units_per_em.is_finite() || !(16.0..=16384.0).contains(&project.metadata.units_per_em) {
        issues.push("UPMが16〜16384の範囲外です".into());
    }
    if !project.metadata.ascender.is_finite() || !project.metadata.descender.is_finite() || !project.metadata.line_gap.is_finite() {
        issues.push("フォントメトリクスに不正な値があります".into());
    }
    for (master_id, metrics) in &project.metrics_by_master {
        if !project.masters.iter().any(|master| master.id == *master_id) {
            issues.push(format!("マスターメトリクスが存在しないマスター '{}' を参照しています", master_id));
        }
        if !metrics.ascender.is_finite()
            || !metrics.descender.is_finite()
            || !metrics.line_gap.is_finite()
            || metrics.ascender < i16::MIN as f64
            || metrics.ascender > i16::MAX as f64
            || metrics.descender < i16::MIN as f64
            || metrics.descender > i16::MAX as f64
            || metrics.line_gap < i16::MIN as f64
            || metrics.line_gap > i16::MAX as f64
            || metrics.ascender.fract() != 0.0
            || metrics.descender.fract() != 0.0
            || metrics.line_gap.fract() != 0.0
        {
            issues.push(format!("マスター '{}' のメトリクスがTrueTypeの範囲外です", master_id));
        }
    }
    for (glyph_name, masters) in &project.background_opacities {
        for (master_id, opacity) in masters {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!("背景画像不透明度が存在しないグリフ '{}' を参照しています", glyph_name));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!("背景画像不透明度が存在しないマスター '{}' を参照しています", master_id));
            }
            if !opacity.is_finite() || !(0.0..=1.0).contains(opacity) {
                issues.push(format!("グリフ '{}' の背景画像不透明度（マスター '{}'）が0〜1の範囲外です", glyph_name, master_id));
            }
        }
    }
    for (glyph_name, masters) in &project.background_images {
        for master_id in masters.keys() {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!("背景画像が存在しないグリフ '{}' を参照しています", glyph_name));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!("背景画像が存在しないマスター '{}' を参照しています", master_id));
            }
        }
    }
    for (glyph_name, masters) in &project.background_transforms {
        for (master_id, transform) in masters {
            if !project.glyphs.contains_key(glyph_name) {
                issues.push(format!("背景画像変形が存在しないグリフ '{}' を参照しています", glyph_name));
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!("背景画像変形が存在しないマスター '{}' を参照しています", master_id));
            }
            if !transform.x.is_finite() || !transform.y.is_finite() || !transform.scale.is_finite() || !transform.rotation.is_finite() || transform.scale <= 0.0 {
                issues.push(format!("グリフ '{}' の背景画像変形（マスター '{}'）に不正な値があります", glyph_name, master_id));
            }
        }
    }
    for (glyph_name, layers) in &project.conditional_layers {
        if !project.glyphs.contains_key(glyph_name) {
            issues.push(format!("条件レイヤーが存在しないグリフ '{}' を参照しています", glyph_name));
        }
        let mut layer_ids = std::collections::HashSet::new();
        for layer in layers {
            if layer.id.trim().is_empty() || !layer_ids.insert(layer.id.clone()) {
                issues.push(format!("グリフ '{}' の条件レイヤーIDが空または重複しています", glyph_name));
            }
            for (tag, range) in &layer.conditions {
                if tag.len() != 4 || !tag.is_ascii() {
                    issues.push(format!("条件レイヤー '{}' の軸タグ '{}' が不正です", layer.id, tag));
                } else if !tag.eq_ignore_ascii_case("wght") && !tag.eq_ignore_ascii_case("wdth") && !project.masters.iter().any(|master| master.axes.keys().any(|axis| axis.eq_ignore_ascii_case(tag)))
                {
                    issues.push(format!("条件レイヤー '{}' が未定義の軸 '{}' を参照しています", layer.id, tag));
                }
                if range.min.zip(range.max).is_some_and(|(min, max)| min > max) || range.min.is_some_and(|value| !value.is_finite()) || range.max.is_some_and(|value| !value.is_finite()) {
                    issues.push(format!("条件レイヤー '{}' の軸範囲が不正です", layer.id));
                }
            }
        }
    }
    let mut unicodes = std::collections::HashMap::<u32, String>::new();
    let mut master_ids = std::collections::HashSet::new();
    let mut master_names = std::collections::HashSet::new();
    for master in &project.masters {
        if master.id.trim().is_empty() {
            issues.push("マスターIDが空です".into());
        } else if !master_ids.insert(master.id.clone()) {
            issues.push(format!("マスターIDが重複しています: {}", master.id));
        }
        if master.name.trim().is_empty() {
            issues.push(format!("マスター '{}' の表示名が空です", master.id));
        } else if !master_names.insert(master.name.clone()) {
            issues.push(format!("マスター名が重複しています: {}", master.name));
        }
        if !master.weight.is_finite() || !master.width.is_finite() {
            issues.push(format!("マスター '{}' のWeightまたはWidthが不正です", master.name));
        }
        for (tag, value) in &master.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                issues.push(format!("マスター '{}' の軸タグ '{}' は4文字ASCIIで指定してください", master.name, tag));
            }
            if !value.is_finite() {
                issues.push(format!("マスター '{}' の軸 '{}' の値が不正です", master.name, tag));
            }
        }
    }
    let mut instance_names = std::collections::HashSet::new();
    for instance in &project.instances {
        if instance.name.trim().is_empty() {
            issues.push("名前付きインスタンスの表示名が空です".into());
        } else if !instance_names.insert(instance.name.trim().to_string()) {
            issues.push(format!("名前付きインスタンス名が重複しています: {}", instance.name.trim()));
        }
        if !instance.weight.is_finite() || !instance.width.is_finite() {
            issues.push(format!("名前付きインスタンス '{}' のWeightまたはWidthが不正です", instance.name));
        }
        for (tag, value) in &instance.axes {
            if tag.len() != 4 || !tag.is_ascii() {
                issues.push(format!("名前付きインスタンス '{}' の軸タグ '{}' は4文字ASCIIで指定してください", instance.name, tag));
            }
            if !value.is_finite() {
                issues.push(format!("名前付きインスタンス '{}' の軸 '{}' の値が不正です", instance.name, tag));
            }
        }
    }
    if project.masters.is_empty() {
        issues.push("マスターが1つもありません".into());
    } else if !master_ids.contains(&project.default_master_id) {
        issues.push(format!("デフォルトマスター '{}' が存在しません", project.default_master_id));
    }
    let mut axis_display_names = std::collections::HashSet::new();
    for (tag, name) in &project.axis_names {
        if !master_ids
            .iter()
            .any(|master_id| project.masters.iter().find(|master| &master.id == master_id).is_some_and(|master| master.axes.contains_key(tag)))
        {
            issues.push(format!("軸名 '{}' が存在しない軸タグを参照しています", tag));
        }
        if name.trim().is_empty() {
            issues.push(format!("軸タグ '{}' の表示名が空です", tag));
        } else if !axis_display_names.insert(name.trim().to_string()) {
            issues.push(format!("可変軸の表示名が重複しています: {}", name.trim()));
        }
    }
    let mut ordered_glyphs = std::collections::HashSet::new();
    for name in &project.glyph_order {
        if !project.glyphs.contains_key(name) {
            issues.push(format!("グリフ順序に未定義グリフがあります: {name}"));
        } else if !ordered_glyphs.insert(name) {
            issues.push(format!("グリフ順序に重複があります: {name}"));
        }
    }
    for (index, guide) in project.guidelines.iter().enumerate() {
        if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
            issues.push(format!("ガイド{}の座標または角度が不正です", index + 1));
        }
    }
    for (name, glyph) in &project.glyphs {
        if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
            issues.push(format!("グリフ名が不正です: '{name}'"));
        }
        if glyph.name != *name {
            issues.push(format!("グリフ名の登録が不一致です: '{name}' / '{}'", glyph.name));
        }
        if !glyph.width.is_finite() || glyph.width < 0.0 {
            issues.push(format!("グリフ '{}' の幅が不正です", name));
        }
        for (label, group) in [("左カーニンググループ", glyph.left_kerning_group.trim()), ("右カーニンググループ", glyph.right_kerning_group.trim())] {
            if group.chars().any(char::is_whitespace) {
                issues.push(format!("グリフ '{}' の{}名に空白があります: '{}'", name, label, group));
            }
        }
        let mut anchor_names = std::collections::HashSet::new();
        for anchor in &glyph.anchors {
            if anchor.name.trim().is_empty() {
                issues.push(format!("グリフ '{}' に名前のないアンカーがあります", name));
            } else if !anchor_names.insert(anchor.name.trim().to_string()) {
                issues.push(format!("グリフ '{}' にアンカー名 '{}' が重複しています", name, anchor.name.trim()));
            }
            if !anchor.x.is_finite() || !anchor.y.is_finite() {
                issues.push(format!("グリフ '{}' のアンカー '{}' の座標が不正です", name, anchor.name));
            }
        }
        for (index, guide) in glyph.guidelines.iter().enumerate() {
            if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
                issues.push(format!("グリフ '{}' のガイド{}の座標または角度が不正です", name, index + 1));
            }
        }
        for (master_id, guides) in &glyph.master_guidelines {
            for (index, guide) in guides.iter().enumerate() {
                if !guide.x.is_finite() || !guide.y.is_finite() || !guide.angle.is_finite() {
                    issues.push(format!("グリフ '{}' のマスター '{}' のガイド{}の座標または角度が不正です", name, master_id, index + 1));
                }
            }
        }
        for (contour_index, contour) in glyph.contours.iter().enumerate() {
            if contour.points.is_empty() {
                issues.push(format!("グリフ '{}' の輪郭{}が空です", name, contour_index + 1));
            }
            validate_contour_topology(contour, &format!("グリフ '{}' の輪郭{}", name, contour_index + 1), &mut issues);
            for point in &contour.points {
                if !point.x.is_finite() || !point.y.is_finite() {
                    issues.push(format!("グリフ '{}' の輪郭{}に不正な座標があります", name, contour_index + 1));
                    break;
                }
            }
        }
        let mut codepoints = glyph.unicodes.clone();
        if let Some(unicode) = glyph.unicode {
            if !codepoints.contains(&unicode) {
                codepoints.push(unicode);
            }
        }
        for unicode in codepoints {
            if unicode > 0x10FFFF || (0xD800..=0xDFFF).contains(&unicode) {
                issues.push(format!("グリフ '{}' のUnicode U+{unicode:04X}が不正です", name));
                continue;
            }
            if is_noncharacter(unicode) {
                issues.push(format!("グリフ '{}' のUnicode U+{unicode:04X}は非文字です", name));
            }
            if let Some(previous) = unicodes.insert(unicode, name.clone()) {
                issues.push(format!("Unicode U+{unicode:04X} が重複: {previous} / {name}"));
            }
        }
        for component in &glyph.components {
            if !project.glyphs.contains_key(&component.base) {
                issues.push(format!("グリフ '{}' が未定義コンポーネント '{}' を参照", name, component.base));
            }
            let transform = [component.x_scale, component.xy_scale, component.yx_scale, component.y_scale, component.x_offset, component.y_offset];
            if transform.iter().any(|value| !value.is_finite()) {
                issues.push(format!("グリフ '{}' のコンポーネント変換が不正です", name));
            }
        }
        for (master_id, layer) in &glyph.layers {
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!("グリフ '{}' に未定義マスター '{}' のレイヤーがあります", name, master_id));
            }
            if !layer.width.is_finite() || layer.width < 0.0 {
                issues.push(format!("グリフ '{}' のマスター '{}' の幅が不正です", name, master_id));
            }
            let mut layer_anchor_names = std::collections::HashSet::new();
            for anchor in &layer.anchors {
                if anchor.name.trim().is_empty() {
                    issues.push(format!("グリフ '{}' のマスター '{}' に名前のないアンカーがあります", name, master_id));
                } else if !layer_anchor_names.insert(anchor.name.trim().to_string()) {
                    issues.push(format!("グリフ '{}' のマスター '{}' にアンカー名 '{}' が重複しています", name, master_id, anchor.name.trim()));
                }
                if !anchor.x.is_finite() || !anchor.y.is_finite() {
                    issues.push(format!("グリフ '{}' のマスター '{}' のアンカー '{}' の座標が不正です", name, master_id, anchor.name));
                }
            }
            for (contour_index, contour) in layer.contours.iter().enumerate() {
                if contour.points.is_empty() {
                    issues.push(format!("グリフ '{}' のマスター '{}' の輪郭{}が空です", name, master_id, contour_index + 1));
                }
                validate_contour_topology(contour, &format!("グリフ '{}' のマスター '{}' の輪郭{}", name, master_id, contour_index + 1), &mut issues);
                if contour.points.iter().any(|point| !point.x.is_finite() || !point.y.is_finite()) {
                    issues.push(format!("グリフ '{}' のマスター '{}' に不正な座標があります", name, master_id));
                }
            }
            if !project.masters.iter().any(|master| master.id == *master_id) {
                issues.push(format!("グリフ '{}' に未定義マスター '{}' のレイヤーがあります", name, master_id));
            }
            for component in &layer.components {
                if !project.glyphs.contains_key(&component.base) {
                    issues.push(format!("グリフ '{}' のマスター '{}' が未定義コンポーネント '{}' を参照", name, master_id, component.base));
                }
                let transform = [component.x_scale, component.xy_scale, component.yx_scale, component.y_scale, component.x_offset, component.y_offset];
                if transform.iter().any(|value| !value.is_finite()) {
                    issues.push(format!("グリフ '{}' のマスター '{}' のコンポーネント変換が不正です", name, master_id));
                }
            }
        }
    }
    let palette_count = project.color_palettes.first().map_or(0, |palette| palette.len());
    for (index, palette) in project.color_palettes.iter().enumerate() {
        if palette.is_empty() {
            issues.push(format!("カラー パレット{}が空です", index + 1));
        }
        if palette.len() != palette_count {
            issues.push(format!("カラー パレット{}の色数が一致しません", index + 1));
        }
    }
    for (base, layers) in &project.color_layers {
        if !project.glyphs.contains_key(base) {
            issues.push(format!("カラー基底グリフ '{}' が存在しません", base));
        }
        for (index, layer) in layers.iter().enumerate() {
            if !project.glyphs.contains_key(&layer.glyph) {
                issues.push(format!("カラーグリフ '{}' の層{}が未定義グリフ '{}' を参照しています", base, index + 1, layer.glyph));
            }
            if usize::from(layer.palette_index) >= palette_count {
                issues.push(format!("カラーグリフ '{}' の層{}のパレット番号が範囲外です", base, index + 1));
            }
            if !layer.alpha.is_finite() || !(0.0..=1.0).contains(&layer.alpha) {
                issues.push(format!("カラーグリフ '{}' の層{}のアルファ値が範囲外です", base, index + 1));
            }
            if let Some(gradient) = &layer.gradient {
                for (label, value) in [
                    ("始点X", gradient.x0),
                    ("始点Y", gradient.y0),
                    ("終点X", gradient.x1),
                    ("終点Y", gradient.y1),
                    ("回転点X", gradient.x2),
                    ("回転点Y", gradient.y2),
                ] {
                    if !value.is_finite() || value < f64::from(i16::MIN) || value > f64::from(i16::MAX) {
                        issues.push(format!("カラーグリフ '{}' の層{}のグラデーション{}が不正です", base, index + 1, label));
                    }
                }
                for (label, palette_index) in [("開始", gradient.start_palette_index), ("終了", gradient.end_palette_index)] {
                    if usize::from(palette_index) >= palette_count {
                        issues.push(format!("カラーグリフ '{}' の層{}のグラデーション{}色が範囲外です", base, index + 1, label));
                    }
                }
                let mut previous_offset = f64::NEG_INFINITY;
                for (stop_index, stop) in gradient.stops.iter().enumerate() {
                    if !stop.offset.is_finite() || stop.offset < -2.0 || stop.offset >= 2.0 || stop.offset < previous_offset {
                        issues.push(format!("カラーグリフ '{}' の層{}の色ストップ{}の位置が不正です", base, index + 1, stop_index + 1));
                    }
                    if !stop.alpha.is_finite() || !(0.0..=1.0).contains(&stop.alpha) {
                        issues.push(format!("カラーグリフ '{}' の層{}の色ストップ{}の不透明度が不正です", base, index + 1, stop_index + 1));
                    }
                    if usize::from(stop.palette_index) >= palette_count {
                        issues.push(format!("カラーグリフ '{}' の層{}の色ストップ{}のパレット番号が範囲外です", base, index + 1, stop_index + 1));
                    }
                    previous_offset = stop.offset;
                }
                if gradient.radius0 < 0.0 || gradient.radius1 < 0.0 {
                    issues.push(format!("カラーグリフ '{}' の層{}のグラデーション半径が負です", base, index + 1));
                }
                if matches!(gradient.kind, crate::font_data::ColorGradientKind::Linear) {
                    let p0p1 = (gradient.x1 - gradient.x0, gradient.y1 - gradient.y0);
                    let p0p2 = (gradient.x2 - gradient.x0, gradient.y2 - gradient.y0);
                    let determinant = p0p1.0 * p0p2.1 - p0p1.1 * p0p2.0;
                    if determinant.abs() <= f64::EPSILON {
                        issues.push(format!("カラーグリフ '{}' の層{}の線形グラデーションの回転点が退化しています", base, index + 1));
                    }
                }
                if matches!(gradient.kind, crate::font_data::ColorGradientKind::Sweep) && (!(0.0..=360.0).contains(&gradient.start_angle) || !(0.0..=360.0).contains(&gradient.end_angle)) {
                    issues.push(format!("カラーグリフ '{}' の層{}のスイープ角度が0〜360度の範囲外です", base, index + 1));
                }
            }
            if let Some(Some(transform)) = project.color_layer_transforms.get(base).and_then(|transforms| transforms.get(index)) {
                for (label, value) in [
                    ("XX", transform.xx),
                    ("YX", transform.yx),
                    ("XY", transform.xy),
                    ("YY", transform.yy),
                    ("DX", transform.dx),
                    ("DY", transform.dy),
                ] {
                    if !value.is_finite() || value * 65_536.0 < f64::from(i32::MIN) || value * 65_536.0 > f64::from(i32::MAX) {
                        issues.push(format!("カラーグリフ '{}' の層{}のCOLR変形{}が不正です", base, index + 1, label));
                    }
                }
            }
        }
        if let Some(transforms) = project.color_layer_transforms.get(base) {
            if transforms.len() > layers.len() {
                issues.push(format!("カラー基底グリフ '{}' のCOLR変形数がカラー層数を超えています", base));
            }
        }
    }
    for base in project.color_layer_transforms.keys() {
        if !project.color_layers.contains_key(base) {
            issues.push(format!("COLR変形がカラー層のない基底グリフ '{}' に設定されています", base));
        }
    }
    fn visit_color_graph(project: &FontProject, name: &str, visiting: &mut Vec<String>, reported: &mut std::collections::HashSet<String>) {
        if let Some(index) = visiting.iter().position(|item| item == name) {
            reported.insert(visiting[index..].join(" -> ") + " -> " + name);
            return;
        }
        let Some(layers) = project.color_layers.get(name) else {
            return;
        };
        visiting.push(name.to_string());
        for layer in layers {
            if project.color_layers.contains_key(&layer.glyph) {
                visit_color_graph(project, &layer.glyph, visiting, reported);
            }
        }
        visiting.pop();
    }
    let mut color_cycles = std::collections::HashSet::new();
    for name in project.color_layers.keys() {
        visit_color_graph(project, name, &mut Vec::new(), &mut color_cycles);
    }
    issues.extend(color_cycles.into_iter().map(|cycle| format!("COLRカラーグリフ循環参照: {cycle}")));
    fn visit_component_graph(project: &FontProject, name: &str, visiting: &mut Vec<String>, reported: &mut std::collections::HashSet<String>) {
        if let Some(index) = visiting.iter().position(|item| item == name) {
            let cycle = visiting[index..].join(" -> ") + " -> " + name;
            reported.insert(cycle);
            return;
        }
        let Some(glyph) = project.glyphs.get(name) else {
            return;
        };
        visiting.push(name.to_string());
        for component in &glyph.components {
            visit_component_graph(project, &component.base, visiting, reported);
        }
        for layer in glyph.layers.values() {
            for component in &layer.components {
                visit_component_graph(project, &component.base, visiting, reported);
            }
        }
        visiting.pop();
    }
    let mut cycles = std::collections::HashSet::new();
    for name in project.glyphs.keys() {
        visit_component_graph(project, name, &mut Vec::new(), &mut cycles);
    }
    issues.extend(cycles.into_iter().map(|cycle| format!("コンポーネント循環参照: {cycle}")));
    for ((left, right), value) in &project.kerning {
        if !value.is_finite() {
            issues.push(format!("カーニング値が不正: {left} / {right}"));
        } else if *value < i16::MIN as f64 || *value > i16::MAX as f64 {
            issues.push(format!("カーニング値が範囲外です: {left} / {right} ({value})"));
        }
        if !project.glyphs.contains_key(left) || !project.glyphs.contains_key(right) {
            issues.push(format!("未定義グリフのカーニング: {left} / {right}"));
        }
    }
    for (master_id, pairs) in &project.kerning_by_master {
        for ((left, right), value) in pairs {
            if !value.is_finite() {
                issues.push(format!("マスター '{}' のカーニング値が不正: {left} / {right}", master_id));
            } else if *value < i16::MIN as f64 || *value > i16::MAX as f64 {
                issues.push(format!("マスター '{}' のカーニング値が範囲外です: {left} / {right} ({value})", master_id));
            }
            if !project.glyphs.contains_key(left) || !project.glyphs.contains_key(right) {
                issues.push(format!("マスター '{}' に未定義グリフのカーニング: {left} / {right}", master_id));
            }
        }
    }
    if project.masters.is_empty() {
        issues.push("マスターがありません".into());
    } else if !project.masters.iter().any(|master| master.id == project.default_master_id) {
        issues.push("基準マスターが見つかりません".into());
    }
    for (name, glyph) in &project.glyphs {
        let Some(reference_id) = glyph
            .layers
            .contains_key(&project.default_master_id)
            .then_some(project.default_master_id.as_str())
            .or_else(|| glyph.layers.keys().next().map(String::as_str))
        else {
            continue;
        };
        let Some(reference) = glyph.layers.get(reference_id) else {
            continue;
        };
        for (master_id, layer) in &glyph.layers {
            if master_id == reference_id {
                continue;
            }
            if reference.interpolate(layer, 0.5).is_none() {
                issues.push(format!("グリフ '{}' のマスター '{}' は基準マスターと補間互換ではありません", name, master_id));
            }
        }
    }
    let feature_source = project.feature_source();
    if let Err(error) = validate_feature_source(&feature_source) {
        issues.push(error);
    } else {
        issues.extend(validate_feature_class_definitions(&feature_source, &project.glyphs));
        issues.extend(validate_feature_glyph_references(&feature_source, &project.glyphs));
    }
    issues
}
