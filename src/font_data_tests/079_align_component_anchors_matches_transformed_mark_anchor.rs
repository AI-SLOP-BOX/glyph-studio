    #[test]
    fn align_component_anchors_matches_transformed_mark_anchor() {
        let mut project = FontProject::new();
        let mut mark = GlyphData::new("mark".into(), None);
        mark.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 20.0,
            y: 30.0,
        });
        project.glyphs.insert("mark".into(), mark);
        let mut accented = GlyphData::new("accented".into(), None);
        accented.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 300.0,
            y: 700.0,
        });
        accented.components.push(GlyphComponent {
            base: "mark".into(),
            x_scale: 2.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 2.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        project.glyphs.insert("accented".into(), accented);
        assert!(project.align_component_anchors("accented", 0));
        let aligned = &project.glyphs["accented"].components[0];
        assert_eq!((aligned.x_offset, aligned.y_offset), (260.0, 640.0));
    }
