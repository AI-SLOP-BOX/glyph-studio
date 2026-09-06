use super::*;

#[allow(clippy::too_many_arguments, clippy::ptr_arg, unused_variables)]
pub fn show_properties_background(
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
    if show_section(&["background", "背景", "画像", "image"]) {
        if let Some(name) = current_glyph.as_ref() {
            egui::CollapsingHeader::new("背景画像")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("画像ファイル:");
                    let image_path = project
                        .background_images
                        .entry(name.clone())
                        .or_default()
                        .entry(current_master_id.to_string())
                        .or_default();
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(image_path).desired_width(140.0));
                        if ui.button("選択…").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("画像", &["png", "jpg", "jpeg", "webp", "svg"])
                                .pick_file()
                            {
                                *image_path = path.display().to_string();
                            }
                        }
                        if ui.small_button("解除").clicked() {
                            image_path.clear();
                        }
                    });
                    let opacity = project
                        .background_opacities
                        .entry(name.clone())
                        .or_default()
                        .entry(current_master_id.to_string())
                        .or_insert(0.35);
                    ui.add(egui::Slider::new(opacity, 0.0..=1.0).text("不透明度"));
                    let transform = project
                        .background_transforms
                        .entry(name.clone())
                        .or_default()
                        .entry(current_master_id.to_string())
                        .or_insert(crate::font_data::BackgroundImageTransform {
                            x: 0.0,
                            y: 0.0,
                            scale: 1.0,
                            rotation: 0.0,
                            flip_x: false,
                            flip_y: false,
                        });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut transform.x)
                                .speed(1.0)
                                .prefix("X "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut transform.y)
                                .speed(1.0)
                                .prefix("Y "),
                        );
                    });
                    ui.add(
                        egui::DragValue::new(&mut transform.scale)
                            .speed(0.01)
                            .range(0.001..=100.0)
                            .prefix("倍率 "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut transform.rotation)
                            .speed(0.5)
                            .prefix("回転° "),
                    );
                    ui.checkbox(&mut transform.flip_x, "左右反転");
                    ui.checkbox(&mut transform.flip_y, "上下反転");
                    if ui.small_button("グリフ幅に合わせる").clicked() {
                        let glyph_width = project
                            .glyphs
                            .get(name)
                            .and_then(|glyph| glyph.layers.get(current_master_id))
                            .map(|layer| layer.width)
                            .unwrap_or(0.0);
                        let image_width = if std::path::Path::new(image_path.as_str())
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
                        {
                            std::fs::read(image_path.as_str())
                                .ok()
                                .and_then(|bytes| {
                                    egui_extras::RetainedImage::from_svg_bytes(
                                        "background-fit",
                                        &bytes,
                                    )
                                    .ok()
                                })
                                .map(|image| image.width() as f64)
                        } else {
                            image::image_dimensions(image_path.as_str())
                                .ok()
                                .map(|(width, _)| f64::from(width))
                        };
                        if let Some(image_width) = image_width {
                            if image_width > 0.0 {
                                transform.scale = (glyph_width / image_width) as f32;
                            }
                        }
                    }
                    if ui.small_button("グリフ中央に配置").clicked() {
                        let glyph_width = project
                            .glyphs
                            .get(name)
                            .and_then(|glyph| glyph.layers.get(current_master_id))
                            .map(|layer| layer.width)
                            .unwrap_or(0.0);
                        let image_width = if std::path::Path::new(image_path.as_str())
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
                        {
                            std::fs::read(image_path.as_str())
                                .ok()
                                .and_then(|bytes| {
                                    egui_extras::RetainedImage::from_svg_bytes(
                                        "background-center",
                                        &bytes,
                                    )
                                    .ok()
                                })
                                .map(|image| image.width() as f64)
                        } else {
                            image::image_dimensions(image_path.as_str())
                                .ok()
                                .map(|(width, _)| f64::from(width))
                        };
                        if let Some(image_width) = image_width {
                            transform.x = ((glyph_width - image_width * f64::from(transform.scale))
                                * 0.5) as f32;
                        }
                    }
                    if ui.small_button("画像の変形をリセット").clicked() {
                        *transform = crate::font_data::BackgroundImageTransform {
                            x: 0.0,
                            y: 0.0,
                            scale: 1.0,
                            rotation: 0.0,
                            flip_x: false,
                            flip_y: false,
                        };
                    }
                });
        }
    }
}
