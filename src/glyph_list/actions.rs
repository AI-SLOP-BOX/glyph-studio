use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlyphAction {
    Add(String),
    Duplicate(String, String),
    DuplicateMany(Vec<String>),
    Delete(String),
    DeleteMany(Vec<String>),
    Move(String, isize),
    Rename(String, String),
    MetricsKeysApplied(usize),
}

pub fn show_glyph_actions(
    ui: &mut Ui,
    project: &mut FontProject,
    current_glyph: &Option<String>,
    rename_input: &mut String,
    selected_glyphs: &mut HashSet<String>,
) -> Option<GlyphAction> {
    let mut action = None;
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("+ 新規グリフ").clicked() {
            let mut index = project.glyphs.len();
            while project.glyphs.contains_key(&format!("glyph_{index}")) {
                index += 1;
            }
            let name = format!("glyph_{index}");
            project.add_glyph(name.clone(), None);
            action = Some(GlyphAction::Add(name));
        }
        if ui.button("複製").clicked() {
            if selected_glyphs.len() > 1 {
                let source_names: Vec<String> = project
                    .glyph_names_sorted()
                    .into_iter()
                    .filter(|name| selected_glyphs.iter().any(|selected| selected == *name))
                    .map(str::to_string)
                    .collect();
                let mut duplicated = Vec::new();
                for source_name in source_names {
                    if let Some(name) = project.duplicate_glyph(&source_name) {
                        duplicated.push(name);
                    }
                }
                if !duplicated.is_empty() {
                    action = Some(GlyphAction::DuplicateMany(duplicated));
                }
            } else if let Some(source_name) = current_glyph {
                if let Some(source) = project.glyphs.get(source_name).cloned() {
                    let mut index = project.glyphs.len();
                    let name = loop {
                        let candidate = format!("{}_copy{index}", source_name);
                        if !project.glyphs.contains_key(&candidate) {
                            break candidate;
                        }
                        index += 1;
                    };
                    let mut duplicate = source;
                    duplicate.name = name.clone();
                    duplicate.unicode = None;
                    duplicate.unicodes.clear();
                    project.glyphs.insert(name.clone(), duplicate);
                    project.glyph_order.push(name.clone());
                    action = Some(GlyphAction::Duplicate(source_name.clone(), name));
                }
            }
        }
        if ui.button("🗑 削除").clicked() {
            if selected_glyphs.len() > 1 {
                let mut names: Vec<String> = selected_glyphs.iter().cloned().collect();
                names.sort();
                for name in &names {
                    project.remove_glyph(name);
                }
                action = Some(GlyphAction::DeleteMany(names));
            } else if let Some(name) = current_glyph {
                project.remove_glyph(name);
                action = Some(GlyphAction::Delete(name.clone()));
            }
        }
        if let Some(name) = current_glyph {
            if ui.small_button("↑").clicked() {
                project.move_glyph(name, -1);
                action = Some(GlyphAction::Move(name.clone(), -1));
            }
            if ui.small_button("↓").clicked() {
                project.move_glyph(name, 1);
                action = Some(GlyphAction::Move(name.clone(), 1));
            }
        }
        let metric_targets: Vec<String> = if selected_glyphs.is_empty() {
            current_glyph.iter().cloned().collect()
        } else {
            selected_glyphs.iter().cloned().collect()
        };
        if !metric_targets.is_empty()
            && ui
                .small_button("↔ キー適用")
                .on_hover_text("選択中のグリフへメトリクスキーを全マスター適用")
                .clicked()
        {
            match project.apply_metrics_keys(&metric_targets) {
                Ok(count) => action = Some(GlyphAction::MetricsKeysApplied(count)),
                Err(error) => {
                    ui.colored_label(Color32::from_rgb(230, 130, 100), error);
                }
            }
        }
    });
    if let Some(name) = current_glyph {
        ui.horizontal(|ui| {
            ui.label("名前:");
            if rename_input.is_empty() {
                rename_input.push_str(name);
            }
            ui.text_edit_singleline(rename_input);
            if ui.button("変更").clicked() && project.rename_glyph(name, rename_input.clone()) {
                action = Some(GlyphAction::Rename(name.clone(), rename_input.clone()));
            }
        });
    }
    action
}
