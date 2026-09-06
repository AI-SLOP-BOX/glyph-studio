use super::*;

#[test]
fn zoom_at_keeps_pointer_position_stable() {
    let mut canvas = CanvasState::default();
    let center = Pos2::new(100.0, 100.0);
    let pointer = Pos2::new(150.0, 100.0);
    let glyph_before = canvas.screen_to_glyph(pointer, center);
    canvas.zoom_at(10.0, pointer, center);
    let new_origin = center + canvas.pan;
    let glyph_after = canvas.screen_to_glyph(
        canvas.glyph_to_screen(glyph_before.0, glyph_before.1, new_origin),
        new_origin,
    );
    assert!((glyph_before.0 - glyph_after.0).abs() < 0.001);
    assert!((glyph_before.1 - glyph_after.1).abs() < 0.001);
}

#[test]
fn snap_point_rounds_to_configured_grid() {
    let canvas = CanvasState {
        snap_to_grid: true,
        grid_size: 100.0,
        ..Default::default()
    };
    assert_eq!(canvas.snap_point(149.0, -151.0), (100.0, -200.0));
}

#[test]
fn snap_point_to_guidelines_snaps_near_horizontal_and_vertical_guides() {
    let canvas = CanvasState {
        snap_to_guidelines: true,
        zoom: 2.0,
        ..Default::default()
    };
    let guides = vec![
        Guideline {
            x: 300.0,
            y: 0.0,
            angle: 90.0,
            name: String::new(),
        },
        Guideline {
            x: 0.0,
            y: 500.0,
            angle: 0.0,
            name: String::new(),
        },
    ];
    assert_eq!(
        canvas.snap_point_to_guidelines(303.0, 496.5, &guides),
        (300.0, 500.0)
    );
}

#[test]
fn gradient_color_interpolates_linear_endpoints() {
    let gradient = ColorGradient {
        start_palette_index: 0,
        end_palette_index: 1,
        kind: ColorGradientKind::Linear,
        extend: ColorGradientExtend::default(),
        x0: 0.0,
        y0: 0.0,
        x1: 100.0,
        y1: 0.0,
        x2: 0.0,
        y2: 100.0,
        stops: vec![
            crate::font_data::ColorGradientStop {
                offset: 0.0,
                palette_index: 0,
                alpha: 1.0,
            },
            crate::font_data::ColorGradientStop {
                offset: 0.5,
                palette_index: 1,
                alpha: 0.5,
            },
            crate::font_data::ColorGradientStop {
                offset: 1.0,
                palette_index: 0,
                alpha: 1.0,
            },
        ],
        radius0: 0.0,
        radius1: 100.0,
        start_angle: 0.0,
        end_angle: 360.0,
    };
    let palette = [[255, 0, 0, 255], [0, 0, 255, 255]];
    assert_eq!(
        gradient_color(Point::new(0.0, 0.0), &gradient, &palette),
        Color32::RED
    );
    assert_eq!(
        gradient_color(Point::new(50.0, 0.0), &gradient, &palette),
        Color32::from_rgba_unmultiplied(0, 0, 255, 128)
    );
    assert_eq!(
        gradient_color(Point::new(100.0, 0.0), &gradient, &palette),
        Color32::RED
    );
}

#[test]
fn gradient_color_applies_repeat_and_reflect_extensions() {
    let mut gradient = ColorGradient {
        start_palette_index: 0,
        end_palette_index: 1,
        kind: ColorGradientKind::Linear,
        extend: ColorGradientExtend::Repeat,
        x0: 0.0,
        y0: 0.0,
        x1: 100.0,
        y1: 0.0,
        x2: 0.0,
        y2: 100.0,
        stops: Vec::new(),
        radius0: 0.0,
        radius1: 100.0,
        start_angle: 0.0,
        end_angle: 360.0,
    };
    let palette = [[255, 0, 0, 255], [0, 0, 255, 255]];
    assert_eq!(
        gradient_color(Point::new(-50.0, 0.0), &gradient, &palette),
        gradient_color(Point::new(50.0, 0.0), &gradient, &palette)
    );
    gradient.extend = ColorGradientExtend::Reflect;
    assert_eq!(
        gradient_color(Point::new(150.0, 0.0), &gradient, &palette),
        gradient_color(Point::new(50.0, 0.0), &gradient, &palette)
    );
}

