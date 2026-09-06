use super::*;

#[allow(clippy::too_many_arguments, clippy::ptr_arg, unused_variables)]
pub fn show_properties_master(
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
    if show_section(&["master", "マスター", "メトリクス", "metrics"]) {
        egui::CollapsingHeader::new("マスター別メトリクス")
            .default_open(false)
            .show(ui, |ui| {
                let mut metrics = project.master_metrics_for(current_master_id);
                ui.small("Variable Fontの軸変化に追従するhhea／OS/2メトリクス");
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("アセンダー:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut metrics.ascender).speed(10.0))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("ディセンダー:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut metrics.descender).speed(10.0))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("行間:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut metrics.line_gap).speed(10.0))
                        .changed();
                    if ui
                        .small_button("共通値へ戻す")
                        .on_hover_text("このマスターだけプロジェクト共通メトリクスを継承")
                        .clicked()
                    {
                        project.clear_master_metrics(current_master_id);
                    }
                });
                if changed {
                    let _ = project.set_master_metrics(current_master_id, metrics);
                }
            });
    }
}
