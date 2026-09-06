    #[test]
    fn align_component_anchors_leaves_unmatched_component_unchanged() {
        let mut project = FontProject::new();
        project
            .glyphs
            .insert("mark".into(), GlyphData::new("mark".into(), None));
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "mark".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 12.0,
            y_offset: 34.0,
        });
        project.glyphs.insert("composite".into(), composite);
        assert!(!project.align_component_anchors("composite", 0));
        let component = &project.glyphs["composite"].components[0];
        assert_eq!((component.x_offset, component.y_offset), (12.0, 34.0));
    }
