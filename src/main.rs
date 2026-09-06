use glyph_studio::app::GlyphStudioApp;
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
        if let Err(error) = cli::run_cli(&arguments) {
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

mod cli;

#[cfg(test)]
mod tests {
    use super::cli;
    use glyph_studio::{font_data, io};

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
        cli::run_cli(&args).unwrap();
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
        cli::run_cli(&duplicate_args).unwrap();
        let duplicated = io::load_project(&duplicate_output).unwrap();
        assert_eq!(duplicated.masters[1].id, "bold.copy1");
        assert_eq!(duplicated.masters[1].name, "Bold Copy");

        std::fs::remove_file(input).unwrap();
        std::fs::remove_file(output).unwrap();
        std::fs::remove_file(duplicate_output).unwrap();
    }
}
