    #[test]
    fn kerning_is_independent_per_master_and_survives_switching() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("V".into(), Some(86));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project.set_kerning_pair("A", "V", -60.0).unwrap();
        project.sync_active_layer("regular");
        project.switch_master("regular", "bold");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-60.0));
        project.set_kerning_pair("A", "V", -120.0).unwrap();
        project.sync_active_layer("bold");
        project.switch_master("bold", "regular");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-60.0));
        project.switch_master("regular", "bold");
        assert_eq!(project.kerning_for_glyphs("A", "V"), Some(-120.0));
    }
