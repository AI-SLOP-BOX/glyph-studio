
impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = crate::font_data::Contour::new();
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x1 as f64, y1 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x1 as f64, y1 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::off_curve(
                x2 as f64, y2 as f64,
            ));
        self.current
            .points
            .push(crate::font_data::ContourPoint::on_curve(x as f64, y as f64));
    }

    fn close(&mut self) {
        if !self.current.points.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}
