    #[test]
    fn project_validation_reports_invalid_master_metadata() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: project.masters[0].name.clone(),
            weight: f64::INFINITY,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::from([("weight".into(), f64::NAN)]),
        });
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("マスター名が重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("WeightまたはWidthが不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("軸 'weight' の値が不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("軸タグ 'weight' は4文字ASCII")));
    }
