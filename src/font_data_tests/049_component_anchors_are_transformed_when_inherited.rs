    #[test]
    fn component_anchors_are_transformed_when_inherited() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 100.0,
            y: 200.0,
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 2.0,
            xy_scale: 0.5,
            yx_scale: 0.0,
            y_scale: 3.0,
            x_offset: 10.0,
            y_offset: -20.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.anchors_for_glyph("composite"),
            vec![GlyphAnchor {
                name: "top".into(),
                x: 310.0,
                y: 580.0,
            }]
        );
        let mut accent = GlyphData::new("accent".into(), None);
        accent.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 0.0,
            y: 0.0,
        });
        project.glyphs.insert("accent".into(), accent);
        let mut accented = GlyphData::new("accented".into(), None);
        accented.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        accented.components.push(GlyphComponent {
            base: "accent".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 200.0,
        });
        project.glyphs.insert("accented".into(), accented);
        assert_eq!(project.anchors_for_glyph("accented").len(), 1);
        assert_eq!(project.anchors_for_glyph("accented")[0].name, "top");
    }
