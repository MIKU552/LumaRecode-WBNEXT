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

Hardware verification list (pending): QSV (Intel Arc / integrated), NVENC (RTX 40), AMF (RDNA 3), VideoToolbox (M series). Until those records exist, this page claims family capability only, not a specific machine.
