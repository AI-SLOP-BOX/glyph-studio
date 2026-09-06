use super::*;

impl FontProject {
    /// Copies one master layer to every other master for only the supplied glyphs.
    pub fn copy_master_to_all_for_glyphs<'a, I>(
        &mut self,
        source_master_id: &str,
        glyph_names: I,
    ) -> usize
    where
        I: IntoIterator<Item = &'a str>,
    {
        let target_ids: Vec<String> = self
            .masters
            .iter()
            .filter(|master| master.id != source_master_id)
            .map(|master| master.id.clone())
            .collect();
        let source_is_default = source_master_id == self.default_master_id;
        let mut copied = 0;
        for name in glyph_names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let Some(source) = glyph
                .layers
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.layer_snapshot()))
            else {
                continue;
            };
            let source_guidelines = glyph
                .master_guidelines
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.guidelines.clone()))
                .unwrap_or_default();
            for target_id in &target_ids {
                glyph.layers.insert(target_id.clone(), source.clone());
                glyph
                    .master_guidelines
                    .insert(target_id.clone(), source_guidelines.clone());
                copied += 1;
            }
        }
        copied
    }
}
