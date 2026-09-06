    #[test]
    fn glyph_order_is_preserved_and_recovers_missing_entries() {
        let mut project = FontProject::new();
        project.add_glyph("z".into(), None);
        project.add_glyph("a".into(), None);
        project.add_glyph("m".into(), None);
        project.move_glyph("m", -2);
        assert_eq!(project.glyph_names_sorted(), vec!["m", "z", "a"]);
        project.remove_glyph("z");
        assert_eq!(project.glyph_names_sorted(), vec!["m", "a"]);
        project
            .glyphs
            .insert("b".into(), GlyphData::new("b".into(), None));
        assert_eq!(project.glyph_names_sorted(), vec!["m", "a", "b"]);
    }
