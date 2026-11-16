<p align="center">
  <img src="Brand/LumaRecode.svg" width="104" height="104" alt="LumaRecode 图标" />
</p>
<h1 align="center">映码 · LumaRecode WBNEXT</h1>

简体中文 | [English](docs/readme/en.md) | [日本語](docs/readme/ja.md) | [Español](docs/readme/es.md)

> **WBNEXT 重构预览版**：本分支是独立于上游的跨平台重构版本，基线为 [LumaRecode 4.4.3](https://github.com/MIKU552/LumaRecode)。重构由 WorkBuddy（DeepSeek-V4.1-Flash-1017，极高推理，1M 上下文）协助完成，**非正式发布版**，不提供上游正式版本的支持承诺。构建产物来自 GitHub Actions Artifacts，进度见[重构实施记录](docs/refactor/README.md)。

基于 C# / .NET 的 Windows 11 视频转码与色彩转换工具，提供原生 WinUI 3 界面和命令行，使用 FFmpeg 完成视频压缩、相机 Log 转 HDR / SDR，以及 HDR 转 SDR。

[下载](https://github.com/MIKU552/LumaRecode/releases/latest) · [文档](docs/README.md) · [命令行文档](docs/cli/zh-CN.md) · [更新记录](CHANGELOG.md)

## 功能

- 逐帧预览与裁剪，支持自选封面帧、HDR / SDR 图片和 Android 动态照片。
- 按快速分享与软件归档选择预设，支持 HEVC、AV1、H.264 和 MP4 / MKV。
- 支持 DJI D-Log M、Samsung Log、Mi-Log、Apple Log 和 Apple Log 2 转换为 HLG HDR 或 Rec.709 SDR，以及 HLG / PQ 转 SDR。
- 支持 ProRes RAW 显影、Dolby Vision / HDR10+ 识别与转换，以及软件编码保留动态元数据。
- 读取分辨率、帧率、位深、色彩标签及音轨，按素材设置默认选项。
- 设置色度采样、Limited / Full 范围及音轨，支持 FLAC 无损音频。
- 显示进度、编码帧率、当前位置与剩余时间，完成后校验输出格式。
- 完成或失败时发送 Windows 通知；通过“打开目录”左侧的勾选框开关通知。
- 提供 Mica、浅色 / 深色 / 系统主题、强调色，以及简体中文、English、日本語和 Español。
- 命令行支持批量、配置文件、参数预演和 JSON 事件输出。

## 快速开始

1. 从 [Releases](https://github.com/MIKU552/LumaRecode/releases) 下载适合设备的 EXE，版本说明见下表。程序内置所需运行时。
2. 准备 Windows 版 [FFmpeg](https://ffmpeg.org/download.html)，包含 `ffmpeg.exe` 和 `ffprobe.exe`。首次运行会从 `PATH` 查找，也可在右上角齿轮中指定路径。
3. 拖入视频或点击“选择视频”，确认输出动态范围、编码预设和色彩转换。
4. 选择保存位置，点击“开始转换”。

“快速分享”使用 Intel QSV；“软件归档”使用 CPU 编码。

| 下载文件 | 平台与要求 |
| --- | --- |
| `LumaRecode.exe` | X64，通用 x64 指令集配置 |
| `LumaRecode-ARM64.exe` | Windows ARM64，原生 ARM64 应用与图片编解码库 |

ARM64 默认使用软件归档。FFmpeg 的编码速度由外部 FFmpeg、硬件与所选预设决定。

Dolby Vision / HDR10+ 素材默认使用软件 HEVC 保留动态元数据；“标准 HDR”可用于硬件快速分享。Dolby Vision 支持单层 Profile 5、8.1、8.4、10.1 和 10.4。ProRes RAW 在确认 FFmpeg 解码兼容性后，根据相机白平衡、色彩矩阵和增益显影，默认输出 HLG。格式与处理流程见[色彩转换](docs/color-conversion.md)。

## 预览、裁剪与图片

“视频转换”中的“预览与逐帧裁剪”提供时间轴、上一帧 / 下一帧和开始 / 结束帧设置，默认转换整段视频。界面帧编号从 1 开始，结束帧包含在输出中。

切换到“图片与动态照片”，选择预览帧和图片格式：

| 格式 | 输出 |
| --- | --- |
| HDR AVIF | BT.2020 / PQ，10 或 12 位 |
| Ultra HDR JPEG | SDR 主图、HDR 增益图与增益图元数据 |
| SDR JPEG | sRGB 图片 |
| SDR PNG | sRGB 无损图片，8 位或 16 位容器 |

JPEG 和 Ultra HDR JPEG 可加入 Android 动态照片视频。封面帧与视频范围分别设置，默认取封面附近约 3 秒。动态照片使用单个 `MP.jpg` 文件，按 [Google Motion Photo 1.0](https://developer.android.com/media/platform/motion-photo-format) 封装；Ultra HDR 使用 Google [libultrahdr](https://github.com/google/libultrahdr) 编码。动态视频默认使用 CPU，可改选 Intel 快速编码。

预览以 SDR 显示，图片按所选格式导出。逐帧裁剪会重新编码音轨，可选 AAC、Opus 或符合条件的 FLAC。

动态视频默认保留原分辨率，也可选择最高 1080p 或 720p。独立的短片预设使用 x264 fast / CRF20、x265 fast / CRF23，或 Intel TU6；音频使用 AAC-LC、48 kHz，保留单声道或输出双声道。参数见 [motion-photo-presets.json](Assets/motion-photo-presets.json)。

## 编码与色彩

| 用途 | 编码器 | 默认封装 | 默认音频 |
| --- | --- | --- | --- |
| 快速分享 | Intel QSV：HEVC / AV1 / H.264 | MP4 | HEVC、H.264 使用 AAC；AV1 使用 Opus |
| 软件归档 | x265 / SVT-AV1 / x264 | MKV | 复制原音轨 |

封装、采样、位深、色彩范围和音频可单独选择。快速分享默认输出 SDR 8 位、HDR 10 位。软件归档默认保留原片位深，按编码器支持的上限选择：HEVC 为 8 / 10 / 12 位，AVC 与 AV1 为 8 / 10 位。

软件 HEVC / H.264 归档默认保持原采样；RGB 输入默认转换为 4:4:4 YCbCr，AV1 和快速分享使用 4:2:0。采样与位深可在原片上限内向下选择，例如 8 位 4:2:0 保持为 8 位 4:2:0，10 位 4:2:2 可选择 10 / 8 位、4:2:2 / 4:2:0。归档默认保持原 Limited / Full 范围，快速分享默认使用 Limited。采样、位深与范围位于“色彩转换”，封装与音轨位于“格式与音频”。

FLAC 用于可确认的整数 PCM 和无损音轨，保留每条音轨的采样率、声道和整数精度。FLAC 使用 MKV 封装。

SDR 素材默认输出 SDR，HLG / PQ 素材默认保留 HDR，已识别曲线的 Log 素材默认转换为 HLG。可在“输入解释”和“相机曲线”中手动选择素材类型。

视频按所选预设重新编码，音频可复制或转码。完整参数见 [recommended-presets.json](Assets/recommended-presets.json)，测试结果见[编码预设测试](docs/benchmarks/encoding-benchmark.md#简体中文)。

## 运行要求

- Windows 11 x64 或 ARM64，满足所选版本的指令集要求。
- 用户提供的 FFmpeg / ffprobe，包含任务所需的编码器和滤镜。
- 快速分享需要相应 Intel QSV 支持。
- Log 转换需要 `zscale`、`lut3d`；HDR / Log 转 SDR 还需要 `libplacebo` 和 Vulkan。

语言和主题默认跟随系统，支持中文、英语、日语和西班牙语系统；其他语言系统默认英文。设置保存在 `%LOCALAPPDATA%\LumaRecode\settings.json`。

首次运行会自动导入 FrameFlow 的语言、外观与工具路径设置。

程序内置 Source Sans 3、思源黑体和 Source Code Pro，无需安装字体。

## 命令行

在 PowerShell 7 中使用输出管道运行命令：

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --help | Out-Host
./LumaRecode.exe probe clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host
```

首次无人值守使用前，运行 `license --text` 阅读组件条款，再运行 `license --accept-terms`。全部命令、批量用法和退出码见[命令行文档](docs/cli/zh-CN.md)。

## 从源码构建

需要 .NET SDK 11.0.100 和 PowerShell 7；可选 AOT 构建还需要 Visual Studio C++ 工具链。

```powershell
git clone https://github.com/MIKU552/LumaRecode.git
cd LumaRecode
./build.ps1
./build.ps1 -Target X64
./build.ps1 -Target X64 -Compilation Aot
./build.ps1 -Target Arm64
```

默认使用 JIT。Native 模式按 Windows 系统架构选择 X64 或 ARM64，产物为 `dist/SelfContained/LumaRecode.exe`。Releases 提供 X64 和 ARM64 两种自包含单文件 EXE。

JIT 在运行设备上选择支持的指令集；x64 设备同时满足 CPU 与 Windows 的 APX 支持条件时，程序在启动阶段启用 APX，并保留用户显式设置。可选 `-Compilation Aot` 使用发布时编译，输出到 `dist/Aot-SelfContained[-Target]/`；APX 固定目标使用 `-Target Apx -Compilation Aot`。

使用 Visual Studio 2026 18.12 或更新版本打开 `LumaRecode.sln`。以 `LumaRecode.App` 为启动项目，Debug 配置用于调试。`Release | Native`、`Release | x64` 和 `Release | ARM64` 生成 JIT 版本；`Release-AOT` 提供对应的 AOT 构建，`Release-APX | x64` 生成 APX AOT 版本。`Packaging` 项目完成单文件打包。安装与配置见[开发文档](docs/dependencies.md)，检查命令见 [QA 指南](qa/README.md)。

## 项目结构

```text
App/       WinUI 3 界面、主题、本地化和任务交互
Bundle/    单文件打包入口
Packaging/ Visual Studio 构建与发布入口
Core/      素材探测、格式规则、FFmpeg 调用与命令行
Assets/    LUT、编码预设与资源来源记录
Brand/     图标源文件与生成工具
tools/     许可收集和本地化代码生成
qa/        自动检查与内存采样工具
docs/      使用说明、开发文档和测试数据
licenses/  第三方许可、声明与来源清单
```

## Code signing policy（代码签名政策）

Windows 发布包目前未签名。接近发布时会申请 SignPath Foundation 的免费开源签名；获批后，安装包由 SignPath.io 构建并签名、证书由 SignPath Foundation 签发，不购买商业证书。未获批或尚未完成时，下载页会标明“未签名”，不转向收费方案。

macOS 版本使用免费 ad-hoc 签名，不使用 Apple Developer ID，也未经过公证。首次打开需在“系统设置 → 隐私与安全性”中选择“仍要打开”，分发说明会写明该步骤，不要求关闭 Gatekeeper。

角色与责任：维护者与发布批准由 MIKU552 承担，提交者与审核者为同一账号（单人项目）；每次正式签名由批准者人工确认。

隐私：程序不包含遥测，不自动上传诊断或素材；仅在你主动打开外部链接或使用系统分享时联网。FFmpeg / ffprobe 由你自行提供，程序不代为下载。

## 开发与许可

项目自主代码由 **Windows 版 Codex** 使用 **[GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol)**、**极高（xhigh）** 推理设置完成。**[MIKU552](https://github.com/MIKU552)** 负责需求、设备与素材，并进行实机核验。

自主代码、文档、图形及项目生成的相机 Log LUT 采用 **[MIT License](LICENSE)**。第三方 LUT、字体和运行时的许可与来源见 [NOTICE](NOTICE) 和[第三方组件](Assets/THIRD_PARTY.md)。

## 文档

- [命令行](docs/cli/zh-CN.md)
- [开发与分发](docs/dependencies.md)
- [性能优化](docs/performance.md)
- [编码预设测试](docs/benchmarks/encoding-benchmark.md#简体中文)
- [更新记录](CHANGELOG.md)
