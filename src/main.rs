use glyph_studio::app::GlyphStudioApp;
use glyph_studio::{export, font_data, io};
use std::path::{Path, PathBuf};

fn main() -> eframe::Result<()> {
    env_logger::init();

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let initial_document = arguments
        .first()
        .filter(|_| arguments.len() == 1)
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .filter(|path| is_supported_document(path));

    if initial_document.is_none() && !arguments.is_empty() {
        if let Err(error) = run_cli(&arguments) {
            eprintln!("glyph-studio: {error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Glyph Studio - フォント制作ツール"),
        ..Default::default()
    };

    eframe::run_native(
        "Glyph Studio",
        options,
        Box::new(move |cc| {
            let mut app = GlyphStudioApp::new(cc);
            if let Some(path) = initial_document.as_deref() {
                if let Err(error) = app.open_document_path(path) {
                    app.status_message = format!("ファイルを開けませんでした: {error}");
                }
            }
            Ok(Box::new(app))
        }),
    )
}

fn is_supported_document(path: &Path) -> bool {
    path.is_dir()
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "json" | "glyphs" | "ufo" | "ttf" | "otf" | "woff" | "woff2"
                )
            })
}

fn run_cli(arguments: &[String]) -> Result<(), String> {
    match arguments.first().map(String::as_str) {
        Some("validate") => {
            let input = arguments
                .get(1)
                .ok_or("使い方: glyph-studio validate <project.json|ufo>")?;
            let project = load_cli_project(Path::new(input))?;
            let issues = export::validate_project_detailed(&project);
            let json_output = arguments.iter().any(|argument| argument == "--json");
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&issues)
                        .map_err(|error| format!("検証結果のJSON化に失敗しました: {error}"))?
                );
                return if issues.is_empty() {
                    Ok(())
                } else {
                    Err("検証に失敗しました".into())
                };
            }
            if issues.is_empty() {
                println!("検証OK: {}グリフ", project.glyphs.len());
                Ok(())
            } else {
                for issue in issues {
                    match issue.glyph_name {
                        Some(glyph) => println!("ERROR [{glyph}] {}", issue.message),
                        None => println!("ERROR {}", issue.message),
                    }
                }
                Err("検証に失敗しました".into())
            }
        }
        Some("export" | "build") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio export <project.json|glyphs|ufo> <output.ttf|otf|woff|woff2|glyphs>",
            )?;
            let output = arguments
                .iter()
                .skip(2)
                .find(|argument| argument.as_str() != "--variable")
                .ok_or("出力先が指定されていません")?;
            let project = load_cli_project(Path::new(input))?;
            let output_path = Path::new(output);
            if output_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("glyphs"))
            {
                io::save_glyphs(&project, output_path)?;
                println!("Glyphs形式で書き出しました: {}", output_path.display());
                return Ok(());
            }
            if arguments[0] == "build" {
                glyph_studio::core::build(&project, output_path)?;
            } else {
                let issues = export::validate_project(&project);
                if !issues.is_empty() {
                    return Err(format!(
                        "書き出し前の検証に失敗しました: {}",
                        issues.join("; ")
                    ));
                }
                export::export_by_extension(&project, output_path)?;
            }
            println!("ビルドしました: {}", output_path.display());
            Ok(())
        }
        Some("export-cff2") => {
            let input = arguments
                .get(1)
                .ok_or("使い方: glyph-studio export-cff2 <project.json|glyphs|ufo> <output.otf>")?;
            let output = arguments.get(2).ok_or("出力先が指定されていません")?;
            let project = load_cli_project(Path::new(input))?;
            let issues = export::validate_project(&project);
            if !issues.is_empty() {
                return Err(format!(
                    "書き出し前の検証に失敗しました: {}",
                    issues.join("; ")
                ));
            }
            export::export_otf_cff2(&project, Path::new(output))?;
            println!("CFF2 OTFを書き出しました: {output}");
            Ok(())
        }
        Some("rename-glyph") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio rename-glyph <project.json> <old> <new> [output.json]",
            )?;
            let old_name = arguments.get(2).ok_or("変更前のグリフ名がありません")?;
            let new_name = arguments.get(3).ok_or("変更後のグリフ名がありません")?;
            let output = arguments.get(4).unwrap_or(input);
            if !output.ends_with(".json") {
                return Err("rename-glyphの出力先はjsonプロジェクトを指定してください".into());
            }
            let mut project = load_cli_project(Path::new(input))?;
            if !project.rename_glyph(old_name, new_name.clone()) {
                return Err(format!(
                    "グリフ '{}' は存在しないか、変更後の名前が重複しています",
                    old_name
                ));
            }
            io::save_project(&project, Path::new(output))?;
            println!("グリフ名を変更しました: {old_name} → {new_name}");
            Ok(())
        }
        Some("move-master") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio move-master <project.json> <master-id> <delta> [output.json]",
            )?;
            let master_id = arguments.get(2).ok_or("マスターIDがありません")?;
            let delta = arguments
                .get(3)
                .ok_or("移動量がありません")?
                .parse::<isize>()
                .map_err(|_| "移動量は整数で指定してください".to_string())?;
            let output = arguments.get(4).unwrap_or(input);
            if !output.ends_with(".json") {
                return Err("move-masterの出力先はjsonプロジェクトを指定してください".into());
            }
            let mut project = load_cli_project(Path::new(input))?;
            if !project.move_master(master_id, delta) {
                return Err(format!(
                    "マスター '{}' を移動できません（ID、移動量、順序を確認してください）",
                    master_id
                ));
            }
            io::save_project(&project, Path::new(output))?;
            println!("マスターの順序を変更しました: {master_id} ({delta:+}) → {output}");
            Ok(())
        }
        Some("duplicate-master") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio duplicate-master <project.json> <master-id> [output.json]",
            )?;
            let master_id = arguments.get(2).ok_or("マスターIDがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            if !output.ends_with(".json") {
                return Err("duplicate-masterの出力先はjsonプロジェクトを指定してください".into());
            }
            let mut project = load_cli_project(Path::new(input))?;
            let new_id = project
                .duplicate_master(master_id)
                .ok_or_else(|| format!("マスター '{}' がありません", master_id))?;
            io::save_project(&project, Path::new(output))?;
            println!("マスターを複製しました: {master_id} → {new_id} → {output}");
            Ok(())
        }
        Some("set-kerning") => {
            let input = arguments
                .get(1)
                .ok_or("使い方: glyph-studio set-kerning <project.json> <left> <right> <value> [output.json]")?;
            let left = arguments.get(2).ok_or("左キーがありません")?;
            let right = arguments.get(3).ok_or("右キーがありません")?;
            let value = arguments
                .get(4)
                .ok_or("カーニング値がありません")?
                .parse::<f64>()
                .map_err(|_| "カーニング値は数値で指定してください".to_string())?;
            let output = arguments.get(5).unwrap_or(input);
            let mut project = load_cli_project(Path::new(input))?;
            project.set_kerning_pair(left, right, value)?;
            io::save_project(&project, Path::new(output))?;
            println!("カーニングを設定しました: {left} / {right} = {value}");
            Ok(())
        }
        Some("set-kerning-master") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-kerning-master <project.json> <master-id> <left> <right> <value> [output.json]",
            )?;
            let master_id = arguments.get(2).ok_or("マスターIDがありません")?;
            let left = arguments.get(3).ok_or("左キーがありません")?;
            let right = arguments.get(4).ok_or("右キーがありません")?;
            let value = arguments
                .get(5)
                .ok_or("カーニング値がありません")?
                .parse::<f64>()
                .map_err(|_| "カーニング値は数値で指定してください".to_string())?;
            let output = arguments.get(6).unwrap_or(input);
            if !output.ends_with(".json") {
                return Err(
                    "set-kerning-masterの出力先はjsonプロジェクトを指定してください".into(),
                );
            }
            let mut project = load_cli_project(Path::new(input))?;
            project.set_kerning_pair_for_master(master_id, left, right, value)?;
            io::save_project(&project, Path::new(output))?;
            println!(
                "マスター '{}' のカーニングを設定しました: {left} / {right} = {value}",
                master_id
            );
            Ok(())
        }
        Some("duplicate-component") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio duplicate-component <project.json> <glyph> <index> [output.json]",
            )?;
            let glyph = arguments.get(2).ok_or("グリフ名がありません")?;
            let index = arguments
                .get(3)
                .ok_or("コンポーネント番号がありません")?
                .parse::<usize>()
                .map_err(|_| "コンポーネント番号は整数で指定してください".to_string())?;
            let output = arguments.get(4).unwrap_or(input);
            let mut project = load_cli_project(Path::new(input))?;
            if !project.duplicate_component_all_layers(glyph, index) {
                return Err(format!(
                    "{glyph} のコンポーネント番号 {index} は存在しません"
                ));
            }
            io::save_project(&project, Path::new(output))?;
            println!("コンポーネントを複製しました: {glyph}[{index}] → {output}");
            Ok(())
        }
        Some("align-component") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio align-component <project.json> <glyph> <index> [output.json]",
            )?;
            let glyph = arguments.get(2).ok_or("グリフ名がありません")?;
            let index = arguments
                .get(3)
                .ok_or("コンポーネント番号がありません")?
                .parse::<usize>()
                .map_err(|_| "コンポーネント番号は整数で指定してください".to_string())?;
            let output = arguments.get(4).unwrap_or(input);
            let mut project = load_cli_project(Path::new(input))?;
            if !project.glyphs.contains_key(glyph) {
                return Err(format!("グリフ '{glyph}' は存在しません"));
            }
            if !project.align_component_anchors_all_layers(glyph, index) {
                return Err(format!(
                    "{glyph} のコンポーネントをアンカー位置合わせできません"
                ));
            }
            io::save_project(&project, Path::new(output))?;
            println!("コンポーネントをアンカー位置合わせしました: {glyph}[{index}] → {output}");
            Ok(())
        }
        Some("align-components") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio align-components <project.json> <glyph> [output.json]",
            )?;
            let glyph = arguments.get(2).ok_or("グリフ名がありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let mut project = load_cli_project(Path::new(input))?;
            if !project.glyphs.contains_key(glyph) {
                return Err(format!("グリフ '{glyph}' は存在しません"));
            }
            let glyph_names = vec![glyph.to_string()];
            let count = project.align_all_component_anchors(&glyph_names);
            if count == 0 {
                return Err(format!(
                    "{glyph} に位置合わせ可能なコンポーネントがありません"
                ));
            }
            io::save_project(&project, Path::new(output))?;
            println!(
                "{glyph} のコンポーネント{}件をアンカー位置合わせしました: {output}",
                count
            );
            Ok(())
        }
        Some("align-components-batch") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio align-components-batch <project.json> <glyphs.txt> [output.json]",
            )?;
            let names_path = arguments.get(2).ok_or("グリフ名リストがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let names = std::fs::read_to_string(names_path)
                .map_err(|error| format!("グリフ名リストを読み込めません: {error}"))?
                .lines()
                .map(str::trim)
                .filter(|name| !name.is_empty() && !name.starts_with('#'))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if names.is_empty() {
                return Err("グリフ名リストに処理対象がありません".into());
            }
            let mut project = load_cli_project(Path::new(input))?;
            let missing = names
                .iter()
                .filter(|name| !project.glyphs.contains_key(*name))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(format!("存在しないグリフ: {}", missing.join(", ")));
            }
            let count = project.align_all_component_anchors(&names);
            io::save_project(&project, Path::new(output))?;
            println!(
                "{}グリフのコンポーネント{}件を位置合わせしました: {output}",
                names.len(),
                count
            );
            Ok(())
        }
        Some("apply-metrics-keys") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio apply-metrics-keys <project.json> <glyphs.txt> [output.json]",
            )?;
            let names_path = arguments.get(2).ok_or("グリフ名リストがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let names = std::fs::read_to_string(names_path)
                .map_err(|error| format!("グリフ名リストを読み込めません: {error}"))?
                .lines()
                .map(str::trim)
                .filter(|name| !name.is_empty() && !name.starts_with('#'))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if names.is_empty() {
                return Err("グリフ名リストに処理対象がありません".into());
            }
            let mut project = load_cli_project(Path::new(input))?;
            let missing = names
                .iter()
                .filter(|name| !project.glyphs.contains_key(*name))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(format!("存在しないグリフ: {}", missing.join(", ")));
            }
            let count = project.apply_metrics_keys(&names)?;
            io::save_project(&project, Path::new(output))?;
            println!("{}グリフのメトリクスキーを適用しました: {output}", count);
            Ok(())
        }
        Some("set-kerning-batch") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-kerning-batch <project.json> <pairs.tsv> [output.json]",
            )?;
            let pairs_path = arguments.get(2).ok_or("ペアファイルがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let contents = std::fs::read_to_string(pairs_path)
                .map_err(|error| format!("ペアファイルを読み込めません: {error}"))?;
            let mut pairs = Vec::new();
            for (line_number, line) in contents.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() != 3 {
                    return Err(format!(
                        "{}行目: TSVは左キー・右キー・値の3列です",
                        line_number + 1
                    ));
                }
                let value = fields[2].parse::<f64>().map_err(|_| {
                    format!("{}行目: カーニング値が数値ではありません", line_number + 1)
                })?;
                pairs.push((fields[0], fields[1], value));
            }
            let mut project = load_cli_project(Path::new(input))?;
            let count = project.set_kerning_pairs(pairs)?;
            io::save_project(&project, Path::new(output))?;
            println!("カーニング{}件を一括設定しました: {output}", count);
            Ok(())
        }
        Some("set-kerning-master-batch") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-kerning-master-batch <project.json> <master-id> <pairs.tsv> [output.json]",
            )?;
            let master_id = arguments.get(2).ok_or("マスターIDがありません")?;
            let pairs_path = arguments.get(3).ok_or("ペアファイルがありません")?;
            let output = arguments.get(4).unwrap_or(input);
            let contents = std::fs::read_to_string(pairs_path)
                .map_err(|error| format!("ペアファイルを読み込めません: {error}"))?;
            let mut pairs = Vec::new();
            for (line_number, line) in contents.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() != 3 {
                    return Err(format!(
                        "{}行目: TSVは左キー・右キー・値の3列です",
                        line_number + 1
                    ));
                }
                let value = fields[2].parse::<f64>().map_err(|_| {
                    format!("{}行目: カーニング値が数値ではありません", line_number + 1)
                })?;
                pairs.push((fields[0].to_string(), fields[1].to_string(), value));
            }
            let mut project = load_cli_project(Path::new(input))?;
            for (left, right, value) in &pairs {
                if !value.is_finite() || left.trim().is_empty() || right.trim().is_empty() {
                    return Err("カーニングTSVに不正な値があります".into());
                }
            }
            for (left, right, value) in pairs {
                project.set_kerning_pair_for_master(master_id, left, right, value)?;
            }
            io::save_project(&project, Path::new(output))?;
            println!(
                "マスター '{}' のカーニングを一括設定しました: {output}",
                master_id
            );
            Ok(())
        }
        Some("set-sidebearings-batch") => {
            let input = arguments.get(1).ok_or("使い方: glyph-studio set-sidebearings-batch <project.json> <bearings.tsv> [output.json]")?;
            let bearings_path = arguments.get(2).ok_or("余白ファイルがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let contents = std::fs::read_to_string(bearings_path)
                .map_err(|error| format!("余白ファイルを読み込めません: {error}"))?;
            let mut rows = Vec::new();
            for (line_number, line) in contents.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() != 3 {
                    return Err(format!(
                        "{}行目: TSVはグリフ名・左余白・右余白の3列です",
                        line_number + 1
                    ));
                }
                let left = fields[1]
                    .parse::<f64>()
                    .map_err(|_| format!("{}行目: 左余白が数値ではありません", line_number + 1))?;
                let right = fields[2]
                    .parse::<f64>()
                    .map_err(|_| format!("{}行目: 右余白が数値ではありません", line_number + 1))?;
                if !left.is_finite() || !right.is_finite() {
                    return Err(format!("{}行目: 余白が不正です", line_number + 1));
                }
                rows.push((fields[0].to_string(), left, right));
            }
            let mut project = load_cli_project(Path::new(input))?;
            let count = project.set_side_bearings_batch(rows)?;
            io::save_project(&project, Path::new(output))?;
            println!("左右余白{}件を一括設定しました: {output}", count);
            Ok(())
        }
        Some("set-width-batch") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-width-batch <project.json> <widths.tsv> [output.json]",
            )?;
            let widths_path = arguments.get(2).ok_or("字幅ファイルがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let contents = std::fs::read_to_string(widths_path)
                .map_err(|error| format!("字幅ファイルを読み込めません: {error}"))?;
            let mut rows = Vec::new();
            for (line_number, line) in contents.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() != 2 {
                    return Err(format!(
                        "{}行目: TSVはグリフ名・字幅の2列です",
                        line_number + 1
                    ));
                }
                let width = fields[1]
                    .parse::<f64>()
                    .map_err(|_| format!("{}行目: 字幅が数値ではありません", line_number + 1))?;
                if !width.is_finite() || width < 0.0 {
                    return Err(format!("{}行目: 字幅が不正です", line_number + 1));
                }
                rows.push((fields[0].to_string(), width));
            }
            let mut project = load_cli_project(Path::new(input))?;
            let count = project.set_widths_batch(rows)?;
            io::save_project(&project, Path::new(output))?;
            println!("字幅{}件を一括設定しました: {output}", count);
            Ok(())
        }
        Some("set-unicode-batch") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-unicode-batch <project.json> <unicode.tsv> [output.json]",
            )?;
            let unicode_path = arguments.get(2).ok_or("Unicodeファイルがありません")?;
            let output = arguments.get(3).unwrap_or(input);
            let contents = std::fs::read_to_string(unicode_path)
                .map_err(|error| format!("Unicodeファイルを読み込めません: {error}"))?;
            let mut assignments = Vec::new();
            for (line_number, line) in contents.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() != 2 {
                    return Err(format!(
                        "{}行目: TSVはグリフ名・Unicodeの2列です",
                        line_number + 1
                    ));
                }
                let code = fields[1]
                    .trim()
                    .strip_prefix("U+")
                    .or_else(|| fields[1].trim().strip_prefix("u+"))
                    .unwrap_or(fields[1].trim());
                let unicode = u32::from_str_radix(code, 16)
                    .map_err(|_| format!("{}行目: Unicodeが不正です", line_number + 1))?;
                if char::from_u32(unicode).is_none() {
                    return Err(format!(
                        "{}行目: Unicodeがコードポイント範囲外です",
                        line_number + 1
                    ));
                }
                assignments.push((fields[0].to_string(), unicode));
            }
            let mut project = load_cli_project(Path::new(input))?;
            let count = project.set_unicode_assignments_strict(&assignments)?;
            io::save_project(&project, Path::new(output))?;
            println!("Unicode{}件を一括設定しました: {output}", count);
            Ok(())
        }
        Some("set-opentype-source") => {
            let input = arguments.get(1).ok_or(
                "使い方: glyph-studio set-opentype-source <project.json> <classes.txt> <features.txt> [output.json]",
            )?;
            let classes_path = arguments.get(2).ok_or("Class定義ファイルがありません")?;
            let features_path = arguments.get(3).ok_or("Feature定義ファイルがありません")?;
            let output = arguments.get(4).unwrap_or(input);
            if !output.ends_with(".json") {
                return Err(
                    "set-opentype-sourceの出力先はjsonプロジェクトを指定してください".into(),
                );
            }
            let classes = std::fs::read_to_string(classes_path)
                .map_err(|error| format!("Class定義を読み込めません: {error}"))?;
            let features = std::fs::read_to_string(features_path)
                .map_err(|error| format!("Feature定義を読み込めません: {error}"))?;
            let mut project = load_cli_project(Path::new(input))?;
            glyph_studio::core::set_opentype_source(&mut project, classes, features)?;
            io::save_project(&project, Path::new(output))?;
            println!("OpenType Class／Featureを設定しました: {output}");
            Ok(())
        }
        Some("check-interpolation") => {
            let input = arguments
                .get(1)
                .ok_or("使い方: glyph-studio check-interpolation <project.json|ufo> <from-master> <to-master>")?;
            let from = arguments.get(2).ok_or("始点マスターがありません")?;
            let to = arguments.get(3).ok_or("終点マスターがありません")?;
            let project = load_cli_project(Path::new(input))?;
            let issues = export::validate_interpolation(&project, from, to);
            if arguments.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&issues)
                        .map_err(|error| format!("補間検証結果のJSON化に失敗しました: {error}"))?
                );
                return if issues.is_empty() {
                    Ok(())
                } else {
                    Err("補間チェックに失敗しました".into())
                };
            }
            if issues.is_empty() {
                println!("補間チェックOK: {}グリフ", project.glyphs.len());
                Ok(())
            } else {
                for issue in issues {
                    match issue.glyph_name {
                        Some(glyph) => println!("ERROR [{glyph}] {}", issue.message),
                        None => println!("ERROR {}", issue.message),
                    }
                }
                Err("補間できないグリフがあります".into())
            }
        }
        Some("help" | "--help" | "-h") => {
            println!(
                "Glyph Studio CLI\n\n  glyph-studio validate <project.json|ufo> [--json]\n  glyph-studio export <project.json|ufo> [--variable] <output.ttf|otf|woff|woff2>\n  glyph-studio build <project.json|ufo> <output.ttf|otf|woff|woff2>\n  glyph-studio rename-glyph <project.json> <old> <new> [output.json]\n  glyph-studio move-master <project.json> <master-id> <delta> [output.json]\n  glyph-studio duplicate-master <project.json> <master-id> [output.json]\n  glyph-studio set-kerning <project.json> <left> <right> <value> [output.json]\n  glyph-studio set-kerning-batch <project.json> <pairs.tsv> [output.json]\n  glyph-studio set-kerning-master-batch <project.json> <master-id> <pairs.tsv> [output.json]\n  glyph-studio set-sidebearings-batch <project.json> <bearings.tsv> [output.json]\n  glyph-studio check-interpolation <project.json|ufo> <from-master> <to-master> [--json]"
            );
            println!("  glyph-studio set-width-batch <project.json> <widths.tsv> [output.json]");
            println!("  glyph-studio set-unicode-batch <project.json> <unicode.tsv> [output.json]");
            println!("  glyph-studio set-opentype-source <project.json> <classes.txt> <features.txt> [output.json]");
            println!("  glyph-studio export-cff2 <project.json|ufo> <output.otf>");
            println!("  glyph-studio apply-metrics-keys <project.json> <glyphs.txt> [output.json]");
            println!(
                "  glyph-studio duplicate-component <project.json> <glyph> <index> [output.json]"
            );
            println!("  glyph-studio align-component <project.json> <glyph> <index> [output.json]");
            println!("  glyph-studio align-components <project.json> <glyph> [output.json]");
            println!(
                "  glyph-studio align-components-batch <project.json> <glyphs.txt> [output.json]"
            );
            Ok(())
        }
        Some(command) => Err(format!(
            "不明なコマンド '{command}'。--help を参照してください"
        )),
        None => unreachable!(),
    }
}

