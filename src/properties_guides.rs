#![allow(clippy::too_many_arguments)]

use super::*;

pub fn show_properties_guides(
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
    if show_section(&["guide", "ガイド"]) {
        egui::CollapsingHeader::new("ガイド")
            .default_open(false)
            .show(ui, |ui| {
                let ascender = project.metadata.ascender;
                let units_per_em = project.metadata.units_per_em;
                let master_guidelines = project.guidelines_for_master_mut(current_master_id);
                ui.horizontal(|ui| {
                    ui.label("ガイド操作");
                    if ui.small_button("水平ガイドを追加").clicked() {
                        master_guidelines.push(crate::font_data::Guideline {
                            x: 0.0,
                            y: ascender,
                            angle: 0.0,
                            name: String::new(),
                        });
                    }
                    if ui.small_button("垂直").clicked() {
                        master_guidelines.push(crate::font_data::Guideline {
                            x: units_per_em / 2.0,
                            y: 0.0,
                            angle: 90.0,
                            name: String::new(),
                        });
                    }
                    if ui.small_button("45°").clicked() {
                        master_guidelines.push(crate::font_data::Guideline {
                            x: 0.0,
                            y: 0.0,
                            angle: 45.0,
                            name: String::new(),
                        });
                    }
                });
                let mut remove_guideline = None;
                for (index, guide) in master_guidelines.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut guide.x).speed(1.0).prefix("x "));
                        ui.add(egui::DragValue::new(&mut guide.y).speed(1.0).prefix("y "));
                        ui.add(
                            egui::DragValue::new(&mut guide.angle)
                                .speed(1.0)
                                .suffix("°"),
                        );
                        ui.add(egui::TextEdit::singleline(&mut guide.name).desired_width(70.0));
                        if ui.small_button("削除").clicked() {
                            remove_guideline = Some(index);
                        }
                    });
                }
                if let Some(index) = remove_guideline {
                    master_guidelines.remove(index);
                }
            });
    }
}
