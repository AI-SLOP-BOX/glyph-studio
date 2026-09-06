    #[test]
    fn explicit_component_anchor_name_wins_over_inherited_duplicate() {
        let mut project = FontProject::new();
        let mut base = GlyphData::new("base".into(), None);
        base.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 10.0,
            y: 20.0,
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 100.0,
            y: 200.0,
        });
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert_eq!(
            project.anchors_for_glyph("composite"),
            vec![GlyphAnchor {
                name: "top".into(),
                x: 100.0,
                y: 200.0
            }]
        );
    }
