    #[test]
    fn component_transform_and_reflection_sync_all_layers() {
        let component = GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.components = vec![component.clone()];
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: Vec::new(),
                components: vec![component],
                anchors: Vec::new(),
            },
        );
        glyph.transform_component_all_layers(0, 2.0, 0.0).unwrap();
        glyph.reflect_component_all_layers(0, true).unwrap();
        assert_eq!(glyph.components[0].x_scale, -2.0);
        assert_eq!(glyph.layers["bold"].components[0].x_scale, -2.0);
    }
