# 代码结构 / Architecture

[简体中文](#简体中文) · [English](#english)

## 简体中文

### 模块

`LumaRecode.sln` 与 `LumaRecode.slnx` 包含应用、核心、原生桥、单文件入口、打包入口与检查项目。规则与参数由 Rust 核心（`native/`）决定，Windows 与 macOS 两个宿主都通过 C ABI（`native/include/luma_core.h`）调用它。Visual Studio 的发布配置通过 `Packaging` 调用仓库根目录的 `build.ps1`。

| 目录 | 职责 |
| --- | --- |
| `native/` | Rust 核心（规则、规划、执行抽象、颜色与元数据）以及 C ABI 与头文件 |
| `Interop/` | C# 到 C ABI 的托管桥，供 Windows 侧调用 |
| `app-macos/` | Swift 应用与终端入口（`convert`、`doctor`、`motionphoto`、`hdr10plus`） |
| `App/` | WinUI 视图、设置、本地化、通知及 GUI / CLI 启动选择 |
| `Core/` | 素材探测、命令规划、视频与图片导出、CLI；规则取自核心 |
| `Bundle/` | 可选 AOT 的单文件入口，传递标准句柄与退出码 |
| `Packaging/` | 将 Visual Studio 配置映射到完整的单文件构建流程 |
| `Assets/` | LUT、视频与动态照片预设、资源来源清单 |
| `qa/` | C# 边界检查、Python 集成检查、PowerShell 采样 |
| `tools/` | 构建辅助、许可收集、资源生成与源代码检查 |
| `docs/` | 使用说明、实现说明与测量数据 |

`App/MainWindow.*.cs` 按职责拆分：`Media` 负责素材，`Options` 负责约束，`Conversion` 负责任务与进度，`Frames` 负责预览和帧范围，`Dialogs` 与 `Localization` 负责对话框和语言。

### 转换任务

GUI 的 `ConversionClient` 和 CLI 的 `NativeCli` 调用同一个转换核心：

```text
MediaProbe → OutputPolicy / Transcoder.Plan → 要求检查 → FFmpeg
                                                   ↓
目标文件 ← 校验通过后替换 ← 临时输出 ← 编码、元数据处理
```

`MediaInfo` 是一次素材探测快照。`EncodeOptions` 保存用户选择，`OutputPolicy` 根据输入位深、采样和编码器能力解析默认值。`Transcoder.Plan` 生成独立的参数数组；参数通过 `ProcessStartInfo.ArgumentList` 传递。

编码写入输出目录下独立的任务临时目录。完成后检查像素格式、色彩标签、音轨及动态 HDR；逐帧任务还检查时间轴。只有校验通过才替换目标文件。取消或失败会先停止并等待子进程，再清理临时目录。

动态 HDR 保留、标准色彩转换和 RAW 显影由各自的规则决定处理路径。`Hdr10PlusBitstream` 优先读取 HEVC 压缩包中的 SEI，时间轴存在歧义时交由解码帧元数据处理。`RawDevelopment` 将逐帧相机参数写入 FFmpeg 校准命令。

`PlatformDefaults` 统一 GUI 与 CLI 的架构默认值：ARM64 默认软件归档，并省去 x86 专用 RAW 解码参数。Ultra HDR 的 P/Invoke 接口在各架构保持一致，发布过程选择对应的原生 DLL。

### 构建与打包

`build.ps1` 默认生成自包含 JIT 单文件。Native 按实际 Windows 架构选择 X64 或 ARM64，JIT 在运行设备上选择指令集。支持 APX 的 x64 设备在显式配置缺省时，以进程环境启用 APX 后重新进入应用；CLI 使用同一 EXE 的工作子进程，保留取消清理、句柄、参数、工作目录与退出码。

`-Compilation Aot` 使用固定目标或构建 CPU 的指令集。`Directory.Build.props` 按编译方式与目标隔离中间文件；App 生成 AOT 载荷，Bundle 打包为单个 EXE。Visual Studio 的 Release 使用 JIT，Release-AOT 和 Release-APX 使用 AOT。构建与输出路径见[开发与分发](dependencies.md)。

### 线程与生命周期

GUI 控件只在 DispatcherQueue 所在线程访问。`ConversionEventPump` 在后台执行转换，最多排队 128 个事件，并按顺序回到调用线程。终态单独传递，确保取消时仍能完成收尾。

每个转换客户端拥有自己的取消源；任务返回前等待子进程退出。素材切换通过代次检查丢弃旧探测结果。窗口关闭会取消探测、预览和运行任务，并解除系统事件订阅。

### 逐帧预览

`FrameTimeline` 保存显示帧的 PTS、时长及关键帧证据。常见逐行编码优先索引压缩包；遇到缺失或重复 PTS、隔行编码等情况使用解码帧索引。内部范围从 0 开始、结束位置不包含在内；GUI 将其转换为从 1 开始的包含结束帧表示。

`FramePreviewSession` 同时运行一个渲染任务，并保留一个可替换的待处理请求。拖动不会反复终止解码器；已完成的画面用于反馈，随后处理最新位置。缓存采用 LRU，同时限制为 12 张 PNG 和 8 MiB，切换素材或关闭预览时释放。

`FramePreview` 使用单次 FFmpeg 图片管道，限制解码线程，在色彩处理前缩小到最长边 1024 像素，并输出 SDR PNG。RAW 在去马赛克后缩小。二进制缓冲使用池化存储并限制输出大小。图片与视频导出使用完整分辨率路径。

拖动时间轴只更新帧位置、范围提示、输出文件名和预览请求。响应式布局只在宽度断点、工作页面或预览可见性改变时调整结构。

### 编码约定

- C# 使用文件范围命名空间、四空格缩进和显式控制流花括号；规则位于 `.editorconfig`。
- 核心异步代码使用取消令牌并明确资源所有权；GUI 回调保留界面线程上下文。
- 数值参数和媒体时间使用 invariant culture。帧裁剪使用源时间基与 PTS。
- 注释记录单位、帧范围、色彩表示、线程约束、格式字段及上游兼容原因。
- 修改预设、字体、LUT 或原生组件时同步来源清单和许可。生成文件通过对应工具更新。
- 构建、采样、样片和临时结果分别放在忽略的 `dist/`、`build-cache/` 或仓库外目录。测试结果汇总到现有指南和数据目录。

运行 `./tools/Check-Source.ps1` 检查格式、资源和核心边界；完整转换与实机检查见 [QA 指南](../qa/README.md)。

## English

### Modules

`LumaRecode.sln` and `LumaRecode.slnx` contain the application, core, managed bridge, single-file launcher, packaging entry and the checks. Rules and parameters come from the Rust core (`native/`), which both hosts call over the C ABI (`native/include/luma_core.h`). Visual Studio release configurations use `Packaging` to invoke `build.ps1` in the repository root.

| Directory | Responsibility |
| --- | --- |
| `native/` | Rust core (rules, planning, the execution seam, colour and metadata) plus the C ABI and its header |
| `Interop/` | Managed bridge to the C ABI for the Windows side |
| `app-macos/` | Swift application and terminal entry (`convert`, `doctor`, `motionphoto`, `hdr10plus`) |
| `App/` | WinUI views, settings, localization, notifications and GUI / CLI startup selection |
| `Core/` | Media probing, command planning, video/image export and CLI; the rules come from the core |
| `Bundle/` | Optional AOT single-file entry; preserve standard handles and exit codes |
| `Packaging/` | Map Visual Studio configurations to the complete single-file build pipeline |
| `Assets/` | LUTs, video and Motion Photo presets, resource provenance |
| `qa/` | C# boundary checks, Python integration checks and PowerShell sampling |
| `tools/` | Build helpers, license collection, resource generation and source checks |
| `docs/` | User guides, implementation notes and measurements |

`App/MainWindow.*.cs` is split by responsibility: `Media` handles sources, `Options` enforces constraints, `Conversion` manages jobs and progress, `Frames` handles previews and frame ranges, and `Dialogs` / `Localization` handle dialogs and language.

### Conversion jobs

The GUI's `ConversionClient` and the CLI's `NativeCli` call the same conversion core:

```text
MediaProbe → OutputPolicy / Transcoder.Plan → Requirements → FFmpeg
                                                              ↓
Destination ← Replace after verification ← Temporary output ← Encoding / metadata
```

`MediaInfo` is a source-inspection snapshot. `EncodeOptions` stores user selections, and `OutputPolicy` resolves defaults against source bit depth, sampling and encoder capabilities. `Transcoder.Plan` produces an argument array passed through `ProcessStartInfo.ArgumentList`.

Encoding writes into a private job directory beside the destination. Verification checks pixel format, color tags, audio and dynamic HDR; frame-accurate jobs also check the timeline. The destination is replaced after verification succeeds. Cancellation and failure stop and join child processes before removing temporary files.

Dynamic HDR preservation, standard color conversion and RAW development each select their processing path through their own policies. `Hdr10PlusBitstream` reads HEVC SEI from compressed packets first and uses decoded-frame metadata for ambiguous timelines. `RawDevelopment` writes per-frame camera calibration into FFmpeg commands.

`PlatformDefaults` provides common GUI / CLI architecture defaults: ARM64 starts with Software archive and omits the x86-specific RAW decoder workaround. Ultra HDR uses the same P/Invoke interface on each architecture; publishing selects the matching native DLL.

### Building and packaging

`build.ps1` produces a self-contained JIT single EXE by default. Native selects X64 or ARM64 from the real Windows architecture; the JIT selects instructions on the running device. On APX-capable x64 devices without an explicit override, startup restarts the application with a process-scoped APX setting. The CLI uses a worker launched through the same EXE to preserve cancellation cleanup, handles, arguments, working directory and exit code.

`-Compilation Aot` uses a fixed target or the build CPU's instructions. `Directory.Build.props` isolates intermediate files by compilation mode and target. App produces the AOT payload; Bundle packages it into one EXE. Visual Studio Release uses JIT; Release-AOT and Release-APX use AOT. See [Development and distribution](dependencies.md#english) for commands and output paths.

### Threads and lifetimes

GUI controls are accessed on their DispatcherQueue thread. `ConversionEventPump` runs conversion work in the background, queues at most 128 events and delivers them in order on the caller's synchronization context. Terminal state is delivered separately so cancellation can finish cleanup.

Each conversion client owns its cancellation source and waits for child processes before returning. Source generations discard stale probe results. Closing a window cancels probing, preview and running jobs, then unsubscribes from system events.

### Frame preview

`FrameTimeline` stores display-frame PTS values, durations and keyframe evidence. Common progressive codecs use compressed-packet indexing first. Missing or duplicate PTS values, interlaced material and similar cases use decoded-frame indexing. Internal ranges are zero-based and end-exclusive; the GUI converts them to one-based ranges with an inclusive end.

`FramePreviewSession` runs one renderer and keeps one replaceable pending request. Scrubbing updates the pending request while completed renders provide feedback. The LRU cache holds at most 12 PNGs and 8 MiB; hiding the preview or changing the source releases it.

`FramePreview` uses one FFmpeg image pipe, limits decoder threads, scales to a maximum dimension of 1024 pixels before color processing and returns SDR PNG. RAW is scaled after demosaicing. Binary buffers are pooled and output size is bounded. Video and image exports use full-resolution processing.

Timeline dragging updates frame position, range hints, output names and preview requests. Responsive layout changes its structure when the width breakpoint, work page or preview visibility changes.

### Coding conventions

- C# uses file-scoped namespaces, four-space indentation and explicit control-flow braces, as defined in `.editorconfig`.
- Core asynchronous code passes cancellation tokens and defines resource ownership. GUI callbacks retain the UI synchronization context.
- Numeric arguments and media times use invariant culture. Frame cuts use source time bases and PTS values.
- Comments describe units, frame ranges, color representations, thread constraints, format fields and upstream compatibility behavior.
- Changes to presets, fonts, LUTs or native components update provenance and licensing. Generated files are updated with their generating tools.
- Build outputs, samples, footage and temporary results belong in ignored `dist/`, `build-cache/` or directories outside the repository. Results are collected in the existing guides and data directories.

Run `./tools/Check-Source.ps1` for formatting, resource and core-boundary checks. See the [QA guide](../qa/README.md#english) for conversion and device checks.
