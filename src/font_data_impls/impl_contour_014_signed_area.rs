use super::*;

impl Contour {
    pub fn signed_area(&self) -> f64 {
        let mut outline = Vec::new();
        flatten(self.to_bezpath(), 0.25, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                outline.push(point);
            }
        });
        outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .take(outline.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f64>()
            * 0.5
    }
}
