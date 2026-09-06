    #[test]
    fn duplicate_component_is_copied_to_every_master_layer() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: std::collections::HashMap::new(),
        });
        project.sync_active_layer("regular");
        let component = GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 40.0,
            y_offset: 0.0,
        };
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.components.push(component.clone());
        glyph
            .layers
            .get_mut("regular")
            .unwrap()
            .components
            .push(component.clone());
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: vec![component],
                anchors: Vec::new(),
            },
        );
        assert!(project.duplicate_component_all_layers("A", 0));
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.components.len(), 2);
        assert_eq!(glyph.layers["regular"].components.len(), 2);
        assert_eq!(glyph.layers["bold"].components.len(), 2);
        assert_eq!(glyph.layers["bold"].components[1].x_offset, 40.0);
        assert!(!project.duplicate_component_all_layers("A", 99));
        assert!(!project.duplicate_component_all_layers("missing", 0));
        assert_eq!(project.glyphs["A"].components.len(), 2);
        assert_eq!(project.glyphs["A"].layers["regular"].components.len(), 2);
        assert_eq!(project.glyphs["A"].layers["bold"].components.len(), 2);
    }
