    #[test]
    fn switch_master_initializes_new_layer_for_every_glyph() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.glyphs.get_mut("A").unwrap().width = 812.0;
        project.glyphs.get_mut("B").unwrap().width = 913.0;
        project.switch_master("regular", "bold");
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert_eq!(project.glyphs["B"].layers["bold"].width, 913.0);
    }
