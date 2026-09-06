    #[test]
    fn instance_axis_validation_rejects_reserved_or_invalid_values() {
        let mut project = FontProject::new();
        project.instances.push(FontInstance {
            name: "Bad".into(),
            axes: HashMap::from([("wght".into(), 500.0)]),
            weight: 500.0,
            width: 100.0,
        });
        assert!(validate_master_axes(&project).is_err());
        project.instances[0].axes.clear();
        project.instances[0].weight = f64::NAN;
        assert!(validate_master_axes(&project).is_err());
    }
