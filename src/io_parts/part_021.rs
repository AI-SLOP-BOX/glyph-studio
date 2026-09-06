
/// Captures the raw payload of SFNT tables that the project model does not
/// regenerate. This lets newer or specialised OpenType/AAT tables survive an
/// import/export cycle without duplicating core outline/layout tables.
fn preserved_sfnt_tables(bytes: &[u8]) -> std::collections::HashMap<String, Vec<u8>> {
    let Ok(font) = FontRef::new(bytes) else {
        return std::collections::HashMap::new();
    };
    font.table_directory
        .table_records()
        .iter()
        .filter_map(|record| {
            let tag = record.tag().into_bytes();
            let tag = String::from_utf8(tag.to_vec()).ok()?;
            // These are regenerated from the project model during export.
            // In particular, retaining CFF alongside generated glyf/loca or
            // retaining stale outline/variation data would create an invalid
            // or misleading output font. Layout tables are retained as a
            // fallback when the source cannot be reconstructed; generated
            // GSUB/GPOS/GDEF always replace them when available.
            if matches!(
                tag.as_str(),
                "CFF "
                    | "CFF2"
                    | "glyf"
                    | "loca"
                    | "fvar"
                    | "gvar"
                    | "avar"
                    | "HVAR"
                    | "VVAR"
                    | "MVAR"
                    | "cmap"
                    | "head"
                    | "hhea"
                    | "hmtx"
                    | "maxp"
                    | "name"
                    | "OS/2"
                    | "post"
                    | "vhea"
                    | "vmtx"
                    | "gasp"
                    | "kern"
            ) {
                return None;
            }
            let data = font.table_data(record.tag())?.as_bytes().to_vec();
            Some((tag, data))
        })
        .collect()
}
