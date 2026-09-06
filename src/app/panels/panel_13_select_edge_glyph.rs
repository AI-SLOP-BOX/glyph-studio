use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn select_edge_glyph(&mut self, last: bool) {
        let names = self.project.glyph_names_sorted();
        let Some(name) = names.get(if last {
            names.len().saturating_sub(1)
        } else {
            0
        }) else {
            return;
        };
        let edge_name = (*name).to_string();
        self.current_glyph = Some(edge_name.clone());
        self.glyph_rename_input = edge_name.clone();
        self.clear_geometry_selection();
        self.selected_glyphs.clear();
        self.status_message = format!("グリフ: {edge_name}");
    }
}
