    #[test]
    fn remove_nodes_updates_every_master_layer_atomically() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours.push(contour.clone());
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 600.0,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .remove_nodes_all_layers(&[(0, 0)])
            .unwrap();
        assert_eq!(project.glyphs["A"].contours[0].points.len(), 3);
        assert_eq!(
            project.glyphs["A"].layers["bold"].contours[0].points.len(),
            3
        );
        assert!(project
            .glyphs
            .get_mut("A")
            .unwrap()
            .remove_nodes_all_layers(&[(0, 99)])
            .is_err());
    }
