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
  构建托管层并运行对照检查（1570 次比较，含预设、探测、时间轴、拒绝路径与桥接回放，无差异），随后以
  `LUMARECODE_DIAGNOSTICS=1` 运行一次 `--cli --version`，确认宿主实际加载了共享核心；
  再用 FFmpeg 生成 2 秒测试源走一次 `convert`，校验输出为 HEVC / yuv420p，
  把 `probe` 与 `plan` 的 JSON 存入构建产物，并用 `doctor --json` 确认 `core.mode` 为 `shared`；
  最后以 `LUMARECODE_CORE=managed` 重跑一次，
  确认原生库被忽略时 CLI 仍可用（回退路径）；另有 ARM64 应用构建与 x64 单文件发布校验，
  发布产物必须恰好是一个 `LumaRecode.exe`。
- 工具链：Rust 1.100.0（Windows 与 macOS 共用同一 `rust-toolchain.toml`）、
  .NET 11.0.100、Swift 6.4.2 与 Xcode 27.2（镜像内最新安装版本）。

**仍未验证**：真实硬件编解码（QSV / AMF / NVENC / VideoToolbox）、GUI 运行时行为与
首帧呈现、签名申请与发布产物。按方案 §12 记为“未运行”，不与通过或失败合并统计。

### 阶段进度

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 基线与研究 | 部分完成 | 分支、工具链、依赖清单已建立；各后端公开资料研究待逐项补 |
| P1 架构验证 | 部分验证 | 最小核心 + Windows/macOS 调用 + 启动骨架已就绪；Windows 调用与对照通过，macOS 构建与启动未验证 |
| P2 共享核心 | 完成（规则与执行抽象） | 模型、输出规则、时间轴、预设、能力、规划、缓存、安全、探测、元数据与执行抽象均已迁移；宿主全面接线由 P3 收口 |
| P3 Windows 接入 | 首片完成 | `ResolveFormat`/`Validate` 已接原生桥，CLI 双模式与转换冒烟通过；参数生成与 GUI 回归未接 |
| P4 多厂商适配 | 完成（实机验证暂略） | 后端映射、支持矩阵、检测要求、运行时探测与厂商参数表均已接线；按用户要求不以实机硬件验证作为完成前提 |
| P5 macOS 应用 | 骨架完成 | 工作区布局与规则面板可用，媒体管线未接入 |
| P6 高级媒体 | 未开始 | HDR 图片、Live Photo、动态 HDR 元数据 |
| P7 校准与优化 | 计划完成 | 标识、策略、存储抽象与宿主计划已接线；真实删除、预算测量、预设候选与长任务待做 |
| P8 发布准备 | 进行中 | 四语文档、签名政策与隐私说明已就绪；SignPath 申请待提交 |
| P9 合并发布 | 未开始 | 合并 main、标签、正式 Release |

### 与方案的已知偏差

1. **跨语言传输**：ABI 1 的全部规划调用走单个 `luma_call_json` 入口与 JSON 信封。
   方案 §3.3 要求进度事件带任务 ID 与顺序信息并可靠送达终态，这需要类型化结构体与
   回调注册；在 P2 完成前不会把进度事件塞进这个信封。头文件已注明该边界。
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

### 下一步（P3 入口条件）

1. 在能构建 Windows App SDK 的环境里，把 `Interop` 与 `qa/NativeChecks` 加入
   `LumaRecode.slnx` 与 `.sln`，让 IDE 与命令行构建使用同一入口。
2. 把 `Core/Transcoder` 的规划段改为“解析选项 → 调核心 → 由后端生成参数”，
   保留现有 QSV 时间戳、VPP、LowPower 策略。
3. 用 `NativeChecks` 扩展迁移对照：加入 `Validate` 的拒绝路径与 FLAC 判定。
4. 探测适配器：`MediaProbe` 输出转换为 `SourceDto`，禁止规则直接读 ffprobe JSON。

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
  application, builds the managed layers and runs the parity check (1570
  comparisons including presets, probe, timeline, rejection paths and the bridge replay, no mismatch), runs
  `--cli --version` to confirm the host loaded the shared core, converts a
  two-second synthetic clip and checks the output is HEVC / yuv420p, and finally
  captures the `probe` and `plan` documents as build artifacts, asserts
  `doctor --json` reports `core.mode` as `shared`, and reruns with
  `LUMARECODE_CORE=managed` to prove the fallback path stays usable. Separate
  jobs build the ARM64 application and publish the x64 single file, requiring
  exactly one `LumaRecode.exe`.
- Toolchain: Rust 1.100.0 (one `rust-toolchain.toml` shared by both hosts),
  .NET 11.0.100, Swift 6.4.2 with Xcode 27.2 (newest installed on the image).

**Still unverified**: real hardware encode/decode (QSV / AMF / NVENC /
VideoToolbox), GUI runtime behaviour and first-frame presentation, signing
application and release artifacts. Plan §12 records these as "not run" rather
than folding them into pass or fail.

### Phase status

| Phase | State | Note |
| --- | --- | --- |
| P0 Baseline | Partial | Branch, toolchain and inventory exist; per-backend research remains |
| P1 Architecture | Partly verified | Minimal core and both hosts exist; the Windows load and parity pass, the macOS build and launch do not |
| P2 Shared core | Done (rules and execution seam) | Model, policy, timeline, presets, capability, planning, cache, safety, probe, metadata and the execution seam are migrated; host wiring closes under P3 |
| P3 Windows integration | First slice | `ResolveFormat` and `Validate` bridged; CLI both modes and a real conversion pass, parameter generation and GUI regression remain |
| P4 Multi-vendor | Done (hardware verification skipped) | Backend mapping, support matrix, detection requirements, runtime detection and vendor parameter tables are wired; real-device verification is not a completion requirement by request |
| P5 macOS app | Shell only | Layout and rules panel work; media pipeline is not connected |
| P6 Advanced media | Not started | HDR images, Live Photo, dynamic HDR metadata |
| P7 Calibration | Planning done | Identity, retention, the store seam and the host plan are wired; deletion, budgets, preset candidates and long jobs remain |
| P8 Release prep | In progress | Four-language docs, signing policy and privacy statement in place; the SignPath application is pending |
| P9 Merge and release | Not started | Merge to `main`, tag, release |

### Known deviations

1. **Transport**: every ABI 1 planning call goes through one `luma_call_json`
   entry with a JSON envelope. Progress events need task ids, ordering and
   guaranteed terminal delivery, which requires typed structs and callback
   registration; they will not be pushed into this envelope. The header states
   the boundary.
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

### Next steps (entry conditions for P3)

1. Add `Interop` and `qa/NativeChecks` to `LumaRecode.slnx` and `.sln` from a
   machine that can build Windows App SDK projects, so the IDE and the command
   line use one entry point.
2. Split `Core/Transcoder` planning into "resolve options → call core → backend
   builds arguments", keeping the existing QSV timestamp, VPP and LowPower
   policies..
3. Extend the parity harness to `Validate` rejection paths and FLAC decisions.
4. Add a probe adapter that turns `MediaProbe` output into `SourceDto`, so no
   rule reads ffprobe JSON directly.
