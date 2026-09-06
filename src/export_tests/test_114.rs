    #[test]
    fn project_validation_reports_duplicate_and_degenerate_contours() {
        let mut project = FontProject::new();
        project.glyphs.insert(
            "broken".into(),
            GlyphData {
                name: "broken".into(),
                contours: vec![Contour {
                    points: vec![
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(0.0, 0.0),
                        ContourPoint::on_curve(10.0, 0.0),
                        ContourPoint::on_curve(20.0, 0.0),
                    ],
                }],
                ..GlyphData::new("broken".into(), Some('A' as u32))
            },
        );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("重複した隣接点")));
        assert!(issues.iter().any(|issue| issue.contains("退化した輪郭")));
    }
