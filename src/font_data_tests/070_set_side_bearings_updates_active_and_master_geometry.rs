    #[test]
    fn set_side_bearings_updates_active_and_master_geometry() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(-20.0, 0.0),
                ContourPoint::on_curve(80.0, 0.0),
                ContourPoint::on_curve(80.0, 100.0),
            ],
        });
        glyph.width = 600.0;
        project.sync_active_layer("regular");
        assert_eq!(project.set_side_bearings(&["A".into()], 30.0, 40.0), 1);
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.width, 170.0);
        assert_eq!(glyph.contours[0].points[0].x, 30.0);
        assert_eq!(glyph.layers["regular"].width, 170.0);
        assert_eq!(glyph.layers["regular"].contours[0].points[0].x, 30.0);
    }
