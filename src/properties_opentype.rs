use super::*;

#[rustfmt::skip]
#[allow(clippy::too_many_arguments, clippy::ptr_arg, unused_variables)]
pub fn show_properties_opentype(
    ui: &mut Ui,
    properties_filter: &mut String,
    project: &mut FontProject,
    current_glyph: &Option<String>,
    component_base: &mut String,
    kerning_right: &mut String,
    kerning_pair_filter: &mut String,
    preview_text: &mut String,
    show_preview: &mut bool,
    feature_left: &mut String,
    feature_right: &mut String,
    feature_replacement: &mut String,
    feature_kerning_value: &mut String,
    feature_target_tag: &mut String,
    feature_anchor_x: &mut String,
    feature_anchor_y: &mut String,
    unicode_alias_input: &mut String,
    unicode_variation_selector: &mut String,
    current_master_id: &mut String,
    master_map_drag: &mut Option<String>,
    color_layer_glyph: &mut String,
    preview_color_palette: &mut usize,
    conditional_layer_axis: &mut String,
    conditional_layer_min: &mut String,
    conditional_layer_max: &mut String,
    conditional_layer_axis_2: &mut String,
    conditional_layer_min_2: &mut String,
    conditional_layer_max_2: &mut String,
    conditional_layer_axis_3: &mut String,
    conditional_layer_min_3: &mut String,
    conditional_layer_max_3: &mut String,
    conditional_layer_axis_4: &mut String,
    conditional_layer_min_4: &mut String,
    conditional_layer_max_4: &mut String,
    conditional_layer_extra: &mut Vec<(String, String, String)>,
) {
    let filter = properties_filter.trim().to_lowercase();
    let show_section = |keywords: &[&str]| {
        filter.is_empty()
            || keywords
                .iter()
                .any(|keyword| filter.contains(&keyword.to_lowercase()))
    };
    let opentype_search_hit =
        !filter.is_empty() && project.opentype_features.to_lowercase().contains(&filter);
    if show_section(&[
        "opentype",
        "open type",
        "feature",
        "class",
        "gsub",
        "gpos",
        "lookup",
        "languagesystem",
        "ss",
        "cv",
        "機能",
    ]) || opentype_search_hit
    {
        egui::CollapsingHeader::new("OpenType 機能").default_open(true).show(ui, |ui| {
            ui.label("クラス定義:");
            ui.add(egui::TextEdit::multiline(&mut project.opentype_classes).desired_rows(4).hint_text("@Upper = [A B C];"));
            if ui.small_button("Class定義をコピー").on_hover_text("Class定義だけをクリップボードへコピー").clicked() {
                ui.ctx().copy_text(project.opentype_classes.clone());
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("Class雛形:");
                for (label, template) in [("@Upper", "@Upper = [A B C];\n"), ("@Lower", "@Lower = [a b c];\n"), ("@Marks", "@Marks = [acute grave];\n")] {
                    let already_present = project.opentype_classes.lines().any(|line| line.trim_start().starts_with(label));
                    ui.add_enabled_ui(!already_present, |ui| {
                        if ui
                            .small_button(label)
                            .on_hover_text(if already_present {
                                "このClassは既に定義されています"
                            } else {
                                "Class定義へ雛形を追記"
                            })
                            .clicked()
                        {
                            project.opentype_classes.push_str(template);
                        }
                    });
                }
            });
            ui.label("Feature定義:");
            ui.add(egui::TextEdit::multiline(&mut project.opentype_features).desired_rows(6).hint_text("feature liga { ... } liga;"));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .small_button(".feaを読み込む…")
                    .on_hover_text("外部OpenType Feature Fileを読み込み、Featureソースへ置き換え")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().add_filter("OpenType Feature", &["fea"]).pick_file() {
                        if let Ok(source) = std::fs::read_to_string(path) {
                            let (classes, features) = split_feature_file_source(&source);
                            project.opentype_classes = classes;
                            project.opentype_features = features;
                        }
                    }
                }
                if ui.small_button(".feaを書き出す…").on_hover_text("ClassとFeatureを合成したFeature Fileを書き出す").clicked() {
                    if let Some(path) = rfd::FileDialog::new().set_file_name("features.fea").add_filter("OpenType Feature", &["fea"]).save_file() {
                        let _ = std::fs::write(path, project.feature_source());
                    }
                }
            });
            let feature_source = project.feature_source();
            match crate::core::validate_feature_source(&feature_source) {
                Ok(()) => {
                    ui.colored_label(egui::Color32::from_rgb(80, 170, 105), "Feature構文: OK");
                    let mut reference_issues = crate::core::validate_feature_class_definitions(&feature_source, &project.glyphs);
                    reference_issues.extend(crate::core::validate_feature_glyph_references(&feature_source, &project.glyphs));
                    reference_issues.sort();
                    reference_issues.dedup();
                    for issue in reference_issues.iter().take(4) {
                        ui.colored_label(egui::Color32::from_rgb(220, 155, 70), issue);
                    }
                    if reference_issues.len() > 4 {
                        ui.colored_label(egui::Color32::from_rgb(220, 155, 70), format!("ほか{}件の参照警告…", reference_issues.len() - 4));
                    }
                }
                Err(error) => {
                    ui.colored_label(egui::Color32::from_rgb(220, 95, 85), format!("Feature構文エラー: {error}"));
                }
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("構成雛形:");
                let languagesystem_template = "languagesystem DFLT dflt;\nlanguagesystem latn dflt;\n";
                if ui.small_button("languagesystem").on_hover_text("既定・Latin用のScript／Language宣言を追加").clicked() && !project.opentype_features.contains("languagesystem") {
                    if !project.opentype_features.trim().is_empty() {
                        project.opentype_features.insert(0, '\n');
                    }
                    project.opentype_features.insert_str(0, languagesystem_template);
                }
                let lookup_template = "lookup GS_Lookup {\n    # sub A by A.alt;\n} GS_Lookup;\n";
                if ui.small_button("名前付きLookup").on_hover_text("Featureから参照できる外部Lookupの雛形を追加").clicked() && !project.opentype_features.contains("lookup GS_Lookup")
                {
                    if !project.opentype_features.trim().is_empty() && !project.opentype_features.ends_with('\n') {
                        project.opentype_features.push('\n');
                    }
                    project.opentype_features.push_str(lookup_template);
                }
            });
            let feature_blocks = crate::core::extract_feature_blocks(&project.opentype_features);
            let mut feature_tags: Vec<String> = feature_blocks.iter().map(|(tag, _)| String::from_utf8_lossy(&tag.to_be_bytes()).to_string()).collect();
            feature_tags.sort();
            feature_tags.dedup();
            if !feature_tags.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("定義済み:");
                    for tag in &feature_tags {
                        if ui.small_button(egui::RichText::new(tag).monospace().strong()).on_hover_text("クリックで操作の挿入先に設定").clicked() {
                            *feature_target_tag = tag.clone();
                        }
                    }
                });
                ui.collapsing("コンパイル診断", |ui| {
                    ui.small("Feature本文を、出力される主な操作種別ごとに集計しています。");
                    for (tag, body) in &feature_blocks {
                        let tag = String::from_utf8_lossy(&tag.to_be_bytes()).to_string();
                        let mut substitutions = 0;
                        let mut positions = 0;
                        let mut lookups = 0;
                        for statement in body.split(';') {
                            let code = statement.lines().map(|line| line.split('#').next().unwrap_or_default()).collect::<Vec<_>>().join(" ");
                            let tokens = code.split_whitespace().collect::<Vec<_>>();
                            match tokens.first().copied() {
                                Some("sub") | Some("reversesub") => substitutions += 1,
                                Some("pos") => positions += 1,
                                Some("lookup") => lookups += 1,
                                _ => {}
                            }
                        }
                        ui.label(format!("{tag}: 置換 {substitutions} / 位置 {positions} / Lookup参照 {lookups}"));
                    }
                });
            }
            ui.collapsing("出力テーブル", |ui| {
                ui.small("次回のフォント書き出しで生成・保持されるOpenTypeテーブル");
                let mut generated = vec!["head", "hhea", "maxp", "hmtx", "cmap", "name"];
                if !project.opentype_features.trim().is_empty() {
                    generated.extend(["GSUB", "GPOS", "GDEF"]);
                }
                if project.masters.len() > 1 {
                    generated.extend(["fvar", "gvar", "STAT", "avar", "HVAR", "VVAR", "MVAR"]);
                }
                if !project.color_layers.is_empty() {
                    generated.extend(["COLR", "CPAL", "SVG "]);
                }
                generated.sort_unstable();
                generated.dedup();
                ui.horizontal_wrapped(|ui| {
                    for tag in generated {
                        ui.label(egui::RichText::new(format!("{tag} 生成")).monospace().color(egui::Color32::from_rgb(100, 190, 135)));
                    }
                    let mut preserved = project.preserved_tables.keys().collect::<Vec<_>>();
                    preserved.sort();
                    for tag in preserved {
                        ui.label(egui::RichText::new(format!("{tag} 保持")).monospace().color(egui::Color32::from_rgb(150, 165, 190)));
                    }
                });
            });
            if !project.preserved_tables.is_empty() {
                let mut preserved = project.preserved_tables.iter().map(|(tag, bytes)| (tag.clone(), bytes.len())).collect::<Vec<_>>();
                preserved.sort_by(|left, right| left.0.cmp(&right.0));
                ui.collapsing("未編集テーブル（再出力時保持）", |ui| {
                    ui.small("このアプリがまだ編集しないOpenType/AATテーブルはrawで保持されます。");
                    for (tag, size) in preserved {
                        ui.label(format!("{}  ·  {} bytes", tag.escape_default(), size));
                    }
                });
            }
            ui.label(egui::RichText::new("Feature雛形").strong());
            egui::ScrollArea::vertical().id_salt("opentype-feature-templates").max_height(190.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (label, template) in [
                        ("liga", "feature liga {\n    # sub f i by fi;\n} liga;\n"),
                        ("kern", "feature kern {\n    # pos A V <0 0 -80 0>;\n} kern;\n"),
                        ("mark", "feature mark {\n    # pos base A <anchor 300 700> mark @top;\n} mark;\n"),
                        ("mkmk", "feature mkmk {\n    # pos mark @top mark @top;\n} mkmk;\n"),
                        ("calt", "feature calt {\n    # sub A B by A.alt;\n} calt;\n"),
                        ("rvrn", "feature rvrn {\n    # sub A by A.alt;\n} rvrn;\n"),
                        ("ccmp", "feature ccmp {\n    # sub A acute by Aacute;\n} ccmp;\n"),
                        ("locl", "feature locl {\n    # sub i by i.loclTRK;\n} locl;\n"),
                        ("rlig", "feature rlig {\n    # sub f i by fi;\n} rlig;\n"),
                        ("clig", "feature clig {\n    # sub f f i by ffi;\n} clig;\n"),
                        ("salt", "feature salt {\n    # sub a by a.salt;\n} salt;\n"),
                        ("aalt", "feature aalt {\n    # sub A from [A.alt A.swash];\n} aalt;\n"),
                        ("frac", "feature frac {\n    # sub one slash four by one.numr slash four.dnom;\n} frac;\n"),
                        ("sups", "feature sups {\n    # sub one by one.superior;\n} sups;\n"),
                        ("subs", "feature subs {\n    # sub one by one.inferior;\n} subs;\n"),
                        ("case", "feature case {\n    # sub hyphen by hyphen.case;\n} case;\n"),
                        ("vert", "feature vert {\n    # sub parentheses by parentheses.vert;\n} vert;\n"),
                        ("vrt2", "feature vrt2 {\n    # sub parentheses by parentheses.vert;\n} vrt2;\n"),
                        ("size", "feature size {\n    parameters 12 1 8 72;\n} size;\n"),
                        ("curs", "feature curs {\n    # pos cursive A <anchor 0 0> <anchor 600 0>;\n} curs;\n"),
                        ("dist", "feature dist {\n    # pos A V' A <0 0 -20 0>;\n} dist;\n"),
                        ("dlig", "feature dlig {\n    # sub f f i by ffi;\n} dlig;\n"),
                        ("hlig", "feature hlig {\n    # sub f i by fi;\n} hlig;\n"),
                        ("smcp", "feature smcp {\n    # sub a by a.sc;\n} smcp;\n"),
                        ("c2sc", "feature c2sc {\n    # sub A by A.sc;\n} c2sc;\n"),
                        ("pcap", "feature pcap {\n    # sub a by a.caps;\n} pcap;\n"),
                        ("c2pc", "feature c2pc {\n    # sub A by A.caps;\n} c2pc;\n"),
                        ("onum", "feature onum {\n    # sub one by one.oldstyle;\n} onum;\n"),
                        ("lnum", "feature lnum {\n    # sub one.oldstyle by one;\n} lnum;\n"),
                        ("pnum", "feature pnum {\n    # sub one.tabular by one;\n} pnum;\n"),
                        ("tnum", "feature tnum {\n    # sub one by one.tabular;\n} tnum;\n"),
                        ("palt", "feature palt {\n    # sub A by A.proportional;\n} palt;\n"),
                        ("halt", "feature halt {\n    # sub A by A.half;\n} halt;\n"),
                        ("hwid", "feature hwid {\n    # sub A by A.halfwidth;\n} hwid;\n"),
                        ("fwid", "feature fwid {\n    # sub A by A.fullwidth;\n} fwid;\n"),
                        ("swsh", "feature swsh {\n    # sub a by a.swash;\n} swsh;\n"),
                        ("titl", "feature titl {\n    # sub A by A.title;\n} titl;\n"),
                        ("ornm", "feature ornm {\n    # sub a by a.ornament;\n} ornm;\n"),
                        ("rand", "feature rand {\n    # sub a from [a.one a.two];\n} rand;\n"),
                        ("ruby", "feature ruby {\n    # sub A by A.ruby;\n} ruby;\n"),
                        ("jalt", "feature jalt {\n    # sub A by A.jalt;\n} jalt;\n"),
                        ("jp78", "feature jp78 {\n    # sub kanji by kanji.jp78;\n} jp78;\n"),
                        ("jp83", "feature jp83 {\n    # sub kanji by kanji.jp83;\n} jp83;\n"),
                        ("jp90", "feature jp90 {\n    # sub kanji by kanji.jp90;\n} jp90;\n"),
                        ("hkna", "feature hkna {\n    # sub kana by kana.hk;\n} hkna;\n"),
                        ("vkna", "feature vkna {\n    # sub kana by kana.vk;\n} vkna;\n"),
                        ("vkrn", "feature vkrn {\n    # pos A V <0 0 -20 0>;\n} vkrn;\n"),
                        ("vpal", "feature vpal {\n    # sub A by A.vpal;\n} vpal;\n"),
                        ("valt", "feature valt {\n    # pos A <0 0 0 -20>;\n} valt;\n"),
                        ("ss01", "feature ss01 {\n    # sub a by a.ss01;\n} ss01;\n"),
                        ("cv01", "feature cv01 {\n    # sub a by a.cv01;\n} cv01;\n"),
                        ("ss02", "feature ss02 {\n    # sub a by a.ss02;\n} ss02;\n"),
                        ("ss03", "feature ss03 {\n    # sub a by a.ss03;\n} ss03;\n"),
                        ("ss04", "feature ss04 {\n    # sub a by a.ss04;\n} ss04;\n"),
                        ("ss05", "feature ss05 {\n    # sub a by a.ss05;\n} ss05;\n"),
                        ("ss06", "feature ss06 {\n    # sub a by a.ss06;\n} ss06;\n"),
                        ("ss07", "feature ss07 {\n    # sub a by a.ss07;\n} ss07;\n"),
                        ("ss08", "feature ss08 {\n    # sub a by a.ss08;\n} ss08;\n"),
                        ("ss09", "feature ss09 {\n    # sub a by a.ss09;\n} ss09;\n"),
                        ("ss10", "feature ss10 {\n    # sub a by a.ss10;\n} ss10;\n"),
                        ("ss11", "feature ss11 {\n    # sub a by a.ss11;\n} ss11;\n"),
                        ("ss12", "feature ss12 {\n    # sub a by a.ss12;\n} ss12;\n"),
                        ("ss13", "feature ss13 {\n    # sub a by a.ss13;\n} ss13;\n"),
                        ("ss14", "feature ss14 {\n    # sub a by a.ss14;\n} ss14;\n"),
                        ("ss15", "feature ss15 {\n    # sub a by a.ss15;\n} ss15;\n"),
                        ("ss16", "feature ss16 {\n    # sub a by a.ss16;\n} ss16;\n"),
                        ("ss17", "feature ss17 {\n    # sub a by a.ss17;\n} ss17;\n"),
                        ("ss18", "feature ss18 {\n    # sub a by a.ss18;\n} ss18;\n"),
                        ("ss19", "feature ss19 {\n    # sub a by a.ss19;\n} ss19;\n"),
                        ("ss20", "feature ss20 {\n    # sub a by a.ss20;\n} ss20;\n"),
                        ("cv02", "feature cv02 {\n    # sub a by a.cv02;\n} cv02;\n"),
                        ("cv03", "feature cv03 {\n    # sub a by a.cv03;\n} cv03;\n"),
                        ("cv04", "feature cv04 {\n    # sub a by a.cv04;\n} cv04;\n"),
                        ("cv05", "feature cv05 {\n    # sub a by a.cv05;\n} cv05;\n"),
                        ("cv06", "feature cv06 {\n    # sub a by a.cv06;\n} cv06;\n"),
                        ("cv07", "feature cv07 {\n    # sub a by a.cv07;\n} cv07;\n"),
                        ("cv08", "feature cv08 {\n    # sub a by a.cv08;\n} cv08;\n"),
                        ("cv09", "feature cv09 {\n    # sub a by a.cv09;\n} cv09;\n"),
                        ("cv10", "feature cv10 {\n    # sub a by a.cv10;\n} cv10;\n"),
                        ("cv11", "feature cv11 {\n    # sub a by a.cv11;\n} cv11;\n"),
                        ("cv12", "feature cv12 {\n    # sub a by a.cv12;\n} cv12;\n"),
                        ("cv13", "feature cv13 {\n    # sub a by a.cv13;\n} cv13;\n"),
                        ("cv14", "feature cv14 {\n    # sub a by a.cv14;\n} cv14;\n"),
                        ("cv15", "feature cv15 {\n    # sub a by a.cv15;\n} cv15;\n"),
                        ("cv16", "feature cv16 {\n    # sub a by a.cv16;\n} cv16;\n"),
                        ("cv17", "feature cv17 {\n    # sub a by a.cv17;\n} cv17;\n"),
                        ("cv18", "feature cv18 {\n    # sub a by a.cv18;\n} cv18;\n"),
                        ("cv19", "feature cv19 {\n    # sub a by a.cv19;\n} cv19;\n"),
                        ("cv20", "feature cv20 {\n    # sub a by a.cv20;\n} cv20;\n"),
                    ] {
                        let already_present = project.opentype_features.lines().any(|line| line.trim_start().starts_with(&format!("feature {label}")));
                        ui.add_enabled_ui(!already_present, |ui| {
                            if ui
                                .small_button(label)
                                .on_hover_text(if already_present {
                                    "このFeatureは既に定義されています"
                                } else {
                                    "雛形をFeature本文へ追記"
                                })
                                .clicked()
                            {
                                if !project.opentype_features.trim().is_empty() {
                                    project.opentype_features.push('\n');
                                }
                                project.opentype_features.push_str(template);
                            }
                        });
                    }
                });
            });
            ui.separator();
            ui.label(egui::RichText::new("よく使う操作を追加").strong());
            ui.small("適切なFeature本文へサンプル行を追加します。未定義ならFeature自体を作成します。");
            ui.horizontal_wrapped(|ui| {
                ui.label("左:");
                ui.add(egui::TextEdit::singleline(feature_left).desired_width(90.0));
                ui.label("右:");
                ui.add(egui::TextEdit::singleline(feature_right).desired_width(90.0));
                ui.label("置換先／Markクラス:");
                ui.add(egui::TextEdit::singleline(feature_replacement).desired_width(90.0));
                ui.label("値:");
                ui.add(egui::TextEdit::singleline(feature_kerning_value).desired_width(60.0));
                ui.label("Mark X/Y:");
                ui.add(egui::TextEdit::singleline(feature_anchor_x).desired_width(45.0));
                ui.add(egui::TextEdit::singleline(feature_anchor_y).desired_width(45.0));
                ui.label("挿入先:");
                ui.add(egui::TextEdit::singleline(feature_target_tag).desired_width(55.0));
            });
            if let Some(name) = current_glyph {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button(format!("「{}」を左へ", name)).on_hover_text("編集中のグリフ名を左グリフ欄へコピー").clicked() {
                        *feature_left = name.clone();
                    }
                    if ui.small_button("現在のグリフを右へ").on_hover_text("編集中のグリフ名を右グリフ欄へコピー").clicked() {
                        *feature_right = name.clone();
                    }
                    if ui.small_button("現在のグリフを置換先へ").on_hover_text("編集中のグリフ名を置換先欄へコピー").clicked() {
                        *feature_replacement = name.clone();
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                ui.label("プリセット:");
                for tag in ["liga", "kern", "mark", "mkmk", "calt", "rvrn", "ccmp", "locl", "rlig", "salt", "frac", "sups", "subs", "vert", "ss01"] {
                    if ui.small_button(tag).clicked() {
                        *feature_target_tag = tag.to_string();
                    }
                }
            });
            ui.label(egui::RichText::new(format!("現在の挿入先: {}", feature_target_tag.trim())).monospace().strong());
            ui.small("Mark系は既存の @クラス名（例: @top）を置換先／Markクラス欄へ入力します。");
            if ui.small_button("入力をクリア").on_hover_text("操作パレットのグリフ名・数値を空にする").clicked() {
                feature_left.clear();
                feature_right.clear();
                feature_replacement.clear();
                feature_kerning_value.clear();
                feature_anchor_x.clear();
                feature_anchor_y.clear();
            }
            let left_exists = project.glyphs.iter().any(|(_, glyph)| glyph.name == feature_left.trim());
            let right_exists = project.glyphs.iter().any(|(_, glyph)| glyph.name == feature_right.trim());
            let replacement_exists = project.glyphs.iter().any(|(_, glyph)| glyph.name == feature_replacement.trim());
            ui.horizontal_wrapped(|ui| {
                for (label, value, exists) in [
                    ("左", feature_left.as_str(), left_exists),
                    ("右", feature_right.as_str(), right_exists),
                    ("置換先", feature_replacement.as_str(), replacement_exists),
                ] {
                    if !value.trim().is_empty() && !value.trim_start().starts_with('@') {
                        ui.colored_label(
                            if exists { egui::Color32::from_rgb(80, 190, 100) } else { egui::Color32::from_rgb(220, 160, 60) },
                            format!("{}: {} {}", label, value.trim(), if exists { "✓" } else { "未登録" }),
                        );
                    }
                }
            });
            let feature_inputs_ready = !feature_left.trim().is_empty() && !feature_replacement.trim().is_empty();
            let kerning_value_ready = feature_kerning_value.trim().parse::<i32>().is_ok();
            let anchor_ready = feature_anchor_x.trim().parse::<i32>().is_ok() && feature_anchor_y.trim().parse::<i32>().is_ok();
            let target_tag = feature_target_tag.trim();
            let feature_tag_ready = target_tag.chars().count() == 4 && target_tag.chars().all(|ch| ch.is_ascii_alphanumeric());
            if !feature_inputs_ready {
                ui.colored_label(egui::Color32::from_rgb(220, 160, 60), "左グリフと置換先を入力すると操作を追加できます（合字は右グリフも必要）");
            }
            if !kerning_value_ready {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 80), "カーニング値は整数で入力してください");
            }
            if !anchor_ready {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 80), "Mark X/Yは整数で入力してください");
            }
            if !feature_tag_ready {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 80), "挿入先Featureタグは英数字4文字で入力してください");
            }
            ui.horizontal_wrapped(|ui| {
                for (label, operation, help) in [
                    ("置換", "sub", "単一グリフを別グリフへ置換"),
                    ("合字", "ligature", "複数グリフを合字へ置換"),
                    ("カーニング", "kern", "2グリフの横位置を調整"),
                    ("例外を無視", "ignore", "条件に一致する置換を抑制"),
                    ("位置例外を無視", "ignore_pos", "条件に一致する位置調整を抑制"),
                    ("Mark位置", "mark", "BaseグリフへMarkアンカー位置のサンプルを追加"),
                    ("Mark-to-Mark", "mkmk", "Mark同士のアンカー位置サンプルを追加"),
                ] {
                    let operation_ready = feature_inputs_ready
                        && (operation != "ligature" || !feature_right.trim().is_empty())
                        && (operation != "kern" || kerning_value_ready)
                        && (operation != "mark" || anchor_ready)
                        && (matches!(operation, "mark" | "mkmk") || feature_tag_ready);
                    let response = ui
                        .add_enabled(operation_ready, egui::Button::new(label))
                        .on_hover_text(if operation_ready { help } else { "左・右・置換先を入力してください" });
                    if response.clicked() {
                        let operation_kind = operation;
                        let mut mark_class = normalize_mark_class(feature_replacement);
                        if matches!(operation_kind, "mark" | "mkmk") && !feature_replacement.trim().starts_with('@') {
                            if let Some(auto_class) = ensure_mark_class_for_glyph(project, feature_replacement) {
                                mark_class = auto_class;
                            }
                        }
                        let operation = match operation_kind {
                            "sub" => {
                                format!("    sub {} by {};\n", feature_left, feature_replacement)
                            }
                            "ligature" => format!("    sub {} {} by {};\n", feature_left, feature_right, feature_replacement),
                            "kern" => {
                                let value = feature_kerning_value.trim().parse::<i32>().unwrap_or_default();
                                format!("    pos {} {} <0 0 {} 0>;\n", feature_left, feature_right, value)
                            }
                            "mark" => format!("    pos base {} <anchor {} {}> mark {};\n", feature_left, feature_anchor_x, feature_anchor_y, mark_class),
                            "mkmk" => format!("    pos mark {} mark {};\n", mark_class, mark_class),
                            "ignore_pos" => {
                                format!("    ignore pos {} {};\n", feature_left, feature_right)
                            }
                            _ => "    ignore sub @Upper @Lower;\n".to_string(),
                        };
                        insert_feature_operation(
                            &mut project.opentype_features,
                            if operation_kind == "mark" {
                                "mark"
                            } else if operation_kind == "mkmk" {
                                "mkmk"
                            } else {
                                target_tag
                            },
                            &operation,
                        );
                    }
                }
            });
            if feature_inputs_ready {
                let preview = format!(
                    "sub {} by {};  /  pos {} {} <0 0 {} 0>;",
                    feature_left.trim(),
                    feature_replacement.trim(),
                    feature_left.trim(),
                    feature_right.trim(),
                    feature_kerning_value.trim()
                );
                ui.label(egui::RichText::new(format!("生成プレビュー: {preview}")).monospace());
                ui.label(
                    egui::RichText::new(format!(
                        "Mark: pos base {} <anchor {} {}> mark {};  /  mkmk: pos mark {} mark {};",
                        feature_left.trim(),
                        feature_anchor_x.trim(),
                        feature_anchor_y.trim(),
                        normalize_mark_class(feature_replacement),
                        normalize_mark_class(feature_replacement),
                        normalize_mark_class(feature_replacement)
                    ))
                    .monospace(),
                );
            }
            let feature_source = project.feature_source();
            match crate::core::validate_feature_source(&feature_source) {
                Ok(()) => {
                    ui.colored_label(egui::Color32::from_rgb(80, 190, 100), "✓ Feature syntax OK");
                    for issue in crate::core::validate_feature_class_definitions(&feature_source, &project.glyphs) {
                        ui.colored_label(egui::Color32::from_rgb(220, 160, 60), issue);
                    }
                    for issue in crate::core::validate_feature_glyph_references(&feature_source, &project.glyphs) {
                        ui.colored_label(egui::Color32::from_rgb(220, 160, 60), issue);
                    }
                }
                Err(error) => {
                    ui.colored_label(egui::Color32::from_rgb(220, 90, 80), error);
                }
            }
            egui::CollapsingHeader::new("書き出し時の合成ソース").default_open(false).show(ui, |ui| {
                if ui.small_button("合成ソースをコピー").on_hover_text("書き出し時のClass＋Featureをクリップボードへコピー").clicked() {
                    ui.ctx().copy_text(feature_source.clone());
                }
                let mut source_preview = feature_source.clone();
                ui.add(egui::TextEdit::multiline(&mut source_preview).desired_rows(8).interactive(false))
                    .on_hover_text("Class定義とFeature定義を連結した、書き出し時の内容");
            });
        });
    }
}
