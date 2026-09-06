use super::*;

impl FontProject {
    pub fn move_glyph(&mut self, name: &str, delta: isize) {
        let Some(index) = self.glyph_order.iter().position(|item| item == name) else {
            return;
        };
        let target = index as isize + delta;
        if target >= 0 && target < self.glyph_order.len() as isize {
            let item = self.glyph_order.remove(index);
            self.glyph_order.insert(target as usize, item);
        }
    }
}
