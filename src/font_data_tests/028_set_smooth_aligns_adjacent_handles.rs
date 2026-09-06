    #[test]
    fn set_smooth_aligns_adjacent_handles() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(40.0, 30.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::off_curve(60.0, 130.0),
            ],
        };
        contour.set_smooth(2, true);
        let before = contour.points[1];
        let after = contour.points[3];
        let incoming = (before.x - 100.0, before.y - 100.0);
        let outgoing = (after.x - 100.0, after.y - 100.0);
        assert!((incoming.0 * outgoing.1 - incoming.1 * outgoing.0).abs() < 1e-9);
        assert!(incoming.0 * outgoing.0 + incoming.1 * outgoing.1 < 0.0);
    }
