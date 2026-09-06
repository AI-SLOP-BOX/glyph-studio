    #[test]
    fn sync_active_layer_preserves_guides_edited_through_master_view() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .guidelines_for_master_mut("regular")
            .push(Guideline {
                x: 80.0,
                y: 650.0,
                angle: 0.0,
                name: "cap".into(),
            });
        project.sync_active_layer("regular");
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.guidelines.len(), 1);
        assert_eq!(glyph.guidelines_for_master("regular")[0].y, 650.0);
    }
