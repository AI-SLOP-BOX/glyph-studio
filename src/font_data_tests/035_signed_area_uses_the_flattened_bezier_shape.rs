    #[test]
    fn signed_area_uses_the_flattened_bezier_shape() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 120.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, -100.0),
                ContourPoint::on_curve(0.0, -100.0),
            ],
        };
        assert!(contour.signed_area() < 0.0);
    }
