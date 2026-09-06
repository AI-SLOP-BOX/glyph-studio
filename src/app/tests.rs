use super::*;

#[test]
fn preview_resolves_standard_and_alias_glyph_names() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    assert_eq!(glyph_name_for_project_char(&project, 'A'), "A");
    let mut alias = crate::font_data::GlyphData::new("alt-a".into(), None);
    alias.unicodes.push('Å' as u32);
    project.glyphs.insert("alt-a".into(), alias);
    assert_eq!(glyph_name_for_project_char(&project, 'Å'), "alt-a");
}

#[test]
fn nested_preview_transform_composes_translation_and_scale() {
    let parent = (2.0, 0.0, 0.0, 2.0, 10.0, 20.0);
    let child = (1.0, 0.0, 0.0, 1.0, 3.0, 4.0);
    assert_eq!(
        compose_preview_transform(parent, child),
        (2.0, 0.0, 0.0, 2.0, 16.0, 28.0)
    );
}

#[test]
fn preview_mark_attachment_uses_matching_anchor_pair() {
    let mut project = FontProject::new();
    let mut base = crate::font_data::GlyphData::new("A".into(), Some('A' as u32));
    base.anchors.push(crate::font_data::GlyphAnchor {
        name: "top".into(),
        x: 250.0,
        y: 700.0,
    });
    project.glyphs.insert("A".into(), base);
    let mut mark = crate::font_data::GlyphData::new("acutecomb".into(), None);
    mark.anchors.push(crate::font_data::GlyphAnchor {
        name: "_top".into(),
        x: 30.0,
        y: 40.0,
    });
    project.glyphs.insert("acutecomb".into(), mark);
    assert_eq!(
        preview_mark_attachment(&project, "A", "acutecomb"),
        Some((220.0, 660.0))
    );
}

#[test]
fn preview_applies_ligature_rules() {
    let mut project = FontProject::new();
    project.add_glyph("f".into(), Some('f' as u32));
    project.add_glyph("i".into(), Some('i' as u32));
    project.add_glyph("fi".into(), None);
    project.opentype_features = "feature liga { sub f i by fi; } liga;".into();
    assert_eq!(preview_glyph_names(&project, "fi", "liga"), vec!["fi"]);
}

#[test]
fn preview_applies_single_substitution_rules() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("A.alt".into(), None);
    project.opentype_features = "feature salt { sub A by A.alt; } salt;".into();
    assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
}

#[test]
fn preview_applies_contextual_substitution_rules() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("B".into(), Some('B' as u32));
    project.add_glyph("A.alt".into(), None);
    project.opentype_features = "feature calt { sub A' B by A.alt; } calt;".into();
    assert_eq!(
        preview_glyph_names(&project, "AB", "calt"),
        vec!["A.alt", "B"]
    );
    assert_eq!(
        preview_glyph_names(&project, "AC", "calt"),
        vec!["A", "uni0043"]
    );
}

#[test]
fn preview_applies_multiple_and_alternate_rules() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("B".into(), Some('B' as u32));
    project.add_glyph("A.alt".into(), None);
    project.add_glyph("B.alt".into(), None);
    project.opentype_features =
        "feature cv01 { sub A by A.alt B.alt; } cv01; feature salt { sub B from [B.alt]; } salt;"
            .into();
    assert_eq!(
        preview_glyph_names(&project, "AB", "cv01,salt"),
        vec!["A.alt", "B.alt", "B.alt"]
    );
}

#[test]
fn preview_applies_class_substitution_rules() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("B".into(), Some('B' as u32));
    project.add_glyph("A.alt".into(), None);
    project.add_glyph("B.alt".into(), None);
    project.opentype_features = "feature ss01 { sub [A B] by [A.alt B.alt]; } ss01;".into();
    assert_eq!(
        preview_glyph_names(&project, "AB", "ss01"),
        vec!["A.alt", "B.alt"]
    );
}

#[test]
fn preview_expands_named_feature_classes() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("A.alt".into(), None);
    project.opentype_features = "@caps = [A]; feature salt { sub @caps by A.alt; } salt;".into();
    assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
}

