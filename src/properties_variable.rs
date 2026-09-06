#![allow(clippy::too_many_arguments, unused_variables)]

use super::*;

pub fn show_properties_variable(
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
    if show_section(&["variable", "可変", "axis", "軸"]) {
        egui::CollapsingHeader::new("可変軸")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("軸名:");
                let mut axis_tags = std::collections::BTreeSet::new();
                for master in &project.masters {
                    axis_tags.extend(master.axes.keys().cloned());
                }
                if project.masters.len() >= 2
                    && project
                        .masters
                        .windows(2)
                        .any(|masters| (masters[0].weight - masters[1].weight).abs() > f64::EPSILON)
                {
                    axis_tags.insert("wght".into());
                }
                if project.masters.len() >= 2
                    && project
                        .masters
                        .windows(2)
                        .any(|masters| (masters[0].width - masters[1].width).abs() > f64::EPSILON)
                {
                    axis_tags.insert("wdth".into());
                }
                let axis_tags: Vec<String> = axis_tags.into_iter().collect();
                for tag in &axis_tags {
                    let axis_name = project.axis_names.entry(tag.clone()).or_insert(tag.clone());
                    ui.horizontal(|ui| {
                        ui.label(tag);
                        ui.text_edit_singleline(axis_name);
                    });
                    let flags = project.axis_flags.entry(tag.clone()).or_default();
                    let mut hidden = *flags & 0x0001 != 0;
                    if ui
                        .checkbox(&mut hidden, "Hidden Axis")
                        .on_hover_text("fvarのHidden Axisフラグ。通常の軸一覧から隠す軸")
                        .changed()
                    {
                        if hidden {
                            *flags |= 0x0001;
                        } else {
                            *flags &= !0x0001;
                        }
                    }
                }
                if !axis_tags.is_empty() {
                    ui.separator();
                    egui::CollapsingHeader::new("非線形軸マッピング（avar）")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.small("正規化座標 -1.0〜1.0 の入力を別の座標へ再マッピングします。");
                            for tag in &axis_tags {
                                let points = project.axis_mappings.entry(tag.clone()).or_default();
                                ui.label(egui::RichText::new(tag).monospace().strong());
                                let mut remove = None;
                                for (index, point) in points.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label("入力");
                                        ui.add(
                                            egui::DragValue::new(&mut point.input)
                                                .range(-1.0..=1.0)
                                                .speed(0.01),
                                        );
                                        ui.label("→ 出力");
                                        ui.add(
                                            egui::DragValue::new(&mut point.output)
                                                .range(-1.0..=1.0)
                                                .speed(0.01),
                                        );
                                        if ui.small_button("削除").clicked() {
                                            remove = Some(index);
                                        }
                                    });
                                }
                                if let Some(index) = remove {
                                    points.remove(index);
                                }
                                if ui.small_button(format!("{} に変換点を追加", tag)).clicked()
                                {
                                    points.push(crate::font_data::AxisMappingPoint {
                                        input: 0.0,
                                        output: 0.0,
                                    });
                                }
                            }
                        });
                }
                if project.masters.len() >= 2 {
                    let mut coordinate_axes = std::collections::BTreeSet::new();
                    for master in &project.masters {
                        coordinate_axes.extend(master.axes.keys().cloned());
                    }
                    if project.masters.len() >= 2
                        && project.masters.windows(2).any(|masters| {
                            (masters[0].weight - masters[1].weight).abs() > f64::EPSILON
                        })
                    {
                        coordinate_axes.insert("wght".into());
                    }
                    if project.masters.len() >= 2
                        && project.masters.windows(2).any(|masters| {
                            (masters[0].width - masters[1].width).abs() > f64::EPSILON
                        })
                    {
                        coordinate_axes.insert("wdth".into());
                    }
                    let coordinate_axes: Vec<String> = coordinate_axes.into_iter().collect();
                    if !coordinate_axes.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new("マスター配置").strong());
                        if coordinate_axes.len() >= 2 {
                            let points: Vec<(f64, f64)> = project
                                .masters
                                .iter()
                                .filter_map(|master| {
                                    Some((
                                        master
                                            .axes
                                            .get(&coordinate_axes[0])
                                            .copied()
                                            .or_else(|| {
                                                (coordinate_axes[0] == "wght")
                                                    .then_some(master.weight)
                                            })
                                            .or_else(|| {
                                                (coordinate_axes[0] == "wdth")
                                                    .then_some(master.width)
                                            })?,
                                        master
                                            .axes
                                            .get(&coordinate_axes[1])
                                            .copied()
                                            .or_else(|| {
                                                (coordinate_axes[1] == "wght")
                                                    .then_some(master.weight)
                                            })
                                            .or_else(|| {
                                                (coordinate_axes[1] == "wdth")
                                                    .then_some(master.width)
                                            })?,
                                    ))
                                })
                                .collect();
                            if let (Some((x_min, x_max)), Some((y_min, y_max))) = (
                                points
                                    .iter()
                                    .map(|point| point.0)
                                    .min_by(f64::total_cmp)
                                    .zip(points.iter().map(|point| point.0).max_by(f64::total_cmp)),
                                points
                                    .iter()
                                    .map(|point| point.1)
                                    .min_by(f64::total_cmp)
                                    .zip(points.iter().map(|point| point.1).max_by(f64::total_cmp)),
                            ) {
                                let (response, painter) = ui.allocate_painter(
                                    egui::vec2(250.0, 150.0),
                                    egui::Sense::click_and_drag(),
                                );
                                let response = response.on_hover_text(
                                    "点をクリックしてマスター切替。ドラッグして軸値を編集",
                                );
                                let plot = response.rect.shrink2(egui::vec2(30.0, 18.0));
                                painter.rect_stroke(
                                    plot,
                                    2.0,
                                    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(90)),
                                    egui::StrokeKind::Inside,
                                );
                                painter.text(
                                    egui::pos2(plot.left(), response.rect.bottom() - 4.0),
                                    egui::Align2::LEFT_BOTTOM,
                                    &coordinate_axes[0],
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::from_gray(170),
                                );
                                painter.text(
                                    egui::pos2(response.rect.left() + 4.0, plot.top()),
                                    egui::Align2::LEFT_TOP,
                                    &coordinate_axes[1],
                                    egui::FontId::monospace(10.0),
                                    egui::Color32::from_gray(170),
                                );
                                painter.text(
                                    egui::pos2(plot.left(), plot.bottom() + 2.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{x_min:.0}"),
                                    egui::FontId::monospace(9.0),
                                    egui::Color32::from_gray(130),
                                );
                                painter.text(
                                    egui::pos2(plot.right(), plot.bottom() + 2.0),
                                    egui::Align2::RIGHT_TOP,
                                    format!("{x_max:.0}"),
                                    egui::FontId::monospace(9.0),
                                    egui::Color32::from_gray(130),
                                );
                                painter.text(
                                    egui::pos2(plot.left() - 4.0, plot.bottom()),
                                    egui::Align2::RIGHT_BOTTOM,
                                    format!("{y_min:.0}"),
                                    egui::FontId::monospace(9.0),
                                    egui::Color32::from_gray(130),
                                );
                                painter.text(
                                    egui::pos2(plot.left() - 4.0, plot.top()),
                                    egui::Align2::RIGHT_TOP,
                                    format!("{y_max:.0}"),
                                    egui::FontId::monospace(9.0),
                                    egui::Color32::from_gray(130),
                                );
                                let x_span = (x_max - x_min).abs().max(1.0);
                                let y_span = (y_max - y_min).abs().max(1.0);
                                let mut clicked_master = None;
                                let mut dragged_master = None;
                                for master in &project.masters {
                                    let (Some(x), Some(y)) = (
                                        master
                                            .axes
                                            .get(&coordinate_axes[0])
                                            .or_else(|| {
                                                (coordinate_axes[0] == "wght")
                                                    .then_some(&master.weight)
                                            })
                                            .or_else(|| {
                                                (coordinate_axes[0] == "wdth")
                                                    .then_some(&master.width)
                                            }),
                                        master
                                            .axes
                                            .get(&coordinate_axes[1])
                                            .or_else(|| {
                                                (coordinate_axes[1] == "wght")
                                                    .then_some(&master.weight)
                                            })
                                            .or_else(|| {
                                                (coordinate_axes[1] == "wdth")
                                                    .then_some(&master.width)
                                            }),
                                    ) else {
                                        continue;
                                    };
                                    let position = egui::pos2(
                                        plot.left() + ((*x - x_min) / x_span) as f32 * plot.width(),
                                        plot.bottom()
                                            - ((*y - y_min) / y_span) as f32 * plot.height(),
                                    );
                                    let near_point = response
                                        .interact_pointer_pos()
                                        .is_some_and(|pointer| pointer.distance(position) <= 10.0);
                                    if near_point {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                    }
                                    if response.drag_started() && near_point {
                                        *master_map_drag = Some(master.id.clone());
                                    }
                                    if master_map_drag.as_deref() == Some(master.id.as_str())
                                        || near_point
                                    {
                                        if response.dragged() {
                                            if let Some(pointer) = response.interact_pointer_pos() {
                                                let x = x_min
                                                    + (((pointer.x - plot.left()) / plot.width())
                                                        .clamp(0.0, 1.0)
                                                        as f64)
                                                        * x_span;
                                                let y = y_min
                                                    + (((plot.bottom() - pointer.y) / plot.height())
                                                        .clamp(0.0, 1.0)
                                                        as f64)
                                                        * y_span;
                                                dragged_master = Some((master.id.clone(), x, y));
                                            }
                                        } else if response.clicked() {
                                            clicked_master = Some(master.id.clone());
                                        }
                                    }
                                    let selected = master.id == current_master_id.as_str();
                                    painter.circle_filled(
                                        position,
                                        if selected { 6.0 } else { 4.0 },
                                        if selected {
                                            egui::Color32::from_rgb(255, 210, 90)
                                        } else {
                                            egui::Color32::from_rgb(120, 190, 235)
                                        },
                                    );
                                    painter.text(
                                        position + egui::vec2(7.0, -7.0),
                                        egui::Align2::LEFT_TOP,
                                        &master.name,
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::from_gray(190),
                                    );
                                }
                                if let Some(master_id) = clicked_master {
                                    *current_master_id = master_id;
                                }
                                if let Some((master_id, x, y)) = dragged_master {
                                    if let Some(master) = project
                                        .masters
                                        .iter_mut()
                                        .find(|master| master.id == master_id)
                                    {
                                        for (tag, value) in [
                                            (coordinate_axes[0].as_str(), x),
                                            (coordinate_axes[1].as_str(), y),
                                        ] {
                                            match tag {
                                                "wght" => master.weight = value,
                                                "wdth" => master.width = value,
                                                _ => {
                                                    master.axes.insert(tag.to_string(), value);
                                                }
                                            }
                                        }
                                        *current_master_id = master_id;
                                    }
                                }
                                if response.drag_stopped() {
                                    *master_map_drag = None;
                                }
                            }
                        }
                        for master in &project.masters {
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(8, 5))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(&master.name).strong());
                                        for tag in &coordinate_axes {
                                            if let Some(value) = master.axes.get(tag) {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{tag} {value:.1}"
                                                    ))
                                                    .monospace()
                                                    .color(egui::Color32::from_rgb(150, 205, 235)),
                                                );
                                            }
                                        }
                                    });
                                });
                        }
                    }
                }
            });
    }
}
