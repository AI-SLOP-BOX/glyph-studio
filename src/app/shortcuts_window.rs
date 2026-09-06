use super::*;

impl GlyphStudioApp {
    pub(crate) fn shortcuts_window(&mut self, ctx: &egui::Context) {
        if self.show_shortcuts {
            egui::Window::new("ショートカット")
                .open(&mut self.show_shortcuts)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("shortcut_grid")
                        .num_columns(2)
                        .spacing(Vec2::new(18.0, 6.0))
                        .show(ui, |ui| {
                            for (key, action) in [
                                ("V", "選択ツール"),
                                ("P", "ペンツール"),
                                ("K", "ナイフツール"),
                                ("H", "ハンドツール"),
                                ("R", "定規ツール"),
                                ("I", "背景画像表示"),
                                ("B", "前後字形表示"),
                                ("D", "輪郭方向表示"),
                                ("M", "メトリクス表示"),
                                ("N", "ノード番号表示"),
                                ("S / C / T", "スムーズ / コーナー / オン・オフ曲線"),
                                ("⌘Z", "取り消す"),
                                ("⌘⇧Z", "やり直す"),
                                ("⌘S", "プロジェクト保存"),
                                ("⌘E", "検証してTTFを書き出し"),
                                ("⌘C / ⌘V", "輪郭・部品コピー／貼り付け"),
                                ("⌘⇧D", "選択中コンポーネントを全マスターへ複製"),
                                ("/ / ⌘F", "グリフ検索へフォーカス"),
                                ("Tab / PageUp / PageDown", "前後のグリフへ移動"),
                                ("⌘↑ / ⌘↓", "前後のマスターへ移動"),
                                ("⌘⇧M", "全マスター編集の切り替え"),
                                ("Shift + ドラッグ", "移動軸を水平／垂直に固定"),
                                ("Option + ドラッグ", "選択部品を複製して移動"),
                                ("Command + 回転", "部品を15度刻みで回転"),
                                ("中ボタン + ドラッグ", "ツールを切り替えずにパン"),
                                ("右クリック", "キャンバス操作メニュー"),
                                ("選択 + ドラッグ", "字幅・LSB・RSBをキャンバス上で調整"),
                            ] {
                                ui.label(egui::RichText::new(key).monospace().strong());
                                ui.label(action);
                                ui.end_row();
                            }
                        });
                });
        }
    }
}
