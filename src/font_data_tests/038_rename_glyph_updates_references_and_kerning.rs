    #[test]
    fn rename_glyph_updates_references_and_kerning() {
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
        assert!(project.rename_glyph("base", "renamed".into()));
        assert!(project.glyphs.contains_key("renamed"));
        assert_eq!(project.glyphs["mark"].components[0].base, "renamed");
        assert_eq!(
            project.kerning.get(&("renamed".into(), "mark".into())),
            Some(&-40.0)
        );
        assert!(project.opentype_features.contains("sub renamed by mark"));
        assert!(project.vertical_metrics.contains_key("renamed"));
        assert_eq!(
            project.background_images["renamed"]["regular"],
            "/tmp/base.png"
        );
        assert_eq!(project.unicode_variation_sequences[0].glyph, "renamed");
        assert_eq!(project.color_layers["mark"][0].glyph, "renamed");
        project.add_glyph("liga".into(), None);
        project.opentype_features = "feature liga { sub liga by mark; } liga;".into();
        assert!(project.rename_glyph("liga", "ligature".into()));
        assert_eq!(
            project.opentype_features,
            "feature liga { sub ligature by mark; } liga;"
        );
    }
