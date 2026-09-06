
fn wrap_gsub_extension_lookup(lookup: gsub::SubstitutionLookup) -> gsub::SubstitutionLookup {
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
                    gsub::ExtensionSubstFormat1::new($extension_type, (**subtable).clone())
                })
                .map(gsub::ExtensionSubtable::$variant)
                .collect();
            let mut wrapped = layout::Lookup::new(lookup_flag, extension_subtables);
            wrapped.mark_filtering_set = mark_filtering_set;
            gsub::SubstitutionLookup::Extension(wrapped)
        }};
    }
    match lookup {
        gsub::SubstitutionLookup::Single(lookup) => wrap_variant!(lookup, 1, Single),
        gsub::SubstitutionLookup::Multiple(lookup) => wrap_variant!(lookup, 2, Multiple),
        gsub::SubstitutionLookup::Alternate(lookup) => wrap_variant!(lookup, 3, Alternate),
        gsub::SubstitutionLookup::Ligature(lookup) => wrap_variant!(lookup, 4, Ligature),
        gsub::SubstitutionLookup::Contextual(lookup) => wrap_variant!(lookup, 5, Contextual),
        gsub::SubstitutionLookup::ChainContextual(lookup) => {
            wrap_variant!(lookup, 6, ChainContextual)
        }
        gsub::SubstitutionLookup::Reverse(lookup) => wrap_variant!(lookup, 8, Reverse),
        gsub::SubstitutionLookup::Extension(lookup) => gsub::SubstitutionLookup::Extension(lookup),
    }
}
