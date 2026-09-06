
/// Emit a conservative BASE table with the standard horizontal and vertical
/// baseline tags. The project's baseline is the font origin (0), which is the
/// interoperable fallback when no script-specific baseline metrics are stored.
fn build_base_table() -> Option<Vec<u8>> {
    let baseline_tags = vec![
        Tag::new(b"hang"),
        Tag::new(b"ideo"),
        Tag::new(b"math"),
        Tag::new(b"romn"),
    ];
    let coordinate = base::BaseCoord::Format1(base::BaseCoordFormat1::new(0));
    let make_script = || {
        base::BaseScript::new(
            Some(base::BaseValues::new(
                3,
                vec![
                    coordinate.clone(),
                    coordinate.clone(),
                    coordinate.clone(),
                    coordinate.clone(),
                ],
            )),
            None,
            Vec::new(),
        )
    };
    let scripts = base::BaseScriptList::new(
        [b"DFLT", b"hang", b"hani", b"kana", b"latn"]
            .into_iter()
            .map(|tag| base::BaseScriptRecord::new(Tag::new(tag), make_script()))
            .collect(),
    );
    let axis = || {
        base::Axis::new(
            Some(base::BaseTagList::new(baseline_tags.clone())),
            scripts.clone(),
        )
    };
    write_fonts::dump_table(&base::Base::new(Some(axis()), Some(axis()))).ok()
}
