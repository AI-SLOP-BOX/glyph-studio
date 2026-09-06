    #[test]
    fn variable_vertical_metrics_emit_vvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project
            .set_vertical_metrics_for_master("A", &project.masters[0].id.clone(), 1000.0, 800.0)
            .unwrap();
        project
            .set_vertical_metrics_for_master("A", &second.id, 1200.0, 900.0)
            .unwrap();
        let bytes = build_vvar(&project, &["A"], &project.masters[0], &["wght".into()]);
        assert!(bytes.is_some());
    }
