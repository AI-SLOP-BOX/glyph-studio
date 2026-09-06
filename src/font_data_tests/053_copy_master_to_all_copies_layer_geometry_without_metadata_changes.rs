    #[test]
    fn copy_master_to_all_copies_layer_geometry_without_metadata_changes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.glyphs.get_mut("A").unwrap().width = 812.0;
        project.sync_active_layer("regular");
        let copied = project.copy_master_to_all("regular");
        assert_eq!(copied, 1);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 812.0);
        assert_eq!(project.glyphs["A"].unicode, Some('A' as u32));
    }
