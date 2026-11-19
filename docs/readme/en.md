<p align="center">
  <img src="../../Brand/LumaRecode.svg" width="104" height="104" alt="LumaRecode icon" />
</p>
<h1 align="center">映码 · LumaRecode WBNEXT</h1>

[简体中文](../../README.md) | English | [日本語](ja.md) | [Español](es.md)

> **WBNEXT refactor preview**: this branch is an independent cross-platform rewrite based on [LumaRecode 4.4.3](https://github.com/MIKU552/LumaRecode), developed with WorkBuddy (DeepSeek-V4.1-Flash-1017, high reasoning, 1M context). It is **not an official release** and carries no support commitment from upstream. Packages are on [Releases](https://github.com/MIKU552/LumaRecode-WBNEXT/releases); progress is tracked in the [refactor record](../refactor/README.md#english).

Video transcoding and color conversion for Windows 11, built with C# and .NET. LumaRecode provides a native WinUI 3 interface and a command line for FFmpeg, with video compression, camera Log to HDR / SDR, and HDR to SDR workflows.

[Download](https://github.com/MIKU552/LumaRecode/releases/latest) · [Documentation](../README.md#english) · [Command line](../cli/en.md) · [Changelog](../../CHANGELOG.md#english)

## Features

- Frame preview and trimming, HDR / SDR stills, and Android Motion Photos with a selected cover frame.
- Quick share and Software archive presets for HEVC, AV1, and H.264, with MP4 / MKV output.
- DJI D-Log M, Samsung Log, Mi-Log, Apple Log and Apple Log 2 to HLG HDR or Rec.709 SDR; HLG / PQ to SDR.
- ProRes RAW development, Dolby Vision / HDR10+ detection and conversion, and dynamic metadata preservation with software encoding.
- Source inspection for resolution, frame rate, bit depth, color tags, and audio tracks, with automatic defaults.
- Chroma sampling, Limited / Full range, and audio controls, including lossless FLAC audio.
- Progress, encoding fps, position, remaining time, and output format verification.
- Windows notifications for completed and failed jobs, controlled by the checkbox beside Open folder.
- Mica, light / dark / system themes, accent colors, and Simplified Chinese, English, Japanese and Spanish.
- Batch processing, configuration files, command previews, and JSON events through the CLI.

## Quick start

1. Download the package for your device from [Releases](https://github.com/MIKU552/LumaRecode-WBNEXT/releases); the table below lists them. Required runtimes are included.
2. Pick one of the two shapes: a plain package expects [FFmpeg](https://ffmpeg.org/download.html) on your machine (`ffmpeg` and `ffprobe`, found on `PATH` or chosen with the gear button), while a `-full` package carries it and works right away.
3. Drop a video or choose **Choose video**, then confirm the output range, preset, and color conversion.
4. Choose the output path and select **Start conversion**.

Quick share uses Intel QSV on Windows and VideoToolbox on macOS. Software archive uses CPU encoding.

| Download | Platform | FFmpeg |
| --- | --- | --- |
| `LumaRecode-5.0.0-winx64.zip` | Windows x64, common instruction-set profile | Bring your own |
| `LumaRecode-5.0.0-winx64-full.zip` | Windows x64 | Included |
| `LumaRecode-5.0.0-winarm64.zip` | Windows ARM64, native app and image codec | Bring your own |
| `LumaRecode-5.0.0-winarm64-full.zip` | Windows ARM64 | Included |
| `LumaRecode-5.0.0-arm64.zip` | macOS 27 or later, Apple silicon | Bring your own |
| `LumaRecode-5.0.0-arm64-full.zip` | macOS 27 or later, Apple silicon | Included, with the MoltenVK libplacebo needs |

ARM64 defaults to Software archive. Encoding speed depends on the FFmpeg in use (yours or the bundled one), the hardware and the preset.

Dolby Vision / HDR10+ sources default to software HEVC with dynamic metadata preservation. Select Standard HDR for hardware quick sharing. Supported Dolby Vision profiles are single-layer 5, 8.1, 8.4, 10.1 and 10.4. ProRes RAW checks FFmpeg decoder compatibility, then develops with camera white balance, color matrix and gain, with HLG output by default. See [Color conversion](../color-conversion.md#english) for formats and processing paths.

## Preview, trimming and images

Open **Preview & frame trimming** on the video page to scrub, step through frames and set the first and last frames. The default is the whole video. GUI frame numbers start at 1 and include the selected last frame.

The **Images & Motion Photos** page exports the selected frame:

| Format | Output |
| --- | --- |
| HDR AVIF | BT.2020 / PQ, 10 or 12 bits |
| Ultra HDR JPEG | SDR base image, HDR gain map and gain-map metadata |
| SDR JPEG | sRGB image |
| SDR PNG | Lossless sRGB image, 8-bit or a 16-bit container |

JPEG and Ultra HDR JPEG can include Android Motion Photo video. Choose the cover and video range independently; the default is about three seconds around the cover. A single `MP.jpg` file follows [Google Motion Photo 1.0](https://developer.android.com/media/platform/motion-photo-format). Ultra HDR encoding uses Google's [libultrahdr](https://github.com/google/libultrahdr). Motion video defaults to CPU encoding, with Intel quick encoding available.

The frame preview is SDR. Export uses the selected image format. Frame-accurate video cuts re-encode audio as AAC, Opus or eligible FLAC.

Motion video keeps the source resolution by default, with optional 1080p and 720p limits. Dedicated short-clip presets use x264 fast / CRF20, x265 fast / CRF23, or Intel TU6. Audio is AAC-LC at 48 kHz, preserving mono or using stereo. See [motion-photo-presets.json](../../Assets/motion-photo-presets.json).

## Encoding and color

| Purpose | Encoders | Default container | Default audio |
| --- | --- | --- | --- |
| Quick share | Intel QSV: HEVC / AV1 / H.264 | MP4 | AAC for HEVC / H.264; Opus for AV1 |
| Software archive | x265 / SVT-AV1 / x264 | MKV | Copy source tracks |

Container, chroma sampling, bit depth, color range, and audio can be selected separately. Quick share defaults to 8-bit SDR and 10-bit HDR. Software archive keeps source bit depth within encoder limits: 8 / 10 / 12 bits for HEVC and 8 / 10 bits for AVC and AV1.

Software HEVC / H.264 archive keeps source sampling. RGB input defaults to 4:4:4 YCbCr; AV1 and Quick share use 4:2:0. Sampling and bit depth can be reduced within source limits: 8-bit 4:2:0 stays 8-bit 4:2:0, while 10-bit 4:2:2 allows 10 / 8 bits and 4:2:2 / 4:2:0. Archive keeps source Limited / Full range; Quick share defaults to Limited. Sampling, depth, and range are in **Color conversion**; container and audio are in **Format & audio**.

FLAC accepts confirmed lossless integer audio, including integer PCM, and preserves each track's sample rate, channels, and integer precision. FLAC audio uses MKV.

SDR sources default to SDR output, HLG / PQ sources retain HDR, and recognized Log profiles convert to HLG. Input interpretation and Camera profile let you select the source type manually.

Video is re-encoded with the selected preset; audio can be copied or transcoded. See [recommended-presets.json](../../Assets/recommended-presets.json) for the settings and the [encoding benchmark](../benchmarks/encoding-benchmark.md#english) for measurements.

## Requirements

- Windows 11 x64 or ARM64, with the instructions required by the selected variant.
- User-provided FFmpeg / ffprobe with the required encoders and filters.
- Intel QSV support for Quick share.
- `zscale` and `lut3d` for Log conversion; `libplacebo` and Vulkan for HDR / Log to SDR.

Language and theme follow the system by default. Chinese, English, Japanese and Spanish system languages are recognized; other languages default to English. Settings are stored at `%LOCALAPPDATA%\LumaRecode\settings.json`.

On first launch, LumaRecode imports language, appearance and tool paths from FrameFlow settings.

Source Sans 3, Source Han Sans, and Source Code Pro are included; no font installation is needed.

## Command line

Run commands through an output pipeline in PowerShell 7:

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --help | Out-Host
./LumaRecode.exe probe clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
```

For first-time unattended use, read the component terms with `license --text`, then run `license --accept-terms`. See the [CLI guide](../cli/en.md) for commands, batch processing, and exit codes.

## Build from source

Requires .NET SDK 11.0.100 and PowerShell 7. Optional AOT builds also require the Visual Studio C++ toolchain.

```powershell
git clone https://github.com/MIKU552/LumaRecode.git
cd LumaRecode
./build.ps1
./build.ps1 -Target X64
./build.ps1 -Target X64 -Compilation Aot
./build.ps1 -Target Arm64
```

JIT is the default. Native selects X64 or ARM64 from the Windows system architecture and writes `dist/SelfContained/LumaRecode.exe`. Releases provide self-contained single EXEs for X64 and ARM64.

The JIT selects supported instructions on the running device. On x64 devices where both the CPU and Windows support APX, startup opts into APX while preserving explicit user settings. Optional `-Compilation Aot` compiles at publish time and writes to `dist/Aot-SelfContained[-Target]/`. The fixed APX target uses `-Target Apx -Compilation Aot`.

Open `LumaRecode.sln` in Visual Studio 2026 18.12 or later. Set `LumaRecode.App` as the startup project and use Debug for debugging. `Release | Native`, `Release | x64` and `Release | ARM64` produce JIT builds. `Release-AOT` provides corresponding AOT builds; `Release-APX | x64` produces APX AOT. `Packaging` creates the single EXE. See [development and distribution](../dependencies.md#english) and the [QA guide](../../qa/README.md#english).

## Project structure

```text
App/       WinUI 3 UI, appearance, localization, and job interaction
Bundle/    Single-file packaging entry
Packaging/ Visual Studio build and distribution entry
Core/      Source inspection, format policies, FFmpeg execution, and CLI
Assets/    LUTs, encoding presets, and source records
Brand/     Icon sources and generation tools
tools/     License collection and localization code generation
qa/        Automated checks and memory sampling
docs/      User guides, development notes, and benchmark data
licenses/  Third-party licenses, notices, and provenance
```

## Builds are unsigned

Windows builds ship **unsigned**; that is the project's decision and no code signing certificate will be requested. The release page publishes a SHA-256 for every package (`SHA256SUMS.txt`) so a download can be checked, and the SmartScreen prompt on first run is expected — no system protection needs to be turned off.

The macOS build uses a free ad-hoc signature: no Apple Developer ID and no notarization. On first launch, choose "Open Anyway" in System Settings → Privacy & Security; the distribution notes state that step and never ask you to disable Gatekeeper.

The FFmpeg inside a `-full` package is a **separate program** distributed under GPL-3.0: its licence and source list travel in `Licenses/` and `SOURCES.txt` while the application stays MIT. The two meet through a subprocess, not a link, so each keeps its own licence.

Privacy: the program has no telemetry and uploads no diagnostics or footage; it reaches the network only when you open an external link or use a system share action. It never downloads FFmpeg — a plain package expects you to supply it, and the copy in a `-full` package runs from disk only.

## Development and license

Original project code was developed with **Codex for Windows**, using **[GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol)** with **Extra high (`xhigh`)** reasoning. **[MIKU552](https://github.com/MIKU552)** defines requirements, supplies hardware and footage, and verifies the application.

Original code, documentation, graphics, and generated camera Log LUTs use the **[MIT License](../../LICENSE)**. Licenses and sources for third-party LUTs, fonts, and runtimes are listed in [NOTICE](../../NOTICE) and [third-party components](../../Assets/THIRD_PARTY.md#english).

## Documentation

- [Command line](../cli/en.md)
- [Development and distribution](../dependencies.md#english)
- [Performance](../performance.md#english)
- [Encoding benchmark](../benchmarks/encoding-benchmark.md#english)
- [Changelog](../../CHANGELOG.md#english)
