    #[test]
    fn master_axis_validation_rejects_non_finite_or_out_of_range_values() {
        let mut project = FontProject::new();
        project.masters[0].weight = f64::NAN;
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].weight = 400.0;
        project.masters[0].width = 0.0;
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].width = 100.0;
        project.masters[0].axes.insert("too".into(), 10.0);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("opsz".into(), f64::NAN);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("wdth".into(), 100.0);
        assert!(validate_master_axes(&project).is_err());
        project.masters[0].axes.clear();
        project.masters[0].axes.insert("wght".into(), 400.0);
        assert!(validate_master_axes(&project).is_err());
    }
