    #[test]
    fn unicode_assignments_remove_conflicting_primary_and_alias_values() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), None);
        project.glyphs.get_mut("B").unwrap().unicodes = vec![65, 66];
        assert_eq!(project.set_unicode_assignments(&[("B".into(), 65)]), 2);
        assert_eq!(project.glyphs["A"].unicode, None);
        assert_eq!(project.glyphs["B"].unicode, Some(65));
        assert_eq!(project.glyphs["B"].unicodes, vec![66]);
    }
