    #[test]
    fn variable_master_kerning_emits_gpos_feature_variations() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("V".into(), Some('V' as u32));
        let mut bold = project.masters[0].clone();
        bold.id = "bold".into();
        bold.name = "Bold".into();
        bold.weight = 700.0;
        project.masters.push(bold);
        project.kerning.insert(("A".into(), "V".into()), -50.0);
        project.kerning_by_master.insert(
            "regular".into(),
            [(("A".into(), "V".into()), -50.0)].into_iter().collect(),
        );
        project.kerning_by_master.insert(
            "bold".into(),
            [(("A".into(), "V".into()), -100.0)].into_iter().collect(),
        );
        let glyph_ids = [("A", 1_u16), ("V", 2_u16)]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let bytes = build_kerning_gpos(&project, &glyph_ids, "").unwrap();
        assert_eq!(&bytes[..4], &[0, 1, 0, 1]);
    }
