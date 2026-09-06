    #[test]
    fn copy_master_to_all_for_glyphs_only_copies_requested_names() {
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
        project.sync_active_layer("regular");
        let copied = project.copy_master_to_all_for_glyphs("regular", ["A"]);
        assert_eq!(copied, 1);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert!(!project.glyphs["B"].layers.contains_key("bold"));
    }
