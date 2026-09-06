
/// Builds the OpenType SVG table from the same outline/component model used
/// by standalone SVG export. A separate document per glyph keeps the table
/// simple and allows color-layer glyphs to carry their palette colors.
fn build_svg_table(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    let mut documents = Vec::<(u16, Vec<u8>)>::new();
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        let has_outline = project
            .glyphs
            .get(name)
            .is_some_and(|glyph| !glyph.contours.is_empty() || !glyph.components.is_empty());
        if !has_outline && !project.color_layers.contains_key(name) {
            continue;
        }
        let document = build_svg_document(project, name)?;
        if document.len() > u32::MAX as usize {
            return None;
        }
        documents.push((glyph_id, document.into_bytes()));
    }
    documents.sort_by_key(|(glyph_id, _)| *glyph_id);
    if documents.is_empty() || documents.len() > u16::MAX as usize {
        return None;
    }
    let list_offset = 10usize;
    let entries_offset = list_offset + 2;
    let documents_offset = entries_offset + documents.len() * 12;
    let total_documents = documents.iter().try_fold(0usize, |total, (_, document)| {
        total.checked_add(document.len())
    })?;
    let total = documents_offset.checked_add(total_documents)?;
    let mut table = Vec::with_capacity(total);
    put_u16(&mut table, 0); // version
    put_u32(&mut table, u32::try_from(list_offset).ok()?);
    put_u32(&mut table, 0); // reserved
    put_u16(&mut table, u16::try_from(documents.len()).ok()?);
    let mut document_offset = documents_offset - list_offset;
    for (glyph_id, document) in &documents {
        put_u16(&mut table, *glyph_id);
        put_u16(&mut table, *glyph_id);
        put_u32(&mut table, u32::try_from(document_offset).ok()?);
        put_u32(&mut table, u32::try_from(document.len()).ok()?);
        document_offset = document_offset.checked_add(document.len())?;
    }
    for (_, document) in documents {
        table.extend_from_slice(&document);
    }
    Some(table)
}
