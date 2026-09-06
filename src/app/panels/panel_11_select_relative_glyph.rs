use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn select_relative_glyph(&mut self, delta: isize) {
        let names = self.project.glyph_names_sorted();
        if names.is_empty() {
            return;
        }
        let current = self
            .current_glyph
            .as_deref()
            .and_then(|name| names.iter().position(|candidate| *candidate == name))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(names.len() as isize) as usize;
        let next_name = names[next].to_string();
        self.current_glyph = Some(next_name.clone());
        self.glyph_rename_input = next_name.clone();
        self.clear_geometry_selection();
        self.selected_glyphs.clear();
        self.status_message = format!("グリフ: {next_name}");
    }
}
