    #[test]
    fn project_validation_rejects_invalid_background_transform() {
        let mut project = FontProject::new();
        project
            .background_transforms
            .entry("A".into())
            .or_default()
            .insert(
                "regular".into(),
                crate::font_data::BackgroundImageTransform {
                    x: 0.0,
                    y: 0.0,
                    scale: 0.0,
                    rotation: f32::NAN,
                    flip_x: false,
                    flip_y: false,
                },
            );
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("背景画像変形")));
    }
