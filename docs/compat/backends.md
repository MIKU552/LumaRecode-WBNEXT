# 编码后端兼容性

简体中文 · [English](#english)

## 简体中文

本页记录**家族级**静态能力：某后端按设计支持的编码、位深与色度采样。它不代表本机一定可用——设备是否存在、驱动是否健康仍由运行时探测决定。

| 后端 | 编码器 | 编码 | 位深 | 采样 | 说明 |
| --- | --- | --- | --- | --- | --- |
| software | libx265 / libx264 / libsvtav1 | HEVC / H.264 / AV1 | 8 / 10 / 12 | 4:2:0 / 4:2:2 / 4:4:4 / 灰度 | AV1 仅 4:2:0 与灰度，位深 8 / 10 |
| qsv | hevc_qsv / h264_qsv / av1_qsv | HEVC / H.264 / AV1 | 8 / 10 | 4:2:0 | 需要 Intel 核显或 Arc |
| nvenc | hevc_nvenc / h264_nvenc / av1_nvenc | HEVC / H.264 / AV1 | 8 / 10 | 4:2:0 | AV1 仅 RTX 40 及更新 |
| amf | hevc_amf / h264_amf | HEVC / H.264 | 8 / 10 | 4:2:0 | 不支持 AV1 |
| media_foundation | hevc_mf / h264_mf | HEVC / H.264 | 8 | 4:2:0 | 系统编码器，能力随驱动变化 |
| videotoolbox | hevc_videotoolbox / h264_videotoolbox | HEVC / H.264 | 8 / 10 | 4:2:0 | Apple 芯片与 T2 |

规则由 `luma-core/src/capability.rs` 提供，C# 侧有同表镜像，`qa/NativeChecks` 对 6 × 3 × 3 × 4 组合逐项比对，任一侧改动都会使构建失败。

### 检测状态

运行时探测把 FFmpeg 报告的编码器映射成三档：`available`（编码器存在，且该家族不依赖设备证据）、`unavailable`（没有编码器）、`unverified`（编码器被编译进来，但还没有设备或驱动证据）。最后一档刻意不算“可用”——通用构建常常编译了 QSV/NVENC/AMF 编码器，却不代表机器上有对应硬件。

按项目要求，实机硬件验证暂不作为完成前提；有设备时再补记录，在此之前 `unverified` 是诚实的默认值。

### 厂商参数

| 后端 | 参数 | 类型 | 取值或含义 |
| --- | --- | --- | --- |
| qsv | preset | 枚举 | fast / medium / slow / slower |
| qsv | async_depth | 整数 | 并发深度 |
| nvenc | preset | 枚举 | p1–p7 |
| nvenc | tune | 枚举 | hq / ll / ull / lossless |
| nvenc | rc | 枚举 | constqp / vbr / cbr |
| amf | quality | 枚举 | speed / balanced / quality |
| amf | usage | 枚举 | transcoding / lowlatency / ultralowlatency / webcam |
| videotoolbox | realtime | 开关 | 优先实时编码 |
| videotoolbox | allow_sw | 开关 | 允许回退软件编码 |
| software | preset | 枚举 | fast / medium / slow / slower |
| software | crf | 整数 | 质量常数 |
| media_foundation | rate_control | 枚举 | cbr / vbr |



实机验证清单（待补）：QSV（Intel Arc / 核显）、NVENC（RTX 40）、AMF（RDNA 3）、VideoToolbox（M 系列）。在完成实机记录前，本页只声明家族能力，不声明具体机器可用。

## English

This page records **family-level** static capability: what a backend supports by design. It does not mean the current machine can use it — presence and driver health are decided by runtime probing.

| Backend | Encoders | Codecs | Depth | Sampling | Notes |
| --- | --- | --- | --- | --- | --- |
| software | libx265 / libx264 / libsvtav1 | HEVC / H.264 / AV1 | 8 / 10 / 12 | 4:2:0 / 4:2:2 / 4:4:4 / gray | AV1 is 4:2:0 and gray only, depth 8 / 10 |
| qsv | hevc_qsv / h264_qsv / av1_qsv | HEVC / H.264 / AV1 | 8 / 10 | 4:2:0 | Requires Intel graphics or Arc |
| nvenc | hevc_nvenc / h264_nvenc / av1_nvenc | HEVC / H.264 / AV1 | 8 / 10 | 4:2:0 | AV1 needs RTX 40 or newer |
| amf | hevc_amf / h264_amf | HEVC / H.264 | 8 / 10 | 4:2:0 | No AV1 |
| media_foundation | hevc_mf / h264_mf | HEVC / H.264 | 8 | 4:2:0 | System encoder; capability follows the driver |
| videotoolbox | hevc_videotoolbox / h264_videotoolbox | HEVC / H.264 | 8 / 10 | 4:2:0 | Apple silicon and T2 |

The rules live in `luma-core/src/capability.rs`, the managed side mirrors the table, and `qa/NativeChecks` compares all 6 × 3 × 3 × 4 combinations, so a one-sided change fails the build.

### Detection states

Runtime detection maps the encoders FFmpeg reports into three states: `available` (an encoder exists and the family does not need device evidence), `unavailable` (no encoder), and `unverified` (an encoder is compiled in, but no device or driver evidence exists yet). The last state deliberately does not count as usable: generic builds often compile QSV, NVENC and AMF encoders even when the machine has no such hardware.

Hardware verification on real devices is not a completion requirement for this project; records will be added when hardware is available, and `unverified` is the honest default until then.

### Vendor parameters

| Backend | Parameter | Type | Values or meaning |
| --- | --- | --- | --- |
| qsv | preset | choice | fast / medium / slow / slower |
| qsv | async_depth | integer | concurrency depth |
| nvenc | preset | choice | p1–p7 |
| nvenc | tune | choice | hq / ll / ull / lossless |
| nvenc | rc | choice | constqp / vbr / cbr |
| amf | quality | choice | speed / balanced / quality |
| amf | usage | choice | transcoding / lowlatency / ultralowlatency / webcam |
| videotoolbox | realtime | flag | prefer realtime encoding |
| videotoolbox | allow_sw | flag | allow a software fallback |
| software | preset | choice | fast / medium / slow / slower |
| software | crf | integer | constant quality |
| media_foundation | rate_control | choice | cbr / vbr |



Hardware verification list (pending): QSV (Intel Arc / integrated), NVENC (RTX 40), AMF (RDNA 3), VideoToolbox (M series). Until those records exist, this page claims family capability only, not a specific machine.
