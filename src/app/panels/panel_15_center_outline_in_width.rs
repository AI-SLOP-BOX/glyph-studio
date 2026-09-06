use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn center_outline_in_width(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        if self.project.center_glyphs_in_width(&[name]) > 0 {
            self.save_state();
            self.status_message = "アウトラインを字幅の中央へ配置しました".to_string();
        }
    }
}
