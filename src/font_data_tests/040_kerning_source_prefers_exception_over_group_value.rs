    #[test]
    fn kerning_source_prefers_exception_over_group_value() {
        let mut project = FontProject::new();
        let mut a = GlyphData::new("A".into(), None);
        a.left_kerning_group = "latin-left".into();
        let mut a_alt = GlyphData::new("A.alt".into(), None);
        a_alt.left_kerning_group = "latin-left".into();
        let mut v = GlyphData::new("V".into(), None);
        v.right_kerning_group = "latin-right".into();
        let mut v_alt = GlyphData::new("V.alt".into(), None);
        v_alt.right_kerning_group = "latin-right".into();
        let mut a_z = GlyphData::new("A.z".into(), None);
        a_z.left_kerning_group = "latin-left".into();
        let mut v_z = GlyphData::new("V.z".into(), None);
        v_z.right_kerning_group = "latin-right".into();
        project.glyphs.extend([
            ("A".into(), a),
            ("A.alt".into(), a_alt),
            ("V".into(), v),
            ("V.alt".into(), v_alt),
            ("A.z".into(), a_z),
            ("V.z".into(), v_z),
        ]);
        project.kerning.insert(("A".into(), "V".into()), -80.0);
        project.kerning.insert(("A.z".into(), "V.z".into()), -60.0);
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(-80.0));
        assert_eq!(
            project.kerning_source_for_glyphs("A.alt", "V.alt"),
            Some((("A".into(), "V".into()), -80.0))
        );
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), -120.0);
        assert_eq!(
            project.kerning_source_for_glyphs("A.alt", "V.alt"),
            Some((("A.alt".into(), "V.alt".into()), -120.0))
        );
        project
            .kerning
            .insert(("A.alt".into(), "V.alt".into()), 0.0);
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(0.0));
        assert_eq!(project.kerning_for_glyphs("A", "missing"), None);
        project
            .glyphs
            .get_mut("V.alt")
            .unwrap()
            .right_kerning_group
            .clear();
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), Some(0.0));
        project.kerning.remove(&("A.alt".into(), "V.alt".into()));
        assert_eq!(project.kerning_for_glyphs("A.alt", "V.alt"), None);
    }
