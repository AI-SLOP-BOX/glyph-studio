
fn interpolation_mismatch_reason(
    from: &crate::font_data::GlyphLayer,
    to: &crate::font_data::GlyphLayer,
) -> Option<String> {
    if from.contours.len() != to.contours.len() {
        return Some(format!(
            "輪郭数が不一致です（{} → {}）",
            from.contours.len(),
            to.contours.len()
        ));
    }
    for (index, (from_contour, to_contour)) in from.contours.iter().zip(&to.contours).enumerate() {
        if from_contour.points.len() != to_contour.points.len() {
            return Some(format!(
                "{}番目の輪郭のノード数が不一致です（{} → {}）",
                index + 1,
                from_contour.points.len(),
                to_contour.points.len()
            ));
        }
        if let Some(point_index) = from_contour
            .points
            .iter()
            .zip(&to_contour.points)
            .position(|(from_point, to_point)| from_point.point_type != to_point.point_type)
        {
            return Some(format!(
                "{}番目の輪郭の{}番目のノード種別が不一致です",
                index + 1,
                point_index + 1
            ));
        }
    }
    if from.components.len() != to.components.len() {
        return Some(format!(
            "コンポーネント数が不一致です（{} → {}）",
            from.components.len(),
            to.components.len()
        ));
    }
    if let Some(index) = from
        .components
        .iter()
        .zip(&to.components)
        .position(|(from_component, to_component)| from_component.base != to_component.base)
    {
        return Some(format!(
            "{}番目のコンポーネントの参照先が不一致です",
            index + 1
        ));
    }
    if from.anchors.len() != to.anchors.len() {
        return Some(format!(
            "アンカー数が不一致です（{} → {}）",
            from.anchors.len(),
            to.anchors.len()
        ));
    }
    if let Some(anchor) = from
        .anchors
        .iter()
        .find(|anchor| !to.anchors.iter().any(|other| other.name == anchor.name))
    {
        return Some(format!(
            "アンカー「{}」が終点マスターにありません",
            anchor.name
        ));
    }
    None
}
