# コマンドライン

[简体中文](zh-CN.md) · [English](en.md) · 日本語 · [Español](es.md) · [README](../readme/ja.md)

**PowerShell 7** から `LumaRecode.exe` を実行します。引数なしでは GUI が開き、`probe` や `convert` などのサブコマンドを指定すると CLI が動作します。

本プログラムは Windows GUI サブシステムを使用します。**テキスト出力は `Out-Host`、JSON 出力は `ConvertFrom-Json` にパイプ接続**すると、終了を待って `$LASTEXITCODE` を取得できます。他のプログラムからはプロセス API で終了を待機できます。

出力は UTF-8 です。以下の例では、現在の PowerShell セッションの出力エンコーディングを設定します。

## 使い始める

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --lang ja --help | Out-Host
./LumaRecode.exe --version | Out-Host
./LumaRecode.exe license --text | Out-Host
./LumaRecode.exe license --accept-terms | Out-Host
./LumaRecode.exe doctor --json | ConvertFrom-Json
./LumaRecode.exe probe "D:\Videos\clip.mp4" --json | ConvertFrom-Json
./LumaRecode.exe convert "D:\Videos\clip.mp4" | Out-Host
```

コンポーネントの利用条件への同意は GUI と CLI で共有します。初回の一括処理では、`license --text` で条件を確認してから `license --accept-terms` を実行するか、タスクに `--accept-terms` を追加します。`--no-settings` は保存済みの GUI 設定を読み込まず、設定も書き込みません。このモードでは `--accept-terms` を追加してください。

ツールは `PATH`、保存済みの GUI パスの順に検索します。`--ffmpeg PATH --ffprobe PATH` で指定できます。`--lang system|en|zh|ja|es` は出力言語を選択します。

動的 HDR 素材はソフトウェア HEVC でメタデータを保持する設定が初期値です。標準 HDR には `--dynamic-hdr standard --purpose share`、SDR には `--range sdr` を使用します。ProRes RAW は自動的に HLG に現像し、`--range sdr` で SDR に出力します。対応形式は[色変換（中国語・英語）](../color-conversion.md#english)を参照してください。

## コマンド

| コマンド | 内容 |
| --- | --- |
| `probe INPUT` | 動画・音声形式、ビット深度、色・Log の識別、ストリーム、タグ、先頭フレームの付加データ |
| `presets` | エンコードプリセットとエンコーダー設定 |
| `profiles` | DJI、Samsung、Xiaomi、Apple Log の内蔵プロファイルと LUT |
| `frames INPUT` | フレーム数、長さ、タイムベース |
| `image INPUT` | HDR / SDR 静止画またはモーションフォトを書き出す |
| `doctor` | ツールのバージョン、エンコーダーとフィルターの一覧。ハードウェア処理は GPU とドライバーに依存 |
| `cache [--purge]` | キャッシュ一覧と回収計画。`--purge` は本プログラムのキャッシュ根だけを削除 |
| `license [--text]` | MIT ライセンス、コンポーネントの利用条件、通知ディレクトリとマニフェスト |
| `plan INPUT...` | 変換設定、FFmpeg の引数、作業ディレクトリ、出力の色設定をプレビュー |
| `convert INPUT...` | ファイル、ディレクトリ、ワイルドカードに一致する入力を順番に変換し、出力を検証 |

各コマンドは `--help` に対応します。素材情報、プリセット一覧、診断、キャッシュ、ライセンス情報の出力は JSON です。テキストの進捗は stderr、結果は stdout に出力します。

## エンコードと色のオプション

`convert` と `plan` では GUI と同じエンコード・色設定を指定できます。

| オプション | 値 |
| --- | --- |
| `--preset` | `share-avc`, `share-hevc`, `share-av1`, `archive-avc`, `archive-hevc`, `archive-av1`, `archive-av1-fast` |
| `--purpose` | `share`（Intel QSV）/ `archive`（CPU）。x64 は共有、ARM64 はアーカイブが初期値。動的 HDR 素材はアーカイブを使用 |
| `--codec` | `hevc`（標準）、`av1`, `avc` |
| `--range` / `--dynamic-range` | `auto`（標準）、`sdr`, `hdr` |
| `--transform` | `auto`（標準）、`none`, `log-hdr`, `log-sdr`, `hdr-sdr` |
| `--profile` | `auto`, `pocket3`, `samsung`, `milog`, `applelog`, `applelog2` |
| `--dynamic-hdr` | `preserve` / `standard` |
| `--input-kind` | `auto`, `sdr`, `hlg`, `pq`, `log`, `raw` |
| `--sampling` | `auto`（標準）、`source`, `420`, `422`, `444`, `gray`。Auto はソフトウェア HEVC / AVC のアーカイブで元のサンプリングを保持し、RGB は 4:4:4、AV1 / 共有は 4:2:0 を選択。422 / 444 / gray はソフトウェア HEVC / AVC を使用 |
| `--color-range` | `auto`（標準）、`source`, `limited`, `full`。Auto はアーカイブで元のレンジを保持し、共有では Limited に変換 |
| `--bit-depth` | `auto`, `8`, `10`, `12`。アーカイブは入力とエンコーダーの対応範囲に従い、共有は SDR 8 ビット / HDR 10 ビット |
| `--container` | `auto`（共有は MP4、アーカイブは MKV）、`mp4`, `mkv` |
| `--audio` | `auto`, `aac`, `opus`, `copy`, `flac`。初期値は GUI と同じ |
| `--variant` | `standard`, `fast`（ソフトウェア AV1 のみ） |
| `--cpu-decode` / `--no-cpu-decode` | ソフトウェアデコードを指定 / デコード方式を自動選択 |
| `--duration SECONDS` | サンプル用に正の有限秒数を指定してエンコード |
| `--config FILE.json` | エンコード設定を読み込み。明示した CLI オプションが優先 |

Auto は SDR → SDR、HLG / PQ → HDR、Log → HLG を選択します。`--profile` で Log 曲線、`--input-kind` で入力の種類を指定できます。HDR 出力には HEVC / AV1 を使用します。

サンプリングとビット深度は入力の上限を確認します。8 ビット 4:2:0 はそのまま保持し、10 ビット 4:2:2 は 8 / 10 ビットと 4:2:0 / 4:2:2 を選択できます。ソフトウェア HEVC は 8 / 10 / 12 ビット、ソフトウェア AVC と AV1 は 8 / 10 ビットに対応します。`--sampling source` は RGB を 4:4:4 YCbCr に変換します。グレースケール入力には `source` または `gray` を使用できます。

レンジ変換は画素値を処理します。レンジタグがない素材では、画素形式とカメラプロファイルから入力を解釈します。

`--audio flac` は全音声トラックを確認し、可逆形式と確認できる整数音声を受け付けます。サンプルレート、チャンネル数、整数精度を保持し、コンテナーには MKV を使用します。

```powershell
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
./LumaRecode.exe convert hdr.mp4 --range sdr | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range sdr | Out-Host
./LumaRecode.exe convert source-422.mov --preset archive-hevc --sampling 422 --audio copy | Out-Host
./LumaRecode.exe plan clip.mp4 --preset archive-av1-fast --json | ConvertFrom-Json
./LumaRecode.exe convert lossless.mov --preset archive-hevc --sampling source --color-range source --audio flac | Out-Host
```

HDR / Log から SDR への変換には FFmpeg の libplacebo と Vulkan が必要です。JSON 設定では上記のエンコード項目をアンダースコア形式のキーで指定でき、`preset` も使用できます。

```json
{"preset":"archive-hevc","range":"auto","sampling":"420","bit_depth":8,"audio":"copy","cpu_decode":true}
```

優先順位は、GUI 相当の初期値 → プリセット → 設定ファイル → 明示した CLI オプションです。CLI の `--preset` は設定ファイルの `preset` より優先され、他の設定項目は引き続き適用されます。設定キーには `color_range` のようにアンダースコアを使用します。`range` はダイナミックレンジで、保存先は `--output` で指定します。

## フレーム範囲と画像

`frames INPUT` はフレーム数、長さ、タイムベースを返します。CLI のフレーム番号は 0 始まりで、終了フレームは含まれません。

```powershell
./LumaRecode.exe frames clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --start-frame 30 --end-frame 90 --audio aac | Out-Host
./LumaRecode.exe convert clip.mp4 --start-time 1.5 --end-time 4 --audio aac | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr -o photo.jpg | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr --motion --motion-start-frame 30 --motion-end-frame 90 -o photo_MP.jpg | Out-Host
```

フレーム範囲と秒数範囲、および `--duration` は併用できません。秒数は指定位置以降の最初のフレームに合わせます。トリミングでは音声を再エンコードします。アーカイブの初期設定では FLAC または AAC を選択し、明示的な `--audio copy` は AAC、Opus、対応する FLAC に変更する必要があります。

`image` は入力を 1 つ指定します。

| オプション | 内容 |
| --- | --- |
| `--frame N` | 書き出すフレーム。初期値 0 |
| `--image-format auto\|hdr\|ultrahdr\|sdr\|png` | HDR AVIF、Ultra HDR JPEG、SDR JPEG、SDR PNG。auto は HDR に Ultra HDR、SDR に JPEG を使用 |
| `--quality 1..100` | 画質。初期値 95。PNG は可逆圧縮 |
| `--motion` | JPEG / Ultra HDR に Android モーションフォト動画を追加 |
| `--motion-start-frame N --motion-end-frame N` | カバーを含む動画範囲。初期設定はカバー前後の約 3 秒 |
| `--motion-size source\|1080\|720` | 動画の解像度。初期値は元の解像度 |
| `--purpose archive\|share` | 動画の CPU / Intel エンコード。初期値 `archive` |
| `--input-kind`、`--profile` | 動画変換と同じ入力解釈とカメラカーブ |
| `-o PATH`、`--overwrite`、`--skip-existing` | 保存先と既存ファイルの処理 |
| `--dry-run` | フレームと動画範囲の情報を表示 |

モーションフォトは単一の `MP.jpg` に保存します。HDR 画像には HDR、Log または RAW 素材が必要です。形式と Google 仕様は [README](../readme/ja.md) を参照してください。

## 一括処理と保存先

```powershell
./LumaRecode.exe convert a.mp4 b.mov --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos" --recursive --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos\**\*.mp4" --recursive --output-dir "D:\Exports" --continue-on-error --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --output "D:\Exports\result.mp4" --overwrite | Out-Host
```

ワイルドカードは引用符で囲みます。`**` には `--recursive` が必要です。ディレクトリの検索対象は一般的な動画拡張子で、相対サブディレクトリ構造を保持します。ワイルドカードでは一致したファイルの共通親ディレクトリを基準にし、個別に指定したファイルにはそのファイル名を使用します。保存先の重複は変換前に検出します。必要な出力ディレクトリは自動作成します。

標準の出力名は、入力と同じディレクトリの `stem_SDR-or-HDR_codec_purpose.mp4-or-mkv` です。単一ファイルには `--output`、一括処理には `--output-dir` を使用します。この 2 つは同時に指定できません。対話実行では、既存の出力を上書きする前に確認し、既定ではキャンセルします。スクリプトでは `--skip-existing` でスキップ、`--overwrite` で上書きを許可できます。一時出力をエンコード・検証してから保存先に置き換えます。入力と出力には異なるパスを指定してください。失敗時やキャンセル時は一時ファイルを削除します。

ジョブは順番に実行し、標準ではエラーで停止します。`--continue-on-error` は残りの入力も処理し、失敗があればバッチの終了コードに反映します。

プレビューの `argv` は最終出力パスを表示します。実際の変換は一時ディレクトリに書き込み、検証後に保存先へ移動します。`--overwrite` はアトミックな置き換えを使用します。

## スクリプトと進捗

`convert --json` / `plan --json` は `schema_version: 1` の JSON Lines を stdout に出力します。イベントは `start`, `log`, `status`, `decoder`, `progress`, `done`, `skipped`, `error`, `cancelled`, `summary` で、プレビューは `plan` を出力します。ジョブのレコードには `job`, `total_jobs`, `input`, `output` とイベントの `data` が含まれます。

`decoder.data`: `software` / `qsv`.

進捗には `fps`, `percent`, `position_seconds`, `duration_seconds`, `elapsed_seconds`, `eta_seconds` が含まれます。不明な進捗率・残り時間は null です。エラーには固定の `code` と説明の `message` があります。`--log FILE` は同じイベントを JSON Lines ファイルに追記し、`--verbose` はテキストモードで FFmpeg のログを表示します。

```powershell
./LumaRecode.exe convert clip.mp4 --json --log job.jsonl | ForEach-Object { $_ | ConvertFrom-Json }
$LASTEXITCODE
```

| 終了コード | 意味 |
| --- | --- |
| 0 | 成功。指定に従ったスキップを含む |
| 1 | 素材解析、エンコード、出力検証の失敗、または一括処理の一部失敗 |
| 2 | 不正な引数、設定、オプションの組み合わせ |
| 3 | ランタイムまたは FFmpeg ツールがない、もしくは無効 |
| 4 | 入力なし、一致なし、入力形式または Log 曲線を特定できない |
| 5 | 出力・ログのパス競合またはエラー |
| 6 | コンポーネントの利用条件に未同意 |
| 130 | CLI の協調的キャンセル |

結果の判定にはプロセスの終了コードまたは `$LASTEXITCODE`、エラー処理には JSON の `code` を使用してください。

対話実行中の Ctrl+C は PowerShell のパイプラインと変換ジョブを中断し、一時出力を削除します。キャンセルイベントと終了コードを取得するには、プロセス API からコンソールのキャンセルシグナルを送り、終了を待ちます。協調的キャンセルは 130 を返します。
