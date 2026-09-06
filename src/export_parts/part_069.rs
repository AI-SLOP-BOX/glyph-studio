
fn apply_lookup_mark_set<T>(
    mut lookup: layout::Lookup<T>,
    tag: Tag,
    lookup_mark_sets: &BTreeMap<Tag, String>,
    mark_sets: &BTreeMap<String, (u16, layout::CoverageTable)>,
) -> layout::Lookup<T> {
    if let Some(name) = lookup_mark_sets.get(&tag) {
        if let Some((index, _)) = mark_sets.get(name) {
            lookup.lookup_flag |= layout::LookupFlag::USE_MARK_FILTERING_SET;
            lookup.mark_filtering_set = Some(*index);
        }
    }
    lookup
}
