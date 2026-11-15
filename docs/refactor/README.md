# 重构实施记录 / Refactor implementation record

[简体中文](#简体中文) · [English](#english)

## 简体中文

本目录记录 `next` 分支上跨平台重构的实际进度，与
[重构实施方案](../../README.md) 的阶段划分一一对应。方案描述目标；本目录只记录已经落地、
可以检查的内容，以及尚未开始的部分。

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

### 已验证与未验证

已经在本仓库内完成、可直接检查的部分：

- 规则实现与 Windows 基线逐条对应，包括 4:2:0 不得上采样、HDR 最低 10 位、
  Mi-Log 默认 Full、动态 HDR 只能走软件归档 HEVC、FLAC 声道布局校验等。
- 预设解析直接读取 `Assets/recommended-presets.json`，单元测试内嵌该文件，
  资源与代码不同步时测试立即失败。
- C# 对照检查会用 12 组源描述 × 12 组选项比较采样、位深、范围、可选值、默认值与
  `ResolveFormat` 的接受/拒绝结果。

**已在本机验证**：Rust 1.90.0（GNU 目标）与 .NET SDK 11.0.100 下，`cargo test`
通过 50 项，`cargo fmt --check` 与 `cargo clippy -D warnings` 无问题，
`qa/NativeChecks` 完成 1176 次比较且无差异。工具链锁定随后上调到 1.100.0（当时最新稳定，Windows 与 macOS 共用同一版本）：
低于 1.99 的 rustc 在 macOS 27 上生成的 proc-macro dylib 会被 dyld 拒绝
（mis-aligned LINKEDIT）。macOS 作业还会选用镜像中最新安装的 Xcode，
后续结论以 CI 结果为准。

**尚未验证**：Swift 应用（本机没有 Swift 与 Xcode）、WinUI 界面（本机没有
Windows App SDK 构建环境），以及 `Build-Native.ps1` 的 MSVC 分支（本机只有
MinGW，故用 GNU 目标验证核心）。这些项目应按方案 §12 记为“未运行”，
不与通过或失败合并统计。具备对应环境时应先执行：

```powershell
# Windows
./tools/Build-Native.ps1 -Configuration Debug -Test
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

```bash
# macOS
bash app-macos/scripts/build-macos.sh --skip-dmg --test
```

### 阶段进度

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 基线与研究 | 部分完成 | 分支、工具链、依赖清单已建立；各后端公开资料研究待逐项补 |
| P1 架构验证 | 部分验证 | 最小核心 + Windows/macOS 调用 + 启动骨架已就绪；Windows 调用与对照通过，macOS 构建与启动未验证 |
| P2 共享核心 | 首片完成 | 模型、输出规则、时间轴、预设、能力、规划已迁移；探测、执行、元数据未迁移 |
| P3 Windows 接入 | 未开始 | 需要 `Transcoder` 规划改为调用核心，并保留 GUI/CLI 行为 |
| P4 多厂商适配 | 未开始 | 能力提供器与参数规则表尚未实现 |
| P5 macOS 应用 | 骨架完成 | 工作区布局与规则面板可用，媒体管线未接入 |
| P6 高级媒体 | 未开始 | HDR 图片、Live Photo、动态 HDR 元数据 |
| P7 校准与优化 | 未开始 | 预设候选、启动首帧、解包缓存与空间回收、长任务 |
| P8 发布准备 | 未开始 | 四语文档、签名申请、打包 |
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

### Verified and unverified

Implemented and reviewable in this repository:

- Rule-by-rule correspondence with the Windows baseline, including the ban on
  widening 4:2:0 chroma, the 10-bit floor for HDR, Full-range default for Mi-Log,
  dynamic HDR limited to software archive HEVC, and FLAC channel-layout checks.
- Preset parsing reads `Assets/recommended-presets.json` directly and the tests
  embed that file, so an asset that no longer matches the registry fails at once.
- The C# parity check compares sampling, depth, range, option lists, defaults and
  `ResolveFormat` accept/reject across 12 source descriptions × 12 option sets.

**Verified locally**: with Rust 1.90.0 (GNU target) and .NET SDK 11.0.100,
`cargo test` passes 50 tests, `cargo fmt --check` and `cargo clippy -D warnings`
are clean, and `qa/NativeChecks` reports 1176 comparisons with no mismatch.
The toolchain pin then moved to 1.100.0, the latest stable at the time and the
single version both hosts use: rustc before 1.99 produced proc-macro dylibs that
macOS 27's dyld rejected (mis-aligned LINKEDIT). The macOS job also selects the
newest installed Xcode, so CI results are authoritative from here on.

**Not verified**: the Swift app (no Swift or Xcode here), the WinUI window (no
Windows App SDK build environment), and the MSVC branch of `Build-Native.ps1`
(only MinGW was available, so the core was verified with the GNU target). Plan
§12 records those as "not run" rather than folding them into pass or fail. With
the right environment, run:

```powershell
# Windows
./tools/Build-Native.ps1 -Configuration Debug -Test
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

```bash
# macOS
bash app-macos/scripts/build-macos.sh --skip-dmg --test
```

### Phase status

| Phase | State | Note |
| --- | --- | --- |
| P0 Baseline | Partial | Branch, toolchain and inventory exist; per-backend research remains |
| P1 Architecture | Partly verified | Minimal core and both hosts exist; the Windows load and parity pass, the macOS build and launch do not |
| P2 Shared core | First slice | Model, policy, timeline, presets, capability, planning; probe, execution and metadata remain |
| P3 Windows integration | Not started | `Transcoder` planning must call the core while GUI/CLI behaviour is preserved |
| P4 Multi-vendor | Not started | Capability providers and parameter rule tables |
| P5 macOS app | Shell only | Layout and rules panel work; media pipeline is not connected |
| P6 Advanced media | Not started | HDR images, Live Photo, dynamic HDR metadata |
| P7 Calibration | Not started | Preset candidates, first-frame timing, extraction cache and space reclamation, long jobs |
| P8 Release prep | Not started | Four-language docs, signing, packaging |
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
