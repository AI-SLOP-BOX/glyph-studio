    #[test]
    fn script_and_language_statements_build_layout_script_list() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"kern")];
        let scripts = build_script_list(
            "feature liga { script latn; language TRK; sub A by A.alt; } liga;",
            &tags,
        );
        assert_eq!(scripts.script_records.len(), 1);
        assert_eq!(scripts.script_records[0].script_tag, Tag::new(b"latn"));
        let script = &scripts.script_records[0].script;
        assert_eq!(script.lang_sys_records.len(), 1);
        assert_eq!(script.lang_sys_records[0].lang_sys_tag, Tag::new(b"TRK "));
    }
