    #[test]
    fn project_validation_reports_invalid_contour_topology() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "broken".into(),
            GlyphData {
                name: "broken".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::off_curve(0.0, 0.0),
                        ContourPoint::off_curve(1.0, 1.0),
                        ContourPoint::off_curve(2.0, 2.0),
                    ],
                }],
                ..GlyphData::new("broken".into(), None)
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("オンカーブ点")));
        assert!(issues.iter().any(|issue| issue.contains("オフカーブ点")));
    }
