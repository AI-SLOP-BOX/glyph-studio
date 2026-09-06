use super::*;

impl GlyphStudioApp {
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_keyboard_input = ctx.wants_keyboard_input();
        let dropped_paths = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if let Some(path) = dropped_paths.first() {
            self.request_open_document_path(path);
        }
        if !wants_keyboard_input
            && ctx.input(|input| input.modifiers.command && input.key_pressed(Key::F))
        {
            self.show_glyph_list = true;
            self.focus_glyph_search = true;
        }
        if !wants_keyboard_input && ctx.input(|input| input.key_pressed(Key::Slash)) {
            self.show_glyph_list = true;
            self.focus_glyph_search = true;
        }
        let export_ufo_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.modifiers.shift
                && input.key_pressed(Key::S)
        });
        if export_ufo_shortcut {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("UFO", &["ufo"])
                .save_file()
            {
                match crate::core::save_ufo(&self.project, &path) {
                    Ok(()) => {
                        self.status_message = format!("UFOを保存しました: {}", path.display());
                    }
                    Err(error) => self.status_message = error,
                }
            }
        }
        let save_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::S)
                && !input.modifiers.shift
        });
        if save_shortcut {
            self.save_project_file();
        }
        let export_ttf_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::E)
                && !input.modifiers.shift
        });
        if export_ttf_shortcut {
            self.export_ttf_file();
        }
        let new_project_shortcut = ctx.input(|input| {
            !wants_keyboard_input && input.modifiers.command && input.key_pressed(Key::N)
        });
        if new_project_shortcut {
            self.request_new_project();
        }
        let open_project_shortcut = ctx.input(|input| {
            !wants_keyboard_input && input.modifiers.command && input.key_pressed(Key::O)
        });
        if open_project_shortcut {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Glyph Studio / Glyphs / UFO / Font",
                    &["json", "glyphs", "ufo", "ttf", "otf", "woff", "woff2"],
                )
                .pick_file()
            {
                self.request_open_document_path(&path);
            }
        }
        let duplicate_glyph_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && !input.modifiers.shift
                && input.key_pressed(Key::D)
        });
        let duplicate_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.modifiers.shift
                && input.key_pressed(Key::D)
                && self.current_glyph.is_some()
                && self.canvas.selected_component.is_some()
        });
        if duplicate_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                if self.project.duplicate_component_all_layers(&name, index) {
                    self.canvas.selected_component = self
                        .project
                        .glyphs
                        .get(&name)
                        .map(|glyph| glyph.components.len().saturating_sub(1));
                    self.save_state();
                    self.status_message = "コンポーネントを複製しました (⌘⇧D)".to_string();
                }
            }
        }
        if duplicate_glyph_shortcut {
            let count = self.duplicate_selected_glyphs();
            if count > 0 {
                self.status_message = format!("{}個のグリフを複製しました", count);
            }
        }
        let copy_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::C)
                && self.canvas.selected_component.is_some()
        });
        if copy_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                self.component_clipboard = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.components.get(index).cloned());
            }
        }
        let cut_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::X)
                && self.canvas.selected_component.is_some()
        });
        if cut_component_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_component)
            {
                let component = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.components.get(index).cloned());
                if let Some(component) = component {
                    if self
                        .project
                        .remove_component_all_layers(&name, index)
                        .is_ok()
                    {
                        self.component_clipboard = Some(component);
                        self.clear_geometry_selection();
                        self.save_state();
                    }
                }
            }
        }
        let cut_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::X)
                && self.canvas.selected_component.is_none()
                && self.canvas.selected_contour.is_some()
        });
        if cut_contour_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_contour)
            {
                let contour = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.contours.get(index).cloned());
                if let Some(contour) = contour {
                    if self.project.remove_contour_all_layers(&name, index).is_ok() {
                        self.contour_clipboard = Some(contour);
                        self.clear_geometry_selection();
                        self.save_state();
                    }
                }
            }
        }
        let copy_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::C)
                && self.canvas.selected_component.is_none()
                && self.canvas.selected_contour.is_some()
        });
        if copy_contour_shortcut {
            if let (Some(name), Some(index)) =
                (self.current_glyph.clone(), self.canvas.selected_contour)
            {
                self.contour_clipboard = self
                    .project
                    .glyphs
                    .get(&name)
                    .and_then(|glyph| glyph.contours.get(index).cloned());
            }
        }
        let paste_component_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::V)
                && self.component_clipboard.is_some()
                && self.current_glyph.is_some()
        });
        if paste_component_shortcut {
            if let (Some(name), Some(component)) =
                (self.current_glyph.clone(), self.component_clipboard.clone())
            {
                if let Some(index) = self.project.add_component_all_layers(&name, component) {
                    self.canvas.selected_component = Some(index);
                    self.canvas.selected_components = vec![index];
                    self.canvas.selected_points.clear();
                    self.canvas.selected_nodes.clear();
                    self.canvas.selected_contour = None;
                    self.save_state();
                }
            }
        }
        let paste_contour_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && input.key_pressed(Key::V)
                && self.canvas.selected_component.is_none()
                && self.contour_clipboard.is_some()
                && self.current_glyph.is_some()
        });
        if paste_contour_shortcut {
            if let (Some(name), Some(mut contour)) =
                (self.current_glyph.clone(), self.contour_clipboard.clone())
            {
                for point in &mut contour.points {
                    point.x += 50.0;
                    point.y += 50.0;
                }
                if let Some(index) = self.project.add_contour_all_layers(&name, contour) {
                    let point_count = self
                        .project
                        .glyphs
                        .get(&name)
                        .and_then(|glyph| glyph.contours.get(index))
                        .map_or(0, |contour| contour.points.len());
                    self.canvas.selected_contour = Some(index);
                    self.canvas.selected_points = (0..point_count).collect();
                    self.canvas.selected_nodes = self
                        .canvas
                        .selected_points
                        .iter()
                        .map(|&point| (index, point))
                        .collect();
                    self.save_state();
                }
            }
        }
        let delete_selected_glyphs_shortcut = ctx.input(|input| {
            !wants_keyboard_input
                && input.modifiers.command
                && (input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace))
                && self.selected_glyphs.len() > 1
        });
        if delete_selected_glyphs_shortcut {
            let names: Vec<String> = self.selected_glyphs.iter().cloned().collect();
            for name in &names {
                self.project.remove_glyph(name);
            }
            self.current_glyph = self
                .project
                .glyph_names_sorted()
                .first()
                .map(|name| name.to_string());
            self.clear_canvas_selection();
            self.save_state();
            self.status_message = format!("{}個のグリフを削除しました", names.len());
        }
    }
}
