    #[test]
    fn translate_points_repairs_a_moved_smooth_handle() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 20.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::off_curve(60.0, 130.0),
            ],
        };
        contour.set_smooth(2, true);
        contour.translate_points(&[3], 10.0, -20.0);
        let incoming = (contour.points[1].x - 100.0, contour.points[1].y - 100.0);
        let outgoing = (contour.points[3].x - 100.0, contour.points[3].y - 100.0);
        assert!((incoming.0 * outgoing.1 - incoming.1 * outgoing.0).abs() < 1e-9);
        assert!(incoming.0 * outgoing.0 + incoming.1 * outgoing.1 < 0.0);
    }
