
fn plist_array(value: &plist::Value) -> Option<&Vec<plist::Value>> {
    value.as_array()
}
