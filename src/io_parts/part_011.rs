fn plist_string(value: &plist::Value) -> Option<String> {
    value.as_string().map(str::to_string)
}
