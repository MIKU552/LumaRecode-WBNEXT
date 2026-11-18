# 迁移台账 / Migration ledger

[简体中文](#简体中文) · [English](#english)

## 简体中文

逐文件记录 Windows 基线与 Rust 共享核心的对应关系。状态只有三种：
**完成**（规则已迁移并有测试）、**首片**（关键规则已迁移，其余仍在 C#）、
**未开始**。旧 C# 实现在对照结束前保留，方案 §16.2 要求切换与回归完成后再移除重复实现。

| 基线 | Rust 目标 | 状态 | 证据 |
| --- | --- | --- | --- |
| `Core/OutputPolicy.cs` | `luma-core/src/policy.rs` | 完成 | `tests/policy.rs` |
| `Core/FrameTimeline.cs`（时间换算、seek、范围） | `luma-core/src/timeline.rs` | 完成 | `tests/timeline.rs`；`qa/NativeChecks` 对照（含按包索引） |
| `Core/FrameTimeline.cs`（ffprobe 读取与回退） | 待定：探测适配器 | 未开始 | — |
| `Assets/recommended-presets.json` | `luma-core/src/presets.rs` | 完成 | `tests/presets.rs` 内嵌资源；`qa/NativeChecks` 逐选择器对照 |
| `Core/Models.cs`（`EncodeOptions`） | `luma-core/src/domain.rs` | 完成 | `tests/policy.rs` |
| `Core/Models.cs`（`MediaInfo`） | `luma-core/src/source.rs` | 首片 | 仅保留规则所需字段 |
| `Core/DynamicHdr.cs`（`Validate`、`Preserve`） | `luma-core/src/policy.rs` | 完成 | `tests/policy.rs` |
| `Core/DynamicHdr.cs`（`ExtractAsync`） | 规则已在 `policy`；编排保留在宿主 | 按设计保留 | 帧字节不过 ABI；宿主用 `execution.run` 跑扫描 |
| `Core/Transcoder.cs`（HDR 元数据参数组合、`HdrMetadata`） | `luma-core/src/metadata.rs` | 完成（`plan.hdr_metadata` 桥接） | `tests/metadata.rs`；`qa/NativeChecks` 逐编码器对照参数串 |
| `Core/Transcoder.cs`（`ResolveFormat`、`Validate`） | `luma-core/src/policy.rs` | 首片（ResolveFormat 与 Validate 已接原生桥） | `qa/NativeChecks` 对照与桥接回放 |
| `Core/Transcoder.cs`（FFmpeg 参数生成） | 保留在平台后端 | 未开始 | 方案 §3.2 明确归属后端 |
| `Core/Transcoder.cs`（执行、原子发布） | `luma-core/src/execution.rs` | 核心侧完成 | 顺序执行、逐步事件与回调已就位；进程启动仍归宿主 |
| `Core/MediaProbe.cs` | 归一化在 `probe.rs` | 完成 | ffprobe 调用保留在宿主（文件与子进程） |
| `Core/FrameExporter.cs`、`FramePreview*.cs` | 保留在宿主 | 按设计保留 | 帧字节不过 ABI；macOS 预览已用 AVFoundation |
| `Core/MotionPhoto.cs` | 保留在宿主（帧字节不过 ABI） | 大部分完成 | 识别、提取与 Ultra HDR 判定已在 macOS 实现并有合成字节测试；写入仍在 Windows |
| `Core/RawDevelopment.cs` | 滤镜链在 `raw.rs`，解包在宿主 | 部分完成 | `plan.raw_filter` 有顺序测试；宿主侧 RAW 解包与校准命令未做 |
| `Core/Hdr10Plus*.cs` | SEI 读取与文档生成在 `sei.rs` | 部分完成 | `metadata.hdr10plus`、`metadata.hdr10plus_document` 各有测试；扫描编排在宿主 |
| `Core/UltraHdrCodec.cs` | 保留在宿主 | 部分完成 | 判定已在 macOS 实现；图像级编解码需要解码器，留在宿主 |
| `Core/ProcessRunner.cs` | `luma-core/src/execution.rs` | 核心侧完成 | 宿主可改用 `execution.run`；Windows 仍用自己的实现 |
| `Core/CliArguments.cs`、`CliReporter.cs`、`NativeCli*.cs` | 保留在宿主，规则调用核心 | 未开始 | P3 |
| `Core/InputFiles.cs` | 保留在宿主 | 按设计保留 | 目录枚举与通配符属文件系统，无规则可迁 |
| `Core/ToolReportCache.cs` | 保留在宿主 | 按设计保留 | 缓存键是进程与文件状态 |
| — | `luma-core/src/capability.rs` | 新增 | 对应方案 §4；`qa/NativeChecks` 对照 C# 后端映射 |
| — | `luma-core/src/plan.rs` | 新增 | 对应方案 §3.2「任务规划」，无基线 |
| — | `luma-core/src/cache.rs` | 新增（§10.4 标识与回收策略） | `tests/cache.rs`；`doctor` 输出计划，`cache --purge` 执行删除，CI 校验 |
| — | `luma-core/src/safety.rs` | 新增（§10.3 输出路径安全） | `tests/safety.rs` |
| — | `luma-core/src/probe.rs` | 新增（ffprobe 归一化） | `tests/probe.rs`；`qa/NativeChecks` 对照真实抓取文档 |
| — | `luma-core/src/metadata.rs` | 新增（DOVI / HDR10+ 解析） | `tests/metadata.rs` |
| — | `luma-core/src/process.rs` | 新增（命令描述与引用规则） | `tests/process.rs` |
| — | `luma-core/src/execution.rs` | 新增（顺序执行与结果报告） | `tests/execution.rs` |
| — | `luma-core/src/providers.rs` | 新增（检测要求、运行时探测、厂商参数表） | `tests/providers.rs`；`doctor` 输出探测结果 |
| — | `luma-core/src/render.rs` | 新增（输入映射、色彩标签及其派生、编码器段、参数串、滤镜链、GOP 间隔、软件色彩转换段、曲线/色调映射/VPP 段） | `tests/render.rs`；`Transcoder` 经 `plan.*` 调用 |
| — | `app-macos/Sources/LumaKit/Conversion.swift` | 新增（macOS 第一条转换命令） | `LumaKitTests`；macOS 流水线真实转出 SDR 与 10 位 HDR |

规则层的“完成”指该文件中的格式决策已逐条迁移并有测试，不代表整个管线可用。
执行、探测与元数据仍在 C# 侧，P3 之前 Windows 应用继续使用现有实现。

## English

File-by-file correspondence between the Windows baseline and the Rust core.
States: **done** (rules migrated and tested), **first slice** (key rules migrated,
the rest still in C#), **not started**. The C# implementation stays until the
comparison and regression work finish, as plan §16.2 requires.

| Baseline | Rust target | State | Evidence |
| --- | --- | --- | --- |
| `Core/OutputPolicy.cs` | `luma-core/src/policy.rs` | Done | `tests/policy.rs` |
| `Core/FrameTimeline.cs` (conversion, seek, range) | `luma-core/src/timeline.rs` | Done | `tests/timeline.rs`; `qa/NativeChecks` parity, packet-indexed included |
| `Core/FrameTimeline.cs` (ffprobe read and fallback) | Probe adapter, TBD | Not started | — |
| `Assets/recommended-presets.json` | `luma-core/src/presets.rs` | Done | `tests/presets.rs` embeds it; `qa/NativeChecks` compares every selector |
| `Core/Models.cs` (`EncodeOptions`) | `luma-core/src/domain.rs` | Done | `tests/policy.rs` |
| `Core/Models.cs` (`MediaInfo`) | `luma-core/src/source.rs` | First slice | Only fields the rules need |
| `Core/DynamicHdr.cs` (`Validate`, `Preserve`) | `luma-core/src/policy.rs` | Done | `tests/policy.rs` |
| `Core/DynamicHdr.cs` (`ExtractAsync`) | Rules are in `policy`; the orchestration stays in the host | Stays by design | Frame bytes never cross the ABI; the host scans with `execution.run` |
| `Core/Transcoder.cs` (HDR metadata parameters, `HdrMetadata`) | `luma-core/src/metadata.rs` | Done (bridged as `plan.hdr_metadata`) | `tests/metadata.rs`; `qa/NativeChecks` compares the parameter string per encoder |
| `Core/Transcoder.cs` (`ResolveFormat`, `Validate`) | `luma-core/src/policy.rs` | First slice (ResolveFormat and Validate bridged) | `qa/NativeChecks` parity and bridge replay |
| `Core/Transcoder.cs` (FFmpeg arguments) | Stays in the platform backend | Not started | Plan §3.2 assigns this to the backend |
| `Core/Transcoder.cs` (execution, atomic publish) | `luma-core/src/execution.rs` | Core side done | Ordered runs, per-step events and the sink exist; starting a process stays with the host |
| `Core/MediaProbe.cs` | Normalization in `probe.rs` | Done | The ffprobe call stays in the host (files and subprocesses) |
| `Core/FrameExporter.cs`, `FramePreview*.cs` | Stays in the host | Stays by design | Frame bytes never cross the ABI; the macOS preview uses AVFoundation |
| `Core/MotionPhoto.cs` | Stays in the host (frame bytes) | Mostly done | Recognition, extraction and the Ultra HDR flag exist on macOS with composed-byte tests; writing remains on Windows |
| `Core/RawDevelopment.cs` | Chain in `raw.rs`, demux in the host | Partial | `plan.raw_filter` has ordering tests; the host-side demux and calibration commands are not done |
| `Core/Hdr10Plus*.cs` | SEI reader and document builder in `sei.rs` | Partial | `metadata.hdr10plus` and `metadata.hdr10plus_document` are tested; the scan orchestration stays in the host |
| `Core/UltraHdrCodec.cs` | Image encoding (P6) | Not started | — |
| `Core/ProcessRunner.cs` | `luma-core/src/execution.rs` | Core side done | A host may move to `execution.run`; Windows keeps its own |
| `Core/CliArguments.cs`, `CliReporter.cs`, `NativeCli*.cs` | Stays in the host, calls the core for rules | Not started | P3 |
| `Core/InputFiles.cs` | Stays in the host | Stays by design | Directory walking and globs are file system work, not rules |
| `Core/ToolReportCache.cs` | Stays in the host | Stays by design | The cache key is process and file state |
| — | `luma-core/src/capability.rs` | New | Plan §4; `qa/NativeChecks` compares the C# backend table |
| — | `luma-core/src/plan.rs` | New | Plan §3.2 "task planning", no baseline |
| — | `luma-core/src/cache.rs` | New (§10.4 identity and retention) | `tests/cache.rs`; `doctor` emits the plan, `cache --purge` deletes, CI checks both |
| — | `luma-core/src/safety.rs` | New (§10.3 output path safety) | `tests/safety.rs` |
| — | `luma-core/src/probe.rs` | New (ffprobe normalization) | `tests/probe.rs`; `qa/NativeChecks` compares a captured document |
| — | `luma-core/src/metadata.rs` | New (DOVI / HDR10+ parsing) | `tests/metadata.rs` |
| — | `luma-core/src/process.rs` | New (command description and quoting) | `tests/process.rs` |
| — | `luma-core/src/execution.rs` | New (sequential execution and reporting) | `tests/execution.rs` |
| — | `luma-core/src/providers.rs` | New (requirements, runtime detection, vendor parameters) | `tests/providers.rs`; `doctor` reports the result |
| — | `luma-core/src/render.rs` | New (input mapping, color tags and their derivation, encoder block, parameter string, filter chain, GOP interval, software colour tail, curve/tone-map/VPP entries) | `tests/render.rs`; `Transcoder` calls `plan.*` |
| — | `app-macos/Sources/LumaKit/Conversion.swift` | New (first macOS conversion command) | `LumaKitTests`; the macOS pipeline converts SDR and 10-bit HDR for real |

"Done" for a rule file means its format decisions are migrated and tested, not
that the pipeline works end to end. Probing, execution and metadata are still in
C#, so the Windows application keeps using the existing implementation until P3.
