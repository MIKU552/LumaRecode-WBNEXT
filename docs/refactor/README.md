# 重构实施记录 / Refactor implementation record

[简体中文](#简体中文) · [English](#english)

## 简体中文

本目录记录 `next` 分支上跨平台重构的实际进度，与
[重构实施方案](../../README.md) 的阶段划分一一对应。方案描述目标；本目录只记录已经落地、
可以检查的内容，以及尚未开始的部分。

本分支的产品标识为 **LumaRecode WBNEXT**（独立于上游 4.4.3 的重构预览版）；开发协助由 WorkBuddy
（DeepSeek-V4.1-Flash-1017，极高推理，1M 上下文）提供，提交不添加协作者记录。

逐条核对方案链接与待澄清问题的记录见[方案核对记录](plan-review.md)。

### 本次落地范围

对应方案 P0（基线与研究）、P1（架构验证），并开始 P2（共享核心）的第一片：

| 内容 | 位置 | 状态 |
| --- | --- | --- |
| 实施分支 | `next`（从 4.4.3 稳定 main 创建） | 完成 |
| Rust 工作区与工具链锁定 | `native/`、`rust-toolchain.toml` | 完成 |
| 共享核心：领域模型、输出规则、时间轴、预设、能力、任务规划 | `native/crates/luma-core/` | 首片完成 |
| 版本化 C ABI | `native/crates/luma-ffi/`、`native/include/luma_core.h` | 完成 |
| Windows 原生调用层 | `Interop/` | 完成（尚未接入 GUI） |
| 新旧规则对照检查 | `qa/NativeChecks/` | 完成（需要本机生成原生库） |
| macOS Swift 包与应用骨架 | `app-macos/` | 完成（工作区骨架） |
| 构建脚本 | `tools/Build-Native.ps1`、`app-macos/scripts/build-macos.sh` | 完成 |
| 持续集成 | `.github/workflows/native.yml` | 完成 |

### 分层与依赖方向

```text
native/crates/luma-core     纯规则，无文件系统、无子进程、无窗口
        ↑
native/crates/luma-ffi      C ABI 边界、JSON 信封、panic 隔离
        ↑                       ↑
Interop/（C# / .NET）      app-macos/（Swift / SwiftUI）
        ↑                       ↑
App/（WinUI 3，P3 接入）     LumaKit + 视图（P5 接入）
```

核心只描述“要做什么”，不生成 FFmpeg 参数：命令构造属于具体后端，两个平台各自实现。
这样 Windows 的 QSV 参数和 macOS 的 VideoToolbox 参数不会互相污染业务规则。

Windows 的 `Transcoder.ResolveFormat` 与 `Validate` 已接入共享核心：原生库可用时由核心解析
采样、范围与位深并给出输入类型，库缺失时自动回退托管规则；核心的拒绝码映射回现有中文文案，
未覆盖的码仍由托管检查给出原文案，因此界面与 CLI 的提示不变。桥接由 `Core/SharedCore.cs`
的委托与 `Interop/SharedCoreBridge.cs` 实现，`qa/NativeChecks` 在安装桥接后重放同一批请求，
比较判定、结果与消息。

编码器的 GOP 间隔同样由核心决定（`plan.gop`）。托管回退与核心共用同一条规则，包括半值一律
远离零取整；这条约定写在两处，`qa/NativeChecks` 逐帧率比较两条路径算出的 `-g`，因此任何一侧
改动都会立刻显形，而不是让装了原生库的机器与没装的机器产出不同的关键帧间隔。

HDR 元数据的参数组合也移到核心（`plan.hdr_metadata`）：保留动态 HDR 需要的码率下限、HDR10+
的 `dhdr10-info`，以及 x265 的 `master-display` / `max-cll` 与 SVT-AV1 的 `mastering-display` /
`content-light`，都按各编码器的十进制精度生成。托管回退保留同一套规则，`qa/NativeChecks` 用
PQ、Dolby Vision 与 HDR10+ 三种素材逐编码器比较完整参数串，两处不会写出不同的标签。

软件色彩转换段同样由核心拼装（`plan.color_filters`）：zscale 的范围与三项色彩参数、10 位共享档
的 P010 交换格式、以及 `setparams` 的标签写法各只写一次。滤镜的串接顺序仍由宿主决定，
`qa/NativeChecks` 比较完整 `-vf` 串，因此这一段换位置或换顺序都会立刻显形。

曲线 LUT、色调映射、Dolby 渲染、动态元数据删除与 QSV 的 VPP 段也在核心（`plan.render_filters`），
按需返回：宿主只决定插在哪一段，字符串只有一份。`qa/NativeChecks` 用 10 组素材比较完整 `-vf`，
覆盖 Log 曲线、HDR→SDR、Dolby 标准 HDR、QSV 通路与软件收尾。

色彩标签同样由核心派生（`plan.color_tags`）：容器没写 primaries/transfer 时按 HDR 或 SDR 的默认
补齐、声明过的值不被覆盖、RGB 与 Dolby Vision profile 5 的表面名各按规则改写，目标侧 SDR 一律落
BT.709、HDR 保留源描述而曲线与 RAW 落 HLG。同一批素材的颜色参数也逐条比较，标签派生错一位就会失败。

**滤镜顺序按设计留在宿主**：预览缩放、动态照片补边、RAW 显影、GPU 通路各自存在与否取决于宿主，
顺序编排是宿主的知识；会漂移的是每一条的拼写，而它们已经全部来自核心。曲线表同理——宿主保留
一份用于离线回退，`qa/NativeChecks` 逐条比较两边的 LUT 文件、矩阵与范围，两份表不可能悄悄分叉。

### 已验证与未验证

已经在本仓库内完成、可直接检查的部分：

- 规则实现与 Windows 基线逐条对应，包括 4:2:0 不得上采样、HDR 最低 10 位、
  Mi-Log 默认 Full、动态 HDR 只能走软件归档 HEVC、FLAC 声道布局校验等。
- 预设解析直接读取 `Assets/recommended-presets.json`，单元测试内嵌该文件，
  资源与代码不同步时测试立即失败。
- C# 对照检查会用 12 组源描述 × 12 组选项比较采样、位深、范围、可选值、默认值与
  `ResolveFormat` 的接受/拒绝结果。

**验证方式**：本机不再编译或测试，结论全部来自 GitHub Actions（提交 `5d93e26`）。

- `native` 工作流：格式与 Clippy 在 Linux 作业单独执行（一项检查只有一个答案），
  Linux / Windows / macOS 三平台分别运行全部单元测试（含缓存、探测与安全用例，
macOS 统一使用 `xcode-27` 镜像）；
  Windows x64、ARM64 原生库构建并暂存；macOS 27 由 `xcode-27` 镜像构建 Swift 包、
  运行 LumaKit 测试、组装 `.app` 并完成 ad-hoc 签名，再用 `--version` 验证终端入口与核心加载。
- `windows` 工作流：.NET SDK 11.0.100 以 `--locked-mode` 恢复并构建 WinUI 应用，
  构建托管层并运行对照检查（1881 次比较，含预设、探测、时间轴、拒绝路径、GOP、HDR 元数据、滤镜、色彩标签、曲线表与桥接回放，无差异），随后以
  `LUMARECODE_DIAGNOSTICS=1` 运行一次 `--cli --version`，确认宿主实际加载了共享核心；
  再用 FFmpeg 生成 2 秒测试源走一次 `convert`，校验输出为 HEVC / yuv420p，
  把 `probe` 与 `plan` 的 JSON 存入构建产物，并用 `doctor --json` 确认 `core.mode` 为 `shared`；
  最后以 `LUMARECODE_CORE=managed` 重跑一次，
  确认原生库被忽略时 CLI 仍可用（回退路径）；另有 ARM64 应用构建与 x64 单文件发布校验，
  发布产物必须恰好是一个 `LumaRecode.exe`。
- 工具链：Rust 1.100.0（Windows 与 macOS 共用同一 `rust-toolchain.toml`）、
  .NET 11.0.100、Swift 6.4.2 与 Xcode 27.2（镜像内最新安装版本）。

**仍未验证**：QSV / AMF / NVENC 的真实硬件编解码、GUI 运行时行为与首帧呈现、发布产物。
VideoToolbox 的共享档（HEVC + AAC）已在 macOS 流水线上真机编码并检查产物。按方案 §12 记为“未运行”，不与通过或失败合并统计。

### 阶段进度

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 基线与研究 | 部分完成 | 分支、工具链、依赖清单已建立；各后端公开资料研究待逐项补 |
| P1 架构验证 | 部分验证 | 最小核心 + Windows/macOS 调用 + 启动骨架已就绪；Windows 调用与对照通过，macOS 构建与启动未验证 |
| P2 共享核心 | 完成（规则与执行抽象） | 模型、输出规则、时间轴、预设、能力、规划、缓存、安全、探测、元数据与执行抽象均已迁移；宿主全面接线由 P3 收口 |
| P3 Windows 接入 | 完成（参数迁移） | 判定、校验、输入映射、色彩标签及其派生、编码器段、参数串、滤镜链拼接、GOP 间隔、HDR 元数据、软件色彩转换段与曲线/色调映射/VPP 段均已走原生桥；滤镜顺序与帧尺寸相关滤镜按设计留在宿主，因为它们依赖 RAW 显影、预览、动态照片与 GPU 通路等宿主阶段 |
| P4 多厂商适配 | 完成（实机验证暂略） | 后端映射、支持矩阵、检测要求、运行时探测与厂商参数表均已接线；按用户要求不以实机硬件验证作为完成前提。VideoToolbox 的共享档已在 macOS 流水线实机验证 |
| P5 macOS 应用 | 首条转换已跑通 | 打开文件、ffprobe 探测、核心归一化与真实流信息显示已完成；`convert` 用核心决定编码器、GOP、映射、参数、HDR 元数据 token、曲线 LUT 与色调映射，CI 里真实转出 SDR、10 位 HDR 与带 mastering display 的 PQ；曲线与 HDR→SDR 的链已生成并通过断言，但 Homebrew 的 FFmpeg 9 缺 zscale 与 libplacebo（lut3d 有），渲染在此环境被逐条明确拒绝；窗口里已能直接转换打开的文件（进度来自 FFmpeg 的 `-progress`，可取消）；预览画布与时间轴接上 AVFoundation 抽帧，可拖动定位；导出面板可选目录、命名并在覆盖前确认；共享档位走 VideoToolbox（参数由本宿主拼装，数值标注为未实测）；`doctor` 用 `execution.run` 逐步探测编码器与滤镜，是进度回调的第一个真实消费者 |
| P6 高级媒体 | 部分完成 | 动态照片识别（含 Ultra HDR 判定）已在 macOS 实现并有合成字节单测；HDR10+ 的 SEI 读取与 `dhdr10-info` 文档生成都进了核心（`metadata.hdr10plus`、`metadata.hdr10plus_document`，逐位与逐帧测试），宿主侧的扫描与导出未做；RAW 显影的滤镜链进了核心（`plan.raw_filter`，顺序有测试），宿主侧的 RAW 解包与校准命令未做 |
| P7 校准与优化 | 回收闭环完成 | 标识、策略、存储抽象、计划与带范围保护的删除已接线；预算测量、预设候选与长任务待做 |
| P8 发布准备 | 完成 | 四语文档与隐私说明就绪；按决定**不做代码签名**，发布页给出 SHA-256 供核对 |
| P9 合并发布 | 未执行 | 分支已就绪（检查全绿）；合并、标签与正式 Release 待执行（不带签名） |

### 与方案的已知偏差

1. **跨语言传输**：ABI 1 的全部规划调用走单个 `luma_call_json` 入口与 JSON 信封。
   方案 §3.3 要求进度事件带任务 ID 与顺序信息并可靠送达终态，这需要类型化结构体与
   回调注册；在 P2 完成前不会把进度事件塞进这个信封。核心侧的事件形状已就绪
   （`ProgressEvent`：任务 ID、序号、步号、比例，失败步自行上报），C 表面（回调注册）
   是剩下的另一半。
2. **单 EXE 与解包缓存**：补充后的方案 §10.4 已给出取舍——保留单 EXE，但要求把解包内容
   最小化、按构建标识复用、只保留当前与近期回退构建、在首屏就绪后异步回收，并提供缓存
   占用与清理入口；§11.3 还要求测试使用独立的解包缓存。当前 App 仍使用 `PublishSingleFile` + `IncludeAllContentForSelfExtract`（.NET 已把全量提取标注为不推荐），
   Rust 原生库尚未接入应用，也没有缓存索引。这部分属于
   P3（原生库接入）与 P7（缓存与空间回收），不能在 P3 之前用「解压到默认 %TEMP%」了事。
3. **Xcode 工程**：macOS 侧用 Swift 包表达工程，可直接用 Xcode 打开 `app-macos/Package.swift`。
   方案 §11.2 要求的共享 Scheme、Archive 与 DMG 流程由脚本承担；若后续需要
   `.xcodeproj` 专有配置（entitlements 编辑器、归档设置），再补生成工程。
4. **CI 镜像**：GitHub 的 macOS 镜像按 Xcode 主版本命名。macOS 27 目前发布在标签
   `xcode-27` 上，实际系统为 macOS 27.0.1、Xcode 27.0/27.1/27.2；没有 `macos-27`
   标签，而 `macos-latest` 指向 macOS 26，无法构建 27.0 部署目标。应用构建作业因此固定
   使用 `xcode-27`。该镜像属于预览镜像，不在 Actions SLA 内，待 macOS 27 正式发布后
   改为对应的稳定标签。

### 下一步

**已完成**：`Interop` 与 `qa/NativeChecks` 已加入 `LumaRecode.slnx` 与 `.sln`，`windows` 作业会读回
两份文件核对，IDE 与命令行现在是同一入口。

1. **执行层是合并前唯一的架构性缺口**：`DynamicHdr.ExtractAsync` 与 HDR10+ 码流扫描需要子进程与
   带任务 ID、序号的进度事件（方案 §3.3）；ABI 1 的 JSON 信封只承载规划调用，事件要走类型化结构体
   与回调。做完这一项，P6 的四个模块才有地方落。
2. 跨制式渲染需要带 `zscale` / `lut3d` / `libplacebo` 的 FFmpeg，以及（对 libplacebo 而言）可用的
   Vulkan 设备。Homebrew 的 ffmpeg 9 缺 `zscale` 与 `libplacebo`（`lut3d` 有），托管 Windows runner 的 full 构建虽带滤镜
   却没有 Vulkan（实测 `FFmpeg conversion failed`），所以两处都断言到「计划完整」与「缺哪个滤镜」
   为止；本机（full 构建 + GPU）用同一条链可以正常出片。
3. SDR→HDR 仍未接（核心显式拒绝，属产品决策）；宿主侧的预览缩放、动态照片补边、RAW 显影与整条链
   的顺序按设计留在宿主。
4. P6 的四个模块（动态照片、RAW 显影、HDR10+ 提取、Ultra HDR 图片）尚未开始，前两项依赖第 1 条。

## English

This directory records what the cross-platform refactor actually landed on the
`next` branch, mapped onto the phases of the refactor plan. The plan states the
target; this directory states what exists, what can be checked, and what has not
started.

The product identity on this branch is **LumaRecode WBNEXT**, an independent
refactor preview of upstream 4.4.3; development assistance comes from WorkBuddy
(DeepSeek-V4.1-Flash-1017, high reasoning, 1M context) and commits carry no
co-author trailer.

Link-by-link verification and the open questions are recorded in
[plan verification](plan-review.md#english).

### Scope of this change

Phase P0 (baseline and research), P1 (architecture validation), and the first
slice of P2 (shared core):

| Item | Location | State |
| --- | --- | --- |
| Implementation branch | `next` (from stable 4.4.3 `main`) | Done |
| Rust workspace and pinned toolchain | `native/`, `rust-toolchain.toml` | Done |
| Shared core: domain, policy, timeline, presets, capability, planning | `native/crates/luma-core/` | First slice done |
| Versioned C ABI | `native/crates/luma-ffi/`, `native/include/luma_core.h` | Done |
| Windows native call layer | `Interop/` | Done (not wired into the GUI) |
| Baseline parity check | `qa/NativeChecks/` | Done (needs a local native build) |
| macOS Swift package and app shell | `app-macos/` | Done (workspace shell) |
| Build scripts | `tools/Build-Native.ps1`, `app-macos/scripts/build-macos.sh` | Done |
| Continuous integration | `.github/workflows/native.yml` | Done |

### Layering

```text
native/crates/luma-core     pure rules: no filesystem, no subprocess, no window
        ↑
native/crates/luma-ffi      C ABI boundary, JSON envelope, panic isolation
        ↑                       ↑
Interop/ (C# / .NET)        app-macos/ (Swift / SwiftUI)
        ↑                       ↑
App/ (WinUI 3, wired in P3) LumaKit + views (wired in P5)
```

The core describes *what* must happen and never builds FFmpeg arguments: command
construction belongs to each backend, so QSV flags and VideoToolbox settings
cannot leak into shared business rules.

`Transcoder.ResolveFormat` and `Validate` now run through the shared core when the
native library is present and fall back to the managed rules when it is not. Core
rejection codes are mapped back to the existing localized messages; a code with no
mapping lets the managed check produce its exact wording, so GUI and CLI text is
unchanged. `Core/SharedCore.cs` holds the delegates and
`Interop/SharedCoreBridge.cs` the implementation; `qa/NativeChecks` installs the
bridge and replays the same requests to compare decision, values and message.

The encoder's GOP interval comes from the core too (`plan.gop`). The managed fallback
carries the same rule, including rounding a half away from zero, and the harness
compares the `-g` value both paths produce across a range of frame rates. An edit to
either side therefore fails at once, instead of letting a machine with the library and
one without write different keyframe intervals.

The HDR metadata parameters come from `plan.hdr_metadata`: the rate-control floor that
Dolby Vision needs, the `dhdr10-info` payload for HDR10+, and the `master-display` /
`max-cll` or `mastering-display` / `content-light` tokens, each at the precision its
encoder family expects. The managed fallback keeps the same rules and the harness
compares the whole parameter string per encoder for PQ, Dolby Vision and HDR10+
sources, so the two paths cannot label a stream differently.

The software colour tail comes from `plan.color_filters`: the zscale range and its three
colour arguments, the P010 surface quick share hands a 10-bit encoder, and the
`setparams` tag spelling are each written once. The host still decides how filters are
ordered, and the harness compares the whole `-vf` string, so moving or reordering that
block fails at once.

**The filter order stays in the host by design**: preview scaling, motion-photo padding, RAW
development and the GPU path each exist only on some hosts, so ordering is host knowledge.
What could drift is the spelling of each entry, and that all comes from the core. The curve
table works the same way: the host keeps a copy for the offline fallback, and
`qa/NativeChecks` compares the LUT file, matrix and range entry by entry, so the two copies
cannot diverge quietly.

Curve LUTs, tone mapping, the Dolby render pass, the dynamic metadata drop and the QSV VPP
stage come from `plan.render_filters`, which returns only the entries that apply: the host
decides where each one goes and the strings exist once. The harness compares the whole
`-vf` for ten sources, covering the log curve, HDR to SDR, Dolby standard HDR, the QSV path
and the software tail.

### Verified and unverified

Implemented and reviewable in this repository:

- Rule-by-rule correspondence with the Windows baseline, including the ban on
  widening 4:2:0 chroma, the 10-bit floor for HDR, Full-range default for Mi-Log,
  dynamic HDR limited to software archive HEVC, and FLAC channel-layout checks.
- Preset parsing reads `Assets/recommended-presets.json` directly and the tests
  embed that file, so an asset that no longer matches the registry fails at once.
- The C# parity check compares sampling, depth, range, option lists, defaults and
  `ResolveFormat` accept/reject across 12 source descriptions × 12 option sets.

**Verification path**: the repository is no longer built or tested locally;
every result below comes from GitHub Actions at commit `5d93e26`.

- `native`: format and Clippy run once on Linux (one answer per check), while Linux,
  Windows and macOS each run the unit tests, including the cache, probe and safety
  cases (macOS always on the `xcode-27` image); the
  Windows x64 and ARM64 libraries build and stage; macOS 27 builds the Swift
  package on the `xcode-27` image, runs the LumaKit tests, assembles `.app`,
  applies the ad-hoc signature and checks the terminal entry with `--version`.
- `windows`: .NET SDK 11.0.100 restores in `--locked-mode`, builds the WinUI
  application, builds the managed layers and runs the parity check (1881
  comparisons including presets, probe, timeline, rejection paths, GOP, HDR metadata, filters, colour tags, the curve table and the bridge replay, no mismatch), runs
  `--cli --version` to confirm the host loaded the shared core, converts a
  two-second synthetic clip and checks the output is HEVC / yuv420p, and finally
  captures the `probe` and `plan` documents as build artifacts, asserts
  `doctor --json` reports `core.mode` as `shared`, and reruns with
  `LUMARECODE_CORE=managed` to prove the fallback path stays usable. Separate
  jobs build the ARM64 application and publish the x64 single file, requiring
  exactly one `LumaRecode.exe`.
- Toolchain: Rust 1.100.0 (one `rust-toolchain.toml` shared by both hosts),
  .NET 11.0.100, Swift 6.4.2 with Xcode 27.2 (newest installed on the image).

**Still unverified**: real hardware encode/decode for QSV / AMF / NVENC, GUI runtime
behaviour and first-frame presentation, and the release artifacts.
VideoToolbox share (HEVC plus AAC) is encoded and checked on the macOS pipeline. Plan §12
records the others as "not run" rather than folding them into pass or fail.

### Phase status

| Phase | State | Note |
| --- | --- | --- |
| P0 Baseline | Partial | Branch, toolchain and inventory exist; per-backend research remains |
| P1 Architecture | Partly verified | Minimal core and both hosts exist; the Windows load and parity pass, the macOS build and launch do not |
| P2 Shared core | Done (rules and execution seam) | Model, policy, timeline, presets, capability, planning, cache, safety, probe, metadata and the execution seam are migrated; host wiring closes under P3 |
| P3 Windows integration | Done (parameters) | Decisions, validation, input mapping, color tags and their derivation, encoder block, parameter string, filter joining, the GOP interval, the HDR metadata parameters, the software colour tail and the curve/tone-map/VPP entries are bridged; the filter order and the frame-size filters stay in the host by design, because they depend on host stages such as RAW development, preview, motion photos and the GPU path |
| P4 Multi-vendor | Done (hardware verification skipped) | Backend mapping, support matrix, detection requirements, runtime detection and vendor parameter tables are wired; real-device verification is not a completion requirement by request |
| P5 macOS app | First conversion runs | Open, ffprobe probe, core normalization and real stream display work; `convert` takes encoder, GOP, mapping, parameters, the HDR metadata tokens, the curve LUT and the tone map from the core and produces SDR, 10-bit HDR and a PQ clip that keeps its mastering display in CI; the curve and HDR to SDR chains are built and asserted, but Homebrew's FFmpeg 9 has lut3d yet neither zscale nor libplacebo, so each render stops with a named refusal; the window can convert the open file directly (progress comes from FFmpeg's own `-progress` stream and the run can be cancelled); the preview canvas and timeline use AVFoundation stills and scrub; the export sheet picks the folder and name and confirms a replacement; share runs on VideoToolbox with host-built flags (the values are marked unmeasured); `doctor` probes encoders and filters through `execution.run`, the first real user of the progress sink |
| P6 Advanced media | Partial | Motion photo recognition (with the Ultra HDR flag) is implemented on macOS and tested on composed bytes; the HDR10+ SEI reader and the `dhdr10-info` document builder are in the core (`metadata.hdr10plus`, `metadata.hdr10plus_document`, tested bit by bit and frame by frame) while the host-side scan and export are not; the RAW development chain is in the core (`plan.raw_filter`, ordering tested) while the host-side demux and calibration commands are not |
| P7 Calibration | Reclamation closed | Identity, retention, the store seam, the plan and guarded deletion are wired; budgets, preset candidates and long jobs remain |
| P8 Release prep | Done | Four-language docs and the privacy statement are in place; builds ship **unsigned** by decision, with a SHA-256 published for checking |
| P9 Merge and release | Not executed | The branch is ready and green; the merge, tag and release await execution, without signing |

### Known deviations

1. **Transport**: every ABI 1 planning call goes through one `luma_call_json`
   entry with a JSON envelope. Progress events need task ids, ordering and
   guaranteed terminal delivery, which requires typed structs and callback
   registration; they will not be pushed into this envelope. The core shape exists now
   (`ProgressEvent`: task id, sequence, step, fraction, with the failing step reporting
   itself); the C surface is the remaining half, and the header states that boundary.
2. **Single EXE and the extraction cache**: the supplemented §10.4 decides this —
   keep the single file, but minimise what is written, reuse an extraction result
   per build identity, retain only the current and a few fallback builds, reclaim
   asynchronously after the first screen, and expose cache size and cleanup in
   settings; §11.3 also requires tests to use a separate extraction cache. The
   app still publishes with `PublishSingleFile` plus `IncludeAllContentForSelfExtract`
   — the full-extraction mode .NET marks as not recommended — the Rust library is
   not wired into it yet, and there is no cache index. That work
   belongs to P3 (native library wiring) and P7 (cache and space reclamation);
   extracting into the default `%TEMP%` is not an acceptable shortcut.
3. **Xcode project**: the macOS side is a Swift package that Xcode opens
   directly. Shared schemes, Archive and DMG packaging are handled by the script;
   a generated `.xcodeproj` is only needed for settings SwiftPM cannot express.
4. **CI image**: GitHub names macOS images after the Xcode major. macOS 27 is
   currently published under the label `xcode-27` — the image itself runs macOS
   27.0.1 with Xcode 27.0/27.1/27.2. No `macos-27` label exists, and
   `macos-latest` resolves to macOS 26, which cannot build a 27.0 deployment
   target, so the app job pins `xcode-27`. That image is a preview and sits
   outside the Actions SLA; switch to the stable label once macOS 27 is GA.

### Next steps

**Done**: `Interop` and `qa/NativeChecks` are in both `LumaRecode.slnx` and `.sln`; the `windows`
job reads each file back, so the IDE and the command line now open the same projects.

1. **The execution layer is the one architectural gap before the merge**: `DynamicHdr.ExtractAsync`
   and the HDR10+ bitstream scan need subprocesses and progress events carrying a task id and a
   sequence number (plan §3.3). The revision-1 JSON envelope only carries planning calls, so events
   need typed structs and a callback; until that lands P6's four modules have nowhere to go.
2. Both platforms build the HDR to SDR chain from the core. Rendering it needs two things:
   an FFmpeg with libplacebo and a usable Vulkan device. Homebrew's formula has no such
   filter, and the hosted Windows runner's full build has the filter but no Vulkan device
   (observed as `FFmpeg conversion failed`), so both jobs assert the complete plan and the
   refusal instead; the same chain renders fine on a machine with a GPU. Homebrew's ffmpeg 9
   has **none** of `zscale`, `lut3d` or `libplacebo`, so every colour conversion there stops
   at the plan. The curve LUT now runs on macOS (the core names the LUT file, its matrix and
   its range for all five curves); SDR to HDR is still missing, and the host's preview
   scaling, motion-photo padding, RAW development and chain order have not moved either.
3. macOS still needs a VideoToolbox share table; static HDR metadata and the colour
   tag derivation both live in the core now. Cross-transfer conversion only needs the
   macOS side to call `plan.render_filters` and pick a curve.
4. Execution: `DynamicHdr.ExtractAsync` and the HDR10+ bitstream scan need
   subprocesses and progress events, which belong to the typed P2 interface and
   not to the JSON envelope.

文档专用流程：仅修改 Markdown 或 `docs/` 时，构建工作流会被路径过滤跳过，只运行结构检查。
