use super::*;

impl GlyphData {
    pub fn translate_geometry(&mut self, dx: f64, dy: f64) {
        self.translate_authored_geometry(dx, dy);
        for layer in self.layers.values_mut() {
            layer.translate_geometry(dx, dy);
        }
        for guides in self.master_guidelines.values_mut() {
            for guide in guides {
                guide.x += dx;
                guide.y += dy;
            }
        }
    }
}
