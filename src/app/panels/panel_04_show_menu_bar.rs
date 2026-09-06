use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.show_file_menu(ui);

                ui.menu_button("編集", |ui| {
                    if ui
                        .add(egui::Button::new("取り消す").shortcut_text("⌘Z"))
                        .clicked()
                    {
                        self.undo();
                        ui.close_menu();
                    }
                    if ui
                        .add(egui::Button::new("やり直す").shortcut_text("⌘⇧Z"))
                        .clicked()
                    {
                        self.redo();
                        ui.close_menu();
                    }
                });

                ui.menu_button("表示", |ui| {
                    ui.checkbox(&mut self.show_glyph_list, "グリフ一覧");
                    ui.checkbox(&mut self.show_properties, "プロパティ");
                    ui.checkbox(&mut self.show_preview, "プレビュー");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("レイアウト")
                            .small()
                            .color(Color32::GRAY),
                    );
                    if ui
                        .button("標準")
                        .on_hover_text("一覧・キャンバス・プロパティ・プレビュー")
                        .clicked()
                    {
                        self.show_glyph_list = true;
                        self.show_properties = true;
                        self.show_preview = true;
                        ui.close_menu();
                    }
                    if ui
                        .button("編集集中")
                        .on_hover_text("キャンバスを広く使う")
                        .clicked()
                    {
                        self.show_glyph_list = true;
                        self.show_properties = false;
                        self.show_preview = false;
                        ui.close_menu();
                    }
                    if ui
                        .button("プレビュー重視")
                        .on_hover_text("プロパティを隠して組み確認")
                        .clicked()
                    {
                        self.show_glyph_list = false;
                        self.show_properties = false;
                        self.show_preview = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.canvas.show_grid, "グリッド");
                    ui.checkbox(&mut self.canvas.snap_to_grid, "グリッドにスナップ");
                    ui.checkbox(&mut self.canvas.snap_to_guidelines, "ガイドにスナップ");
                    ui.checkbox(&mut self.canvas.snap_to_anchors, "アンカーにスナップ");
                    ui.horizontal(|ui| {
                        ui.label("間隔:");
                        ui.add(
                            egui::DragValue::new(&mut self.canvas.grid_size)
                                .speed(1.0)
                                .range(1.0..=1000.0),
                        );
                    });
                    ui.checkbox(&mut self.canvas.show_metrics, "メトリクス");
                    ui.checkbox(&mut self.canvas.show_guidelines, "ガイド (G)");
                    ui.checkbox(&mut self.canvas.show_background_images, "背景画像");
                    ui.checkbox(&mut self.canvas.show_contour_direction, "輪郭方向");
                    ui.checkbox(&mut self.canvas.show_node_indices, "ノード番号");
                    ui.checkbox(&mut self.canvas.show_anchors, "アンカー");
                });
            });
        });
    }
}
