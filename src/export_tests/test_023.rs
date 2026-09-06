    #[test]
    fn simple_gsub_supports_reverse_chain_substitution() {
        let ids = [("A", 1), ("A.alt", 2), ("B", 3), ("C", 4)]
            .into_iter()
            .collect();
        let bytes =
            build_simple_gsub("feature rvrn { reversesub B A' C by A.alt; } rvrn;", &ids).unwrap();
        assert!(bytes.len() > 40);
        let shorthand =
            build_simple_gsub("feature rvrn { rsub B A' C by A.alt; } rvrn;", &ids).unwrap();
        assert_eq!(bytes, shorthand);
    }
