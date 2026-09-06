    #[test]
    fn project_validation_finds_component_cycles_and_invalid_geometry() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.components.push(GlyphComponent {
            base: "B".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        let mut b = GlyphData::new("B".into(), None);
        b.components.push(GlyphComponent {
            base: "A".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        a.width = f64::NAN;
        a.anchors.push(GlyphAnchor {
            name: String::new(),
            x: f64::NAN,
            y: 0.0,
        });
        project.glyphs.insert("A".into(), a);
        project.glyphs.insert("B".into(), b);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("循環参照")));
        assert!(issues.iter().any(|issue| issue.contains("幅が不正")));
        assert!(issues.iter().any(|issue| issue.contains("アンカー")));
    }
