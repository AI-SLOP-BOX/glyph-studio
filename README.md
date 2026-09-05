# Glyph Studio

[![CI](https://github.com/AI-SLOP-BOX/glyph-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/AI-SLOP-BOX/glyph-studio/actions/workflows/ci.yml)

Rust／egui製のフォント制作アプリです。輪郭、ノード、コンポーネント、マスター、カーニング、OpenType機能を編集できます。

現在は **v0.1.0 MVP** です。GlyphsやFontLabの完全互換を目指すのではなく、個人の普段のフォント制作に必要な導線を、無料・OSSのGUIとCLIで気持ちよくつなぐことを目標にしています。対応範囲と既知の制限は [MVP.md](MVP.md) にまとめています。

GUI・CLI・再利用可能なCore APIを同じRustコードで提供します。push／Pull Request時にはテスト、Clippy、Rustdocを自動検証します。

## 起動

```sh
cargo run --release
```

Apple Silicon向けの実行ファイルは `target/release/glyph-studio` に生成されます。

## 対応形式

- プロジェクト: JSON、UFO、Glyphs（`.glyphs`）
- 読み込み: TTF、OTF、WOFF、WOFF2、SVG
- 出力: TTF、CFF/OTF、WOFF、WOFF2、SVG
- マスター別の静的フォント出力、補間インスタンス出力
- 可変フォント軸名の編集とname／STATへの反映
- 可変フォント: 名前付きインスタンスを任意の軸座標で登録し、fvar／name／STATへ出力
- Alternate／Bracketレイヤー: グリフごとの軸min／max条件（任意軸数）で追加・命名変更・削除・表示、通常レイヤーへの反映
- 可変フォント: 輪郭の補間と、同一構造のコンポーネントの位置変化をgvarへ出力。条件レイヤーは代替グリフとGSUB Feature Variations（rvrn）へ反映
- 可変フォントWOFF2: 複数マスター時はTrueType可変テーブル（fvar／gvar／HVAR／VVAR／MVAR）を保持して圧縮。単一マスターは静的フォントとして出力
- 2軸補間プレビュー: 矩形に配置された4マスターを自動検出し、X/Y補間率を独立操作。キャンバスと組版プレビューで同じ結果を表示し、非矩形配置は2マスター補間へフォールバック

## 主な操作

- `V`／`P`／`H`／`K`: 選択／ペン／ハンド／ナイフ、`I`: 背景画像表示切り替え、`B`: 前後字形表示切り替え、`D`: 輪郭方向表示切り替え、`M`: メトリクス表示切り替え、`N`: ノード番号表示切り替え
- `Esc`: キャンバス上のノード／部品／ガイド選択と、ペン／ナイフ操作を解除
- 矢印: 選択ノードまたはコンポーネントを移動（Shiftで10単位）
- `⌘C`／`⌘V`／`⌘X`: 輪郭またはコンポーネントのコピー／貼り付け／カット（全マスター同期）
- `Delete`／`Backspace`: 選択ノードまたは部品を全マスターから削除
- `⌘D`: グリフを複製（複数選択にも対応）
- `⌘⇧D`: 選択中コンポーネントを全マスターへ複製
- Unicode一括設定: `グリフ名=U+XXXX`形式の複数行入力に対応
- `⌘Z`／`⌘⇧Z`: Undo／Redo
- `Tab`／`Shift+Tab`: 次／前のグリフ、`Home`／`End`: 先頭／末尾のグリフ
- `⌘⇧A`／`Ctrl+Shift+A`: 全グリフを選択（`⌘A`／`Ctrl+A`は現在グリフのノード選択）
- `⌘S`: プロジェクト保存、`⌘⇧S`: UFO保存
- `⌘E`: 検証してTTFを書き出し
- ツールバーの「保存」／「書き出し」: 保存または検証付きのTTF・OTF・WOFF2・WOFF書き出し
- FinderからJSON／Glyphs／UFO／TTF／OTF／WOFF／WOFF2をウィンドウへドラッグ＆ドロップして開く
- macOS App Bundleへ対応ファイルをダブルクリック／「このアプリケーションで開く」で渡すと、そのまま編集画面で開く
- 未保存変更がある状態で開く／新規作成すると、保存・破棄・キャンセルを選べる確認ダイアログを表示
- キャンバス上部: 前後グリフ移動、前後マスター移動、マスター切り替え、字幅編集（全マスターへ反映）、全体表示、グリッド／グリッド吸着／ガイド吸着／アンカー吸着／ガイド／アンカー／背景画像の表示切り替え
- キャンバス上部に現在の選択対象（ノード数または部品名）を表示
- ガイドライン: グローバルガイドとグリフごとのガイドをマスター単位で独立編集・表示。JSON／UFO／Glyphsへ保存
- キャンバス上のガイドをクリック選択し、黄色の選択表示と Delete 削除に対応
- キャンバスを右クリック: 選択解除、全ノード選択、ノード属性変更、全体表示
- ズーム: `−`／`＋`、倍率の直接入力、`100%`リセット、`F`で全体表示、マウスホイール
- キャンバス上のカーソル座標: フォント座標（X, Y）をカーソル付近に表示
- `Space`を押しながらドラッグ: 一時的にキャンバスをパン（手のひらツール）
- 中ボタンでドラッグ: ツールを切り替えずにキャンバスをパン
- 選択ツールで`Shift`を押しながらドラッグ: 水平または垂直に移動軸を固定
- `⌘⇧M`: 全マスター編集の切り替え
- `⌘F`: グリフ一覧の検索欄へフォーカス
- `Tab`／`Shift+Tab`／`PageUp`／`PageDown`: 前後のグリフへ移動
- `⌘↑`／`⌘↓`: 前後のマスターへ移動
- グリフ一覧: 名前検索、名前順／Unicode順、Unicode未設定フィルター
- グリフ一覧のグリッド表示: Bezierアウトラインと複合グリフのサムネイルを表示。パネル幅に応じて列数を自動調整
- 複合グリフの部品を選択すると、選択枠・四隅ハンドル・部品名を表示（通常マスターでは回転ハンドルも表示）
- 複合グリフの部品をShiftクリック: 複数部品を選択して一括移動・変形・削除
- 複数部品選択時のインスペクタ: 選択部品のアンカー整列・削除を全マスターで一括実行
- 複数部品選択時の輪郭操作: 整列・分布・反転を部品の見た目の中心基準で一括実行
- キャンバスの空白からドラッグ: 部品も矩形選択の対象に含める
- 部品選択枠の四隅ハンドルをドラッグ: 反対側の角を固定して部品を拡大／縮小
- 部品選択枠の上側ハンドルをドラッグ: 部品を回転（全マスター編集にも対応）
- 部品回転中に`Command`: 15度刻みにスナップ
- 複合グリフの部品を`Option`ドラッグ: 部品を複製して、そのまま移動
- 複合グリフの部品をダブルクリック: 参照元グリフを開く
- 条件レイヤー: 「条件軸を追加」で軸行を増やし、複数軸のmin／max条件を編集。既存条件の複製にも対応
- カーニング: 左右グループの実効値を表示し、編集時は個別ペアの例外として保存
- カーニング: マスターごとに独立した値を保持し、静的マスター出力・補間出力へ反映
- カーニング一覧: 現在の左グリフに対するペアを右グリフ名で絞り込み、値を直接編集・削除
- カーニング一覧検索は右グリフのUnicode文字にも対応
- 一括編集: 選択中または指定した複数グリフへ、左右カーニンググループをまとめて設定・解除
- ツールバーの「カーニング」: 全ペアを専用ウィンドウで一覧・検索・直接編集・削除。ペア選択で対象グリフへ移動
- 専用カーニング一覧の「プレビュー」: Unicode文字またはグリフ名で対象ペアを組版プレビューへ送る
- ウィンドウタイトルの `*`: 保存後に未保存の編集あり
- ステータスバーの「保存」: 未保存時に画面下から直接プロジェクトを保存
- プレビューの「左右確認」: 現在グリフを基準字形（H／O／n／o）で挟んでスペーシング確認
- プレビューの定型パターン: `HH`／`HO`／`nn`／`oo` をワンクリック表示
- プレビュー: 複数行テキスト、行間調整、カーニング、マークアンカー、OpenType機能を確認。`liga`／`kern`／`mark`／`mkmk`／`calt`／`rvrn`は個別切り替え可能
- プレビューの字形クリック: 対象グリフをキャンバス編集へ切り替え
- キャンバス上部の `LSB`／`RSB`: 現在グリフの左右余白を直接編集（全マスター反映）
- メトリクスキー: `=H` のように左右余白を基準グリフへリンクし、全マスターへ適用
- キャンバスの「前後字形」: 現在グリフの左右に隣接字形を薄く表示し、字幅・カーニングを見ながらスペーシングを調整
- 「輪郭方向」表示: 輪郭の進行方向をキャンバス上で確認
- 「アンカー」表示: アンカーの表示／非表示を切り替え。非表示時はアンカーのヒット判定も無効化
- アンカー編集: 選択ツールでアンカーを直接ドラッグして位置を変更。アンカー吸着にも対応
- コンポーネント編集: 参照先グリフを「開く」で直接編集し、対応アンカーを「アンカーで位置合わせ」で全マスターへ自動配置。「選択部品を複製」は全マスターへ部品を追加
- コンポーネントを輪郭化: 現在グリフの全マスターで、各マスターの参照形状を使って輪郭へ変換
- コンポーネントの一括輪郭化: 選択中または指定した複数グリフを全マスターで変換
- コンポーネント変形: 「縦横比を固定」でX／Y倍率を連動し、意図しない変形を防止
- 選択ツールでキャンバスの字幅境界／アウトライン端をドラッグ: 字幅・LSB・RSBを直接調整
- 「左余白を0に揃える」: 輪郭・部品・アンカーを全マスターへ同時反映
- 「アウトラインを中央配置」: 各マスターの字幅とアウトラインから移動量を算出し、輪郭・部品・アンカーを個別に中央配置
- 「アウトライン右端へ字幅をフィット」: 現在グリフと各マスターのアウトライン右端へ、それぞれの字幅を同期
- キャンバス上部の「比較」: マスター間の補間オーバーレイと補間率を直接操作
- 比較元と比較先には異なるマスターを選択し、キャンバス上で補間形状を確認
- 「全マスター」比較: 現在のマスター以外の輪郭とコンポーネントを色分けして重ね、凡例付きで差分を確認
- 「全マスターへ反映」: キャンバス上のノード／コンポーネント移動・変形・反転・整列・分布を全マスターへ原子的に同期
- マスター配置マップ: 最初の2軸を2D表示し、点のクリックでマスター切替、ドラッグで軸値を直接編集
- マスター管理: 名称・軸値の編集、追加・削除、上下移動、基準マスター設定
- ツールバーの「検証」: 書き出し前にフォント全体をチェック
- 検証後に問題が残る場合、ツールバーへ現在グリフ／全体の警告件数を表示
- 検証内容: Unicode／IVSの重複・不正値、アンカー重複／座標、輪郭・補間・コンポーネント循環、OpenType参照を確認
- ファイルメニューの出力項目は、主要フォント／マスター別／補間インスタンス／SVGに分類
- ノード編集: スムーズ／コーナー／オン・オフ曲線を全マスターへ同期（オンカーブ点3点未満の不正輪郭は拒否）
- 選択ツールで曲線をダブルクリック: 既存のBezier形状を保ったままノードを追加（全マスター対応）
- ペンツールでドラッグ: 接線ハンドル付きのBezierノードを作成。次のノードには反対側ハンドルも自動生成
- ペンドラッグ中: 確定前のアンカーとハンドルをキャンバスにプレビュー表示
- ナイフツール: 輪郭上を2点クリックして閉じた輪郭を分割。全マスターで構造が一致しない場合は変更しない
- ノード編集ショートカット: `S`（スムーズ）、`C`（コーナー）、`T`（オン／オフ曲線切替）。修飾キー付きの保存・コピー操作とは衝突しない
- 複数ノード選択時のインスペクタ: スムーズ／コーナー／オン・オフ曲線切替を全マスターへ一括適用
- 複数ノード選択時のインスペクタ: X/Y移動量を数値入力して一括適用
- 「重複ノードを整理」: 現在または選択中のグリフを全マスターで修復
- 「方向反転」／「方向を自動調整」／「全輪郭の方向を調整」: 輪郭の向きを全マスターで同期
- 「選択輪郭と次を統合」／「全輪郭を統合」／「選択輪郭から次を削除」／「選択輪郭と次の交差部分」／「選択輪郭と次のXOR」: 曲線を保持したBoolean処理（全マスター対応）。Booleanエンジンが返す円弧も編集可能なBezierへ自動変換
- OpenType Feature編集: 構文エラー、未定義グリフ、未定義クラス、名前付きLookupの未定義／重複を編集中に表示
- OpenType Feature雛形: 基本置換・位置調整に加え、`size`／`smcp`／`c2sc`／`onum`／`lnum`／`pnum`／`tnum`／`palt`／`vkrn`／`vert`／`jp78`／`jp83`／`jp90`／`ss01`〜`ss20`／`cv01`〜`cv20`などをワンクリック挿入
- OpenType Class雛形: `@Upper`／`@Lower`／`@Marks` の基本定義をワンクリック挿入
- OpenType Class定義を単独でクリップボードへコピー可能
- OpenType合成ソース: ClassとFeatureを連結した書き出し内容を読み取り専用で確認
- OpenType合成ソースをクリップボードへコピー可能
- Feature本文から認識した定義済みFeatureタグを一覧表示
- OpenType操作パレット: 左／右／置換先グリフ、カーニング値、Mark X/Yから `sub`／合字／`pos`／`ignore sub`／`ignore pos`／Mark／Mark-to-Mark位置のサンプルを生成し、既存Feature本文へ挿入
- Mark系操作は標準の `mark`／`mkmk` Featureへ自動挿入し、Mark X/Y座標も指定可能。置換・合字・カーニング・例外は指定したFeatureタグへ挿入
- Mark位置は `pos base <anchor X Y> mark @class;`、Mark-to-Markは `pos mark @class mark @class;` の形で生成
- OpenType操作欄から現在編集中のグリフ名を左グリフ欄へワンクリック転送可能
- OpenType操作欄の入力を一括クリアして、次の操作へすぐ切り替え可能
- OpenType操作の挿入先: 標準プリセットに加えて任意の英数字4文字Featureタグを指定可能。未登録グリフと不正値は入力中に警告
- OpenType位置調整の出力: Feature内の `pos` による単体位置調整、ペア位置調整、グリフクラス、4値／8値のValueRecordをGPOSへコンパイル
- OpenTypeクラスカーニング: 左右のカーニンググループをGPOS PairPos Format 2へ圧縮し、個別例外はFormat 1で保持
- OpenType名前付きLookup: 外部`lookup name { ... } name;`をFeatureから参照する構文をGSUB／GPOSへ展開
- OpenType Feature参照: `feature liga { feature dlig; } liga;`のようなFeature間参照をGSUB／GPOSへ登録し、多段参照も循環停止付きで展開
- OpenType言語システム: `languagesystem` によるScript／Languageの既定適用と、`dflt`／3文字言語タグをScriptListへ反映
- OpenType Language制御: `language dflt`をDefault LangSysとして扱い、`required`をRequiredFeatureIndexへ出力。既定Lookupの`exclude_dflt`も反映
- OpenType構成雛形: UIから`languagesystem`と名前付き`lookup`の雛形を追加可能
- OpenType検索導線: プロパティ検索で`GSUB`／`GPOS`／`lookup`／`languagesystem`／`ss`／`cv`やFeatureソース内の文字列からOpenType欄を表示
- OpenType Feature Parameters: `ss01`〜`ss20`／`cv01`〜`cv20`へ標準のStylistic Set／Character Variantメタデータを付加し、対応アプリの機能一覧へ表示
- OpenType Feature名: Feature内の`featureNames`を`ss##`／`cv##`のname IDへ反映し、標準名を上書き。複数のPlatform／Script／Language名にも対応
- OpenType代替字形: 明示的な`aalt`がない場合も、`salt`／`ss##`／`cv##`などの単一置換から全代替字形Featureを自動生成
- OpenTypeコンテキスト位置調整: `pos A' V <...>;` のような後続条件付き位置調整をGPOS Lookup Type 7へコンパイル（クラス展開対応）
- OpenType前後コンテキスト位置調整: `pos A V' A <...>;` のような前後条件付き位置調整をGPOS Lookup Type 8へコンパイル
- OpenType除外ルール: `ignore sub`／`ignore pos` をクラス指定を含めてGSUB／GPOSのチェーンコンテキストへコンパイル
- OpenType Feature File互換: 長形式`substitute`／`position`と列挙形式`enum sub`／`enum pos`を短形式と同じGSUB／GPOSへコンパイル
- OpenType Feature File別名: `enumerate sub`／`enumerate pos`、`rsub`、旧表記`excludeDFLT`を正規化して処理
- OpenType再利用ValueRecord: `valueRecordDef`で定義した固定位置調整値を`pos`／ペア位置調整へ展開
- OpenType Deviceテーブル: ValueRecord内の`<device ppem delta, ...>`／`<device NULL>`をGPOSへ出力し、ppem別の微調整に対応
- OpenType再利用Anchor: `anchorDef`で定義した固定Anchorをmark／mkmk／ligature／cursiveの`<anchor name>`参照へ展開
- OpenType Extension Lookup: `useExtension`をGSUB Lookup Type 7／GPOS Lookup Type 9へ変換し、大規模Featureの32-bit offsetを確保
- OpenType削除置換: `sub glyph by NULL;` を空のGSUB Multiple Substitutionとして出力
- OpenType接続位置の出力: `entry`／`exit` アンカーから cursive positioning（GPOS Lookup Type 3）、`top_1`／`top_2` などから mark-to-ligature（Lookup Type 5）を生成
- OpenType Feature cursive: Feature Fileの`pos cursive`アンカー指定（`NULL`を含む）もGPOS Lookup Type 3へコンパイル
- OpenType Feature mark-to-ligature: Feature Fileの`pos ligature`と`markClass`をGPOS Lookup Type 5へコンパイル（`NULL`スロット対応）
- OpenType複数Markアンカー: 1つの`pos base`文に複数の`<anchor> mark @class`を指定してGPOSへ出力
- OpenType lookup flags: `RightToLeft`、`IgnoreBaseGlyphs`、`IgnoreLigatures`、`IgnoreMarks`、`MarkAttachmentType` をFeature内から読み取り、対応lookupへ反映
- OpenType GDEF明示指定: Feature Fileの`GlyphClassDef`を読み取り、Base／Ligature／Mark／Component分類を推測値より優先
- OpenType GDEF合字カーソル: `LigatureCaretByPos`／`LigatureCaretByIndex`を読み取り、自動計算値より明示指定を優先
- OpenType GDEFアタッチメント: `Attach`のGlyph／GlyphClassと輪郭点インデックスをAttachment Point Listへ出力
- OpenType GDEFラウンドトリップ: TTFのAttachment Point List／LigCaretListをFeatureソースへ復元し、再編集可能
- OpenType GDEF caret Device: DeviceなしのLigCaret format 3も座標へ復元し、Device付きはraw保持
- OpenTypeテーブル上書き: `table head`の`FontRevision`／`Flags`／`LowestRecPPEM`／`FontDirectionHint`と`table hhea`のAscender／Descender／LineGapを出力へ反映
- OpenType hheaカーソル: `CaretSlopeRise`／`CaretSlopeRun`／`CaretOffset`をUI／JSON／Feature File／TTF入出力で保持
- OpenType vheaカーソル: 縦組み用`CaretSlopeRise`／`CaretSlopeRun`／`CaretOffset`をUI／JSON／Feature File／TTF入出力で保持
- OpenType postテーブル: `ItalicAngle`／`UnderlinePosition`／`UnderlineThickness`／`IsFixedPitch`をFeature Fileから上書き
- OpenType縦メトリクス上書き: `table vmtx`の`VertOriginY`／`VertAdvanceY`をグリフ単位で出力へ反映
- OpenType OS/2メタデータ: Vendor ID、WeightClass、WidthClass、DefaultChar、BreakChar、MaxContextをUI／Feature File／TTF入出力で管理
- OpenType OS/2メトリクス: `TypoAscender`／`TypoDescender`／`TypoLineGap`／`XHeight`／`CapHeight`の上書きを出力へ反映
- OpenType OS/2ライセンス: `FSType`をUI／JSON／Feature File／TTF入出力で管理し、埋め込み制限を保持
- OpenType OS/2選択フラグ: `fsSelection`を読み込み／JSON／Feature File／TTF出力で保持し、Regular／Italic／Bold／USE_TYPO_METRICS／WWS等のビットを既存フォントから復元
- OpenType OS/2 PANOSE: 10バイトのフォント分類情報をUI／JSON／Feature File／TTF入出力で保持
- OpenType OS/2補助メトリクス: 下付き／上付き文字、打消し線、`sFamilyClass`をUI／JSON／Feature File／TTF入出力で保持
- OpenType OS/2 Version 5: 光学サイズ下限／上限をUI／JSON／Feature File／TTF入出力で保持
- OpenType OS/2 Windowsメトリクス: `usWinAscent`／`usWinDescent`をUI／JSON／Feature File／TTF入出力で保持
- OpenType headメタデータ: `head.flags`、`macStyle`、`lowestRecPPEM`、`fontDirectionHint`をUI／JSON／Feature File／TTF入出力で保持
- OpenType nameテーブル: `table name`のカスタムName IDとPlatform／Encoding／Language指定をTTFへ出力
- OpenType nameラウンドトリップ: TTFのUnicode系カスタムName IDを`table name`ソースへ復元
- OpenType GSUBラウンドトリップ: TTFの単置換／Multiple／Alternate／LigatureをFeatureソースへ復元
- OpenType GSUB Reverse Chain: Reverse Chaining Single Substitutionと前後Coverageを復元
- OpenType GSUB Context復元: Coverage／Glyph／Class形式のContext／ChainContext、単一置換Lookup参照をFeature Fileへ復元
- OpenType GPOSラウンドトリップ: TTFの単体位置調整を4値`pos`ソースへ復元し、Device付き値はraw保持
- OpenType GPOS Pairラウンドトリップ: `kern`以外のPairPos format 1をペア`pos`ソースへ復元
- OpenType GPOS Class Pairラウンドトリップ: PairPos format 2を自動生成クラス付き`pos`ソースへ復元
- OpenType GPOS Context復元: Coverage／Glyph／Class形式のContextual／Chain Contextual Positioningと単一位置調整Lookup参照をFeature Fileへ復元
- OpenType Lookup Flagラウンドトリップ: TTFのRTL／Ignore系／MarkAttachmentTypeをFeatureソースへ復元
- OpenType Script／Languageラウンドトリップ: TTFのGSUB／GPOS ScriptList・LangSysを`languagesystem`へ復元
- OpenType Mark Filtering Set: GDEF MarkGlyphSetsと`UseMarkFilteringSet`を名前付きクラスへ復元
- OpenType Markラウンドトリップ: GPOS MarkToBaseのマーク／基底アンカーをグリフへ復元
- OpenType Cursiveラウンドトリップ: GPOS Cursiveの`entry`／`exit`アンカーをグリフへ復元
- OpenType Mark-to-Markラウンドトリップ: GPOS MarkToMarkの上下アンカーをグリフへ復元
- OpenType Mark-to-Ligatureラウンドトリップ: GPOS MarkToLigatureのコンポーネント別アンカーを復元
- OpenType Script／Language: Feature内の `script`／`language` 指定をGSUB／GPOSのScriptList／LangSysへ反映（3文字言語タグも対応）
- OpenTypeテーブル保持: 未編集のGSUB／GPOS／COLR／CPAL／SVGを含むMATH／JSTF／bitmap／AAT／meta／DSIGなどをTTF読み込み時に保持し、JSON／Glyphs／UFOを経由して再出力時にも復元。Feature／カラーソースから新しいテーブルを生成した場合は生成結果を優先し、glyf／CFF／可変コア等は再構築
- CPAL v1パレット: パレット名をUIで編集し、palette labelsとnameテーブルへ出力。明るい／暗い背景向けのPalette Typeフラグも編集・TTF／Glyphs保存・読み込みに対応
- GDEF合字カーソル: コンポーネント合字の境界位置からLigCaretListを自動生成し、合字内のカーソル移動を改善
- BASEベースライン: 水平／垂直軸へ`romn`／`ideo`／`hang`／`math`の標準ベースライン情報を出力し、混在スクリプトの配置基盤を追加
- Variable Fontメトリクス: マスター間の字幅差分をHVARへ出力し、軸変更時のadvance widthを保持
- Variable Font軸マッピング: 正規化座標の非線形変換をGUI／JSON／Glyphs／UFOから管理し、既存TTFからも読み込んで`avar`へ出力。未設定軸は恒等マッピングを補完
- Variable Font軸ラベル: 既存TTFのSTAT DesignAxis、またはSTATがない場合のfvar axisNameIDに紐づくnameレコードを読み込み、軸表示名として復元
- Variable Font軸フラグ: fvarのHidden AxisフラグをUI／JSON／TTF入出力で保持
- Variable Font縦メトリクス: マスター間の縦アドバンス差分をVVARへ出力し、縦組み時の軸変化を保持
- Variable Fontグローバルメトリクス: マスター別のAscender／Descender／Line Gapを編集し、hhea／OS/2の差分をMVARへ出力
- Variable Fontカーニング: マスター別の直接カーニング差分をGPOS Feature Variationsへ出力し、軸範囲に応じてLookupを切り替え
- OTF互換性: CFF Type 2 charstringへグリフ幅と空グリフ終端を正しく出力し、静的CFF/OTFを実用的に生成
- 縦組みOTF: CFF/OTFへVORGを生成し、グリフごとの縦原点と縦メトリクスを保持
- CFF2 OTF: 基準マスターからCFF2コンテナの静的OpenTypeを書き出し可能
- CFF2検証: HarfBuzzが利用可能な環境では、生成CFF2 OTFを実際にシェイプする回帰テストを実行
- OS/2／gasp: Unicode範囲・Windowsコードページビットを自動計算し、アウトライン向けのgasp設定を出力
- Unicode Variation Sequence: IVS（`FE00`〜`FE0F`／`E0100`〜`E01EF`）の割り当てをGUI／JSON／Glyphs／UFOから管理し、cmap format 14へ出力
- Unicode cmap互換性: Windows用Platform 3に加え、macOS／CoreText向けPlatform 0のformat 4／12／14を併記
- OpenTypeメタデータ: `head.macStyle`、`OS/2.fsSelection`、Typographic／WWS Name ID、既定文字・改行文字・最大Featureコンテキストをスタイル／ソースから自動生成
- Feature markClass: `markClass` と `pos base ... <anchor ...> mark ...` を読み取り、外部Featureソース由来のmark-to-baseもGPOSへ出力
- Feature mark-to-mark: `pos mark ... mark ...` を `markClass` と組み合わせて解析し、外部Featureソース由来のmkmkもGPOSへ出力
- 日常制作の導線: グリフ編集、コンポーネント、スペーシング、カーニング、マスター／補間、プレビュー、OpenType、検証、書き出しを一つの画面から連続して操作
- macOS配布: `Info.plist`を含む正規のApp Bundleとして起動可能。Retina／最小macOSバージョンも明示
- OpenType Class編集: feature本文から分離して編集・JSON/UFO保存し、書き出し時に自動合成
- カラーグリフ: カラー層・パレットの編集とCOLR v0互換レコード＋COLR v1 PaintColrLayers／PaintGlyph／PaintColrGlyph／PaintTransform／PaintSolid／PaintLinearGradient／PaintRadialGradient／PaintSweepGradient、CPAL v0出力。層ごとの変形、グラデーション種別・複数色ストップ・アルファ・範囲外拡張（Pad／Repeat／Reflect）をUIから編集可能
- カラーグリフ読み込み: COLR v1のPaintGlyph／Solid／Linear・Radial・Sweep Gradient／Transform／PaintColrGlyph／PaintColrLayersを編集可能なカラー層へ復元。Compositeなどモデル外のPaintはrawテーブルを保持
- SVG-in-OpenType: 通常グリフとカラー層を`SVG `テーブルへ埋め込み、対応レンダラ向けのSVGフォールバックを生成
- COLR v1 Composite: source／backdropのペイントグラフを順序付きカラー層へ展開し、Compositeを含むカラーグリフも編集可能な形で読み込み
- フォントメトリクス: UPM、Ascender、Descender、Line Gap、x-height、Cap height、縦アドバンス、縦TSB（TTF／UFO／JSON往復対応）
- 背景画像: PNG／JPEG／WebP／SVGをグリフ・マスター別に読み込み、編集キャンバスへ不透明度付きで表示
- 背景画像の位置・倍率・回転: X/Yオフセット、倍率、回転角をマスター別に調整・保存
- 背景画像の変形リセット: 位置・倍率・回転をワンクリックで初期化
- 背景画像の幅合わせ: 画像幅を現在グリフの幅へ自動フィット
- 背景画像の中央配置: 変形後の画像を現在グリフの中央へ配置
- 背景画像の反転: 左右／上下反転に対応

テキスト入力中は、入力欄のキーボード操作が優先されます。

## 検証

```sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## CLI／自動化

GUIを起動せず、プロジェクトの検証・補間チェック・グリフ名変更・フォント生成を実行できます。検証やチェックで問題が見つかった場合は終了コード1になります。

```sh
# プロジェクト全体を検証
target/release/glyph-studio validate MyFont.json
target/release/glyph-studio validate MyFont.json --json

# 2マスター間で補間できないグリフを確認
target/release/glyph-studio check-interpolation MyFont.json regular bold
target/release/glyph-studio check-interpolation MyFont.json regular bold --json

# グリフ名と参照を更新して別ファイルへ保存
target/release/glyph-studio rename-glyph MyFont.json old.name new.name MyFont-renamed.json

# マスター順を変更（deltaは前後へ移動する数。-1で前、+1で後）
target/release/glyph-studio move-master MyFont.json bold -1 MyFont-reordered.json

# マスターを複製（軸値・全グリフのレイヤーを引き継ぐ）
target/release/glyph-studio duplicate-master MyFont.json regular MyFont-duplicated.json

# カーニングペアを更新
target/release/glyph-studio set-kerning MyFont.json A V -80 MyFont-kerned.json

# 特定マスターのカーニングペアを更新
target/release/glyph-studio set-kerning-master MyFont.json bold A V -120 MyFont-bold-kerned.json

# TSV（左キー<TAB>右キー<TAB>値）から一括更新
target/release/glyph-studio set-kerning-batch MyFont.json kerning.tsv MyFont-kerned.json

# 特定マスターのカーニングをTSVから一括更新
target/release/glyph-studio set-kerning-master-batch MyFont.json bold kerning-bold.tsv MyFont-bold-kerned.json

# メトリクスキーをグリフ名リストへ一括適用
target/release/glyph-studio apply-metrics-keys MyFont.json metric-key-glyphs.txt MyFont-spaced.json

# TSV（グリフ名<TAB>左余白<TAB>右余白）から一括設定
target/release/glyph-studio set-sidebearings-batch MyFont.json bearings.tsv MyFont-spaced.json

# TSV（グリフ名<TAB>字幅）から一括設定
target/release/glyph-studio set-width-batch MyFont.json widths.tsv MyFont-widths.json

# TSV（グリフ名<TAB>Unicode）から一括設定
target/release/glyph-studio set-unicode-batch MyFont.json unicode.tsv MyFont-unicode.json

# コンポーネントを全マスターへ複製（番号は0始まり）
target/release/glyph-studio duplicate-component MyFont.json A 0 MyFont-components.json

# 対応アンカーでコンポーネントを全マスター位置合わせ
target/release/glyph-studio align-component MyFont.json A 0 MyFont-aligned.json

# グリフ内の全コンポーネントを一括位置合わせ
target/release/glyph-studio align-components MyFont.json A MyFont-aligned.json

# グリフ名リスト（1行1グリフ）を一括位置合わせ（#行はコメント）
target/release/glyph-studio align-components-batch MyFont.json glyphs.txt MyFont-aligned.json

# 検証してからフォントを書き出し（複数マスターならVariable TTF）
target/release/glyph-studio export MyFont.json --variable MyFont.ttf
target/release/glyph-studio export-cff2 MyFont.json MyFont-cff2.otf
target/release/glyph-studio build MyFont.json MyFont.woff2
target/release/glyph-studio set-opentype-source MyFont.json classes.txt features.txt

# Glyphs形式との変換・検証
target/release/glyph-studio export MyFont.json MyFont.glyphs
target/release/glyph-studio validate MyFont.glyphs
```

macOS App Bundleを作成する場合は、リポジトリルートで次を実行します。

```sh
sh scripts/package-macos.sh
```

入力はJSON／Glyphs／UFO／TTF／OTF／WOFF／WOFF2、出力はTTF／OTF／WOFF／WOFF2／Glyphsに対応しています。
`--json`を付けた検証コマンドは、`message`と対象`glyph_name`を持つJSON配列を標準出力へ返します。

Rustからは、GUIに依存しないCore APIも利用できます。

```rust
use glyph_studio::core::{build, export_ttf_at_interpolation, load_project, validate_project_detailed};
use std::path::Path;

let project = load_project(Path::new("MyFont.json"))?;
build(&project, Path::new("MyFont.ttf"))?;
export_ttf_at_interpolation(&project, "regular", "bold", 0.5, Path::new("MyFont-Medium.ttf"))?;
for issue in validate_project_detailed(&project) {
    eprintln!("{}: {}", issue.glyph_name.as_deref().unwrap_or("font"), issue.message);
}
if let Some(((left, right), value)) = project.kerning_source_for_glyphs("A", "V") {
    eprintln!("実効カーニング {left}/{right}: {value}");
}
# Ok::<(), String>(())
```
