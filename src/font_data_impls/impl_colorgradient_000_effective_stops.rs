use super::*;

impl ColorGradient {
    pub fn effective_stops(&self) -> Vec<ColorGradientStop> {
        if self.stops.is_empty() {
            vec![
                ColorGradientStop {
                    offset: 0.0,
                    palette_index: self.start_palette_index,
                    alpha: 1.0,
                },
                ColorGradientStop {
                    offset: 1.0,
                    palette_index: self.end_palette_index,
                    alpha: 1.0,
                },
            ]
        } else {
            self.stops.clone()
        }
    }
}
