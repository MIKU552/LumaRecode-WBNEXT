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
| `Core/DynamicHdr.cs`（`ExtractAsync`） | 未迁移 | 未开始 | 依赖执行层 |
| `Core/Transcoder.cs`（`ResolveFormat`、`Validate`） | `luma-core/src/policy.rs` | 首片（ResolveFormat 与 Validate 已接原生桥） | `qa/NativeChecks` 对照与桥接回放 |
| `Core/Transcoder.cs`（FFmpeg 参数生成） | 保留在平台后端 | 未开始 | 方案 §3.2 明确归属后端 |
| `Core/Transcoder.cs`（执行、原子发布） | 执行层（P2 后续） | 未开始 | — |
| `Core/MediaProbe.cs` | 探测适配器（P2 后续） | 未开始 | — |
| `Core/FrameExporter.cs`、`FramePreview*.cs` | 帧服务（P2 后续） | 未开始 | — |
| `Core/MotionPhoto.cs` | 动态照片（P6） | 未开始 | — |
| `Core/RawDevelopment.cs` | RAW 显影（P6） | 未开始 | — |
| `Core/Hdr10Plus*.cs` | 动态 HDR（P6） | 未开始 | — |
| `Core/UltraHdrCodec.cs` | 图片编码（P6） | 未开始 | — |
| `Core/ProcessRunner.cs` | 执行层（P2 后续） | 未开始 | — |
| `Core/CliArguments.cs`、`CliReporter.cs`、`NativeCli*.cs` | 保留在宿主，规则调用核心 | 未开始 | P3 |
| `Core/InputFiles.cs` | 文件身份与覆盖保护 | 未开始 | P2 后续 |
| `Core/ToolReportCache.cs` | 能力缓存 | 未开始 | P4 |
| — | `luma-core/src/capability.rs` | 新增 | 对应方案 §4；`qa/NativeChecks` 对照 C# 后端映射 |
| — | `luma-core/src/plan.rs` | 新增 | 对应方案 §3.2「任务规划」，无基线 |
| — | `luma-core/src/cache.rs` | 新增（§10.4 标识与回收策略） | `tests/cache.rs`；`doctor` 输出计划，`cache --purge` 执行删除，CI 校验 |
| — | `luma-core/src/safety.rs` | 新增（§10.3 输出路径安全） | `tests/safety.rs` |
| — | `luma-core/src/probe.rs` | 新增（ffprobe 归一化） | `tests/probe.rs`；`qa/NativeChecks` 对照真实抓取文档 |
| — | `luma-core/src/metadata.rs` | 新增（DOVI / HDR10+ 解析） | `tests/metadata.rs` |
| — | `luma-core/src/process.rs` | 新增（命令描述与引用规则） | `tests/process.rs` |
| — | `luma-core/src/execution.rs` | 新增（顺序执行与结果报告） | `tests/execution.rs` |
| — | `luma-core/src/providers.rs` | 新增（检测要求、运行时探测、厂商参数表） | `tests/providers.rs`；`doctor` 输出探测结果 |
| — | `luma-core/src/render.rs` | 新增（输入映射、色彩标签、编码器段、参数串、滤镜链） | `tests/render.rs`；`Transcoder` 经 `plan.*` 调用 |

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
| `Core/DynamicHdr.cs` (`ExtractAsync`) | Not migrated | Not started | Needs the execution layer |
| `Core/Transcoder.cs` (`ResolveFormat`, `Validate`) | `luma-core/src/policy.rs` | First slice (ResolveFormat and Validate bridged) | `qa/NativeChecks` parity and bridge replay |
| `Core/Transcoder.cs` (FFmpeg arguments) | Stays in the platform backend | Not started | Plan §3.2 assigns this to the backend |
| `Core/Transcoder.cs` (execution, atomic publish) | Execution layer, later in P2 | Not started | — |
| `Core/MediaProbe.cs` | Probe adapter, later in P2 | Not started | — |
| `Core/FrameExporter.cs`, `FramePreview*.cs` | Frame services, later in P2 | Not started | — |
| `Core/MotionPhoto.cs` | Motion photos (P6) | Not started | — |
| `Core/RawDevelopment.cs` | RAW development (P6) | Not started | — |
| `Core/Hdr10Plus*.cs` | Dynamic HDR (P6) | Not started | — |
| `Core/UltraHdrCodec.cs` | Image encoding (P6) | Not started | — |
| `Core/ProcessRunner.cs` | Execution layer, later in P2 | Not started | — |
| `Core/CliArguments.cs`, `CliReporter.cs`, `NativeCli*.cs` | Stays in the host, calls the core for rules | Not started | P3 |
| `Core/InputFiles.cs` | File identity and overwrite protection | Not started | Later in P2 |
| `Core/ToolReportCache.cs` | Capability cache | Not started | P4 |
| — | `luma-core/src/capability.rs` | New | Plan §4; `qa/NativeChecks` compares the C# backend table |
| — | `luma-core/src/plan.rs` | New | Plan §3.2 "task planning", no baseline |
| — | `luma-core/src/cache.rs` | New (§10.4 identity and retention) | `tests/cache.rs`; `doctor` emits the plan, `cache --purge` deletes, CI checks both |
| — | `luma-core/src/safety.rs` | New (§10.3 output path safety) | `tests/safety.rs` |
| — | `luma-core/src/probe.rs` | New (ffprobe normalization) | `tests/probe.rs`; `qa/NativeChecks` compares a captured document |
| — | `luma-core/src/metadata.rs` | New (DOVI / HDR10+ parsing) | `tests/metadata.rs` |
| — | `luma-core/src/process.rs` | New (command description and quoting) | `tests/process.rs` |
| — | `luma-core/src/execution.rs` | New (sequential execution and reporting) | `tests/execution.rs` |
| — | `luma-core/src/providers.rs` | New (requirements, runtime detection, vendor parameters) | `tests/providers.rs`; `doctor` reports the result |
| — | `luma-core/src/render.rs` | New (input mapping, color tags, encoder block, parameter string, filter chain) | `tests/render.rs`; `Transcoder` calls `plan.*` |

"Done" for a rule file means its format decisions are migrated and tested, not
that the pipeline works end to end. Probing, execution and metadata are still in
C#, so the Windows application keeps using the existing implementation until P3.
