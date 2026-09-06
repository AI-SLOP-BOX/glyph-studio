    #[test]
    fn set_side_bearings_rejects_invalid_values_without_changes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().width = 600.0;
        let before = project.clone();
        assert_eq!(project.set_side_bearings(&["A".into()], -1.0, 20.0), 0);
        assert_eq!(project, before);
        assert_eq!(project.set_side_bearings(&["A".into()], f64::NAN, 20.0), 0);
        assert_eq!(project, before);
    }
