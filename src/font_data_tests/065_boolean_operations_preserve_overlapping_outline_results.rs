    #[test]
    fn boolean_operations_preserve_overlapping_outline_results() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let left = rectangle(0.0);
        let right = rectangle(50.0);
        let union = left.union(&right).unwrap();
        assert!(!union.is_empty());
        assert!(union.iter().all(|contour| contour.points.len() >= 3));
        let difference = left.difference(&right).unwrap();
        assert!(!difference.is_empty());
        assert!(difference.iter().all(|contour| contour.points.len() >= 3));
        let intersection = left.intersection(&right).unwrap();
        assert!(!intersection.is_empty());
        assert!(intersection.iter().all(|contour| contour.points.len() >= 3));
        let xor = left.xor(&right).unwrap();
        assert!(!xor.is_empty());
        assert!(xor.iter().all(|contour| contour.points.len() >= 3));
    }
