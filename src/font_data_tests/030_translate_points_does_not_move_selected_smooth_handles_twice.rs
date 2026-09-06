    #[test]
    fn translate_points_does_not_move_selected_smooth_handles_twice() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(30.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(70.0, 0.0),
            ],
        };
        contour.set_smooth(2, true);
        let before: Vec<(f64, f64)> = contour
            .points
            .iter()
            .map(|point| (point.x, point.y))
            .collect();
        contour.translate_points(&[0, 1, 2, 3], 12.0, -7.0);
        for (point, (x, y)) in contour.points.iter().zip(before) {
            assert_eq!(point.x, x + 12.0);
            assert_eq!(point.y, y - 7.0);
        }
    }
