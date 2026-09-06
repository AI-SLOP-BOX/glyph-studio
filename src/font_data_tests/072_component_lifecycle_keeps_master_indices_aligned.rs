    #[test]
    fn component_lifecycle_keeps_master_indices_aligned() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let component = GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 12.0,
            y_offset: 24.0,
        };
        assert_eq!(
            project.add_component_all_layers("A", component.clone()),
            Some(0)
        );
        assert_eq!(
            project.glyphs["A"].layers["bold"].components,
            vec![component]
        );
        assert!(project.move_component_all_layers("A", 0, 1).is_err());
        assert!(project.remove_component_all_layers("A", 0).is_ok());
        assert!(project.glyphs["A"].components.is_empty());
        assert!(project.glyphs["A"].layers["bold"].components.is_empty());
    }
