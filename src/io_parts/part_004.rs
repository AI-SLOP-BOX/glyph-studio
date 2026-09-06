
impl OutlineCollector {
    fn new() -> Self {
        Self {
            contours: Vec::new(),
            current: crate::font_data::Contour::new(),
        }
    }
}
