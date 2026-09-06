    #[test]
    fn translate_nodes_and_components_can_sync_all_layers_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.components.push(GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: glyph.components.clone(),
                anchors: Vec::new(),
            },
        );
        glyph
            .translate_nodes_all_layers(&[(0, 1)], 12.0, -4.0)
            .unwrap();
        glyph.translate_component_all_layers(0, 5.0, 7.0).unwrap();
        assert_eq!(glyph.contours[0].points[1].x, 112.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 112.0);
        assert_eq!(glyph.components[0].x_offset, 15.0);
        assert_eq!(glyph.layers["bold"].components[0].y_offset, 27.0);
    }
