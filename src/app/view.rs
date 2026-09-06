use super::*;

impl eframe::App for GlyphStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dirty = self
            .saved_project
            .as_ref()
            .is_none_or(|saved| saved != &self.project);
        let title = self
            .project_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| {
                let marker = if dirty { " *" } else { "" };
                format!("{}{} — Glyph Studio", name.to_string_lossy(), marker)
            })
            .unwrap_or_else(|| {
                format!(
                    "Glyph Studio{} — 未保存のプロジェクト",
                    if dirty { " *" } else { "" }
                )
            });
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        self.handle_keyboard_shortcuts(ctx);
        self.show_menu_bar(ctx);
        self.validation_window(ctx);
        self.kerning_window(ctx);

        self.shortcuts_window(ctx);
        self.show_tool_bar(ctx);
        self.show_status_bar(ctx);

        self.glyph_panel(ctx);

        self.properties_panel(ctx);

        self.preview_panel(ctx);

        self.show_glyph_canvas(ctx);
        self.show_unsaved_open_dialog(ctx);
    }
}
