    #[test]
    fn bezier_contour_converts_to_curve_boolean_shape() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let shape = contour.to_curve_shape().unwrap();
        assert_eq!(shape.len(), 1);
        assert_eq!(shape.segment_count(), 3);
        assert!(contour.difference(&contour).unwrap().is_empty());
    }
