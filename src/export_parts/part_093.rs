
fn build_script_list(source: &str, feature_tags: &[Tag]) -> layout::ScriptList {
    let default_script = Tag::new(b"DFLT");
    let mut assignments = BTreeMap::<(Tag, Option<Tag>), std::collections::BTreeSet<Tag>>::new();
    let mut required_assignments =
        BTreeMap::<(Tag, Option<Tag>), std::collections::BTreeSet<Tag>>::new();
    let mut global_defaults = std::collections::BTreeSet::<Tag>::new();
    let mut script_defaults = BTreeMap::<Tag, std::collections::BTreeSet<Tag>>::new();
    let mut excluded_defaults = std::collections::BTreeSet::<(Tag, Option<Tag>, Tag)>::new();
    let mut language_systems = Vec::<(Tag, Option<Tag>)>::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.first() != Some(&"languagesystem") {
            continue;
        }
        let (Some(script), Some(language)) = (
            tokens.get(1).and_then(|value| layout_tag(value)),
            tokens.get(2).and_then(|value| layout_language_tag(value)),
        ) else {
            continue;
        };
        let language = (!tokens
            .get(2)
            .is_some_and(|value| value.eq_ignore_ascii_case("dflt")))
        .then_some(language);
        if !language_systems.contains(&(script, language)) {
            language_systems.push((script, language));
        }
    }
    let mut has_explicit_scope = false;
    let source = expand_named_feature_lookups(source);
    for (feature_tag, block) in extract_feature_blocks(&source) {
        let mut script = default_script;
        let mut language = None;
        let mut required = false;
        let mut saw_script_or_language = false;
        let mut script_default_active = false;
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            match tokens.first().copied() {
                Some("script") => {
                    if let Some(tag) = tokens.get(1).and_then(|value| layout_tag(value)) {
                        script = tag;
                        language = None;
                        required = false;
                        saw_script_or_language = true;
                        script_default_active = true;
                        has_explicit_scope = true;
                    }
                }
                Some("language") => {
                    language = tokens.get(1).and_then(|value| {
                        (!value.eq_ignore_ascii_case("dflt"))
                            .then(|| layout_language_tag(value))
                            .flatten()
                    });
                    required = tokens
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case("required"));
                    saw_script_or_language = true;
                    has_explicit_scope = true;
                    if language.is_some()
                        && tokens.iter().any(|value| {
                            value.eq_ignore_ascii_case("exclude_dflt")
                                || value.eq_ignore_ascii_case("excludeDFLT")
                        })
                    {
                        excluded_defaults.insert((script, language, feature_tag));
                    }
                    if language.is_some() {
                        script_default_active = false;
                    }
                }
                Some("sub") | Some("reversesub") | Some("pos") | Some("ignore")
                | Some("lookup") => {
                    if required {
                        let key = (script, language);
                        required_assignments
                            .entry(key)
                            .or_default()
                            .insert(feature_tag);
                    }
                    if !saw_script_or_language {
                        global_defaults.insert(feature_tag);
                    } else if script_default_active {
                        script_defaults
                            .entry(script)
                            .or_default()
                            .insert(feature_tag);
                    } else {
                        let key = (script, language);
                        assignments.entry(key).or_default().insert(feature_tag);
                    }
                }
                _ => {}
            }
        }
    }
    let mut systems = language_systems.clone();
    systems.extend(assignments.keys().copied());
    systems.extend(script_defaults.keys().map(|script| (*script, None)));
    systems.sort_by_key(|(script, language)| {
        (script.to_be_bytes(), language.map(|tag| tag.to_be_bytes()))
    });
    systems.dedup();
    if systems.is_empty() {
        systems.push((default_script, None));
    }
    for (script, language) in systems {
        let key = (script, language);
        assignments.entry(key).or_default();
        let excluded = |tag: Tag| excluded_defaults.contains(&(script, language, tag));
        for tag in &global_defaults {
            if !excluded(*tag) {
                assignments.entry(key).or_default().insert(*tag);
            }
        }
        if let Some(defaults) = script_defaults.get(&script) {
            for tag in defaults {
                if !excluded(*tag) {
                    assignments.entry(key).or_default().insert(*tag);
                }
            }
        }
    }
    if !has_explicit_scope {
        let all = feature_tags
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for key in assignments.keys().copied().collect::<Vec<_>>() {
            assignments
                .entry(key)
                .or_default()
                .extend(all.iter().copied());
        }
    }
    let feature_index = |tag: &Tag| feature_tags.iter().position(|candidate| candidate == tag);
    let mut by_script = BTreeMap::<Tag, BTreeMap<Option<Tag>, Vec<u16>>>::new();
    for ((script, language), tags) in assignments {
        let indices = tags
            .iter()
            .filter_map(feature_index)
            .map(|index| index as u16)
            .collect::<Vec<_>>();
        by_script
            .entry(script)
            .or_default()
            .insert(language, indices);
    }
    let records = by_script
        .into_iter()
        .map(|(script_tag, languages)| {
            let make_lang_sys = |language: Option<Tag>, indices: Vec<u16>| {
                let mut lang_sys = layout::LangSys::new(indices);
                lang_sys.required_feature_index = required_assignments
                    .get(&(script_tag, language))
                    .and_then(|tags| tags.iter().next())
                    .and_then(feature_index)
                    .map(|index| index as u16)
                    .unwrap_or(0xFFFF);
                lang_sys
            };
            let default = languages
                .get(&None)
                .cloned()
                .map(|indices| make_lang_sys(None, indices));
            let language_records = languages
                .into_iter()
                .filter_map(|(language, indices)| {
                    language.map(|language| {
                        layout::LangSysRecord::new(language, make_lang_sys(Some(language), indices))
                    })
                })
                .collect();
            layout::ScriptRecord::new(script_tag, layout::Script::new(default, language_records))
        })
        .collect();
    layout::ScriptList::new(records)
}
