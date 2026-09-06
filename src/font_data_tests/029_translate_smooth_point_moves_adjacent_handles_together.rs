    #[test]
    fn translate_smooth_point_moves_adjacent_handles_together() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        contour.set_smooth(2, true);
        let before = contour.points[1];
        let after = contour.points[3];
        contour.translate_point(2, 12.0, -7.0);
        assert_eq!(contour.points[2].x, 112.0);
        assert_eq!(contour.points[2].y, -7.0);
        assert_eq!(contour.points[1].x, before.x + 12.0);
        assert_eq!(contour.points[1].y, before.y - 7.0);
        assert_eq!(contour.points[3].x, after.x + 12.0);
        assert_eq!(contour.points[3].y, after.y - 7.0);
    }
