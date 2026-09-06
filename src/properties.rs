#![allow(deprecated)]

use crate::font_data::FontProject;
use egui::Ui;

#[path = "properties_font.rs"]
mod properties_font;

fn insert_feature_operation(source: &mut String, preferred_tag: &str, operation: &str) {
    let marker = format!("feature {preferred_tag} {{");
    if let Some(block_start) = source.find(&marker) {
        let mut depth = 0usize;
        for (offset, ch) in source[block_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' if depth > 0 => {
                    depth -= 1;
                    if depth == 0 {
                        source.insert_str(block_start + offset, operation);
                        return;
                    }
                }
                _ => {}
            }
        }
    }
    if !source.is_empty() && !source.ends_with('\n') {
        source.push('\n');
    }
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    source.push_str(separator);
    source.push_str(&format!(
        "feature {preferred_tag} {{\n{operation}}} {preferred_tag};\n"
    ));
}

fn normalize_mark_class(value: &str) -> String {
    let value = value.trim();
    if value.starts_with('@') {
        value.to_string()
    } else {
        format!("@{value}")
    }
}

fn split_feature_file_source(source: &str) -> (String, String) {
    let mut classes = String::new();
    let mut features = String::new();
    for statement in source.split_inclusive(';') {
        let trimmed = statement.trim_start();
        if trimmed.starts_with('@') && trimmed.contains('=') {
            classes.push_str(statement);
        } else {
            features.push_str(statement);
        }
    }
    (classes, features)
}

fn ensure_mark_class_for_glyph(project: &mut FontProject, glyph_name: &str) -> Option<String> {
    let (anchor_name, x, y) = project
        .glyphs
        .get(glyph_name.trim())?
        .anchors
        .iter()
        .find(|anchor| anchor.name.starts_with('_'))
        .map(|anchor| {
            (
                anchor.name.trim_start_matches('_').to_string(),
                anchor.x.round() as i16,
                anchor.y.round() as i16,
            )
        })?;
    if anchor_name.is_empty()
        || !anchor_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    let class_name = format!("@{anchor_name}");
    let declaration = format!(
        "    markClass {} <anchor {} {}> {};\n",
        glyph_name.trim(),
        x,
        y,
        class_name
    );
    if !project.opentype_features.contains(&declaration) {
        insert_feature_operation(&mut project.opentype_features, "mark", &declaration);
    }
    Some(class_name)
}

fn remove_color_palette_entry(project: &mut FontProject, index: usize) {
    if index < project.color_palette_entry_names.len() {
        project.color_palette_entry_names.remove(index);
    }
    for palette in &mut project.color_palettes {
        if index < palette.len() {
            palette.remove(index);
        }
    }
    let bases = project.color_layers.keys().cloned().collect::<Vec<_>>();
    for base in bases {
        let old_transforms = project
            .color_layer_transforms
            .get(&base)
            .cloned()
            .unwrap_or_default();
        let mut new_transforms = Vec::new();
        let Some(layers) = project.color_layers.get_mut(&base) else {
            continue;
        };
        let old_layers = std::mem::take(layers);
        for (old_index, mut layer) in old_layers.into_iter().enumerate() {
            if usize::from(layer.palette_index) == index {
                continue;
            }
            if usize::from(layer.palette_index) > index {
                layer.palette_index -= 1;
            }
            if let Some(gradient) = layer.gradient.as_mut() {
                gradient
                    .stops
                    .retain(|stop| usize::from(stop.palette_index) != index);
                for stop in &mut gradient.stops {
                    if usize::from(stop.palette_index) > index {
                        stop.palette_index -= 1;
                    }
                }
                if usize::from(gradient.start_palette_index) > index {
                    gradient.start_palette_index -= 1;
                } else if usize::from(gradient.start_palette_index) == index {
                    gradient.start_palette_index = 0;
                }
                if usize::from(gradient.end_palette_index) > index {
                    gradient.end_palette_index -= 1;
                } else if usize::from(gradient.end_palette_index) == index {
                    gradient.end_palette_index = 0;
                }
            }
            layers.push(layer);
            new_transforms.push(old_transforms.get(old_index).copied().flatten());
        }
        project.color_layer_transforms.insert(base, new_transforms);
    }
}

