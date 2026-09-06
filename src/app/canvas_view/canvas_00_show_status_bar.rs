use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_status_bar(&mut self, ctx: &egui::Context) {
        let mut save_requested = false;
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.set_min_height(26.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_message).small());
                if !self.canvas.selected_nodes.is_empty() {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "{}ノード選択",
                            self.canvas.selected_nodes.len()
                        ))
                        .small()
                        .color(Color32::LIGHT_BLUE),
                    );
                }
                if self.edit_all_masters {
                    ui.separator();
                    ui.label(
                        egui::RichText::new("⚠ 全マスター編集 ON")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(245, 183, 77)),
                    )
                    .on_hover_text("ノードとコンポーネントのドラッグが全マスターへ反映されます");
                }
                let dirty = self
                    .saved_project
                    .as_ref()
                    .is_none_or(|saved| saved != &self.project);
                if dirty
                    && ui
                        .small_button("保存")
                        .on_hover_text("現在のプロジェクトを保存（⌘S）")
                        .clicked()
                {
                    save_requested = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("⌘Z Undo  ·  ⌘⇧Z Redo")
                            .small()
                            .color(Color32::GRAY),
                    );
                    ui.label(format!("ズーム: {:.0}%", self.canvas.zoom * 100.0));
                    let master_name = self
                        .project
                        .masters
                        .iter()
                        .find(|master| master.id == self.current_master_id)
                        .map(|master| master.name.as_str())
                        .unwrap_or(self.current_master_id.as_str());
                    ui.label(format!("マスター: {master_name}"));
                    if let Some(name) = &self.current_glyph {
                        ui.label(format!("グリフ: {}", name));
                    }
                });
            });
        });
        if save_requested {
            self.save_project_file();
        }
    }
}
