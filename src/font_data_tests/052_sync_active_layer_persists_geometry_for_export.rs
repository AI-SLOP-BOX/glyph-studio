    #[test]
    fn sync_active_layer_persists_geometry_for_export() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().width = 777.0;
        project.sync_active_layer("regular");
        assert_eq!(project.glyphs["A"].layers["regular"].width, 777.0);
    }
