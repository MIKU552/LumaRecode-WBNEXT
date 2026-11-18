# 方案核对记录 / Plan verification record

[简体中文](#简体中文) · [English](#english)

## 简体中文

### 核对方式

按方案 §17「官方参考与研究入口」逐条打开影响实现的链接，记录页面当前内容与本仓库的关系。
纯资料入口（Rust 发布公告、FFmpeg 编码器列表、Apple 字体页等）只确认可访问，未逐页摘录。
§17 末尾的四条问题报告按方案要求视为研究起点，不作为已成立的结论。

### 链接核对

| 链接 | 页面当前内容 | 对本项目的影响 |
| --- | --- | --- |
| macOS 27 镜像 | 镜像为 macOS 27.0.1（Darwin 27.0.0），含 Xcode 27.0 / 27.1（默认）/ 27.2；YAML 标签是 `xcode-27` 或 `xcode-27-xlarge`，仅 arm64，属预览镜像 | 应用构建作业固定 `xcode-27`；`macos-latest` 是 macOS 26，无法构建 27.0 部署目标 |
| NVIDIA Video Codec SDK 13.1 | 支持 Windows ARM64 原生构建，明确不支持 ARM64EC；驱动 610 及以上、CUDA 13.1 及以上；示例针对 FFmpeg 7.1 编译 | ARM64 版 FFmpeg、原生依赖和 SDK 桩库必须全部是 ARM64 构建 |
| AMF 在 FFmpeg 中的设置 | `rc` 默认 -1 属内部保留，必须显式指定；`qvbr_quality_level` 需配合 `rc=qvbr`；`bf` 只适用于 H.264；页面没有 10 位说明 | AMF 的 10 位与 HDR 预设不能只依据该页，需查 AMF SDK 文档并实测 |
| .NET 单文件部署 | 默认只打包托管程序集；原生库需 `IncludeNativeLibrariesForSelfExtract`，且在启动前解压到 `%TEMP%/.net`；压缩会额外增加启动开销 | 方案 §10.4 已定；标识与回收规则已在 `luma-core/src/cache.rs` 实现并有测试，落盘与清理仍待 P7 |
| macOS 27 首次打开 | 逐应用在「系统设置 → 隐私与安全性」点「仍要打开」，按钮仅在尝试打开后约一小时内可用，需输入密码；不要求全局关闭 Gatekeeper | 与方案 §14.1 一致，分发说明按此撰写 |
| Liquid Glass | 用新 SDK 重建即自动采用标准控件的材质；自定义背景会干扰效果；`UIDesignRequiresCompatibility` 可临时保留旧外观；应用图标改为分层设计 | 现有 SwiftUI 骨架的自定义背景需要移除，图标需用 Icon Composer 重制 |

### 需要澄清的问题

1. **单 EXE 与解包缓存**（已由补充方案回答）：§10.4 保留单 EXE 并要求缓存索引、构建标识复用与空间回收。仍需确定的是构建标识的取值（源码提交 + 工具链？）、缓存根目录与保留预算，这三项要在 P3 定标识、P7 测量后定数值。
2. **macOS 预览镜像**：`xcode-27` 是预览镜像，不在 Actions SLA 内。正式发布构建是否允许使用预览镜像，还是等 macOS 27 转正后再发？
3. **Swift 与 Xcode 对应关系**：方案写 Swift 6.4 系列，需在镜像上以 `swift --version` 确认实际版本并写入构建记录。
5. **预设字段含义**：方案 §5.1 的「TU4」与 `recommended-presets.json` 里的 `"preset": "4"` 是否同一含义？建议在预设表增加说明字段，避免后续按其他后端的 TU 语义误读。
6. **macOS 的 FFmpeg 来源**：方案 §14.3 说 FFmpeg 由用户提供，但 macOS 从 Finder 启动时没有终端 PATH；查找顺序（应用内、`/usr/local/bin`、`/opt/homebrew/bin`、用户设置）需要在 P5 明确。

### 已确认一致的部分

- 输出规则与现有实现一致：SDR 8 位、HDR 10 位、Limited 默认、归档保留原范围、4:2:2 及以上仅软件 HEVC / AVC。
- 曲线顺序、Mi-Log 默认 Full、HDR10+ 仅软件 HEVC、Dolby Vision Profile 5 / 8 / 10 单层限制，与基线代码一致。
- 上述两类规则由 `qa/NativeChecks` 覆盖 1176 项比较，当前无差异，因此方案与基线在此范围内没有分歧。

## English

### Method

Every link in plan §17 that affects implementation was opened and compared with
this repository. Reference-only entries (Rust release notes, the FFmpeg codec
list, the Apple fonts page) were checked for reachability but not excerpted. The
four issue reports at the end of §17 are treated as research leads, not settled
conclusions, as the plan itself states.

### Link verification

| Link | Current content | Effect on this project |
| --- | --- | --- |
| macOS 27 image | Image runs macOS 27.0.1 (Darwin 27.0.0) with Xcode 27.0 / 27.1 (default) / 27.2; the YAML label is `xcode-27` or `xcode-27-xlarge`, arm64 only, preview status | The app job pins `xcode-27`; `macos-latest` is macOS 26 and cannot build a 27.0 deployment target |
| NVIDIA Video Codec SDK 13.1 | Native Windows ARM64 builds are supported; ARM64EC is explicitly not; driver 610+, CUDA 13.1+; samples built against FFmpeg 7.1 | ARM64 FFmpeg, native dependencies and SDK stubs must all be ARM64 builds |
| AMF settings in FFmpeg | `rc` defaults to -1 and is reserved, so it must be set explicitly; `qvbr_quality_level` requires `rc=qvbr`; `bf` applies to H.264 only; the page says nothing about 10-bit | AMF 10-bit and HDR presets cannot rely on that page; the SDK docs plus measurement are required |
| .NET single-file | Bundles managed assemblies only; native libraries need `IncludeNativeLibrariesForSelfExtract`, which extracts to `%TEMP%/.net` before start; compression adds startup cost | §10.4 decides it; the identity and retention rules now live in `luma-core/src/cache.rs` with tests, while extraction and deletion wait for P7 |
| macOS 27 first open | Per-app override in System Settings → Privacy & Security → "Open Anyway", available for about an hour after the attempt, password required; no global Gatekeeper override | Matches plan §14.1; distribution notes should say exactly this |
| Liquid Glass | Standard controls adopt the material when rebuilt with the new SDK; custom backgrounds interfere; `UIDesignRequiresCompatibility` keeps the old look temporarily; app icons move to layered artwork | The SwiftUI shell's custom backgrounds must go; the icon needs Icon Composer artwork |

### Open questions

1. **Single EXE and the extraction cache** (answered by the supplement): §10.4
   keeps the single file and requires a cache index, per-build reuse and space
   reclamation. What remains is the build identity value (source commit plus
   toolchain?), the cache root, and the retention budget — identity in P3,
   numbers after the P7 measurements.
2. **Preview runner image**: `xcode-27` is a preview outside the Actions SLA. May
   release builds use it, or should the macOS release wait for macOS 27 GA?
3. **Swift and Xcode mapping**: the plan names the Swift 6.4 series. Confirm the
   actual version with `swift --version` on the image and record it in the build
   information.
4. **Signing policy location**: does the required "Code signing policy" section
   live in the root README or on a separate page linked from it?
5. **Preset field meaning**: is plan §5.1's "TU4" the same as `"preset": "4"` in
   `recommended-presets.json`? A comment field in the table would stop a later
   reader from applying another backend's target-usage meaning.
6. **FFmpeg on macOS**: plan §14.3 has the user supply FFmpeg, but a Finder launch
   has no terminal PATH. The search order (inside the app, `/usr/local/bin`,
   `/opt/homebrew/bin`, a user setting) must be fixed in P5.

### Confirmed agreements

- Output rules match the current implementation: SDR 8-bit, HDR 10-bit, Limited
  by default, archive keeps the source range, 4:2:2 and above only on software
  HEVC / AVC.
- Curve order, Mi-Log defaulting to Full, HDR10+ restricted to software HEVC, and
  single-layer Dolby Vision 5 / 8 / 10 match the baseline code.
- `qa/NativeChecks` covers both groups with 1176 comparisons and no mismatch, so
  the plan and the baseline do not disagree in this area.