#[test]
fn preview_applies_contextual_target_class() {
    let mut project = FontProject::new();
    for (name, unicode) in [("A", 'A'), ("C", 'C')] {
        project.add_glyph(name.into(), Some(unicode as u32));
    }
    project.add_glyph("C.alt".into(), None);
    project.opentype_features = "feature calt { sub A [C]' by C.alt; } calt;".into();
    assert_eq!(
        preview_glyph_names(&project, "AC", "calt"),
        vec!["A", "C.alt"]
    );
}

#[test]
fn preview_applies_each_choice_in_contextual_target_class() {
    let mut project = FontProject::new();
    for (name, unicode) in [("A", 'A'), ("C", 'C'), ("D", 'D')] {
        project.add_glyph(name.into(), Some(unicode as u32));
    }
    project.add_glyph("C.alt".into(), None);
    project.opentype_features = "feature calt { sub A [C D]' by C.alt; } calt;".into();
    assert_eq!(
        preview_glyph_names(&project, "ACD", "calt"),
        vec!["A", "C.alt", "D"]
    );
    assert_eq!(
        preview_glyph_names(&project, "AD", "calt"),
        vec!["A", "C.alt"]
    );
}

#[test]
fn preview_feature_enabled_matches_comma_or_space_separated_tags() {
    assert!(preview_feature_enabled("liga, kern", "kern"));
    assert!(!preview_feature_enabled("liga salt", "kern"));
}

#[test]
fn toggle_preview_feature_preserves_other_tags_and_toggles_requested_tag() {
    let mut features = "liga, kern".to_string();
    toggle_preview_feature(&mut features, "kern");
    assert_eq!(features, "liga");
    toggle_preview_feature(&mut features, "mark");
    assert_eq!(features, "liga,mark");
}

#[test]
fn preview_ignores_disabled_feature_rules() {
    let mut project = FontProject::new();
    project.add_glyph("A".into(), Some('A' as u32));
    project.add_glyph("A.alt".into(), None);
    project.opentype_features = "feature salt { sub A by A.alt; } salt;".into();
    assert_eq!(preview_glyph_names(&project, "A", "liga"), vec!["A"]);
    assert_eq!(preview_glyph_names(&project, "A", "salt"), vec!["A.alt"]);
}

