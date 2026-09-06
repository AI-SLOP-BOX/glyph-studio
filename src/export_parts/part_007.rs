
fn svg_color_layer_transform(project: &FontProject, base_name: &str, index: usize) -> String {
    let Some(Some(transform)) = project
        .color_layer_transforms
        .get(base_name)
        .and_then(|transforms| transforms.get(index))
    else {
        return String::new();
    };
    format!(
        " transform=\"matrix({} {} {} {} {} {})\"",
        transform.xx, transform.yx, transform.xy, transform.yy, transform.dx, transform.dy
    )
}
