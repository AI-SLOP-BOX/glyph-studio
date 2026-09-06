    #[test]
    fn feature_file_accepts_long_form_substitute_and_position_keywords() {
        let ids = HashMap::from([("A", 1_u16), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub("feature salt { substitute A by A.alt; } salt;", &ids)
            .expect("long-form substitute should produce GSUB");
        assert!(!gsub.is_empty());
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(0x41));
        project.add_glyph("V".into(), Some(0x56));
        project.kerning.insert(("A".into(), "V".into()), -50.0);
        let gpos = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { position A V < -50 0 0 0 >; } kern;",
        )
        .expect("long-form position should produce GPOS");
        assert!(!gpos.is_empty());
    }
