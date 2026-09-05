# Glyph Studio v0.1.0 MVP

## 目的

個人が日常的なフォント制作を、無料・OSSのGUIで一通り完了できる最初の公開版を作る。

## v0.1.0に含めるもの

- macOSで起動できるGUI
- グリフの輪郭・ノード・コンポーネント編集
- グリフのスペーシングと基本的なカーニング編集
- 複数マスターの編集と基本的な補間プレビュー
- JSON／Glyphs／UFOの読み書き
- TTF／OTF／WOFF／WOFF2／SVGの入出力
- 基本的なGSUB／GPOS／GDEFとFeatureソース編集
- OpenTypeを含む書き出し前の検証
- CLIからの検証・ビルド・書き出し
- README、ライセンス、CI、スクリーンショット、既知の制限の公開

## v0.1.0で約束しないもの

- GlyphsやFontLabとの完全互換
- OpenType全仕様のGUI操作
- 高度なカラー・複雑なペイントグラフの完全編集
- Windows／Linux向けの配布品質
- 既存フォントの全テーブルを意味論付きで編集できること

## 完了条件

1. 新規プロジェクトを作成し、グリフを編集して保存できる。
2. GlyphsまたはUFOを読み込み、編集後にTTFを書き出せる。
3. 書き出したTTFを再読み込みできる。
4. 2マスター以上のフォントを補間し、静的フォントを書き出せる。
5. Featureソースとカーニングを反映した出力を検証できる。
6. テスト、Clippy、ビルドがCIで再現できる。
7. 初見の利用者がREADMEだけで起動・保存・書き出しまで試せる。

## 公開時の方針

v0.1.0は「完成したGlyphsクローン」ではなく、「日常制作の主要導線が動く実験的なOSS」として公開する。未対応範囲は隠さず、READMEの既知の制限に記載する。

## 公開前チェック

- [ ] ライセンスを確定する
- [ ] READMEをMVP向けに整理する
- [ ] スクリーンショットを追加する
- [ ] `cargo test --all-targets` を通す
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` を通す
- [ ] `cargo build --release` を通す
- [ ] macOS App Bundleを起動確認する
- [ ] GitHubでPublicリポジトリを作成する
- [ ] 初回コミットとタグ `v0.1.0` を作成する