fn load_cli_project(path: &Path) -> Result<font_data::FontProject, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => io::load_project(path),
        Some("glyphs") => io::load_glyphs(path),
        Some("ufo") => io::load_ufo(path),
        Some("ttf") | Some("otf") => io::load_ttf(path),
        Some("woff") => io::load_woff(path),
        Some("woff2") => io::load_woff2(path),
        _ => {
            Err("入力形式は json / glyphs / ufo / ttf / otf / woff / woff2 に対応しています".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_master_cli_reorders_and_saves_json_project() {
        let base = std::env::temp_dir().join(format!(
            "glyph-studio-move-master-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let input = base.with_extension("json");
        let output = base.with_extension("reordered.json");
        let mut project = font_data::FontProject::new();
        project.masters.push(font_data::FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            ..font_data::FontMaster::default()
        });
        project
            .glyphs
            .insert("A".into(), font_data::GlyphData::new("A".into(), None));
        io::save_project(&project, &input).unwrap();

        let args = vec![
            "move-master".to_string(),
            input.display().to_string(),
            "bold".to_string(),
            "-1".to_string(),
            output.display().to_string(),
        ];
        run_cli(&args).unwrap();
        let reordered = io::load_project(&output).unwrap();
        assert_eq!(
            reordered
                .masters
                .iter()
                .map(|master| master.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bold", "regular"]
        );

        let duplicate_output = base.with_extension("duplicated.json");
        let duplicate_args = vec![
            "duplicate-master".to_string(),
            output.display().to_string(),
            "bold".to_string(),
            duplicate_output.display().to_string(),
        ];
        run_cli(&duplicate_args).unwrap();
        let duplicated = io::load_project(&duplicate_output).unwrap();
        assert_eq!(duplicated.masters[1].id, "bold.copy1");
        assert_eq!(duplicated.masters[1].name, "Bold Copy");

        std::fs::remove_file(input).unwrap();
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(duplicate_output).unwrap();
    }
}