#[test]
fn decomposition_flattens_nested_components_and_stops_cycles() {
    let mut project = FontProject::new();
    let mut base = crate::font_data::GlyphData::new("base".into(), None);
    base.contours.push(Contour {
        points: vec![crate::font_data::ContourPoint::on_curve(10.0, 20.0)],
    });
    project.glyphs.insert("base".into(), base);

    let mut middle = crate::font_data::GlyphData::new("middle".into(), None);
    middle.components.push(GlyphComponent {
        base: "base".into(),
        x_scale: 2.0,
        xy_scale: 0.0,
        yx_scale: 0.0,
        y_scale: 2.0,
        x_offset: 5.0,
        y_offset: 7.0,
    });
    project.glyphs.insert("middle".into(), middle);

    let mut cycle = crate::font_data::GlyphData::new("cycle".into(), None);
    cycle.components.push(GlyphComponent {
        base: "cycle".into(),
        x_scale: 1.0,
        xy_scale: 0.0,
        yx_scale: 0.0,
        y_scale: 1.0,
        x_offset: 0.0,
        y_offset: 0.0,
    });
    project.glyphs.insert("cycle".into(), cycle);

    let mut output = Vec::new();
    let mut visiting = std::collections::HashSet::new();
    collect_decomposed_contours(
        &project,
        "middle",
        (1.0, 0.0, 0.0, 1.0, 3.0, 4.0),
        &mut visiting,
        &mut output,
    );
    assert_eq!(output.len(), 1);
    assert_eq!((output[0].points[0].x, output[0].points[0].y), (28.0, 51.0));

    output.clear();
    collect_decomposed_contours(
        &project,
        "cycle",
        (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        &mut visiting,
        &mut output,
    );
    assert!(output.is_empty());
}

#[test]
fn projected_outline_bounds_include_nested_component_offsets() {
    let mut project = FontProject::new();
    let mut base = crate::font_data::GlyphData::new("base".into(), None);
    base.contours.push(Contour {
        points: vec![
            crate::font_data::ContourPoint::on_curve(-20.0, 0.0),
            crate::font_data::ContourPoint::on_curve(40.0, 0.0),
        ],
    });
    project.glyphs.insert("base".into(), base);
    let mut composite = crate::font_data::GlyphData::new("composite".into(), None);
    composite.components.push(GlyphComponent {
        base: "base".into(),
        x_scale: 1.0,
        xy_scale: 0.0,
        yx_scale: 0.0,
        y_scale: 1.0,
        x_offset: 100.0,
        y_offset: 0.0,
    });
    project.glyphs.insert("composite".into(), composite);
    let mut visiting = std::collections::HashSet::new();
    assert_eq!(
        min_projected_outline_x(
            &project,
            "composite",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut visiting,
        ),
        Some(80.0)
    );
    assert_eq!(
        max_projected_outline_x(
            &project,
            "composite",
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut visiting,
        ),
        Some(140.0)
    );
}

#[test]
fn relative_glyph_selection_wraps_in_glyph_order() {
    let mut app = GlyphStudioApp::default();
    app.project.add_glyph("A".into(), Some('A' as u32));
    app.project.add_glyph("B".into(), Some('B' as u32));
    app.current_glyph = Some("B".into());
    app.select_relative_glyph(1);
    assert_eq!(app.current_glyph.as_deref(), Some("A"));
    app.select_relative_glyph(-1);
    assert_eq!(app.current_glyph.as_deref(), Some("B"));
    app.select_edge_glyph(false);
    assert_eq!(app.current_glyph.as_deref(), Some("A"));
    app.select_edge_glyph(true);
    assert_eq!(app.current_glyph.as_deref(), Some("B"));
}

#[test]
fn master_compatibility_reports_structural_mismatch() {
    let mut project = FontProject::new();
    let mut glyph = crate::font_data::GlyphData::new("A".into(), Some('A' as u32));
    glyph.layers.insert(
        "regular".into(),
        crate::font_data::GlyphLayer {
            width: 600.0,
            contours: vec![Contour {
                points: vec![
                    crate::font_data::ContourPoint::on_curve(0.0, 0.0),
                    crate::font_data::ContourPoint::on_curve(100.0, 0.0),
                    crate::font_data::ContourPoint::on_curve(0.0, 100.0),
                ],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        },
    );
    glyph.layers.insert(
        "bold".into(),
        crate::font_data::GlyphLayer {
            width: 600.0,
            contours: vec![],
            components: Vec::new(),
            anchors: Vec::new(),
        },
    );
    project.glyphs.insert("A".into(), glyph);
    let issues = master_compatibility_issues(&project, "regular", "bold");
    assert_eq!(issues, vec!["A: 輪郭数が不一致"]);

    let mut component_glyph = crate::font_data::GlyphData::new("B".into(), None);
    let component = |base: &str| crate::font_data::GlyphComponent {
        base: base.into(),
        x_scale: 1.0,
        xy_scale: 0.0,
        yx_scale: 0.0,
        y_scale: 1.0,
        x_offset: 0.0,
        y_offset: 0.0,
    };
    component_glyph.layers.insert(
        "regular".into(),
        crate::font_data::GlyphLayer {
            width: 600.0,
            contours: Vec::new(),
            components: vec![component("acute")],
            anchors: Vec::new(),
        },
    );
    component_glyph.layers.insert(
        "bold".into(),
        crate::font_data::GlyphLayer {
            width: 600.0,
            contours: Vec::new(),
            components: vec![component("grave")],
            anchors: Vec::new(),
        },
    );
    project.glyphs.insert("B".into(), component_glyph);
    let issues = master_compatibility_issues(&project, "regular", "bold");
    assert_eq!(
        issues,
        vec!["A: 輪郭数が不一致", "B: コンポーネント名が不一致"]
    );
}
