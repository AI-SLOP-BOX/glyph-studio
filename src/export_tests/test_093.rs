    #[test]
    fn languagesystem_declarations_populate_default_and_language_systems() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"locl")];
        let scripts = build_script_list(
            "languagesystem latn dflt; languagesystem latn TRK;\nfeature liga { sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        let script = &scripts.script_records[0].script;
        assert!(script.default_lang_sys.is_some());
        assert_eq!(script.lang_sys_records.len(), 1);
        assert_eq!(script.lang_sys_records[0].lang_sys_tag, Tag::new(b"TRK "));
    }
