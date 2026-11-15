# 重构实施记录 / Refactor implementation record

[简体中文](#简体中文) · [English](#english)

## 简体中文

本目录记录 `next` 分支上跨平台重构的实际进度，与
[重构实施方案](../../README.md) 的阶段划分一一对应。方案描述目标；本目录只记录已经落地、
可以检查的内容，以及尚未开始的部分。

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

**尚未验证**：本次实施环境没有 Rust、Swift 和 .NET SDK（只有 .NET 运行时），
因此上述代码没有在本机编译或运行过。首个具备工具链的环境必须先执行：

```powershell
# Windows
./tools/Build-Native.ps1 -Configuration Debug -Test
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

```bash
# macOS
bash app-macos/scripts/build-macos.sh --skip-dmg --test
```

在 `cargo test` 与 `NativeChecks` 通过之前，不应把任何一条规则视为“已验证”。
这也是方案 §12 把“编译与打包”“单元与属性测试”“计划与模拟测试”分开记录的原因。

### 阶段进度

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 基线与研究 | 部分完成 | 分支、工具链、依赖清单已建立；各后端公开资料研究待逐项补 |
| P1 架构验证 | 代码完成，待实机验证 | 最小核心 + Windows/macOS 调用 + 启动骨架已就绪 |
| P2 共享核心 | 首片完成 | 模型、输出规则、时间轴、预设、能力、规划已迁移；探测、执行、元数据未迁移 |
| P3 Windows 接入 | 未开始 | 需要 `Transcoder` 规划改为调用核心，并保留 GUI/CLI 行为 |
| P4 多厂商适配 | 未开始 | 能力提供器与参数规则表尚未实现 |
| P5 macOS 应用 | 骨架完成 | 工作区布局与规则面板可用，媒体管线未接入 |
| P6 高级媒体 | 未开始 | HDR 图片、Live Photo、动态 HDR 元数据 |
| P7 校准与优化 | 未开始 | 预设候选、启动首帧、长任务 |
| P8 发布准备 | 未开始 | 四语文档、签名申请、打包 |
| P9 合并发布 | 未开始 | 合并 main、标签、正式 Release |

### 与方案的已知偏差

1. **跨语言传输**：ABI 1 的全部规划调用走单个 `luma_call_json` 入口与 JSON 信封。
   方案 §3.3 要求进度事件带任务 ID 与顺序信息并可靠送达终态，这需要类型化结构体与
   回调注册；在 P2 完成前不会把进度事件塞进这个信封。头文件已注明该边界。
2. **FLAC 浮点音轨**：Windows 基线 `OutputPolicy.FlacTracks` 实际允许
   `sample_fmt` 为 `flt`/`dbl` 的音轨，与其注释和方案 §6.2“仅整数无损”矛盾。
   Rust 实现按方案拒绝浮点，`qa/NativeChecks` 不比较该项。基线需要在 P2 同步修正。
3. **Xcode 工程**：macOS 侧用 Swift 包表达工程，可直接用 Xcode 打开 `app-macos/Package.swift`。
   方案 §11.2 要求的共享 Scheme、Archive 与 DMG 流程由脚本承担；若后续需要
   `.xcodeproj` 专有配置（entitlements 编辑器、归档设置），再补生成工程。
4. **CI 镜像**：方案要求 macOS 27 镜像；当前工作流使用 `macos-latest` 并在注释中标明
   待镜像发布后改为固定标签，避免引用不存在的标签导致整条流水线不可用。

### 下一步（P3 入口条件）

1. 在有工具链的机器上跑通 `cargo test`、`NativeChecks`，把结果记入本目录。
2. 把 `Core/Transcoder` 的规划段改为“解析选项 → 调核心 → 由后端生成参数”，
   保留现有 QSV 时间戳、VPP、LowPower 策略。
3. 用 `NativeChecks` 扩展迁移对照：加入 `Validate` 的拒绝路径与 FLAC 判定。
4. 探测适配器：`MediaProbe` 输出转换为 `SourceDto`，禁止规则直接读 ffprobe JSON。

## English

This directory records what the cross-platform refactor actually landed on the
`next` branch, mapped onto the phases of the refactor plan. The plan states the
target; this directory states what exists, what can be checked, and what has not
started.

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

**Not verified**: this environment has no Rust, Swift or .NET SDK (only the .NET
runtime), so none of the above was compiled or executed here. The first machine
with a toolchain must run the commands shown in the Chinese section.

Nothing above counts as "verified" until `cargo test` and `NativeChecks` pass, so
the first machine with a toolchain must run:

```powershell
# Windows
./tools/Build-Native.ps1 -Configuration Debug -Test
dotnet run --project qa/NativeChecks/NativeChecks.csproj -c Release
```

```bash
# macOS
bash app-macos/scripts/build-macos.sh --skip-dmg --test
```

The plan's §12 separates compile/package, unit and property tests, and plan and
simulation tests for exactly this reason: none of them substitutes for the next.

### Phase status

| Phase | State | Note |
| --- | --- | --- |
| P0 Baseline | Partial | Branch, toolchain and inventory exist; per-backend research remains |
| P1 Architecture | Coded, unverified | Minimal core plus both hosts and the startup shell are in place |
| P2 Shared core | First slice | Model, policy, timeline, presets, capability, planning; probe, execution and metadata remain |
| P3 Windows integration | Not started | `Transcoder` planning must call the core while GUI/CLI behaviour is preserved |
| P4 Multi-vendor | Not started | Capability providers and parameter rule tables |
| P5 macOS app | Shell only | Layout and rules panel work; media pipeline is not connected |
| P6 Advanced media | Not started | HDR images, Live Photo, dynamic HDR metadata |
| P7 Calibration | Not started | Preset candidates, first-frame timing, long jobs |
| P8 Release prep | Not started | Four-language docs, signing, packaging |
| P9 Merge and release | Not started | Merge to `main`, tag, release |

### Known deviations

1. **Transport**: every ABI 1 planning call goes through one `luma_call_json`
   entry with a JSON envelope. Progress events need task ids, ordering and
   guaranteed terminal delivery, which requires typed structs and callback
   registration; they will not be pushed into this envelope. The header states
   the boundary.
2. **FLAC float tracks**: the baseline `OutputPolicy.FlacTracks` accepts tracks
   whose `sample_fmt` is `flt`/`dbl`, contradicting its own comment and plan
   §6.2. The Rust implementation rejects them; the parity harness skips the case.
   The baseline needs the same correction in P2.
3. **Xcode project**: the macOS side is a Swift package that Xcode opens
   directly. Shared schemes, Archive and DMG packaging are handled by the script;
   a generated `.xcodeproj` is only needed for settings SwiftPM cannot express.
4. **CI image**: the plan asks for a macOS 27 runner. The workflow uses
   `macos-latest` with a comment to pin the label once the image exists, so the
   pipeline never references a label that does not resolve.

### Next steps (entry conditions for P3)

1. Run `cargo test` and `NativeChecks` on a machine with the toolchains, and
   record the results here.
2. Split `Core/Transcoder` planning into "resolve options → call core → backend
   builds arguments", keeping the existing QSV timestamp, VPP and LowPower
   policies.
3. Extend the parity harness to `Validate` rejection paths and FLAC decisions.
4. Add a probe adapter that turns `MediaProbe` output into `SourceDto`, so no
   rule reads ffprobe JSON directly.
