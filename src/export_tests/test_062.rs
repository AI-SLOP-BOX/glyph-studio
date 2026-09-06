    #[test]
    fn kerning_groups_expand_into_gpos_pairs() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.left_kerning_group = "left".into();
        let mut a_alt = GlyphData::new("A.alt".into(), None);
        a_alt.left_kerning_group = "left".into();
        let mut v = GlyphData::new("V".into(), None);
        v.right_kerning_group = "right".into();
        let mut v_alt = GlyphData::new("V.alt".into(), None);
        v_alt.right_kerning_group = "right".into();
        project.glyphs.extend([
            ("A".into(), a),
            ("A.alt".into(), a_alt),
            ("V".into(), v),
            ("V.alt".into(), v_alt),
        ]);
        project.kerning.insert(("A".into(), "V".into()), -80.0);
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), -120.0);
        let ids = [("A", 1), ("A.alt", 2), ("V", 3), ("V.alt", 4)]
            .into_iter()
            .collect();
        let first = build_kerning_gpos(&project, &ids, "").unwrap();
        let second = build_kerning_gpos(&project, &ids, "").unwrap();
        assert_eq!(first, second);
    }
