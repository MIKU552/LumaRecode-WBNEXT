# 开发检查 / Development checks

[简体中文](#简体中文) · [English](#english)

## 简体中文

在仓库根目录运行以下命令。.NET SDK 版本由 `global.json` 指定，Python 检查需要本机安装 Python 3.14 或更新版本。

### 源码检查

```powershell
./tools/Check-Source.ps1
```

依次检查 C# 格式、资源、序列化、核心边界、预览调度、进程管道、文档与构建目标。文档检查覆盖链接、锚点、双语结构和语言回退标注。此命令使用独立的测试子进程，无需样片或 GPU。

### 构建与架构

```powershell
python qa/BuildChecks/check_build.py
python qa/BuildChecks/check_build.py --artifacts
python qa/BuildChecks/check_build.py --aot-artifacts
```

基础检查覆盖 JIT / AOT 的目标选择、APX 与 ARM64 主机组合、MSBuild 映射、中间目录隔离、锁文件和原生库哈希。`--artifacts` 检查 Native、X64、ARM64 的 JIT 单文件、打包内容与分发许可；`--aot-artifacts` 在四种 AOT 目标全部构建后检查载荷、指令集和 APX 启动保护。

ARM64 完整运行检查需要 ARM64 Windows 设备。APX 转码检查需要支持 APX 的 CPU 与 Windows。

### 资源与序列化

```powershell
dotnet run --project qa/LanguageChecks/LanguageChecks.csproj -c Release
dotnet run --project qa/SerializationChecks/SerializationChecks.csproj -c Release
dotnet run --project qa/BackendChecks/BackendChecks.csproj -c Release -- --check
python qa/LanguageChecks/check_cli_languages.py
python tools/Update-ViewText.py --check
python tools/generate_hlg_luts.py --check
```

核心边界检查还覆盖数值解析、选项值缺失、工具查找、日志错误、通配符匹配、UTF-8 输出字节上限、单行长度、索引预算，以及超限、取消和超时后的子进程回收。

长片索引与流式读取检查：

```powershell
dotnet run --project qa/BackendChecks/BackendChecks.csproj -c Release -- --resource-stress
```

此检查生成 12 小时、60 fps 的时间戳索引，并流式读取 1 亿字符的模拟元数据。

这些检查覆盖四种语言的选择、翻译资源与格式占位符、XAML 映射、设置格式、命令行 JSON 格式和生成的 LUT。可给 `check_cli_languages.py` 传入 `--exe dist/SelfContained/LumaRecode.exe`，检查打包后的 CLI 语言资源。

### 共享核心对照

```powershell
./tools/Build-Native.ps1 -Configuration Release -Target Native
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

先构建 Rust 原生库。对照检查用 12 组源描述与 12 组用途、编码器、输出组合，比较 C# `OutputPolicy` 与共享核心的采样、位深、范围、可选值、默认值，以及 `ResolveFormat` 的接受与拒绝结果，最后输出比较次数和不一致数量。退出码 3 表示原生库未构建，应与通过、失败分别记录。

### CLI 与转换核心

先运行 `./build.ps1`，再执行集成检查：

```powershell
python qa/CliChecks/check_cli.py --exe dist/SelfContained/LumaRecode.exe
python qa/RuntimeChecks/check_runtime.py --exe dist/SelfContained/LumaRecode.exe
dotnet build qa/BackendChecks/BackendChecks.csproj -c Release
python qa/TimingChecks/check_timing.py
python qa/TimingChecks/check_qsv_recovery.py
```

集成检查需要 FFmpeg、ffprobe、可用的 Intel QSV 编码器和 Vulkan / libplacebo。它们执行实际编码、输出校验、PowerShell 管道、取消和临时文件清理。测试使用临时目录与合成输入。

时间戳检查覆盖可变帧率的 QSV 编码、8/10 位转换与 SDR/HLG/PQ，逐帧比较显示时间并检查 DTS 顺序。可给 `check_timing.py` 传入 `--sample PATH`，检查相机原片和时间戳冲突后的自动重试。

QSV 恢复检查覆盖 AV1 HDR、旋转和镜像、240 fps GOP，以及解码、LowPower、时间戳故障后的重试上限和输出文件保护。兼容策略与上游报告见 [QSV 兼容性](../docs/qsv-compatibility.md)。

### GUI 启动

发布前直接打开最终的 `LumaRecode.exe`，确认主窗口完成加载，再检查语言切换、配色面板和“关于”窗口。关闭后重新打开，确认语言设置恢复。检查 `%LOCALAPPDATA%\LumaRecode\error.log` 的更新时间及内容。

### 采样、范围与音频

```powershell
python qa/FormatChecks/check_formats.py
```

`qa/BackendChecks/` 提供 C# 核心的测试入口，格式检查通过它执行原生代码。

检查 SDR / HLG / PQ 编码后的黑白像素值、8 / 10 / 12 位与 4:2:0 / 4:2:2 / 4:4:4 / RGB / 灰度输入、源格式上限，以及 FLAC 多音轨的解码样本哈希。需要 Intel QSV 和 Vulkan / libplacebo。

### 性能与并发

```powershell
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release -- --check
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release -- --tools
```

检查通知、事件顺序、接收线程上下文、取消、日志容量、Unicode 边界和能力查询缓存。预览检查覆盖请求合并、LRU 淘汰、取消后的回调、源切换释放、错误恢复，以及二进制管道的字节边界与子进程回收。微基准输出五轮测量的中位数；`--tools` 需要 PATH 中的 FFmpeg。

预览渲染测速先读取素材和逐帧索引，再测量每个请求的渲染耗时、PNG 大小与托管分配量：

```powershell
dotnet build qa/BackendChecks/BackendChecks.csproj -c Release
python qa/PerformanceChecks/benchmark_preview.py --sdr D:/MediaTests/sdr.mp4 --hlg D:/MediaTests/hlg.mp4 --log D:/MediaTests/dlog.mp4 --profile pocket3 --output D:/MediaTests/preview.json
```

可分别指定任一素材。`--frames 0,30,60,90` 设置帧序号，`--repeat 3` 设置轮数。

### 内存采样

保持窗口大小、主题、HDR 和输入法一致，每次仅运行一个待测实例，在菜单操作和编码之前采样：

```powershell
./qa/MemoryChecks/Measure-Process.ps1 -Executable '<实际 GUI 路径>/LumaRecode.exe' -Label startup -OutputFile samples.jsonl
```

脚本等待进程年龄至少 30 秒后采样三次，记录私有工作集、私有提交、总工作集和累计 CPU 时间。菜单操作后可用 `-AgeSeconds 0 -Label panels` 单独记录。

AOT 单文件版的实际 GUI 位于 `.net/LumaRecode/<bundle-id>/Application/LumaRecode.exe`。GUI 按主程序采样；CLI 按入口、主程序和任务子进程的合计占用采样。

预览解码器可单独采样。先读取实际 GUI 的进程 ID，再运行以下脚本并操作预览时间轴：

```powershell
./qa/MemoryChecks/Measure-ChildProcesses.ps1 -RootProcessId 12345 -Label preview -DurationSeconds 60 -OutputFile D:/MediaTests/preview-memory.jsonl
```

脚本追踪 GUI 启动的 FFmpeg 及 PATH 垫片下的解码进程，记录每个进程的峰值工作集。

优化设计与结果见[性能优化](../docs/performance.md)。

### HDR 与 RAW

```powershell
python qa/HdrChecks/check_hdr.py
python qa/HdrChecks/check_hdr.py --samples D:/MediaTests/HDR
python tools/generate_hlg_luts.py --check
```

HDR 检查覆盖 Apple 传递曲线与色域计算、明确的 Log 标记识别、逐帧 Dolby Vision 元数据、HDR10+ JSON 往返和 ProRes RAW 解码兼容性。下载样片的来源与截取命令见 [sample-sources.json](HdrChecks/sample-sources.json)。完整 HDR10+ 对照使用 PATH 中的 hdr10plus_tool 作为开发参考，生产程序使用 C# 解析器。样片保存在源码目录外。`--metadata-only` 检查 SEI 排序、edit list、稀疏元数据与导出。

### 帧与图片导出

```powershell
python qa/FrameChecks/check_frames.py --reference D:/Dev/libultrahdr/ultrahdr_app.exe
```

检查覆盖跨关键帧的 CFR / VFR 帧身份、精确到采样的 PCM-to-FLAC 裁剪、HDR / SDR 图片、RGB 通道顺序、Google 增益图解码、动态照片字节范围和封面帧时间戳。可选的上游解码器用于开发对照。

## English

Run these commands from the repository root. `global.json` selects the .NET SDK. Python checks require Python 3.14 or newer.

### Source checks

```powershell
./tools/Check-Source.ps1
```

Runs C# formatting, resources, serialization, core boundaries, preview scheduling, process pipes, documentation and build-target checks in order. Documentation checks cover links, anchors, parallel language sections and fallback labels. These checks use isolated test processes and require no footage or GPU.

### Builds and architectures

```powershell
python qa/BuildChecks/check_build.py
python qa/BuildChecks/check_build.py --artifacts
python qa/BuildChecks/check_build.py --aot-artifacts
```

Basic checks cover JIT/AOT target selection, APX and ARM64 host combinations, MSBuild mappings, isolated intermediate files, lock files and native-library hashes. `--artifacts` checks Native, X64 and ARM64 JIT single EXEs, bundled content and distribution licenses. After all four AOT targets have been built, `--aot-artifacts` checks payloads, instruction sets and APX startup protection.

Full ARM64 runtime checks require an ARM64 Windows device. APX hardware conversion checks require an APX-capable CPU and Windows support.

### Resources and serialization

```powershell
dotnet run --project qa/LanguageChecks/LanguageChecks.csproj -c Release
dotnet run --project qa/SerializationChecks/SerializationChecks.csproj -c Release
dotnet run --project qa/BackendChecks/BackendChecks.csproj -c Release -- --check
python qa/LanguageChecks/check_cli_languages.py
python tools/Update-ViewText.py --check
python tools/generate_hlg_luts.py --check
```

Core boundary checks also cover numeric parsing, missing option values, tool resolution, log errors, wildcard matching, UTF-8 output byte limits, line lengths, index budgets, and child-process cleanup after overflow, cancellation or timeout.

Check long-recording indexes and streaming reads:

```powershell
dotnet run --project qa/BackendChecks/BackendChecks.csproj -c Release -- --resource-stress
```

This check generates a timestamp index for 12 hours at 60 fps and streams 100 million characters of simulated metadata.

These checks cover language selection for all four languages, translation resources and format placeholders, XAML mappings, settings formats, CLI JSON contracts and generated LUTs. Pass `--exe dist/SelfContained/LumaRecode.exe` to `check_cli_languages.py` to check packaged CLI language resources.

### Shared core parity

```powershell
./tools/Build-Native.ps1 -Configuration Release -Target Native
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

Build the Rust native library first. The parity check compares C# `OutputPolicy`
with the shared core across 12 source descriptions and 12 purpose/codec/output
combinations: sampling, depth, range, option lists, defaults, and whether
`ResolveFormat` accepts or rejects the request. It reports the comparison and
mismatch counts. Exit code 3 means the native library was not built and must be
recorded separately from pass and fail.

### CLI and conversion core

Run `./build.ps1` first, then execute the integration checks:

```powershell
python qa/CliChecks/check_cli.py --exe dist/SelfContained/LumaRecode.exe
python qa/RuntimeChecks/check_runtime.py --exe dist/SelfContained/LumaRecode.exe
dotnet build qa/BackendChecks/BackendChecks.csproj -c Release
python qa/TimingChecks/check_timing.py
python qa/TimingChecks/check_qsv_recovery.py
```

Integration checks require FFmpeg, ffprobe, working Intel QSV encoders and Vulkan / libplacebo. They run actual encodes, output verification, PowerShell pipelines, cancellation and temporary-file cleanup using temporary directories and synthetic inputs.

Timestamp checks cover VFR QSV encoding, 8/10-bit conversion and SDR/HLG/PQ. They compare frame presentation times and check DTS ordering. Pass `--sample PATH` to `check_timing.py` to exercise a camera original and automatic recovery from timestamp conflicts.

QSV recovery checks cover AV1 HDR, rotation/reflection, 240 fps GOPs, bounded retries after decoder/LowPower/timestamp failures and existing-output protection. Policies and upstream reports are documented in [QSV compatibility](../docs/qsv-compatibility.md#english).

### GUI startup

Before publishing, open the final `LumaRecode.exe` directly and confirm the main window loads. Check language switching, the appearance panel and About. Close and reopen the application to check language restoration. Check the modification time and contents of `%LOCALAPPDATA%\LumaRecode\error.log`.

### Sampling, range and audio

```powershell
python qa/FormatChecks/check_formats.py
```

`qa/BackendChecks/` provides the C# core test entry used by format checks.

Checks cover encoded black/white pixel values for SDR / HLG / PQ; 8 / 10 / 12-bit and 4:2:0 / 4:2:2 / 4:4:4 / RGB / grayscale inputs; source-format limits; and decoded sample hashes for multitrack FLAC. Intel QSV and Vulkan / libplacebo are required.

### Performance and concurrency

```powershell
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release -- --check
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release
dotnet run --project qa/PerformanceChecks/PerformanceChecks.csproj -c Release -- --tools
```

Checks cover notifications, event ordering, receiver synchronization context, cancellation, log capacity, Unicode boundaries and capability-query caching. Preview checks cover request coalescing, LRU eviction, callbacks after cancellation, release on source changes, error recovery, binary pipe boundaries and child-process cleanup. Microbenchmarks report the median of five runs. `--tools` requires FFmpeg on PATH.

The preview benchmark probes footage and builds frame indexes before timing each render request, PNG size and managed allocation:

```powershell
dotnet build qa/BackendChecks/BackendChecks.csproj -c Release
python qa/PerformanceChecks/benchmark_preview.py --sdr D:/MediaTests/sdr.mp4 --hlg D:/MediaTests/hlg.mp4 --log D:/MediaTests/dlog.mp4 --profile pocket3 --output D:/MediaTests/preview.json
```

Each source is optional. `--frames 0,30,60,90` sets frame indices; `--repeat 3` sets repetitions.

### Memory sampling

Keep window size, theme, HDR and input method the same. Run one instance at a time and collect startup samples before opening panels or encoding:

```powershell
./qa/MemoryChecks/Measure-Process.ps1 -Executable 'D:/Application/LumaRecode.exe' -Label startup -OutputFile samples.jsonl
```

The script waits until process age is at least 30 seconds, then samples three times. It records private working set, private commit, total working set and cumulative CPU time. After opening panels, use `-AgeSeconds 0 -Label panels` for a separate sample.

The actual AOT GUI is `.net/LumaRecode/<bundle-id>/Application/LumaRecode.exe`. Sample that process for GUI measurements; include the entry, main application and job children for CLI measurements.

To sample preview decoding separately, obtain the GUI PID, run this command and scrub the preview timeline:

```powershell
./qa/MemoryChecks/Measure-ChildProcesses.ps1 -RootProcessId 12345 -Label preview -DurationSeconds 60 -OutputFile D:/MediaTests/preview-memory.jsonl
```

The script tracks FFmpeg children started by the GUI, including actual decoder processes behind PATH shims, and records each process's peak working set.

See [Performance](../docs/performance.md#english) for implementation and results.

### HDR and RAW

```powershell
python qa/HdrChecks/check_hdr.py
python qa/HdrChecks/check_hdr.py --samples D:/MediaTests/HDR
python tools/generate_hlg_luts.py --check
```

HDR checks cover Apple transfer/gamut math, explicit Log detection, per-frame Dolby Vision metadata, HDR10+ JSON round-tripping and ProRes RAW decoder compatibility. Downloaded sample origins and excerpt commands are in [sample-sources.json](HdrChecks/sample-sources.json). The full HDR10+ comparison uses hdr10plus_tool on PATH as a development reference. Production uses the C# parser. Store footage outside the source tree. `--metadata-only` checks SEI ordering, edit lists, sparse metadata and export.

### Frame and image exports

```powershell
python qa/FrameChecks/check_frames.py --reference D:/Dev/libultrahdr/ultrahdr_app.exe
```

Checks cover CFR / VFR frame identity across keyframes, sample-accurate PCM-to-FLAC cuts, HDR / SDR image exports, RGB channel order, Google gain-map decoding, Motion Photo byte ranges and cover-frame timestamps. The optional upstream decoder is a development reference.
