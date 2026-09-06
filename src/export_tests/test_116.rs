    #[test]
    fn project_validation_reports_incompatible_master_layers() {
        let mut project = FontProject::new();
        project.masters.push(crate::font_data::FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 0.0),
                        ContourPoint::on_curve(0.0, 100.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 0.0),
                    ],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("補間互換ではありません")));
        let interpolation_issues = validate_interpolation(&project, "regular", "bold");
        assert_eq!(interpolation_issues.len(), 1);
        assert!(interpolation_issues[0].message.contains("ノード数が不一致"));
    }
