
/// Loads a WOFF 1.0 file by rebuilding its uncompressed SFNT payload.
pub fn load_woff(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("WOFF読み込みエラー: {e}"))?;
    if bytes.len() < 44 || &bytes[0..4] != b"wOFF" {
        return Err("WOFFヘッダーが不正です".into());
    }
    let read_u16 = |offset: usize| -> Result<u16, String> {
        let end = offset.checked_add(2).ok_or("WOFFヘッダーが不正です")?;
        bytes
            .get(offset..end)
            .and_then(|value| value.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or_else(|| "WOFFヘッダーが不正です".into())
    };
    let read_u32 = |offset: usize| -> Result<u32, String> {
        let end = offset.checked_add(4).ok_or("WOFFヘッダーが不正です")?;
        bytes
            .get(offset..end)
            .and_then(|value| value.try_into().ok())
            .map(u32::from_be_bytes)
            .ok_or_else(|| "WOFFヘッダーが不正です".into())
    };
    let flavor = &bytes[4..8];
    let table_count = read_u16(12)? as usize;
    let directory_end = 44usize
        .checked_add(
            table_count
                .checked_mul(20)
                .ok_or("WOFFテーブル数が不正です")?,
        )
        .ok_or("WOFFディレクトリが不正です")?;
    if directory_end > bytes.len() {
        return Err("WOFFテーブルディレクトリが不正です".into());
    }
    let sfnt_header_len = 12usize
        .checked_add(
            table_count
                .checked_mul(16)
                .ok_or("SFNTテーブル数が不正です")?,
        )
        .ok_or("SFNTヘッダーが不正です")?;
    let mut sfnt = Vec::with_capacity(read_u32(16)? as usize);
    sfnt.extend_from_slice(flavor);
    sfnt.extend_from_slice(&(table_count as u16).to_be_bytes());
    sfnt.extend_from_slice(&[0; 6]);
    let mut payloads = Vec::with_capacity(table_count);
    for index in 0..table_count {
        let base = 44 + index * 20;
        let tag = &bytes[base..base + 4];
        let offset = read_u32(base + 4)? as usize;
        let compressed_len = read_u32(base + 8)? as usize;
        let original_len = read_u32(base + 12)? as usize;
        let checksum = &bytes[base + 16..base + 20];
        let end = offset
            .checked_add(compressed_len)
            .ok_or("WOFFテーブル範囲が不正です")?;
        if end > bytes.len() {
            return Err("WOFFテーブル範囲が不正です".into());
        }
        let compressed = &bytes[offset..end];
        let data = if compressed_len < original_len {
            let mut decoder = flate2::read::ZlibDecoder::new(compressed);
            let mut data = Vec::with_capacity(original_len);
            decoder
                .read_to_end(&mut data)
                .map_err(|error| format!("WOFF圧縮データの展開に失敗しました: {error}"))?;
            data
        } else {
            compressed.to_vec()
        };
        if data.len() != original_len {
            return Err("WOFFテーブル長が不一致です".into());
        }
        payloads.push((tag.to_vec(), checksum.to_vec(), data));
    }
    let mut offset = sfnt_header_len;
    for (tag, checksum, data) in &payloads {
        sfnt.extend_from_slice(tag);
        sfnt.extend_from_slice(checksum);
        sfnt.extend_from_slice(&(offset as u32).to_be_bytes());
        sfnt.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset = offset
            .checked_add((data.len() + 3) & !3)
            .ok_or("SFNTサイズが大きすぎます")?;
    }
    for (_, _, data) in payloads {
        sfnt.extend_from_slice(&data);
        while sfnt.len() % 4 != 0 {
            sfnt.push(0);
        }
    }
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff-import-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&temp, sfnt).map_err(|error| error.to_string())?;
    let result = load_ttf(&temp);
    let _ = std::fs::remove_file(temp);
    result
}
