
fn wrap_gpos_extension_lookup(lookup: gpos::PositionLookup) -> gpos::PositionLookup {
    macro_rules! wrap_variant {
        ($lookup:expr, $extension_type:expr, $variant:ident) => {{
            let layout::Lookup {
                lookup_flag,
                subtables,
                mark_filtering_set,
            } = $lookup;
            let extension_subtables = subtables
                .iter()
                .map(|subtable| {
                    gpos::ExtensionPosFormat1::new($extension_type, (**subtable).clone())
                })
                .map(gpos::ExtensionSubtable::$variant)
                .collect();
            let mut wrapped = layout::Lookup::new(lookup_flag, extension_subtables);
            wrapped.mark_filtering_set = mark_filtering_set;
            gpos::PositionLookup::Extension(wrapped)
        }};
    }
    match lookup {
        gpos::PositionLookup::Single(lookup) => wrap_variant!(lookup, 1, Single),
        gpos::PositionLookup::Pair(lookup) => wrap_variant!(lookup, 2, Pair),
        gpos::PositionLookup::Cursive(lookup) => wrap_variant!(lookup, 3, Cursive),
        gpos::PositionLookup::MarkToBase(lookup) => wrap_variant!(lookup, 4, MarkToBase),
        gpos::PositionLookup::MarkToLig(lookup) => wrap_variant!(lookup, 5, MarkToLig),
        gpos::PositionLookup::MarkToMark(lookup) => wrap_variant!(lookup, 6, MarkToMark),
        gpos::PositionLookup::Contextual(lookup) => wrap_variant!(lookup, 7, Contextual),
        gpos::PositionLookup::ChainContextual(lookup) => {
            wrap_variant!(lookup, 8, ChainContextual)
        }
        gpos::PositionLookup::Extension(lookup) => gpos::PositionLookup::Extension(lookup),
    }
}
