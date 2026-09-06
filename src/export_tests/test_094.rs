    #[test]
    fn language_dflt_and_required_feature_are_encoded_in_langsys() {
        let tags = vec![Tag::new(b"liga")];
        let scripts = build_script_list(
            "feature liga { script DFLT; language dflt required; sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        let default = scripts.script_records[0]
            .script
            .default_lang_sys
            .as_ref()
            .expect("language dflt should use the default LangSys");
        assert_eq!(default.required_feature_index, 0);
    }
