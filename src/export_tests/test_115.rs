    #[test]
    fn project_validation_reports_self_intersecting_contours() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "cross".into(),
            GlyphData {
                name: "cross".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(100.0, 100.0),
                        ContourPoint::on_curve(0.0, 100.0),
                        ContourPoint::on_curve(100.0, 0.0),
                    ],
                }],
                ..GlyphData::new("cross".into(), Some('A' as u32))
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("自己交差")));
    }
