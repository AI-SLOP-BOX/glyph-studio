#![allow(clippy::too_many_arguments, unused_variables)]

use super::*;

pub fn show_properties_color(
    ui: &mut Ui,
    properties_filter: &mut String,
    project: &mut FontProject,
    current_glyph: &Option<String>,
    color_layer_glyph: &mut String,
    preview_color_palette: &mut usize,
) {
    let filter = properties_filter.trim().to_lowercase();
    let show_section = |keywords: &[&str]| {
        filter.is_empty()
            || keywords
                .iter()
                .any(|keyword| filter.contains(&keyword.to_lowercase()))
    };
    if show_section(&["color", "カラー", "palette", "パレット"]) {
        egui::CollapsingHeader::new("カラーグリフ")
            .default_open(false)
            .show(ui, |ui| {
                ui.heading("カラーグリフ");
                let mut remove_color = None;
                let mut remove_palette = None;
                project
                    .color_palette_names
                    .resize(project.color_palettes.len(), String::new());
                project
                    .color_palette_types
                    .resize(project.color_palettes.len(), 0);
                let color_count = project.color_palettes.first().map_or(0, Vec::len);
                project
                    .color_palette_entry_names
                    .resize(color_count, String::new());
                let palette_names = &mut project.color_palette_names;
                let palette_types = &mut project.color_palette_types;
                let entry_names = &mut project.color_palette_entry_names;
                for (palette_index, palette) in project.color_palettes.iter_mut().enumerate() {
                    egui::CollapsingHeader::new(format!("パレット {}", palette_index + 1))
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("ラベル");
                                ui.add(
                                    egui::TextEdit::singleline(&mut palette_names[palette_index])
                                        .hint_text("Dark / Light / High Contrast"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("用途");
                                let mut light_background =
                                    palette_types[palette_index] & 0x0000_0001 != 0;
                                if ui.checkbox(&mut light_background, "明るい背景").changed() {
                                    if light_background {
                                        palette_types[palette_index] |= 0x0000_0001;
                                    } else {
                                        palette_types[palette_index] &= !0x0000_0001;
                                    }
                                }
                                let mut dark_background =
                                    palette_types[palette_index] & 0x0000_0002 != 0;
                                if ui.checkbox(&mut dark_background, "暗い背景").changed() {
                                    if dark_background {
                                        palette_types[palette_index] |= 0x0000_0002;
                                    } else {
                                        palette_types[palette_index] &= !0x0000_0002;
                                    }
                                }
                            });
                            for (color_index, color) in palette.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}", color_index));
                                    let mut picked = egui::Color32::from_rgba_unmultiplied(
                                        color[0], color[1], color[2], color[3],
                                    );
                                    if ui.color_edit_button_srgba(&mut picked).changed() {
                                        *color = [picked.r(), picked.g(), picked.b(), picked.a()];
                                    }
                                    for channel in color.iter_mut() {
                                        ui.add(egui::DragValue::new(channel).range(0..=255));
                                    }
                                    if palette_index == 0 {
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut entry_names[color_index],
                                            )
                                            .hint_text("Fill / Outline"),
                                        );
                                    }
                                    if ui.small_button("削除").clicked() {
                                        remove_color = Some(color_index);
                                    }
                                });
                            }
                            if ui.small_button("このパレットを削除").clicked() {
                                remove_palette = Some(palette_index);
                            }
                        });
                }
                if let Some(index) = remove_color {
                    remove_color_palette_entry(project, index);
                }
                if let Some(index) = remove_palette {
                    project.color_palettes.remove(index);
                    project.color_palette_names.remove(index);
                    project.color_palette_types.remove(index);
                    if project.color_palettes.is_empty() {
                        project.color_layers.clear();
                        project.color_palette_entry_names.clear();
                    } else {
                        for layers in project.color_layers.values_mut() {
                            for layer in layers {
                                if usize::from(layer.palette_index) > index {
                                    layer.palette_index -= 1;
                                } else if usize::from(layer.palette_index) == index {
                                    layer.palette_index = 0;
                                }
                            }
                        }
                    }
                }
                if !project.color_palettes.is_empty() && ui.button("全パレットに色を追加").clicked()
                {
                    for palette in &mut project.color_palettes {
                        palette.push([0, 0, 0, 255]);
                    }
                    project.color_palette_entry_names.push(String::new());
                }
                if !project.color_palettes.is_empty() {
                    *preview_color_palette =
                        (*preview_color_palette).min(project.color_palettes.len() - 1);
                    ui.horizontal(|ui| {
                        ui.label("プレビュー:");
                        egui::ComboBox::from_id_salt("preview-color-palette")
                            .selected_text(format!("パレット {}", *preview_color_palette + 1))
                            .show_ui(ui, |ui| {
                                for index in 0..project.color_palettes.len() {
                                    ui.selectable_value(
                                        preview_color_palette,
                                        index,
                                        format!("パレット {}", index + 1),
                                    );
                                }
                            });
                    });
                } else {
                    *preview_color_palette = 0;
                }
                if !project.color_palettes.is_empty() && ui.button("現在のパレットを複製").clicked()
                {
                    if let Some(palette) =
                        project.color_palettes.get(*preview_color_palette).cloned()
                    {
                        let palette_type = project
                            .color_palette_types
                            .get(*preview_color_palette)
                            .copied()
                            .unwrap_or(0);
                        project.color_palettes.push(palette);
                        project.color_palette_names.push(String::new());
                        project.color_palette_types.push(palette_type);
                        *preview_color_palette = project.color_palettes.len() - 1;
                    }
                }
                if ui.button("パレットを追加").clicked() {
                    let count = project.color_palettes.first().map_or(1, Vec::len);
                    project.color_palettes.push(vec![[0, 0, 0, 255]; count]);
                    project.color_palette_names.push(String::new());
                    project.color_palette_types.push(0);
                }
                if let Some(name) = current_glyph {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} の層", name));
                        egui::ComboBox::from_id_salt("color-layer-glyph")
                            .selected_text(if color_layer_glyph.is_empty() {
                                "層グリフを選択"
                            } else {
                                color_layer_glyph.as_str()
                            })
                            .show_ui(ui, |ui| {
                                for glyph_name in project.glyph_names_sorted() {
                                    ui.selectable_value(
                                        color_layer_glyph,
                                        glyph_name.to_string(),
                                        glyph_name,
                                    );
                                }
                            });
                        if ui.button("追加").clicked()
                            && !color_layer_glyph.is_empty()
                            && !project.color_palettes.is_empty()
                        {
                            project.color_layers.entry(name.clone()).or_default().push(
                                crate::font_data::ColorLayer {
                                    glyph: color_layer_glyph.clone(),
                                    palette_index: 0,
                                    gradient: None,
                                    alpha: 1.0,
                                },
                            );
                            project
                                .color_layer_transforms
                                .entry(name.clone())
                                .or_default()
                                .push(None);
                            color_layer_glyph.clear();
                        }
                    });
                    let mut remove = None;
                    let mut move_layer = None;
                    let transforms = project
                        .color_layer_transforms
                        .entry(name.clone())
                        .or_default();
                    let layer_palette = project.color_palettes.first().cloned().unwrap_or_default();
                    if let Some(layers) = project.color_layers.get_mut(name) {
                        let layer_count = layers.len();
                        transforms.resize(layer_count, None);
                        for (index, layer) in layers.iter_mut().enumerate() {
                            let swatch = layer_palette
                                .get(usize::from(layer.palette_index))
                                .copied()
                                .unwrap_or([0, 0, 0, 0]);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("●")
                                        .color(egui::Color32::from_rgba_unmultiplied(
                                            swatch[0], swatch[1], swatch[2], swatch[3],
                                        ))
                                        .size(18.0),
                                )
                                .on_hover_text(format!(
                                    "パレット色 {}  ·  RGBA({}, {}, {}, {})",
                                    layer.palette_index, swatch[0], swatch[1], swatch[2], swatch[3]
                                ));
                                ui.label(format!("{}: {}", index + 1, layer.glyph));
                                let max_palette = project
                                    .color_palettes
                                    .first()
                                    .map_or(0, |palette| palette.len().saturating_sub(1));
                                ui.add(
                                    egui::DragValue::new(&mut layer.palette_index)
                                        .range(0..=u16::try_from(max_palette).unwrap_or(u16::MAX)),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut layer.alpha)
                                        .range(0.0..=1.0)
                                        .speed(0.01)
                                        .prefix("α "),
                                );
                                if ui.small_button("削除").clicked() {
                                    remove = Some(index);
                                }
                                if index > 0 && ui.small_button("↑").clicked() {
                                    move_layer = Some((index, index - 1));
                                }
                                if index + 1 < layer_count && ui.small_button("↓").clicked() {
                                    move_layer = Some((index, index + 1));
                                }
                            });
                            let mut gradient_enabled = layer.gradient.is_some();
                            if ui
                                .checkbox(&mut gradient_enabled, "グラデーションを有効化")
                                .changed()
                            {
                                let max_palette = project
                                    .color_palettes
                                    .first()
                                    .map_or(0, |palette| palette.len().saturating_sub(1));
                                layer.gradient =
                                    gradient_enabled.then(|| crate::font_data::ColorGradient {
                                        start_palette_index: layer.palette_index,
                                        end_palette_index: layer
                                            .palette_index
                                            .saturating_add(1)
                                            .min(u16::try_from(max_palette).unwrap_or(u16::MAX)),
                                        kind: crate::font_data::ColorGradientKind::Linear,
                                        extend: crate::font_data::ColorGradientExtend::default(),
                                        x0: 0.0,
                                        y0: 0.0,
                                        x1: 1000.0,
                                        y1: 0.0,
                                        x2: 0.0,
                                        y2: 1000.0,
                                        stops: Vec::new(),
                                        radius0: 0.0,
                                        radius1: 500.0,
                                        start_angle: 0.0,
                                        end_angle: 360.0,
                                    });
                            }
                            if let Some(gradient) = layer.gradient.as_mut() {
                                egui::CollapsingHeader::new("グラデーション設定")
                                    .id_salt(("color-gradient", index))
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("種類");
                                            egui::ComboBox::from_id_salt((
                                                "color-gradient-kind",
                                                index,
                                            ))
                                            .selected_text(match gradient.kind {
                                                crate::font_data::ColorGradientKind::Linear => {
                                                    "線形"
                                                }
                                                crate::font_data::ColorGradientKind::Radial => {
                                                    "放射"
                                                }
                                                crate::font_data::ColorGradientKind::Sweep => {
                                                    "スイープ"
                                                }
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut gradient.kind,
                                                        crate::font_data::ColorGradientKind::Linear,
                                                        "線形",
                                                    );
                                                    ui.selectable_value(
                                                        &mut gradient.kind,
                                                        crate::font_data::ColorGradientKind::Radial,
                                                        "放射",
                                                    );
                                                    ui.selectable_value(
                                                        &mut gradient.kind,
                                                        crate::font_data::ColorGradientKind::Sweep,
                                                        "スイープ",
                                                    );
                                                },
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("範囲外");
                                            egui::ComboBox::from_id_salt((
                                                "color-gradient-extend",
                                                index,
                                            ))
                                            .selected_text(match gradient.extend {
                                                crate::font_data::ColorGradientExtend::Pad => {
                                                    "端色で延長"
                                                }
                                                crate::font_data::ColorGradientExtend::Repeat => {
                                                    "繰り返し"
                                                }
                                                crate::font_data::ColorGradientExtend::Reflect => {
                                                    "反転繰り返し"
                                                }
                                            })
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    ui.selectable_value(
                                                        &mut gradient.extend,
                                                        crate::font_data::ColorGradientExtend::Pad,
                                                        "端色で延長",
                                                    );
                                                    ui.selectable_value(
                                                    &mut gradient.extend,
                                                    crate::font_data::ColorGradientExtend::Repeat,
                                                    "繰り返し",
                                                );
                                                    ui.selectable_value(
                                                    &mut gradient.extend,
                                                    crate::font_data::ColorGradientExtend::Reflect,
                                                    "反転繰り返し",
                                                );
                                                },
                                            );
                                        });
                                        let max_palette = project
                                            .color_palettes
                                            .first()
                                            .map_or(0, |palette| palette.len().saturating_sub(1));
                                        ui.horizontal(|ui| {
                                            ui.label("色");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut gradient.start_palette_index,
                                                )
                                                .range(
                                                    0..=u16::try_from(max_palette)
                                                        .unwrap_or(u16::MAX),
                                                ),
                                            );
                                            ui.label("→");
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut gradient.end_palette_index,
                                                )
                                                .range(
                                                    0..=u16::try_from(max_palette)
                                                        .unwrap_or(u16::MAX),
                                                ),
                                            );
                                        });
                                        if gradient.stops.is_empty() {
                                            if ui.button("中間色ストップを編集").clicked()
                                            {
                                                gradient.stops = vec![
                                                    crate::font_data::ColorGradientStop {
                                                        offset: 0.0,
                                                        palette_index: gradient.start_palette_index,
                                                        alpha: 1.0,
                                                    },
                                                    crate::font_data::ColorGradientStop {
                                                        offset: 1.0,
                                                        palette_index: gradient.end_palette_index,
                                                        alpha: 1.0,
                                                    },
                                                ];
                                            }
                                        } else {
                                            ui.label("色ストップ");
                                            let mut remove_stop = None;
                                            for (stop_index, stop) in
                                                gradient.stops.iter_mut().enumerate()
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.add(
                                                        egui::DragValue::new(&mut stop.offset)
                                                            .speed(0.01)
                                                            .range(-2.0..=1.999),
                                                    );
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut stop.palette_index,
                                                        )
                                                        .range(
                                                            0..=u16::try_from(max_palette)
                                                                .unwrap_or(u16::MAX),
                                                        ),
                                                    );
                                                    ui.add(
                                                        egui::DragValue::new(&mut stop.alpha)
                                                            .speed(0.01)
                                                            .range(0.0..=1.0),
                                                    );
                                                    if ui.small_button("削除").clicked() {
                                                        remove_stop = Some(stop_index);
                                                    }
                                                });
                                            }
                                            if let Some(stop_index) = remove_stop {
                                                gradient.stops.remove(stop_index);
                                            }
                                            if ui.button("ストップを追加").clicked() {
                                                gradient.stops.push(
                                                    crate::font_data::ColorGradientStop {
                                                        offset: 0.5,
                                                        palette_index: gradient.end_palette_index,
                                                        alpha: 1.0,
                                                    },
                                                );
                                            }
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label("始点");
                                            ui.add(
                                                egui::DragValue::new(&mut gradient.x0).speed(1.0),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut gradient.y0).speed(1.0),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("終点");
                                            ui.add(
                                                egui::DragValue::new(&mut gradient.x1).speed(1.0),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut gradient.y1).speed(1.0),
                                            );
                                        });
                                        if gradient.kind
                                            == crate::font_data::ColorGradientKind::Linear
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("回転点");
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.x2)
                                                        .speed(1.0),
                                                );
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.y2)
                                                        .speed(1.0),
                                                );
                                            });
                                        }
                                        if gradient.kind
                                            == crate::font_data::ColorGradientKind::Radial
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("半径");
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.radius0)
                                                        .speed(1.0),
                                                );
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.radius1)
                                                        .speed(1.0),
                                                );
                                            });
                                        } else if gradient.kind
                                            == crate::font_data::ColorGradientKind::Sweep
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("角度");
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.start_angle)
                                                        .speed(1.0),
                                                );
                                                ui.add(
                                                    egui::DragValue::new(&mut gradient.end_angle)
                                                        .speed(1.0),
                                                );
                                            });
                                        }
                                    });
                            }
                            let transform = &mut transforms[index];
                            let mut transform_enabled = transform.is_some();
                            if ui.checkbox(&mut transform_enabled, "層を変形").changed() {
                                *transform = transform_enabled
                                    .then_some(crate::font_data::ColorLayerTransform::default());
                            }
                            if let Some(transform) = transform.as_mut() {
                                egui::CollapsingHeader::new("層変形設定")
                                    .id_salt(("color-layer-transform", index))
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("移動");
                                            ui.add(
                                                egui::DragValue::new(&mut transform.dx).speed(1.0),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut transform.dy).speed(1.0),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("横倍率");
                                            ui.add(
                                                egui::DragValue::new(&mut transform.xx).speed(0.01),
                                            );
                                            ui.label("縦倍率");
                                            ui.add(
                                                egui::DragValue::new(&mut transform.yy).speed(0.01),
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("斜交");
                                            ui.add(
                                                egui::DragValue::new(&mut transform.xy).speed(0.01),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut transform.yx).speed(0.01),
                                            );
                                        });
                                    });
                            }
                        }
                        if let Some(index) = remove {
                            layers.remove(index);
                            transforms.remove(index);
                        } else if let Some((from, to)) = move_layer {
                            layers.swap(from, to);
                            transforms.swap(from, to);
                        }
                    }
                }
            });
    }
}
