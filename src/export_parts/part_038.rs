
/// Writes a WOFF 1.0 wrapper around the generated TrueType font.
pub fn export_woff(project: &FontProject, path: &Path) -> Result<(), String> {
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    let export_result = export_ttf(project, &temp);
    if let Err(error) = export_result {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    let sfnt = match std::fs::read(&temp) {
        Ok(sfnt) => sfnt,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error.to_string());
        }
    };
    let _ = std::fs::remove_file(&temp);
    if sfnt.len() < 12 {
        return Err("生成されたTTFが不正です".into());
    }
    let count = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    if sfnt.len() < 12 + count * 16 {
        return Err("TTFテーブルディレクトリが不正です".into());
    }
    let mut records = Vec::new();
    let mut body = Vec::new();
    for index in 0..count {
        let base = 12 + index * 16;
        let offset = u32::from_be_bytes(sfnt[base + 8..base + 12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(sfnt[base + 12..base + 16].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(length)
            .ok_or("TTFテーブル範囲が不正です")?;
        if end > sfnt.len() {
            return Err("TTFテーブル範囲が不正です".into());
        }
        let original = &sfnt[offset..end];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(original)
            .map_err(|error| error.to_string())?;
        let compressed = encoder.finish().map_err(|error| error.to_string())?;
        let data = if compressed.len() < original.len() {
            compressed
        } else {
            original.to_vec()
        };
        while body.len() % 4 != 0 {
            body.push(0);
        }
        let body_offset = 44 + count * 20 + body.len();
        let checksum = u32::from_be_bytes(sfnt[base + 4..base + 8].try_into().unwrap());
        records.push((
            sfnt[base..base + 4].to_vec(),
            body_offset as u32,
            data.len() as u32,
            length as u32,
            checksum,
        ));
        body.extend(data);
    }
    let total = 44 + count * 20 + body.len();
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(b"wOFF");
    output.extend_from_slice(&sfnt[0..4]);
    output.extend_from_slice(&(total as u32).to_be_bytes());
    output.extend_from_slice(&(count as u16).to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&(sfnt.len() as u32).to_be_bytes());
    output.extend_from_slice(&1u16.to_be_bytes());
    output.extend_from_slice(&0u16.to_be_bytes());
    output.extend_from_slice(&[0u8; 20]);
    for (tag, offset, compressed, original, checksum) in records {
        output.extend_from_slice(&tag);
        output.extend_from_slice(&offset.to_be_bytes());
        output.extend_from_slice(&compressed.to_be_bytes());
        output.extend_from_slice(&original.to_be_bytes());
        output.extend_from_slice(&checksum.to_be_bytes());
    }
    output.extend_from_slice(&body);
    std::fs::write(path, output).map_err(|error| error.to_string())
}
