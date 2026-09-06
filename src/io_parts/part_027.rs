
fn imported_read_lookup_flag_source(
    flag: read_fonts::tables::layout::LookupFlag,
) -> Option<String> {
    let mut values = Vec::new();
    if flag.contains(read_fonts::tables::layout::LookupFlag::RIGHT_TO_LEFT) {
        values.push("RightToLeft".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_BASE_GLYPHS) {
        values.push("IgnoreBaseGlyphs".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_LIGATURES) {
        values.push("IgnoreLigatures".to_string());
    }
    if flag.contains(read_fonts::tables::layout::LookupFlag::IGNORE_MARKS) {
        values.push("IgnoreMarks".to_string());
    }
    if let Some(class) = flag.mark_attachment_class() {
        values.push(format!("MarkAttachmentType {class}"));
    }
    (!values.is_empty()).then(|| format!("lookupflag {};", values.join(" ")))
}
