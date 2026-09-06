
#[derive(Debug, Clone)]
struct ConditionalSubstitution {
    base: String,
    alternate: String,
    conditions: std::collections::HashMap<String, crate::font_data::AxisRange>,
}
