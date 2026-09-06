
fn push_imported_color_layer(
    names: &[String],
    leaf_glyph: Option<u16>,
    transform: Option<crate::font_data::ColorLayerTransform>,
    palette_index: u16,
    gradient: Option<crate::font_data::ColorGradient>,
    output: &mut Vec<crate::font_data::ColorLayer>,
    transforms: &mut Vec<Option<crate::font_data::ColorLayerTransform>>,
) {
    let Some(glyph) = leaf_glyph.and_then(|id| names.get(usize::from(id))) else {
        return;
    };
    output.push(crate::font_data::ColorLayer {
        glyph: glyph.clone(),
        palette_index,
        gradient,
        alpha: 1.0,
    });
    transforms.push(transform);
}
