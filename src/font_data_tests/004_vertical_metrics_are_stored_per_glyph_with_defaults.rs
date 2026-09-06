    #[test]
    fn vertical_metrics_are_stored_per_glyph_with_defaults() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        let defaults = project.vertical_metrics_for_glyph("A");
        assert_eq!(defaults.advance_height, 1000.0);
        assert_eq!(defaults.top_side_bearing, 800.0);
        project.set_vertical_metrics("A", 1200.0, 700.0).unwrap();
        assert_eq!(
            project.vertical_metrics_for_glyph("A").advance_height,
            1200.0
        );
        assert!(project
            .set_vertical_metrics("missing", 1000.0, 0.0)
            .is_err());
    }
