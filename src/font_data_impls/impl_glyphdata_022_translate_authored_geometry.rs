use super::*;

impl GlyphData {
    pub(crate) fn translate_authored_geometry(&mut self, dx: f64, dy: f64) {
        for contour in &mut self.contours {
            for point in &mut contour.points {
                point.x += dx;
                point.y += dy;
            }
        }
        for component in &mut self.components {
            component.x_offset += dx;
            component.y_offset += dy;
        }
        for anchor in &mut self.anchors {
            anchor.x += dx;
            anchor.y += dy;
        }
        for guide in &mut self.guidelines {
            guide.x += dx;
            guide.y += dy;
        }
    }
}
