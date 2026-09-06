    #[test]
    fn cut_at_nodes_preserves_both_closed_paths() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let (first, second) = contour.cut_at_nodes(0, 2).unwrap();
        assert_eq!(first.points.len(), 3);
        assert_eq!(second.points.len(), 3);
        assert_eq!(first.points.first().unwrap().x, 0.0);
        assert_eq!(first.points.last().unwrap().x, 100.0);
        assert_eq!(second.points.first().unwrap().x, 100.0);
        assert_eq!(second.points.last().unwrap().x, 0.0);
    }
