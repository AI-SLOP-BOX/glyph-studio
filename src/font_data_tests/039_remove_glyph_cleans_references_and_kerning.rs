    #[test]
    fn remove_glyph_cleans_references_and_kerning() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), None);
        project.add_glyph("mark".into(), None);
        project
            .glyphs
            .get_mut("mark")
            .unwrap()
            .components
            .push(GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        project
            .kerning
            .insert(("base".into(), "mark".into()), -40.0);
        project.vertical_metrics.insert(
            "base".into(),
            VerticalMetrics {
                advance_height: 1100.0,
                top_side_bearing: 700.0,
            },
        );
        project.color_layers.insert(
            "mark".into(),
            vec![ColorLayer {
                glyph: "base".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project
            .background_images
            .entry("base".into())
            .or_default()
            .insert("regular".into(), "/tmp/base.png".into());
        project
            .unicode_variation_sequences
            .push(UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "base".into(),
            });
        project.opentype_features = "feature liga { sub base by mark; } liga;".into();
        project.remove_glyph("base");
        assert!(project.kerning.is_empty());
        assert!(project.glyphs["mark"].components.is_empty());
        assert!(!project.vertical_metrics.contains_key("base"));
        assert!(!project.background_images.contains_key("base"));
        assert!(!project.color_layers.contains_key("mark"));
        assert!(project.unicode_variation_sequences.is_empty());
        assert!(project.opentype_features.contains("sub .notdef by mark"));
    }
