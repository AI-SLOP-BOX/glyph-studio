
fn contour_self_intersects(contour: &crate::font_data::Contour) -> bool {
    if contour.points.len() < 4 {
        return false;
    }
    let mut vertices = Vec::new();
    kurbo::flatten(contour.to_bezpath(), 0.5, |element| {
        if let kurbo::PathEl::MoveTo(point) | kurbo::PathEl::LineTo(point) = element {
            vertices.push(point);
        }
    });
    if vertices.len() < 4 {
        return false;
    }
    let intersects = |a: kurbo::Point, b: kurbo::Point, c: kurbo::Point, d: kurbo::Point| {
        let cross = |p: kurbo::Point, q: kurbo::Point, r: kurbo::Point| {
            (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
        };
        let ab_c = cross(a, b, c);
        let ab_d = cross(a, b, d);
        let cd_a = cross(c, d, a);
        let cd_b = cross(c, d, b);
        let eps = 1e-7;
        ((ab_c > eps && ab_d < -eps) || (ab_c < -eps && ab_d > eps))
            && ((cd_a > eps && cd_b < -eps) || (cd_a < -eps && cd_b > eps))
    };
    let segment_count = vertices.len();
    for first in 0..segment_count {
        let first_end = (first + 1) % segment_count;
        for second in (first + 1)..segment_count {
            let second_end = (second + 1) % segment_count;
            if first == second
                || first_end == second
                || second_end == first
                || (first == 0 && second_end == 0)
            {
                continue;
            }
            if intersects(
                vertices[first],
                vertices[first_end],
                vertices[second],
                vertices[second_end],
            ) {
                return true;
            }
        }
    }
    false
}
