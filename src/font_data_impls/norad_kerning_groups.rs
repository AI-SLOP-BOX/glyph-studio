use super::*;

impl FontProject {
    fn norad_kerning_groups(&self, font: &mut norad::Font) -> Result<(), String> {
            let mut left_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
            let mut right_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
            for glyph in self.glyphs.values() {
                if !glyph.left_kerning_group.trim().is_empty() {
                    left_groups
                        .entry(format!("public.kern1.{}", glyph.left_kerning_group.trim()))
                        .or_default()
                        .push(glyph.name.clone());
                }
                if !glyph.right_kerning_group.trim().is_empty() {
                    right_groups
                        .entry(format!("public.kern2.{}", glyph.right_kerning_group.trim()))
                        .or_default()
                        .push(glyph.name.clone());
                }
            }
            for (group, members) in left_groups.into_iter().chain(right_groups) {
                let group_name = norad::Name::new(&group)
                    .map_err(|error| format!("カーニンググループ名が不正です: {error}"))?;
                let member_names = members
                    .into_iter()
                    .map(|member| {
                        norad::Name::new(&member)
                            .map_err(|error| format!("グループ所属グリフ名が不正です: {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                font.groups.insert(group_name, member_names);
            }
        Ok(())
    }
}
