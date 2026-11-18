<p align="center">
  <img src="../../Brand/LumaRecode.svg" width="104" height="104" alt="LumaRecode アイコン" />
</p>
<h1 align="center">映码 · LumaRecode WBNEXT</h1>

[简体中文](../../README.md) | [English](en.md) | 日本語 | [Español](es.md)

> **WBNEXT リファクタープレビュー版**：[LumaRecode 4.4.3](https://github.com/MIKU552/LumaRecode) を基にした独立のクロスプラットフォーム版です。WorkBuddy（DeepSeek-V4.1-Flash-1017、高推論、1M コンテキスト）の支援で開発しており、**正式リリースではありません**。ビルド成果物は GitHub Actions の Artifacts から取得し、進捗は[リファクタリング記録（中国語・英語）](../refactor/README.md#english)にまとめています。

C# / .NET で開発した Windows 11 向けの動画変換・色変換ツールです。ネイティブ WinUI 3 インターフェイスとコマンドラインを備え、FFmpeg による動画圧縮、カメラ Log から HDR / SDR への変換、HDR から SDR への変換を行います。

[ダウンロード](https://github.com/MIKU552/LumaRecode/releases/latest) · [ドキュメント（中国語・英語）](../README.md#english) · [コマンドライン](../cli/ja.md) · [変更履歴（中国語・英語）](../../CHANGELOG.md#english)

## 主な機能

- フレームプレビューとトリミング、HDR / SDR 画像、カバーフレームを指定した Android モーションフォト。
- クイック共有・アーカイブ用プリセット。HEVC、AV1、H.264 と MP4 / MKV に対応。
- DJI D-Log M、Samsung Log、Mi-Log、Apple Log、Apple Log 2 から HLG HDR / Rec.709 SDR への変換、HLG / PQ から SDR への変換。
- ProRes RAW の現像、Dolby Vision / HDR10+ の識別と変換、ソフトウェアエンコードによる動的メタデータの保持。
- 解像度、フレームレート、ビット深度、色タグ、音声トラックを読み取り、素材に応じた初期設定を適用。
- クロマサブサンプリング、Limited / Full レンジ、音声形式の選択。可逆音声の FLAC 変換に対応。
- 進捗、エンコード fps、処理位置、残り時間の表示と出力形式の検証。
- 変換の完了・失敗を Windows 通知でお知らせします。「フォルダーを開く」の左側で切り替えられます。
- Mica、ライト / ダーク / システムテーマ、アクセントカラー。簡体字中国語、英語、日本語、スペイン語に対応。
- CLI での一括処理、設定ファイル、コマンドのプレビュー、JSON イベント出力。

## 使い始める

1. [Releases](https://github.com/MIKU552/LumaRecode/releases) から、お使いのデバイスに合う EXE をダウンロードします。各版の要件は次の表を参照してください。必要なランタイムは同梱されています。
2. `ffmpeg.exe` と `ffprobe.exe` を含む Windows 版 [FFmpeg（英語）](https://ffmpeg.org/download.html) を用意します。初回起動時に `PATH` を検索します。右上の歯車からパスを指定することもできます。
3. 動画をドロップするか「動画を選択」を押し、出力のダイナミックレンジ、プリセット、色変換を確認します。
4. 保存先を指定し、「変換を開始」を押します。

クイック共有は Intel QSV、アーカイブは CPU エンコードを使用します。

| ファイル | プラットフォームと要件 |
| --- | --- |
| `LumaRecode.exe` | X64。汎用 x64 命令セット構成 |
| `LumaRecode-ARM64.exe` | Windows ARM64。アプリと画像コーデックは ARM64 ネイティブ |

ARM64 の初期設定はアーカイブです。エンコード速度は外部 FFmpeg、ハードウェア、プリセットによって変わります。

Dolby Vision / HDR10+ は、ソフトウェア HEVC で動的メタデータを保持する設定が初期値です。共有用にハードウェアで出力する場合は「標準 HDR」を選択します。対応する Dolby Vision プロファイルは単層の 5、8.1、8.4、10.1、10.4 です。ProRes RAW は FFmpeg の対応状況を確認し、カメラのホワイトバランス、色変換行列、ゲインで現像し、標準で HLG に出力します。対応形式は[色変換（中国語・英語）](../color-conversion.md#english)を参照してください。

## プレビュー・トリミング・画像

動画ページの「プレビューとフレーム単位のトリミング」で、タイムラインの移動、コマ送り、開始・終了フレームの指定ができます。初期設定では動画全体を変換します。画面上のフレーム番号は 1 始まりで、終了フレームも含まれます。

「画像・モーションフォト」ページでは、選択したフレームを次の形式で書き出せます。

| 形式 | 出力 |
| --- | --- |
| HDR AVIF | BT.2020 / PQ、10 または 12 ビット |
| Ultra HDR JPEG | SDR 基本画像、HDR ゲインマップとメタデータ |
| SDR JPEG | sRGB 画像 |
| SDR PNG | 可逆圧縮の sRGB 画像、8 ビットまたは 16 ビットコンテナー |

JPEG と Ultra HDR JPEG には Android モーションフォト動画を追加できます。カバーフレームと動画範囲を個別に指定でき、初期範囲はカバー前後の約 3 秒です。[Google Motion Photo 1.0（英語）](https://developer.android.com/media/platform/motion-photo-format) に従い、単一の `MP.jpg` ファイルに保存します。Ultra HDR のエンコードには Google の [libultrahdr（英語）](https://github.com/google/libultrahdr) を使用します。動画は CPU エンコードが初期設定で、Intel による高速エンコードも選択できます。

プレビューは SDR 表示です。画像は選択した形式で書き出します。フレーム単位のトリミングでは、音声を AAC、Opus または条件に合う FLAC に再エンコードします。

動画の解像度は元のままが初期設定で、最大 1080p または 720p も選べます。短い動画用の専用プリセットは x264 fast / CRF20、x265 fast / CRF23、または Intel TU6 を使用します。音声は AAC-LC、48 kHz で、モノラルを保持し、それ以外はステレオにします。設定は [motion-photo-presets.json](../../Assets/motion-photo-presets.json) を参照してください。

## エンコードと色変換

| 用途 | エンコーダー | 標準のコンテナー | 標準の音声処理 |
| --- | --- | --- | --- |
| クイック共有 | Intel QSV：HEVC / AV1 / H.264 | MP4 | HEVC / H.264 は AAC、AV1 は Opus |
| アーカイブ | x265 / SVT-AV1 / x264 | MKV | 元の音声トラックをコピー |

コンテナー、サンプリング、ビット深度、カラーレンジ、音声形式は個別に選択できます。クイック共有の初期値は SDR が 8 ビット、HDR が 10 ビットです。アーカイブはエンコーダーの対応範囲内で元のビット深度を保持します。HEVC は 8 / 10 / 12 ビット、AVC と AV1 は 8 / 10 ビットに対応します。

ソフトウェア HEVC / H.264 のアーカイブでは元のサンプリングを保持します。RGB 入力は標準で 4:4:4 YCbCr に変換し、AV1 とクイック共有は 4:2:0 を使用します。サンプリングとビット深度は入力の範囲内で下げられます。8 ビット 4:2:0 はそのまま保持し、10 ビット 4:2:2 は 10 / 8 ビット、4:2:2 / 4:2:0 を選択できます。アーカイブは元の Limited / Full レンジを保持し、クイック共有は Limited を使用します。サンプリング、ビット深度、レンジは「色変換」、コンテナーと音声は「形式と音声」にあります。

FLAC は整数 PCM と、可逆形式と確認できる整数音声に対応し、各トラックのサンプルレート、チャンネル数、整数精度を保持します。FLAC 音声には MKV を使用します。

SDR 素材は SDR 出力、HLG / PQ 素材は HDR 出力が初期値です。曲線を識別できた Log 素材は HLG に変換します。「入力形式」と「カメラプロファイル」で素材の種類を手動指定できます。

動画は選択したプリセットで再エンコードします。音声はコピーまたは変換できます。設定値は [recommended-presets.json](../../Assets/recommended-presets.json)、測定結果は[エンコードベンチマーク（中国語・英語）](../benchmarks/encoding-benchmark.md#english)を参照してください。

## 動作要件

- Windows 11 x64 または ARM64。選択した版の命令セット要件を満たすこと。
- 必要なエンコーダーとフィルターを含む FFmpeg / ffprobe。
- クイック共有には対応する Intel QSV 環境。
- Log 変換には `zscale` と `lut3d`。HDR / Log から SDR への変換には `libplacebo` と Vulkan。

言語とテーマは初期状態でシステムに従います。中国語、英語、日本語、スペイン語のシステム言語に対応し、それ以外は英語になります。設定は `%LOCALAPPDATA%\LumaRecode\settings.json` に保存します。

初回起動時に、FrameFlow の言語、外観、ツールのパス設定を引き継ぎます。

Source Sans 3、源ノ角ゴシック（Source Han Sans）、Source Code Pro を同梱しているため、フォントのインストールは不要です。

## コマンドライン

PowerShell 7 で出力をパイプに接続して実行します。

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --lang ja --help | Out-Host
./LumaRecode.exe probe clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
```

初めて無人実行する前に `license --text` でコンポーネントの利用条件を確認し、`license --accept-terms` を実行してください。全コマンド、一括処理、終了コードは[CLI ガイド](../cli/ja.md)に記載しています。

## ソースからビルド

.NET SDK 11.0.100 と PowerShell 7 が必要です。オプションの AOT ビルドには Visual Studio C++ ツールチェーンも必要です。

```powershell
git clone https://github.com/MIKU552/LumaRecode.git
cd LumaRecode
./build.ps1
./build.ps1 -Target X64
./build.ps1 -Target X64 -Compilation Aot
./build.ps1 -Target Arm64
```

既定は JIT です。Native は Windows のシステムアーキテクチャに応じて X64 または ARM64 を選択し、`dist/SelfContained/LumaRecode.exe` を生成します。Releases では X64 と ARM64 の自己完結型単一 EXE を配布します。

JIT は実行環境で利用できる命令セットを選択します。CPU と Windows が APX に対応する x64 環境では、明示的なユーザー設定を維持しながら起動時に APX を有効にします。`-Compilation Aot` は公開時にコンパイルし、`dist/Aot-SelfContained[-Target]/` に出力します。APX 固定ターゲットには `-Target Apx -Compilation Aot` を指定します。

Visual Studio 2026 18.12 以降で `LumaRecode.sln` を開き、`LumaRecode.App` をスタートアッププロジェクトに設定します。Debug はデバッグ用です。`Release | Native`、`Release | x64`、`Release | ARM64` は JIT、`Release-AOT` は対応する AOT、`Release-APX | x64` は APX AOT を生成します。`Packaging` が単一 EXE を作成します。[開発・配布（中国語・英語）](../dependencies.md#english)と [QA ガイド（中国語・英語）](../../qa/README.md#english)を参照してください。

## ディレクトリ構成

```text
App/       WinUI 3 UI、外観、ローカライズ、ジョブ操作
Bundle/    単一ファイルへのパッケージ化
Packaging/ Visual Studio のビルド・配布処理
Core/      素材解析、形式の規則、FFmpeg の実行、CLI
Assets/    LUT、エンコードプリセット、配布元の記録
Brand/     アイコンのソースと生成ツール
tools/     ライセンス収集とローカライズコードの生成
qa/        自動チェックとメモリ測定
docs/      使用方法、開発ドキュメント、ベンチマーク
licenses/  第三者のライセンス、通知、配布元の記録
```

## Code signing policy

Windows 版は**署名しません**。これはプロジェクトの決定であり、コード署名証明書の申請も行いません。リリースページにパッケージの SHA-256 を掲載するので、ダウンロード後に照合できます。初回起動時の SmartScreen の警告は想定どおりで、システムの保護を無効にする必要はありません。

macOS 版は無償の ad-hoc 署名を使用し、Apple Developer ID も公証も使いません。初回起動時は「システム設定 → プライバシーとセキュリティ」で「このまま開く」を選択します。配布資料にこの手順を記載し、Gatekeeper の無効化は求めません。

役割: 保守とリリース承認は MIKU552 が担当し、コミッターとレビュアーは同一アカウントです（単独保守）。正式署名は毎回承認者が手動で確認します。

プライバシー: テレメトリはなく、診断情報や素材を自動送信しません。外部リンクを開くかシステムの共有を使うときだけ通信します。FFmpeg / ffprobe は利用者が用意し、本プログラムが代わりにダウンロードすることはありません。

## 開発とライセンス

本プロジェクトの独自コードは、**Windows 版 Codex** で **[GPT-6.1 Sol（英語）](https://developers.openai.com/api/docs/models/gpt-6.1-sol)** と **Extra high（`xhigh`）** 推論設定を使用して開発しました。**[MIKU552](https://github.com/MIKU552)** が要件を定義し、機材と素材を提供して実機検証を行っています。

独自のコード、ドキュメント、画像、生成したカメラ Log LUT には **[MIT License（英語）](../../LICENSE)** が適用されます。第三者の LUT、フォント、ランタイムのライセンスと配布元は [NOTICE（英語）](../../NOTICE) と[第三者コンポーネント（中国語・英語）](../../Assets/THIRD_PARTY.md#english)に記載しています。

## ドキュメント

- [コマンドライン](../cli/ja.md)
- [開発と配布（中国語・英語）](../dependencies.md#english)
- [性能（中国語・英語）](../performance.md#english)
- [エンコードベンチマーク（中国語・英語）](../benchmarks/encoding-benchmark.md#english)
- [変更履歴（中国語・英語）](../../CHANGELOG.md#english)
