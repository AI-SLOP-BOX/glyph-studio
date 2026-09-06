    #[test]
    fn translate_nodes_all_layers_moves_selected_smooth_geometry_once() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.contours[0].set_smooth(2, true);
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![glyph.contours[0].clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.contours[0].points.clone();
        glyph
            .translate_nodes_all_layers(&[(0, 0), (0, 1), (0, 2), (0, 3)], 12.0, -7.0)
            .unwrap();
        for points in [
            &glyph.contours[0].points,
            &glyph.layers["bold"].contours[0].points,
        ] {
            for (point, original) in points.iter().zip(&before) {
                assert_eq!(point.x, original.x + 12.0);
                assert_eq!(point.y, original.y - 7.0);
            }
        }
    }