#[test]
fn snap_point_to_guidelines_leaves_distant_points_and_disabled_snap_unchanged() {
    let guides = vec![Guideline {
        x: 300.0,
        y: 500.0,
        angle: 0.0,
        name: String::new(),
    }];
    let canvas = CanvasState::default();
    assert_eq!(
        canvas.snap_point_to_guidelines(303.0, 496.0, &guides),
        (303.0, 496.0)
    );
    let canvas = CanvasState {
        snap_to_guidelines: true,
        ..Default::default()
    };
    assert_eq!(
        canvas.snap_point_to_guidelines(330.0, 488.0, &guides),
        (330.0, 488.0)
    );
}

#[test]
fn snap_point_to_anchors_uses_nearest_anchor_with_zoom_scaled_threshold() {
    let canvas = CanvasState {
        snap_to_anchors: true,
        zoom: 2.0,
        ..Default::default()
    };
    let anchors = vec![
        GlyphAnchor {
            name: "top".into(),
            x: 300.0,
            y: 500.0,
        },
        GlyphAnchor {
            name: "bottom".into(),
            x: 100.0,
            y: 0.0,
        },
    ];
    assert_eq!(
        canvas.snap_point_to_anchors(303.0, 502.0, &anchors),
        (300.0, 500.0)
    );
    assert_eq!(
        canvas.snap_point_to_anchors(310.0, 500.0, &anchors),
        (310.0, 500.0)
    );
}

#[test]
fn hit_test_segment_finds_closed_line_segment_and_factor() {
    let canvas = CanvasState::default();
    let glyph = GlyphData {
        name: "square".into(),
        unicode: None,
        unicodes: Vec::new(),
        width: 600.0,
        left_kerning_group: String::new(),
        right_kerning_group: String::new(),
        left_metrics_key: String::new(),
        right_metrics_key: String::new(),
        anchors: Vec::new(),
        contours: vec![Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        }],
        components: Vec::new(),
        layers: std::collections::HashMap::new(),
        guidelines: Vec::new(),
        master_guidelines: std::collections::HashMap::new(),
    };
    let hit = canvas
        .hit_test_segment(Pos2::new(50.0, 0.0), &glyph, Pos2::ZERO)
        .expect("segment should be hit");
    assert_eq!(hit.0, 0);
    assert_eq!(hit.1, 0);
    assert!((hit.2 - 0.5).abs() < 0.01);
}

#[test]
fn component_hit_test_uses_transformed_component_bounds() {
    let mut project = FontProject::new();
    let mut base = GlyphData::new("base".into(), None);
    base.contours.push(Contour {
        points: vec![
            ContourPoint::on_curve(0.0, 0.0),
            ContourPoint::on_curve(100.0, 0.0),
            ContourPoint::on_curve(0.0, 100.0),
        ],
    });
    project.glyphs.insert("base".into(), base);
    let mut composite = GlyphData::new("composite".into(), None);
    composite.components.push(GlyphComponent {
        base: "base".into(),
        x_scale: 2.0,
        xy_scale: 0.0,
        yx_scale: 0.0,
        y_scale: 2.0,
        x_offset: 300.0,
        y_offset: 200.0,
    });
    let canvas = CanvasState::default();
    assert_eq!(
        canvas.hit_test_component(Pos2::new(350.0, -300.0), &project, &composite, Pos2::ZERO,),
        Some(0)
    );
    assert_eq!(
        canvas.hit_test_component(Pos2::new(100.0, 100.0), &project, &composite, Pos2::ZERO,),
        None
    );
}
