    #[test]
    fn exclude_dflt_omits_global_default_feature_from_language() {
        let tags = vec![Tag::new(b"liga"), Tag::new(b"locl")];
        let source = "languagesystem DFLT dflt; languagesystem latn dflt; languagesystem latn DEU; feature liga { sub A by A.alt; script latn; language DEU excludeDFLT; } liga;";
        let scripts = build_script_list(source, &tags);
        let script = scripts
            .script_records
            .iter()
            .find(|record| record.script_tag == Tag::new(b"latn"))
            .expect("latn script should be emitted");
        assert!(script.script.default_lang_sys.is_some());
        let deu = script
            .script
            .lang_sys_records
            .iter()
            .find(|record| record.lang_sys_tag == Tag::new(b"DEU "))
            .expect("DEU language should be emitted");
        assert!(deu.lang_sys.feature_indices.is_empty());
    }
