    #[test]
    fn split_quadratic_preserves_curve_shape() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
            ],
        };
        let index = contour.split_segment(0, 0.5).unwrap();
        assert_eq!(index, 2);
        assert_eq!(contour.points.len(), 5);
        assert!((contour.points[index].x - 50.0).abs() < 1e-9);
        assert!((contour.points[index].y - 50.0).abs() < 1e-9);
        assert!(!contour.points[1].is_on_curve());
        assert!(!contour.points[3].is_on_curve());
    }
