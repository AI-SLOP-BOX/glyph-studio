use super::*;

impl FontProject {
    pub(crate) fn normalize_contour_list(contours: &mut [Contour]) -> bool {
        let polygons: Vec<Vec<Point>> = contours
            .iter()
            .map(|contour| {
                let mut polygon = Vec::new();
                flatten(contour.to_bezpath(), 0.25, |element| {
                    if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                        polygon.push(point);
                    }
                });
                polygon
            })
            .collect();
        let contains = |polygon: &[Point], point: Point| {
            if polygon.len() < 3 {
                return false;
            }
            let mut inside = false;
            for (a, b) in polygon
                .iter()
                .zip(polygon.iter().cycle().skip(1))
                .take(polygon.len())
            {
                if (a.y > point.y) != (b.y > point.y)
                    && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
                {
                    inside = !inside;
                }
            }
            inside
        };
        let mut changed = false;
        for index in 0..contours.len() {
            let Some(&probe) = polygons[index].first() else {
                continue;
            };
            let depth = polygons
                .iter()
                .enumerate()
                .filter(|(other, polygon)| *other != index && contains(polygon, probe))
                .count();
            let area = contours[index].signed_area();
            let wants_positive = depth % 2 == 1;
            if (wants_positive && area < 0.0) || (!wants_positive && area > 0.0) {
                contours[index].reverse_direction();
                changed = true;
            }
        }
        changed
    }
}
