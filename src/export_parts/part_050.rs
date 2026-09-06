
fn validate_contour_topology(
    contour: &crate::font_data::Contour,
    label: &str,
    issues: &mut Vec<String>,
) {
    for (index, pair) in contour
        .points
        .iter()
        .zip(contour.points.iter().cycle().skip(1))
        .take(contour.points.len())
        .enumerate()
    {
        if (pair.0.x - pair.1.x).abs() < 1e-9 && (pair.0.y - pair.1.y).abs() < 1e-9 {
            issues.push(format!(
                "{label}に重複した隣接点があります（{}番）",
                index + 1
            ));
            break;
        }
    }
    let on_curve_count = contour
        .points
        .iter()
        .filter(|point| point.is_on_curve())
        .count();
    if !contour.points.is_empty() && on_curve_count < 2 {
        issues.push(format!("{label}にオンカーブ点が2つ未満です"));
    }
    if contour.points.len() >= 2 {
        let mut consecutive_off = 0;
        for point in contour.points.iter().chain(contour.points.first()) {
            if point.is_on_curve() {
                consecutive_off = 0;
            } else {
                consecutive_off += 1;
                if consecutive_off > 2 {
                    issues.push(format!("{label}にオフカーブ点が3つ以上連続しています"));
                    break;
                }
            }
        }
    }
    if contour_self_intersects(contour) {
        issues.push(format!("{label}が自己交差しています"));
    }
    let on_curve: Vec<_> = contour
        .points
        .iter()
        .filter(|point| point.is_on_curve())
        .collect();
    if on_curve.len() >= 3 {
        let area = on_curve
            .iter()
            .zip(on_curve.iter().cycle().skip(1))
            .take(on_curve.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum::<f64>()
            .abs()
            * 0.5;
        if area < 1e-9 {
            issues.push(format!("{label}の面積が0です（退化した輪郭）"));
        }
    }
}
