    #[test]
    fn feature_source_positioning_compiles_single_pair_and_class_rules() {
        let project = FontProject::new();
        let ids = [("A", 1), ("A.alt", 2), ("V", 3), ("V.alt", 4)]
            .into_iter()
            .collect();
        let source = r#"
            feature kern { pos A V <0 0 -80 0>; } kern;
            feature mark { pos [A A.alt] <10 20 0 0>; } mark;
            feature calt { pos [A A.alt] [V V.alt] <0 0 -40 0>; } calt;
            feature ccmp { pos A' V <0 0 -20 0>; } ccmp;
            feature dist { pos A V' A <0 0 -30 0>; } dist;
            feature ss01 { pos A.alt <50>; } ss01;
            feature kern2 { pos A.alt V.alt <10 20> <-5 0>; } kern2;
        "#;
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
        assert!(bytes.windows(2).any(|window| window == [0, 7]));
        assert!(bytes.windows(2).any(|window| window == [0, 8]));
    }
