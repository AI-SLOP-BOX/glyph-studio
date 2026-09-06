    #[test]
    fn duplicate_glyph_copies_layers_and_clears_unicode() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.vertical_metrics.insert(
            "A".into(),
            VerticalMetrics {
                advance_height: 1100.0,
                top_side_bearing: 700.0,
            },
        );
        project.color_layers.insert(
            "A".into(),
            vec![ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .background_images
            .entry("A".into())
            .or_default()
            .insert("regular".into(), "/tmp/A.png".into());
        project
            .background_opacities
            .entry("A".into())
            .or_default()
            .insert("regular".into(), 0.5);
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![Contour::new()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let name = project.duplicate_glyph("A").unwrap();
        assert_eq!(name, "A.copy1");
        assert_eq!(project.glyphs[&name].unicode, None);
        assert!(project.glyphs[&name].layers.contains_key("regular"));
        assert!(project.vertical_metrics.contains_key(&name));
        assert!(project.color_layers.contains_key(&name));
        assert_eq!(project.background_images[&name]["regular"], "/tmp/A.png");
        assert_eq!(project.background_opacities[&name]["regular"], 0.5);
        assert!(project.duplicate_glyph("missing").is_none());
    }