#[path = "properties_guides.rs"]
mod properties_guides;
#[allow(clippy::too_many_arguments)]
#[path = "properties_master.rs"]
mod properties_master;

#[path = "properties_background.rs"]
mod properties_background;
#[path = "properties_color.rs"]
mod properties_color;
#[path = "properties_kerning.rs"]
mod properties_kerning;
#[path = "properties_variable.rs"]
mod properties_variable;

#[path = "properties_opentype.rs"]
mod properties_opentype;

#[path = "properties_alternate.rs"]
mod properties_alternate;

pub fn show_properties(
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
    let mut align_component_request = None;
    let mut align_all_components = false;
    ui.horizontal(|ui| {
        ui.label("検索");
        ui.add(
            egui::TextEdit::singleline(properties_filter)
                .hint_text("OpenType / 可変軸 / カーニング / カラー / 背景 / Alternate")
                .desired_width((ui.available_width() - 42.0).max(120.0)),
        );
        if !properties_filter.is_empty() && ui.small_button("クリア").clicked() {
            properties_filter.clear();
        }
    });
    let filter = properties_filter.trim().to_lowercase();
    let show_section = |keywords: &[&str]| {
        filter.is_empty()
            || keywords
                .iter()
                .any(|keyword| filter.contains(&keyword.to_lowercase()))
    };
    ui.separator();
    properties_font::show(ui, &filter, project);

    ui.separator();
    properties_master::show_properties_master(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );

    ui.separator();
    properties_guides::show_properties_guides(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );

    ui.separator();
    properties_opentype::show_properties_opentype(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );
    ui.separator();

    properties_variable::show_properties_variable(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );
    ui.separator();

    properties_background::show_properties_background(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );
    ui.separator();

    properties_alternate::show_properties_alternate(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
        &mut align_component_request,
        &mut align_all_components,
    );

    if let (Some(name), Some(index)) = (current_glyph.as_deref(), align_component_request) {
        let _ = project.align_component_anchors(name, index);
    }
    if align_all_components {
        if let Some(name) = current_glyph.as_deref() {
            let count = project
                .glyphs
                .get(name)
                .map(|glyph| glyph.components.len())
                .unwrap_or(0);
            for index in 0..count {
                let _ = project.align_component_anchors(name, index);
            }
        }
    }

    properties_kerning::show_properties_kerning(
        ui,
        properties_filter,
        project,
        current_glyph,
        component_base,
        kerning_right,
        kerning_pair_filter,
        preview_text,
        show_preview,
        feature_left,
        feature_right,
        feature_replacement,
        feature_kerning_value,
        feature_target_tag,
        feature_anchor_x,
        feature_anchor_y,
        unicode_alias_input,
        unicode_variation_selector,
        current_master_id,
        master_map_drag,
        color_layer_glyph,
        preview_color_palette,
        conditional_layer_axis,
        conditional_layer_min,
        conditional_layer_max,
        conditional_layer_axis_2,
        conditional_layer_min_2,
        conditional_layer_max_2,
        conditional_layer_axis_3,
        conditional_layer_min_3,
        conditional_layer_max_3,
        conditional_layer_axis_4,
        conditional_layer_min_4,
        conditional_layer_max_4,
        conditional_layer_extra,
    );

    ui.separator();
    properties_color::show_properties_color(
        ui,
        properties_filter,
        project,
        current_glyph,
        color_layer_glyph,
        preview_color_palette,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("properties_tests/000_mark_class_input_is_normalized_once.rs");
    include!(
        "properties_tests/001_feature_file_source_separates_class_declarations_from_features.rs"
    );
    include!("properties_tests/002_mark_class_can_be_generated_from_a_mark_anchor.rs");
    include!(
        "properties_tests/003_removing_palette_color_updates_layer_and_gradient_references.rs"
    );
    include!("properties_tests/004_operation_is_inserted_inside_feature_block.rs");
    include!("properties_tests/005_operation_is_appended_when_no_block_exists.rs");
    include!("properties_tests/006_tag_matching_does_not_use_a_similar_feature_name.rs");
    include!("properties_tests/007_operation_is_inserted_after_nested_lookup_block.rs");
}
