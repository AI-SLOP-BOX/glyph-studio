    #[test]
    fn quadratic_and_cubic_nodes_survive_boolean_conversion() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::off_curve(125.0, -50.0),
                ContourPoint::off_curve(175.0, -50.0),
                ContourPoint::on_curve(200.0, 0.0),
            ],
        };
        let shape = contour.to_curve_shape().unwrap();
        assert_eq!(shape.segment_count(), 4);
        let shifted = Contour {
            points: contour
                .points
                .iter()
                .map(|point| ContourPoint {
                    x: point.x + 50.0,
                    y: point.y,
                    ..*point
                })
                .collect(),
        };
        let union = contour.union(&shifted).unwrap();
        assert!(!union.is_empty());
    }
