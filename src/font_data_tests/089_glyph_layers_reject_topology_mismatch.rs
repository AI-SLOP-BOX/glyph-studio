    #[test]
    fn glyph_layers_reject_topology_mismatch() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour::new()],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let b = GlyphLayer {
            width: 500.0,
            contours: Vec::new(),
            components: Vec::new(),
            anchors: Vec::new(),
        };
        assert!(a.interpolate(&b, 0.5).is_none());
    }
