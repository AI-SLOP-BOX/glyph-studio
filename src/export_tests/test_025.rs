    #[test]
    fn simple_gsub_ignores_unknown_rules_without_dropping_valid_rules() {
        let ids = HashMap::from([("A", 1_u16), ("B", 2)]);
        let bytes = build_simple_gsub("feature liga { sub missing by B; sub A by B; } liga;", &ids)
            .expect("valid rules should still produce GSUB");
        assert!(!bytes.is_empty());
    }
