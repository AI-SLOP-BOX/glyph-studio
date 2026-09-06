    #[test]
    fn reverse_direction_preserves_node_data_in_reverse_order() {
        let mut contour = Contour {
            points: vec![
                ContourPoint::on_curve(1.0, 2.0),
                ContourPoint::off_curve(3.0, 4.0),
                ContourPoint::on_curve(5.0, 6.0),
            ],
        };
        let original = contour.points.clone();
        contour.reverse_direction();
        assert_eq!(
            contour.points,
            original.into_iter().rev().collect::<Vec<_>>()
        );
    }
