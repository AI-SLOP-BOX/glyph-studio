use super::*;

fn show_section(filter: &str, keywords: &[&str]) -> bool {
    filter.is_empty()
        || keywords
            .iter()
            .any(|keyword| filter.contains(&keyword.to_lowercase()))
}

pub(super) fn show(ui: &mut Ui, filter: &str, project: &mut FontProject) {
    if show_section(filter, &["font", "フォント", "メトリクス", "metrics"]) {
        egui::CollapsingHeader::new("フォント情報・メトリクス")
            .default_open(false)
            .show(ui, |ui| {
                let meta = &mut project.metadata;

                ui.label("ファミリー名:");
                ui.text_edit_singleline(&mut meta.family_name);
                ui.label("スタイル名:");
                ui.text_edit_singleline(&mut meta.style_name);
                ui.label("著作権:");
                ui.text_edit_singleline(&mut meta.copyright);
                ui.label("デザイナー:");
                ui.text_edit_singleline(&mut meta.designer);
                ui.label("メーカー:");
                ui.text_edit_singleline(&mut meta.manufacturer);
                ui.horizontal(|ui| {
                    ui.label("バージョン:");
                    ui.add(
                        egui::DragValue::new(&mut meta.font_revision)
                            .speed(0.01)
                            .range(0.0..=65535.0),
                    );
                });

                ui.separator();
                ui.label("メトリクス");

                ui.horizontal(|ui| {
                    ui.label("UPM:");
                    ui.add(
                        egui::DragValue::new(&mut meta.units_per_em)
                            .speed(10.0)
                            .range(16.0..=16384.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("アセンダー:");
                    ui.add(egui::DragValue::new(&mut meta.ascender).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("ディセンダー:");
                    ui.add(egui::DragValue::new(&mut meta.descender).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("行間:");
                    ui.add(egui::DragValue::new(&mut meta.line_gap).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("イタリック角:");
                    ui.add(egui::DragValue::new(&mut meta.italic_angle).speed(0.5));
                });
                ui.horizontal(|ui| {
                    ui.label("下線位置:");
                    ui.add(egui::DragValue::new(&mut meta.underline_position).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("下線太さ:");
                    ui.add(egui::DragValue::new(&mut meta.underline_thickness).speed(1.0));
                });
                ui.checkbox(&mut meta.is_fixed_pitch, "Fixed pitch (等幅フォント)");
                ui.horizontal(|ui| {
                    ui.label("xハイト:");
                    ui.add(egui::DragValue::new(&mut meta.x_height).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("キャップハイト:");
                    ui.add(egui::DragValue::new(&mut meta.cap_height).speed(10.0));
                });
                ui.horizontal(|ui| {
                    ui.label("ウェイト:");
                    ui.add(egui::DragValue::new(&mut meta.weight_class).range(1..=1000));
                });
                ui.horizontal(|ui| {
                    ui.label("幅:");
                    ui.add(egui::DragValue::new(&mut meta.width_class).range(1..=9));
                });
                ui.horizontal(|ui| {
                    ui.label("Vendor ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut meta.vendor_id)
                            .desired_width(80.0)
                            .char_limit(4),
                    );
                    ui.small("OS/2の4文字識別子");
                });
                ui.horizontal(|ui| {
                    ui.label("FSType:");
                    ui.add(egui::DragValue::new(&mut meta.fs_type).range(0..=u16::MAX));
                    ui.small("埋め込み／ライセンス制限");
                });
                ui.horizontal(|ui| {
                    ui.label("fsSelection:");
                    ui.add(egui::DragValue::new(&mut meta.fs_selection).range(0..=u16::MAX));
                    if ui
                        .small_button("自動")
                        .on_hover_text("0に戻し、スタイルから自動生成")
                        .clicked()
                    {
                        meta.fs_selection = 0;
                    }
                    ui.small("OS/2選択ビット");
                });
                ui.horizontal(|ui| {
                    ui.label("DefaultChar:");
                    ui.add(egui::DragValue::new(&mut meta.default_char).range(0..=u16::MAX));
                    if ui.small_button("自動").clicked() {
                        meta.default_char = 0;
                    }
                    ui.small("OS/2の未対応文字");
                });
                ui.horizontal(|ui| {
                    ui.label("BreakChar:");
                    ui.add(egui::DragValue::new(&mut meta.break_char).range(0..=u16::MAX));
                    if ui.small_button("自動").clicked() {
                        meta.break_char = 0;
                    }
                    ui.small("OS/2の区切り文字");
                });
                ui.horizontal(|ui| {
                    ui.label("MaxContext:");
                    ui.add(egui::DragValue::new(&mut meta.max_context).range(0..=u16::MAX));
                    if ui.small_button("自動").clicked() {
                        meta.max_context = 0;
                    }
                    ui.small("0ならFeatureから自動計算");
                });
                ui.horizontal(|ui| {
                    ui.label("head flags:");
                    ui.add(egui::DragValue::new(&mut meta.head_flags).range(0..=u16::MAX));
                    if ui
                        .small_button("自動")
                        .on_hover_text("標準のhead.flagsへ戻す")
                        .clicked()
                    {
                        meta.head_flags = 0;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("macStyle:");
                    ui.add(egui::DragValue::new(&mut meta.head_mac_style).range(0..=u16::MAX));
                    if ui.small_button("自動").clicked() {
                        meta.head_mac_style = 0;
                    }
                    ui.small("Bold/Italic等のheadフラグ");
                });
                ui.horizontal(|ui| {
                    ui.label("最低PPEM:");
                    ui.add(egui::DragValue::new(&mut meta.lowest_rec_ppem).range(0..=u16::MAX));
                    if ui
                        .small_button("自動")
                        .on_hover_text("標準のlowestRecPPEMへ戻す")
                        .clicked()
                    {
                        meta.lowest_rec_ppem = 0;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("方向ヒント:");
                    ui.add(egui::DragValue::new(&mut meta.font_direction_hint));
                    ui.small("head.fontDirectionHint");
                });
                ui.horizontal(|ui| {
                    ui.label("カーソル傾き:");
                    ui.add(egui::DragValue::new(&mut meta.caret_slope_rise));
                    ui.label("/ ");
                    ui.add(egui::DragValue::new(&mut meta.caret_slope_run));
                    ui.small("hhea caretSlopeRise / Run（Rise=0は自動）");
                });
                ui.horizontal(|ui| {
                    ui.label("カーソルオフセット:");
                    ui.add(egui::DragValue::new(&mut meta.caret_offset));
                    ui.small("hhea caretOffset");
                });
                ui.horizontal(|ui| {
                    ui.label("縦カーソル傾き:");
                    ui.add(egui::DragValue::new(&mut meta.vertical_caret_slope_rise));
                    ui.label("/ ");
                    ui.add(egui::DragValue::new(&mut meta.vertical_caret_slope_run));
                    ui.small("vhea caretSlopeRise / Run（Rise=0は自動）");
                });
                ui.horizontal(|ui| {
                    ui.label("縦カーソルオフセット:");
                    ui.add(egui::DragValue::new(&mut meta.vertical_caret_offset));
                    ui.small("vhea caretOffset");
                });
                ui.collapsing("PANOSE分類", |ui| {
                    ui.small("OS/2のフォント分類バイト（10項目）");
                    ui.horizontal_wrapped(|ui| {
                        for (index, value) in meta.panose.iter_mut().enumerate() {
                            ui.add(
                                egui::DragValue::new(value)
                                    .range(0..=u8::MAX)
                                    .prefix(format!("{}: ", index + 1)),
                            );
                        }
                    });
                });
                ui.collapsing("OS/2補助メトリクス", |ui| {
                    ui.small("下付き・上付き・打消し線。0の項目はUPMから自動計算");
                    ui.horizontal(|ui| {
                        ui.label("下付き X/Y:");
                        ui.add(egui::DragValue::new(&mut meta.subscript_x_size));
                        ui.add(egui::DragValue::new(&mut meta.subscript_y_size));
                        ui.label("Offset X/Y:");
                        ui.add(egui::DragValue::new(&mut meta.subscript_x_offset));
                        ui.add(egui::DragValue::new(&mut meta.subscript_y_offset));
                    });
                    ui.horizontal(|ui| {
                        ui.label("上付き X/Y:");
                        ui.add(egui::DragValue::new(&mut meta.superscript_x_size));
                        ui.add(egui::DragValue::new(&mut meta.superscript_y_size));
                        ui.label("Offset X/Y:");
                        ui.add(egui::DragValue::new(&mut meta.superscript_x_offset));
                        ui.add(egui::DragValue::new(&mut meta.superscript_y_offset));
                    });
                    ui.horizontal(|ui| {
                        ui.label("打消し線 Size/Position:");
                        ui.add(egui::DragValue::new(&mut meta.strikeout_size));
                        ui.add(egui::DragValue::new(&mut meta.strikeout_position));
                        ui.label("FamilyClass:");
                        ui.add(egui::DragValue::new(&mut meta.family_class));
                    });
                    ui.horizontal(|ui| {
                        ui.label("光学サイズ下限/上限:");
                        ui.add(egui::DragValue::new(&mut meta.lower_optical_point_size));
                        ui.add(egui::DragValue::new(&mut meta.upper_optical_point_size));
                        ui.small("OS/2 v5（0なら出力しない）");
                    });
                    ui.horizontal(|ui| {
                        ui.label("WinAscent/Descent:");
                        ui.add(egui::DragValue::new(&mut meta.win_ascent));
                        ui.add(egui::DragValue::new(&mut meta.win_descent));
                        ui.small("0なら輪郭から自動計算");
                    });
                });
            });
    }
}
