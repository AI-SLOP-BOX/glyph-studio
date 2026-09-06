    #[test]
    fn master_vertical_metrics_override_legacy_glyph_value() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.set_vertical_metrics("A", 1000.0, 800.0).unwrap();
        project
            .set_vertical_metrics_for_master("A", "bold", 1200.0, 640.0)
            .unwrap();
        assert_eq!(
            project
                .vertical_metrics_for_glyph_in_master("A", "regular")
                .advance_height,
            1000.0
        );
        assert_eq!(
            project
                .vertical_metrics_for_glyph_in_master("A", "bold")
                .top_side_bearing,
            640.0
        );
    }
