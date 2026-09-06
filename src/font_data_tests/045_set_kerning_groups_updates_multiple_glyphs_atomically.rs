    #[test]
    fn set_kerning_groups_updates_multiple_glyphs_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), None);
        project.add_glyph("A.alt".into(), None);
        assert_eq!(
            project
                .set_kerning_groups(&["A".into(), "A.alt".into()], "upper-left", "upper-right",)
                .unwrap(),
            2
        );
        assert_eq!(project.glyphs["A"].left_kerning_group, "upper-left");
        assert!(project
            .set_kerning_groups(&["A".into(), "missing".into()], "x", "y")
            .is_err());
        assert_eq!(project.glyphs["A"].left_kerning_group, "upper-left");
    }
