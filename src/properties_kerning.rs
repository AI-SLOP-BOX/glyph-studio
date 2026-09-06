#![allow(clippy::too_many_arguments, unused_variables)]

use super::*;

pub fn show_properties_kerning(
    ui: &mut Ui,
    properties_filter: &mut String,
    project: &mut FontProject,
    current_glyph: &Option<String>,
    component_base: &mut String,
    kerning_right: &mut String,
    kerning_pair_filter: &mut String,
    preview_text: &mut String,
    show_preview: &mut bool,
    feature_left: &mut String,
    feature_right: &mut String,
    feature_replacement: &mut String,
    feature_kerning_value: &mut String,
    feature_target_tag: &mut String,
    feature_anchor_x: &mut String,
    feature_anchor_y: &mut String,
    unicode_alias_input: &mut String,
    unicode_variation_selector: &mut String,
    current_master_id: &mut String,
    master_map_drag: &mut Option<String>,
    color_layer_glyph: &mut String,
    preview_color_palette: &mut usize,
    conditional_layer_axis: &mut String,
    conditional_layer_min: &mut String,
    conditional_layer_max: &mut String,
    conditional_layer_axis_2: &mut String,
    conditional_layer_min_2: &mut String,
    conditional_layer_max_2: &mut String,
    conditional_layer_axis_3: &mut String,
    conditional_layer_min_3: &mut String,
    conditional_layer_max_3: &mut String,
    conditional_layer_axis_4: &mut String,
    conditional_layer_min_4: &mut String,
    conditional_layer_max_4: &mut String,
    conditional_layer_extra: &mut Vec<(String, String, String)>,
) {
    let filter = properties_filter.trim().to_lowercase();
    let show_section = |keywords: &[&str]| {
        filter.is_empty()
            || keywords
                .iter()
                .any(|keyword| filter.contains(&keyword.to_lowercase()))
    };
    if show_section(&["kerning", "カーニング"]) {
        egui::CollapsingHeader::new("カーニング")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(left) = current_glyph {
                    ui.separator();
                    ui.heading("カーニング");
                    let master_name = project
                        .masters
                        .iter()
                        .find(|master| master.id == *current_master_id)
                        .map(|master| master.name.as_str())
                        .unwrap_or(current_master_id.as_str());
                    ui.small(format!("編集対象マスター: {master_name}"));
                    ui.horizontal(|ui| {
                        ui.label(format!("{} →", left));
                        ui.add(egui::TextEdit::singleline(kerning_right).hint_text("右グリフ名"));
                    });
                    egui::ComboBox::from_id_salt("kerning-right-glyph")
                        .selected_text(if kerning_right.trim().is_empty() {
                            "右グリフを選択"
                        } else {
                            kerning_right.trim()
                        })
                        .show_ui(ui, |ui| {
                            for glyph_name in project.glyph_names_sorted() {
                                if ui
                                    .selectable_label(
                                        kerning_right.trim() == glyph_name,
                                        glyph_name,
                                    )
                                    .clicked()
                                {
                                    *kerning_right = glyph_name.to_string();
                                }
                            }
                        });
                    if !kerning_right.trim().is_empty()
                        && project.glyphs.contains_key(kerning_right.trim())
                    {
                        let key = (left.clone(), kerning_right.trim().to_string());
                        let inherited_source =
                            project.kerning_source_for_glyphs(left, kerning_right.trim());
                        let inherited_value = inherited_source
                            .as_ref()
                            .map(|(_, value)| *value)
                            .unwrap_or(0.0);
                        let is_exception = project.kerning.contains_key(&key);
                        let uses_group = project
                            .glyphs
                            .get(left)
                            .is_some_and(|glyph| !glyph.left_kerning_group.trim().is_empty())
                            || project
                                .glyphs
                                .get(kerning_right.trim())
                                .is_some_and(|glyph| !glyph.right_kerning_group.trim().is_empty());
                        if !is_exception && uses_group {
                            if let Some(((source_left, source_right), _)) = &inherited_source {
                                ui.label(format!("グループ値 ← {source_left} / {source_right}"))
                                    .on_hover_text(
                                        "値を変更すると、このグリフペア専用の例外になります",
                                    );
                            } else {
                                ui.label("グループ値").on_hover_text(
                                    "値を変更すると、このグリフペア専用の例外になります",
                                );
                            }
                        }
                        let mut value = project
                            .kerning
                            .get(&key)
                            .copied()
                            .unwrap_or(inherited_value);
                        let value_response = ui
                            .add(egui::DragValue::new(&mut value).speed(1.0).suffix(" units"))
                            .on_hover_text(if is_exception {
                                "このペア専用のカーニング値"
                            } else if inherited_source.is_some() {
                                "グループ値を編集すると、このペア専用の例外を作成します"
                            } else {
                                "このグリフペアのカーニング値"
                            });
                        if value_response.changed() {
                            project.kerning.insert(key.clone(), value);
                        }
                        let left_char = project
                            .glyphs
                            .get(left)
                            .and_then(|glyph| glyph.unicode)
                            .and_then(char::from_u32)
                            .unwrap_or('□');
                        let right_char = project
                            .glyphs
                            .get(kerning_right.trim())
                            .and_then(|glyph| glyph.unicode)
                            .and_then(char::from_u32)
                            .unwrap_or('□');
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{left_char}{right_char}"))
                                        .size(28.0)
                                        .strong(),
                                );
                                ui.vertical(|ui| {
                                    ui.small(format!("{}  /  {}", left, kerning_right.trim()));
                                    ui.label(format!("{value:.0} units"));
                                    ui.label(
                                        egui::RichText::new(if is_exception {
                                            "このペア専用の例外"
                                        } else if inherited_source.is_some() {
                                            "グループ値を使用中"
                                        } else {
                                            "値未設定"
                                        })
                                        .small()
                                        .color(egui::Color32::GRAY),
                                    );
                                });
                            });
                        });
                        if ui
                            .small_button("このペアをプレビュー")
                            .on_hover_text("下部プレビューをこの2グリフに切り替え")
                            .clicked()
                        {
                            *show_preview = true;
                            let left_char = project
                                .glyphs
                                .get(left)
                                .and_then(|glyph| glyph.unicode)
                                .and_then(char::from_u32)
                                .unwrap_or('□');
                            let right_char = project
                                .glyphs
                                .get(kerning_right.trim())
                                .and_then(|glyph| glyph.unicode)
                                .and_then(char::from_u32)
                                .unwrap_or('□');
                            *preview_text = format!("{left_char}{right_char}");
                        }
                        if ui.button("ペアを削除").clicked() {
                            project.kerning.remove(&key);
                        }
                    }
                    let mut pairs: Vec<((String, String), f64)> = project
                        .kerning
                        .iter()
                        .filter(|((pair_left, _), _)| {
                            if pair_left == left {
                                return true;
                            }
                            let left_group = project
                                .glyphs
                                .get(left)
                                .map(|glyph| glyph.left_kerning_group.trim())
                                .unwrap_or_default();
                            !left_group.is_empty()
                                && project.glyphs.get(pair_left).is_some_and(|glyph| {
                                    glyph.left_kerning_group.trim() == left_group
                                })
                        })
                        .map(|(pair, value)| (pair.clone(), *value))
                        .collect();
                    pairs.sort_by(|a, b| a.0 .1.cmp(&b.0 .1));
                    if !pairs.is_empty() {
                        let current_index = pairs
                            .iter()
                            .position(|((_, right), _)| right == kerning_right.trim())
                            .unwrap_or(0);
                        ui.horizontal(|ui| {
                            if ui
                                .small_button("‹ ペア")
                                .on_hover_text("同じ左グリフの前のカーニングペア")
                                .clicked()
                            {
                                let index = (current_index + pairs.len() - 1) % pairs.len();
                                *kerning_right = pairs[index].0 .1.clone();
                            }
                            if ui
                                .small_button("ペア ›")
                                .on_hover_text("同じ左グリフの次のカーニングペア")
                                .clicked()
                            {
                                let index = (current_index + 1) % pairs.len();
                                *kerning_right = pairs[index].0 .1.clone();
                            }
                            ui.small(format!("{}/{}", current_index + 1, pairs.len()));
                        });
                        ui.separator();
                        ui.label(
                            if project
                                .glyphs
                                .get(left)
                                .is_some_and(|glyph| !glyph.left_kerning_group.trim().is_empty())
                            {
                                "既存ペア（左グループを含む）"
                            } else {
                                "既存ペア"
                            },
                        );
                        ui.horizontal(|ui| {
                            ui.label("一覧検索");
                            ui.add(
                                egui::TextEdit::singleline(kerning_pair_filter)
                                    .hint_text("右グリフ名")
                                    .desired_width(140.0),
                            );
                            if !kerning_pair_filter.is_empty()
                                && ui.small_button("クリア").clicked()
                            {
                                kerning_pair_filter.clear();
                            }
                        });
                        let mut remove = None;
                        let filtered_pairs: Vec<_> = pairs
                            .into_iter()
                            .filter(|((_, right), _)| {
                                let filter = kerning_pair_filter.trim().to_ascii_lowercase();
                                if filter.is_empty() {
                                    return true;
                                }
                                right.to_ascii_lowercase().contains(&filter)
                                    || project
                                        .glyphs
                                        .get(right)
                                        .and_then(|glyph| glyph.unicode)
                                        .and_then(char::from_u32)
                                        .is_some_and(|character| character.to_string() == filter)
                            })
                            .collect();
                        if !kerning_pair_filter.trim().is_empty() {
                            ui.small(format!("{}件", filtered_pairs.len()));
                        }
                        for ((pair_left, right), _) in filtered_pairs {
                            ui.horizontal(|ui| {
                                let source_label = if pair_left == *left {
                                    left.as_str()
                                } else {
                                    "グループ"
                                };
                                let selected_pair = right == kerning_right.trim();
                                if ui
                                    .selectable_label(
                                        selected_pair,
                                        format!("{} → {}", source_label, right),
                                    )
                                    .on_hover_text("クリックしてこのペアを編集")
                                    .clicked()
                                {
                                    *kerning_right = right.clone();
                                }
                                if let Some(current) =
                                    project.kerning.get_mut(&(pair_left.clone(), right.clone()))
                                {
                                    ui.add(egui::DragValue::new(current).speed(1.0));
                                }
                                if ui.small_button("削除").clicked() {
                                    remove = Some((pair_left, right));
                                }
                            });
                        }
                        if let Some(key) = remove {
                            project.kerning.remove(&key);
                        }
                    }
                }
            });
    }
}
