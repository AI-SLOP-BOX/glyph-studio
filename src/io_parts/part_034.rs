
fn add_imported_anchor(project: &mut FontProject, glyph_name: &str, name: String, x: i16, y: i16) {
    let Some(glyph) = project.glyphs.get_mut(glyph_name) else {
        return;
    };
    if glyph
        .anchors
        .iter()
        .any(|anchor| anchor.name == name && anchor.x == f64::from(x) && anchor.y == f64::from(y))
    {
        return;
    }
    glyph.anchors.push(crate::font_data::GlyphAnchor {
        name,
        x: f64::from(x),
        y: f64::from(y),
    });
}
