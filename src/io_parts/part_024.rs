
fn feature_glyph_name(names: &[String], glyph_id: ttf_parser::GlyphId) -> Option<&str> {
    let name = names.get(glyph_id.0 as usize)?.as_str();
    (!name.is_empty()
        && !name
            .chars()
            .any(|character| character.is_whitespace() || "[]{}();,'\"".contains(character)))
    .then_some(name)
}

macro_rules! imported_lookup_flag_source {
    ($lookup:expr) => {{
        let lookup = $lookup;
        let flags = lookup.flags;
        let mut values = Vec::new();
        if flags.right_to_left() {
            values.push("RightToLeft".to_string());
        }
        if flags.ignore_base_glyphs() {
            values.push("IgnoreBaseGlyphs".to_string());
        }
        if flags.ignore_ligatures() {
            values.push("IgnoreLigatures".to_string());
        }
        if flags.ignore_marks() {
            values.push("IgnoreMarks".to_string());
        }
        let mark_attachment_type = flags.mark_attachment_type();
        if mark_attachment_type != 0 {
            values.push(format!("MarkAttachmentType {mark_attachment_type}"));
        }
        let mark_filtering_set = lookup
            .mark_filtering_set
            .map(|index| format!(" UseMarkFilteringSet @GSMarkSet{index}"))
            .unwrap_or_default();
        (!values.is_empty() || !mark_filtering_set.is_empty())
            .then(|| format!("lookupflag {}{};", values.join(" "), mark_filtering_set))
    }};
}

macro_rules! collect_imported_languagesystems {
    ($table:expr, $systems:expr) => {
        for script in $table.scripts {
            if let Some(language) = script.default_language {
                $systems.insert(format!("languagesystem {} {};", script.tag, language.tag));
            }
            for language in script.languages {
                $systems.insert(format!("languagesystem {} {};", script.tag, language.tag));
            }
        }
    };
}

macro_rules! imported_coverage_class {
    ($coverage:expr, $names:expr, $definitions:expr, $serial:expr, $prefix:expr) => {{
        let mut glyphs = Vec::new();
        for raw_id in 0..$names.len() {
            let glyph_id = ttf_parser::GlyphId(raw_id as u16);
            if $coverage.get(glyph_id).is_some() {
                if let Some(name) = feature_glyph_name($names, glyph_id) {
                    glyphs.push(name.to_string());
                }
            }
        }
        if glyphs.is_empty() {
            None
        } else {
            $serial += 1;
            let class_name = format!("@{}{}", $prefix, $serial);
            $definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
            Some(class_name)
        }
    }};
}
