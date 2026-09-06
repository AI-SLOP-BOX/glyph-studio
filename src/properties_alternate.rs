use super::*;

#[rustfmt::skip]
#[allow(clippy::too_many_arguments, clippy::ptr_arg, unused_variables)]
pub fn show_properties_alternate(
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

    align_component_request: &mut Option<usize>,
    align_all_components: &mut bool,
) {
    let filter = properties_filter.trim().to_lowercase();
    let show_section = |keywords: &[&str]| filter.is_empty() || keywords.iter().any(|keyword| filter.contains(&keyword.to_lowercase()));
    if show_section(&["alternate", "bracket", "条件", "レイヤー", "layer", "glyph", "グリフ"]) {
        egui::CollapsingHeader::new("Alternate / Bracket レイヤー").default_open(false).show(ui, |ui| {
            if let Some(name) = current_glyph {
                ui.separator();
                ui.label("条件レイヤー:");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(conditional_layer_axis).hint_text("軸タグ"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_min).hint_text("min"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_max).hint_text("max"));
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(conditional_layer_axis_2).hint_text("2軸目"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_min_2).hint_text("min"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_max_2).hint_text("max"));
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(conditional_layer_axis_3).hint_text("3軸目"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_min_3).hint_text("min"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_max_3).hint_text("max"));
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(conditional_layer_axis_4).hint_text("4軸目"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_min_4).hint_text("min"));
                    ui.add(egui::TextEdit::singleline(conditional_layer_max_4).hint_text("max"));
                });
                let mut remove_extra = None;
                let mut move_extra = None;
                let extra_len = conditional_layer_extra.len();
                for (index, (axis, min, max)) in conditional_layer_extra.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.small_button("↑").clicked() && index > 0 {
                            move_extra = Some((index, index - 1));
                        }
                        if ui.small_button("↓").clicked() && index + 1 < extra_len {
                            move_extra = Some((index, index + 1));
                        }
                        ui.add(egui::TextEdit::singleline(axis).hint_text("追加軸"));
                        ui.add(egui::TextEdit::singleline(min).hint_text("min"));
                        ui.add(egui::TextEdit::singleline(max).hint_text("max"));
                        if ui.small_button("削除").clicked() {
                            remove_extra = Some(index);
                        }
                    });
                }
                if let Some(index) = remove_extra {
                    conditional_layer_extra.remove(index);
                } else if let Some((from, to)) = move_extra {
                    conditional_layer_extra.swap(from, to);
                }
                if ui.small_button("＋ 条件軸を追加").on_hover_text("5軸目以降の条件軸を追加できます").clicked() {
                    conditional_layer_extra.push((String::new(), String::new(), String::new()));
                }
                if ui.small_button("条件入力をクリア").on_hover_text("固定軸と追加軸の入力をすべて消去").clicked() {
                    conditional_layer_axis.clear();
                    conditional_layer_min.clear();
                    conditional_layer_max.clear();
                    conditional_layer_axis_2.clear();
                    conditional_layer_min_2.clear();
                    conditional_layer_max_2.clear();
                    conditional_layer_axis_3.clear();
                    conditional_layer_min_3.clear();
                    conditional_layer_max_3.clear();
                    conditional_layer_axis_4.clear();
                    conditional_layer_min_4.clear();
                    conditional_layer_max_4.clear();
                    conditional_layer_extra.clear();
                }
                let mut condition_error = None;
                let mut seen_axes = std::collections::HashSet::new();
                let condition_inputs = [
                    (conditional_layer_axis.as_str(), conditional_layer_min.as_str(), conditional_layer_max.as_str()),
                    (conditional_layer_axis_2.as_str(), conditional_layer_min_2.as_str(), conditional_layer_max_2.as_str()),
                    (conditional_layer_axis_3.as_str(), conditional_layer_min_3.as_str(), conditional_layer_max_3.as_str()),
                    (conditional_layer_axis_4.as_str(), conditional_layer_min_4.as_str(), conditional_layer_max_4.as_str()),
                ];
                for (axis, min, max) in condition_inputs.into_iter().chain(conditional_layer_extra.iter().map(|(axis, min, max)| (axis.as_str(), min.as_str(), max.as_str()))) {
                    if axis.trim().is_empty() && min.trim().is_empty() && max.trim().is_empty() {
                        continue;
                    }
                    let axis = axis.trim().to_ascii_lowercase();
                    let min = min.trim().parse::<f64>().ok();
                    let max = max.trim().parse::<f64>().ok();
                    if axis.len() != 4 || !axis.is_ascii() {
                        condition_error = Some("軸タグはASCII 4文字で指定してください");
                        break;
                    }
                    if !seen_axes.insert(axis) {
                        condition_error = Some("条件軸が重複しています");
                        break;
                    }
                    if min.is_none() && max.is_none() {
                        condition_error = Some("minまたはmaxを指定してください");
                        break;
                    }
                    if min.zip(max).is_some_and(|(min, max)| min > max) {
                        condition_error = Some("minはmax以下にしてください");
                        break;
                    }
                }
                if let Some(error) = condition_error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                let condition_axis_count = seen_axes.iter().filter(|axis| !axis.is_empty()).count();
                ui.label(format!("条件軸: {condition_axis_count}軸"));
                let conditions_ready = condition_error.is_none() && !conditional_layer_axis.trim().is_empty();
                if ui.add_enabled(conditions_ready, egui::Button::new("現在レイヤーから条件レイヤーを追加")).clicked() {
                    let axis = conditional_layer_axis.trim().to_ascii_lowercase();
                    let min = conditional_layer_min.trim().parse::<f64>().ok();
                    let max = conditional_layer_max.trim().parse::<f64>().ok();
                    let axis_2 = conditional_layer_axis_2.trim().to_ascii_lowercase();
                    let min_2 = conditional_layer_min_2.trim().parse::<f64>().ok();
                    let max_2 = conditional_layer_max_2.trim().parse::<f64>().ok();
                    let axis_3 = conditional_layer_axis_3.trim().to_ascii_lowercase();
                    let min_3 = conditional_layer_min_3.trim().parse::<f64>().ok();
                    let max_3 = conditional_layer_max_3.trim().parse::<f64>().ok();
                    let axis_4 = conditional_layer_axis_4.trim().to_ascii_lowercase();
                    let min_4 = conditional_layer_min_4.trim().parse::<f64>().ok();
                    let max_4 = conditional_layer_max_4.trim().parse::<f64>().ok();
                    let mut used_axes = std::collections::HashSet::from([axis.clone(), axis_2.clone(), axis_3.clone(), axis_4.clone()]);
                    let extra_conditions: Vec<_> = conditional_layer_extra.iter().map(|(tag, min, max)| (tag.trim().to_ascii_lowercase(), min.trim().parse::<f64>().ok(), max.trim().parse::<f64>().ok())).collect();
                    let extra_is_valid = extra_conditions.iter().all(|(tag, min, max)| tag.len() == 4 && tag.is_ascii() && !tag.is_empty() && used_axes.insert(tag.clone()) && (min.is_some() || max.is_some()) && min.zip(*max).is_none_or(|(min, max)| min <= max));
                    let second_is_valid = axis_2.is_empty() || (axis_2.len() == 4 && axis_2.is_ascii() && axis_2 != axis && (min_2.is_some() || max_2.is_some()) && min_2.zip(max_2).is_none_or(|(min, max)| min <= max));
                    let third_is_valid = axis_3.is_empty() || (axis_3.len() == 4 && axis_3.is_ascii() && axis_3 != axis && axis_3 != axis_2 && (min_3.is_some() || max_3.is_some()) && min_3.zip(max_3).is_none_or(|(min, max)| min <= max));
                    let fourth_is_valid = axis_4.is_empty() || (axis_4.len() == 4 && axis_4.is_ascii() && axis_4 != axis && axis_4 != axis_2 && axis_4 != axis_3 && (min_4.is_some() || max_4.is_some()) && min_4.zip(max_4).is_none_or(|(min, max)| min <= max));
                    if axis.len() == 4 && axis.is_ascii() && (min.is_some() || max.is_some()) && min.zip(max).is_none_or(|(min, max)| min <= max) && second_is_valid && third_is_valid && fourth_is_valid && extra_is_valid {
                        if let Some(base) = project.glyphs.get(name).and_then(|glyph| glyph.layers.get(current_master_id)).cloned() {
                            let layers = project.conditional_layers.entry(name.clone()).or_default();
                            let mut conditions = std::collections::HashMap::from([(axis, crate::font_data::AxisRange { min, max })]);
                            if !axis_2.is_empty() {
                                conditions.insert(axis_2, crate::font_data::AxisRange { min: min_2, max: max_2 });
                            }
                            if !axis_3.is_empty() {
                                conditions.insert(axis_3, crate::font_data::AxisRange { min: min_3, max: max_3 });
                            }
                            if !axis_4.is_empty() {
                                conditions.insert(axis_4, crate::font_data::AxisRange { min: min_4, max: max_4 });
                            }
                            for (tag, min, max) in extra_conditions {
                                conditions.insert(tag, crate::font_data::AxisRange { min, max });
                            }
                            layers.push(crate::font_data::ConditionalLayer { id: format!("alternate-{}", layers.len() + 1), conditions, layer: base });
                        }
                    }
                }
                let mut remove_conditional = None;
                let mut apply_conditional = None;
                let mut duplicate_conditional = None;
                if let Some(layers) = project.conditional_layers.get_mut(name) {
                    for (index, layer) in layers.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut layer.id).desired_width(120.0).hint_text("レイヤーID"));
                            let mut condition_tags: Vec<String> = layer.conditions.keys().cloned().collect();
                            condition_tags.sort();
                            for tag in condition_tags {
                                let Some(range) = layer.conditions.get_mut(&tag) else {
                                    continue;
                                };
                                ui.label(&tag);
                                let mut has_min = range.min.is_some();
                                if ui.checkbox(&mut has_min, "min").changed() {
                                    range.min = has_min.then_some(range.min.unwrap_or(0.0));
                                }
                                if let Some(min) = range.min.as_mut() {
                                    ui.add(egui::DragValue::new(min).speed(1.0));
                                }
                                let mut has_max = range.max.is_some();
                                if ui.checkbox(&mut has_max, "max").changed() {
                                    range.max = has_max.then_some(range.max.unwrap_or(0.0));
                                }
                                if let Some(max) = range.max.as_mut() {
                                    ui.add(egui::DragValue::new(max).speed(1.0));
                                }
                                if let (Some(min), Some(max)) = (range.min, range.max) {
                                    if min > max {
                                        range.max = Some(min);
                                    }
                                }
                            }
                            if ui.small_button("削除").clicked() {
                                remove_conditional = Some(index);
                            }
                            if ui.small_button("複製").clicked() {
                                duplicate_conditional = Some(layer.clone());
                            }
                            if ui.small_button("編集用に反映").clicked() {
                                apply_conditional = Some((index, layer.layer.clone()));
                            }
                        });
                    }
                    if let Some(index) = remove_conditional {
                        layers.remove(index);
                    }
                    if let Some(mut duplicate) = duplicate_conditional {
                        let base_id = duplicate.id.clone();
                        let mut suffix = 2;
                        while layers.iter().any(|layer| layer.id == duplicate.id) {
                            duplicate.id = format!("{base_id}-{suffix}");
                            suffix += 1;
                        }
                        layers.push(duplicate);
                    }
                }
                if let Some((index, layer)) = apply_conditional {
                    if let Some(layers) = project.conditional_layers.get_mut(name) {
                        if index < layers.len() {
                            layers.remove(index);
                        }
                        if layers.is_empty() {
                            project.conditional_layers.remove(name);
                        }
                    }
                    if let Some(glyph) = project.glyphs.get_mut(name) {
                        glyph.width = layer.width;
                        glyph.contours = layer.contours.clone();
                        glyph.components = layer.components.clone();
                        glyph.anchors = layer.anchors.clone();
                        glyph.layers.insert(current_master_id.to_string(), layer);
                    }
                }

                let component_exists = project.glyphs.contains_key(component_base.trim());
                let outline_bounds = project.outline_bounds_for_glyph(name);
                let vertical_defaults = project.vertical_metrics_for_glyph_in_master(name, current_master_id);
                let mut bearing_request = None;
                let mut vertical_request = None;
                let mut metrics_key_request = false;
                if let Some(glyph) = project.glyphs.get_mut(name) {
                    ui.heading("グリフ情報");
                    ui.label(format!("名前: {}", glyph.name));
                    ui.horizontal(|ui| {
                        ui.label("幅:");
                        ui.add(egui::DragValue::new(&mut glyph.width).speed(10.0).range(0.0..=5000.0));
                    });
                    let mut advance_height = vertical_defaults.advance_height;
                    let mut top_side_bearing = vertical_defaults.top_side_bearing;
                    ui.horizontal(|ui| {
                        ui.label("縦アドバンス:");
                        let advance_changed = ui.add(egui::DragValue::new(&mut advance_height).speed(10.0)).changed();
                        ui.label("縦TSB:");
                        let bearing_changed = ui.add(egui::DragValue::new(&mut top_side_bearing).speed(1.0)).changed();
                        if advance_changed || bearing_changed {
                            vertical_request = Some((advance_height, top_side_bearing));
                        }
                    });
                    if let Some((min_x, _, max_x, _)) = outline_bounds {
                        let mut left_bearing = min_x;
                        let mut right_bearing = glyph.width - max_x;
                        ui.horizontal(|ui| {
                            ui.label("左サイドベアリング:");
                            let left_linked = !glyph.left_metrics_key.trim().is_empty();
                            let left_response = ui.add_enabled(!left_linked, egui::DragValue::new(&mut left_bearing).speed(1.0));
                            let left_changed = left_response.changed();
                            if left_linked {
                                left_response.on_hover_text("メトリクスキーでリンク中。基準グリフから適用してください");
                            }
                            ui.label("右サイドベアリング:");
                            let right_linked = !glyph.right_metrics_key.trim().is_empty();
                            let right_response = ui.add_enabled(!right_linked, egui::DragValue::new(&mut right_bearing).speed(1.0));
                            let right_changed = right_response.changed();
                            if right_linked {
                                right_response.on_hover_text("メトリクスキーでリンク中。基準グリフから適用してください");
                            }
                            if left_changed || right_changed {
                                bearing_request = Some((left_bearing, right_bearing));
                            }
                        });
                        ui.label(format!("外形幅: {:.1}", max_x - min_x));
                    }
                    ui.collapsing("メトリクスキー", |ui| {
                        ui.label(egui::RichText::new("=H のように基準グリフへ左右余白をリンク").small().color(egui::Color32::GRAY));
                        ui.horizontal(|ui| {
                            ui.label("左:");
                            ui.add(egui::TextEdit::singleline(&mut glyph.left_metrics_key).hint_text("=H").desired_width(80.0));
                            ui.label("右:");
                            ui.add(egui::TextEdit::singleline(&mut glyph.right_metrics_key).hint_text("=H").desired_width(80.0));
                        });
                        if ui.button("基準グリフから余白を適用").on_hover_text("入力したメトリクスキーを全マスターへ適用").clicked() {
                            metrics_key_request = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("左カーニンググループ:");
                        ui.text_edit_singleline(&mut glyph.left_kerning_group);
                    });
                    ui.horizontal(|ui| {
                        ui.label("右カーニンググループ:");
                        ui.text_edit_singleline(&mut glyph.right_kerning_group);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Unicode:");
                        let mut code = glyph.unicode.map(|u| format!("U+{:04X}", u)).unwrap_or_default();
                        if ui.text_edit_singleline(&mut code).changed() {
                            if let Some(hex) = code.strip_prefix("U+") {
                                glyph.unicode = u32::from_str_radix(hex, 16).ok();
                            } else {
                                glyph.unicode = u32::from_str_radix(&code, 16).ok();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("追加Unicode:");
                        ui.add(egui::TextEdit::singleline(unicode_alias_input).hint_text("U+XXXX"));
                        if ui.button("追加").clicked() {
                            let value = unicode_alias_input.strip_prefix("U+").unwrap_or(unicode_alias_input).trim();
                            if let Ok(unicode) = u32::from_str_radix(value, 16) {
                                if unicode <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&unicode) && glyph.unicode != Some(unicode) && !glyph.unicodes.contains(&unicode) {
                                    glyph.unicodes.push(unicode);
                                    unicode_alias_input.clear();
                                }
                            }
                        }
                    });
                    let mut remove_unicode = None;
                    for (index, unicode) in glyph.unicodes.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("追加: U+{unicode:04X}"));
                            if ui.small_button("削除").clicked() {
                                remove_unicode = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove_unicode {
                        glyph.unicodes.remove(index);
                    }
                    ui.collapsing("Unicode Variation Sequence", |ui| {
                        ui.small("現在のグリフを、異体字セレクタ用グリフへ割り当てます。");
                        ui.horizontal(|ui| {
                            ui.label("Selector:");
                            ui.add(egui::TextEdit::singleline(unicode_variation_selector).hint_text("FE00 / E0100").desired_width(90.0));
                            if ui.button("追加／更新").clicked() {
                                let selector_text = unicode_variation_selector.trim().strip_prefix("U+").or_else(|| unicode_variation_selector.trim().strip_prefix("u+")).unwrap_or(unicode_variation_selector.trim());
                                if let (Some(base), Ok(selector)) = (glyph.unicode, u32::from_str_radix(selector_text, 16)) {
                                    if (0xFE00..=0xFE0F).contains(&selector) || (0xE0100..=0xE01EF).contains(&selector) {
                                        if let Some(entry) = project.unicode_variation_sequences.iter_mut().find(|entry| entry.base == base && entry.selector == selector) {
                                            entry.glyph = glyph.name.clone();
                                        } else {
                                            project.unicode_variation_sequences.push(crate::font_data::UnicodeVariationSequence { base, selector, glyph: glyph.name.clone() });
                                        }
                                    }
                                }
                            }
                        });
                        let mut remove_variation = None;
                        for (index, entry) in project.unicode_variation_sequences.iter().enumerate().filter(|(_, entry)| entry.glyph == glyph.name) {
                            ui.horizontal(|ui| {
                                ui.label(format!("U+{:04X} U+{:04X} → {}", entry.base, entry.selector, entry.glyph));
                                if ui.small_button("削除").clicked() {
                                    remove_variation = Some(index);
                                }
                            });
                        }
                        if let Some(index) = remove_variation {
                            project.unicode_variation_sequences.remove(index);
                        }
                    });
                    ui.label(format!("輪郭数: {}", glyph.contours.len()));
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("アンカー");
                        if ui.small_button("topを追加").clicked() && !glyph.anchors.iter().any(|anchor| anchor.name == "top") {
                            glyph.anchors.push(crate::font_data::GlyphAnchor { name: "top".into(), x: glyph.width / 2.0, y: project.metadata.ascender });
                        }
                        if ui.small_button("topを輪郭へ").clicked() {
                            if let Some((min_x, _, max_x, y)) = outline_bounds {
                                if let Some(anchor) = glyph.anchors.iter_mut().find(|a| a.name == "top") {
                                    anchor.x = (min_x + max_x) * 0.5;
                                    anchor.y = y;
                                }
                            }
                        }
                        if ui.small_button("bottomを追加").clicked() && !glyph.anchors.iter().any(|anchor| anchor.name == "bottom") {
                            glyph.anchors.push(crate::font_data::GlyphAnchor { name: "bottom".into(), x: glyph.width / 2.0, y: project.metadata.descender });
                        }
                        if ui.small_button("_topを追加").clicked() && !glyph.anchors.iter().any(|anchor| anchor.name == "_top") {
                            glyph.anchors.push(crate::font_data::GlyphAnchor { name: "_top".into(), x: glyph.width / 2.0, y: project.metadata.ascender });
                        }
                        if ui.small_button("_bottomを追加").clicked() && !glyph.anchors.iter().any(|anchor| anchor.name == "_bottom") {
                            glyph.anchors.push(crate::font_data::GlyphAnchor { name: "_bottom".into(), x: glyph.width / 2.0, y: project.metadata.descender });
                        }
                        if ui.small_button("_topを輪郭へ").clicked() {
                            if let Some((min_x, _, max_x, y)) = outline_bounds {
                                if let Some(anchor) = glyph.anchors.iter_mut().find(|a| a.name == "_top") {
                                    anchor.x = (min_x + max_x) * 0.5;
                                    anchor.y = y;
                                }
                            }
                        }
                        if ui.small_button("_bottomを輪郭へ").clicked() {
                            if let Some((min_x, y, max_x, _)) = outline_bounds {
                                if let Some(anchor) = glyph.anchors.iter_mut().find(|a| a.name == "_bottom") {
                                    anchor.x = (min_x + max_x) * 0.5;
                                    anchor.y = y;
                                }
                            }
                        }
                        if ui.small_button("bottomを輪郭へ").clicked() {
                            if let Some((min_x, y, max_x, _)) = outline_bounds {
                                if let Some(anchor) = glyph.anchors.iter_mut().find(|a| a.name == "bottom") {
                                    anchor.x = (min_x + max_x) * 0.5;
                                    anchor.y = y;
                                }
                            }
                        }
                    });
                    let mut remove_anchor = None;
                    for (index, anchor) in glyph.anchors.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut anchor.name).desired_width(70.0));
                            ui.add(egui::DragValue::new(&mut anchor.x).speed(1.0).prefix("x "));
                            ui.add(egui::DragValue::new(&mut anchor.y).speed(1.0).prefix("y "));
                            if ui.small_button("削除").clicked() {
                                remove_anchor = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove_anchor {
                        glyph.anchors.remove(index);
                    }
                    egui::CollapsingHeader::new("グリフガイド").default_open(false).show(ui, |ui| {
                        ui.separator();
                        let ascender = project.metadata.ascender;
                        let glyph_width = glyph.width;
                        let master_guidelines = glyph.guidelines_for_master_mut(current_master_id);
                        ui.horizontal(|ui| {
                            ui.label("グリフガイド");
                            if ui.small_button("水平ガイドを追加").clicked() {
                                master_guidelines.push(crate::font_data::Guideline { x: 0.0, y: ascender, angle: 0.0, name: String::new() });
                            }
                            if ui.small_button("垂直").clicked() {
                                master_guidelines.push(crate::font_data::Guideline { x: glyph_width / 2.0, y: 0.0, angle: 90.0, name: String::new() });
                            }
                            if ui.small_button("45°").clicked() {
                                master_guidelines.push(crate::font_data::Guideline { x: 0.0, y: 0.0, angle: 45.0, name: String::new() });
                            }
                        });
                        let mut remove_glyph_guideline = None;
                        for (index, guide) in master_guidelines.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut guide.x).speed(1.0).prefix("x "));
                                ui.add(egui::DragValue::new(&mut guide.y).speed(1.0).prefix("y "));
                                ui.add(egui::DragValue::new(&mut guide.angle).speed(1.0).suffix("°"));
                                ui.add(egui::TextEdit::singleline(&mut guide.name).desired_width(70.0));
                                if ui.small_button("削除").clicked() {
                                    remove_glyph_guideline = Some(index);
                                }
                            });
                        }
                        if let Some(index) = remove_glyph_guideline {
                            master_guidelines.remove(index);
                        }
                    });
                    ui.separator();
                    ui.label("コンポーネント追加");
                    ui.horizontal(|ui| {
                        if ui.small_button("全てアンカー整列").clicked() {
                            *align_all_components = true;
                        }
                        ui.add(egui::TextEdit::singleline(component_base).hint_text("基底グリフ名"));
                        if ui.button("追加").clicked() && !component_base.trim().is_empty() && component_exists && component_base.trim() != name {
                            let component = crate::font_data::GlyphComponent {
                                base: component_base.trim().to_string(),
                                x_scale: 1.0,
                                xy_scale: 0.0,
                                yx_scale: 0.0,
                                y_scale: 1.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                            };
                            glyph.components.push(component.clone());
                            for layer in glyph.layers.values_mut() {
                                layer.components.push(component.clone());
                            }
                            *align_component_request = Some(glyph.components.len() - 1);
                            component_base.clear();
                        }
                    });
                    let mut remove_component = None;
                    let mut move_component = None;
                    let component_count = glyph.components.len();
                    for (index, component) in glyph.components.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.small_button("↑").clicked() && index > 0 {
                                move_component = Some((index, -1));
                            }
                            if ui.small_button("↓").clicked() && index + 1 < component_count {
                                move_component = Some((index, 1));
                            }
                            ui.label(&component.base);
                            ui.add(egui::DragValue::new(&mut component.x_scale).speed(0.01));
                            ui.add(egui::DragValue::new(&mut component.xy_scale).speed(0.01));
                            ui.add(egui::DragValue::new(&mut component.yx_scale).speed(0.01));
                            ui.add(egui::DragValue::new(&mut component.y_scale).speed(0.01));
                            ui.add(egui::DragValue::new(&mut component.x_offset).speed(1.0));
                            ui.add(egui::DragValue::new(&mut component.y_offset).speed(1.0));
                            if ui.small_button("アンカー整列").clicked() {
                                *align_component_request = Some(index);
                            }
                            if ui.small_button("削除").clicked() {
                                remove_component = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove_component {
                        if index < glyph.components.len() && glyph.layers.values().all(|layer| index < layer.components.len()) {
                            glyph.components.remove(index);
                            for layer in glyph.layers.values_mut() {
                                layer.components.remove(index);
                            }
                        }
                    }
                    if let Some((index, direction)) = move_component {
                        let target = if direction < 0 { index - 1 } else { index + 1 };
                        if target < glyph.components.len() && glyph.layers.values().all(|layer| target < layer.components.len()) {
                            glyph.components.swap(index, target);
                            for layer in glyph.layers.values_mut() {
                                layer.components.swap(index, target);
                            }
                        }
                    }
                }
                if let Some((left, right)) = bearing_request {
                    let names = vec![name.to_string()];
                    project.set_side_bearings(&names, left, right);
                }
                if let Some((advance_height, top_side_bearing)) = vertical_request {
                    let _ = project.set_vertical_metrics_for_master(name, current_master_id, advance_height, top_side_bearing);
                }
                if metrics_key_request {
                    let metrics_key_names = vec![name.to_string()];
                    match project.apply_metrics_keys(&metrics_key_names) {
                        Ok(count) if count > 0 => {}
                        Ok(_) => {}
                        Err(error) => {
                            // Keep the edit in place so the user can correct
                            // the reference without losing the entered key.
                            ui.colored_label(egui::Color32::from_rgb(230, 130, 100), error);
                        }
                    }
                }
            }
        });
    }
}
