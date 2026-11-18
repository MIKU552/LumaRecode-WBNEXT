# 命令行

简体中文 · [English](en.md) · [日本語](ja.md) · [Español](es.md) · [README](../../README.md)

在 **PowerShell 7** 中运行 `LumaRecode.exe`。无参数时打开 GUI，带 `probe`、`convert` 等子命令时运行 CLI。

程序使用 Windows GUI 子系统。终端调用时，**文本输出接 `| Out-Host`，JSON 输出接 `| ConvertFrom-Json`**，通过管道等待命令结束并读取 `$LASTEXITCODE`。其他程序可使用进程 API 等待退出。

输出编码为 UTF-8。下面的命令设置当前 PowerShell 会话的输出编码，并显示帮助、工具信息和转换示例。

## 开始使用

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
./LumaRecode.exe --help | Out-Host
./LumaRecode.exe --version | Out-Host
./LumaRecode.exe license --text | Out-Host
./LumaRecode.exe license --accept-terms | Out-Host
./LumaRecode.exe doctor --json | ConvertFrom-Json
./LumaRecode.exe probe "D:\Videos\clip.mp4" --json | ConvertFrom-Json
./LumaRecode.exe convert "D:\Videos\clip.mp4" | Out-Host
```

GUI 和 CLI 共用组件条款的接受记录。首次批处理可使用 `license --text` 阅读条款，再运行 `license --accept-terms`；也可在任务命令中添加 `--accept-terms`。

FFmpeg / ffprobe 默认先从 PATH 查找，再使用 GUI 保存的路径。可用 `--ffmpeg PATH --ffprobe PATH` 覆盖。`--no-settings` 使用独立配置，忽略 GUI 设置并禁用设置写入；使用此模式时添加 `--accept-terms`。`--lang system|zh|en|ja|es` 控制输出语言。

动态 HDR 素材默认使用软件 HEVC 保留元数据。`--dynamic-hdr standard --purpose share` 输出标准 HDR；`--range sdr` 转为 SDR。ProRes RAW 自动显影为 HLG，可用 `--range sdr` 输出 SDR。支持格式见[色彩转换](../color-conversion.md)。

## 命令

| 命令 | 功能 |
| --- | --- |
| `probe INPUT` | 音视频格式、位深、色彩与 Log 识别信息、流标签及首帧侧数据 |
| `presets` | 编码预设与编码器参数 |
| `profiles` | 内置 DJI、Samsung、Xiaomi 和 Apple Log 曲线与 LUT |
| `frames INPUT` | 显示帧数、时长与时间基 |
| `image INPUT` | 导出所选帧的 HDR / SDR 图片或动态照片 |
| `doctor` | 工具版本及编码器、滤镜列表；硬件模式由显卡与驱动决定 |
| `cache [--purge]` | 缓存清单与回收计划；`--purge` 按计划删除，仅限本程序缓存根目录 |
| `license [--text]` | 项目许可、第三方条款、完整许可目录与清单 |
| `plan INPUT...` | 预览转换选项、FFmpeg 参数、工作目录和输出色彩 |
| `convert INPUT...` | 单文件或顺序批量转换，完成后校验输出 |

各命令均可使用 `--help`。`presets`、`profiles`、`probe`、`doctor`、`cache`、`license` 返回 JSON；转换的文本进度写入 stderr，结果写入 stdout。

## 编码与色彩选项

`convert` 与 `plan` 共用下列选项：

| 选项 | 值 / 默认行为 |
| --- | --- |
| `--preset` | `share-avc`、`share-hevc`、`share-av1`、`archive-avc`、`archive-hevc`、`archive-av1`、`archive-av1-fast` |
| `--purpose` | `share` / `archive`；x64 默认 Intel QSV 快速分享，ARM64 默认软件归档；动态 HDR 素材默认归档 |
| `--codec` | `hevc` / `av1` / `avc`；默认 `hevc` |
| `--range` / `--dynamic-range` | `auto` / `sdr` / `hdr`；默认按素材动态范围选择 |
| `--transform` | `auto` / `none` / `log-hdr` / `log-sdr` / `hdr-sdr` |
| `--profile` | `auto` / `pocket3` / `samsung` / `milog` / `applelog` / `applelog2` |
| `--dynamic-hdr` | `preserve` / `standard` |
| `--input-kind` | `auto` / `sdr` / `hlg` / `pq` / `log` / `raw`；用于人工纠正原片标签 |
| `--sampling` | `auto` / `source` / `420` / `422` / `444` / `gray`；默认 `auto`：软件 HEVC / AVC 归档保持原采样，RGB 使用 4:4:4；AV1 和分享使用 4:2:0；422 / 444 / gray 需要软件 HEVC / AVC |
| `--color-range` | `auto` / `source` / `limited` / `full`；默认 `auto`：归档保持原范围，分享转换为 Limited |
| `--bit-depth` | `auto` / `8` / `10` / `12`；归档按原片位深和编码器上限选择，分享默认 SDR 8 位、HDR 10 位 |
| `--container` | `auto` / `mp4` / `mkv`；分享默认 MP4、归档默认 MKV |
| `--audio` | `auto` / `aac` / `opus` / `copy` / `flac`；自动策略与 GUI 一致 |
| `--variant` | `standard` / `fast`；fast 仅用于软件 AV1 备用档 |
| `--cpu-decode` / `--no-cpu-decode` | 强制软件解码 / 使用自动解码路径 |
| `--duration SECONDS` | 只转换指定的正数秒数，便于试片 |
| `--config FILE.json` | 载入编码选项；显式命令行选项优先 |

自动模式使用 SDR → SDR、HLG / PQ → HDR、Log → HLG。可通过 `--profile` 指定 Log 曲线，通过 `--input-kind` 指定素材类型。HDR 使用 HEVC / AV1。

采样和位深按原片上限校验。例如，8 位 4:2:0 可输出 8 位 4:2:0；10 位 4:2:2 可选择 8 / 10 位、4:2:0 / 4:2:2。软件 HEVC 支持 8 / 10 / 12 位，软件 AVC 与 AV1 支持 8 / 10 位。`--sampling source` 将 RGB 转为 4:4:4 YCbCr，灰度可使用 `source` 或 `gray`。

范围转换处理实际像素值；范围标签缺失时，程序参考像素格式与相机曲线解释输入。

`--audio flac` 检查全部音轨，仅接受可确认的无损整数输入，保留采样率、声道和整数精度，使用 MKV 封装。

```powershell
# CPU 归档
./LumaRecode.exe convert clip.mp4 --preset archive-hevc | Out-Host

# Pocket 3 D-Log M 转 HLG HEVC
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range hdr | Out-Host

# HDR 转 SDR，或 Log 转 SDR；需要 FFmpeg libplacebo / Vulkan
./LumaRecode.exe convert hdr.mp4 --range sdr | Out-Host
./LumaRecode.exe convert dlog.mp4 --input-kind log --profile pocket3 --range sdr | Out-Host

# 422 归档并保留原音轨
./LumaRecode.exe convert source-422.mov --preset archive-hevc --sampling 422 --audio copy | Out-Host

# 预览转换参数
./LumaRecode.exe plan clip.mp4 --preset archive-av1-fast --json | ConvertFrom-Json

# 归档保持采样与范围，音轨转为 FLAC
./LumaRecode.exe convert lossless.mov --preset archive-hevc --sampling source --color-range source --audio flac | Out-Host
```

配置文件使用下划线字段名；`range` 对应动态范围，输出文件路径通过 `--output` 指定。

```json
{"preset":"archive-hevc","range":"auto","sampling":"420","bit_depth":8,"audio":"copy","cpu_decode":true}
```

配置字段为上表编码选项和 `preset`，例如 `color_range` 对应 `--color-range`。优先级为：GUI 等价默认值 → 预设 → 配置字段 → 显式命令行字段。若命令指定 `--preset`，其预设选择覆盖配置中的 `preset`，其他配置字段仍生效。

## 裁剪与图片

`frames INPUT` 返回帧数、总时长和时间基。命令行帧编号从 0 开始，结束帧不包含在输出中。

```powershell
./LumaRecode.exe frames clip.mp4 --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --start-frame 30 --end-frame 90 --audio aac | Out-Host
./LumaRecode.exe convert clip.mp4 --start-time 1.5 --end-time 4 --audio aac | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr -o photo.jpg | Out-Host
./LumaRecode.exe image hdr.mp4 --frame 60 --image-format ultrahdr --motion --motion-start-frame 30 --motion-end-frame 90 -o photo_MP.jpg | Out-Host
```

帧范围与时间范围不能混用，也不能同时指定 `--duration`。时间以秒为单位，吸附到不早于所选时间的第一帧。逐帧裁剪重新编码音轨；归档默认音轨会自动选取 FLAC 或 AAC，手动指定的 `--audio copy` 需要改为 AAC、Opus 或符合条件的 FLAC。

`image` 接受单个输入，支持以下选项：

| 选项 | 说明 |
| --- | --- |
| `--frame N` | 所选帧，默认 0 |
| `--image-format auto\|hdr\|ultrahdr\|sdr\|png` | HDR AVIF、Ultra HDR JPEG、SDR JPEG 或 SDR PNG；自动按 HDR / SDR 选择 Ultra HDR / JPEG |
| `--quality 1..100` | 图片质量，默认 95；PNG 使用无损编码 |
| `--motion` | 为 JPEG 或 Ultra HDR 添加 Android 动态照片视频 |
| `--motion-start-frame N --motion-end-frame N` | 动态视频范围，默认封面附近约 3 秒，须包含封面帧 |
| `--motion-size source\|1080\|720` | 视频分辨率，默认保留原分辨率 |
| `--purpose archive\|share` | 动态视频的 CPU / Intel 编码，默认 `archive` |
| `--input-kind`、`--profile` | 与视频转换相同的输入解释和相机曲线 |
| `-o PATH`、`--overwrite`、`--skip-existing` | 输出路径、确认覆盖或跳过已有文件 |
| `--dry-run` | 输出图片和视频范围信息 |

动态照片保存为单个 `MP.jpg` 文件。HDR 图片要求 HDR、Log 或 RAW 素材；输出格式与 Google 规范见 [README](../../README.md#预览裁剪与图片)。

## 批量、输出与覆盖

```powershell
./LumaRecode.exe convert a.mp4 b.mov --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos" --recursive --output-dir "D:\Exports" | Out-Host
./LumaRecode.exe convert "D:\Videos\**\*.mp4" --recursive --output-dir "D:\Exports" --continue-on-error --json | ConvertFrom-Json
./LumaRecode.exe convert clip.mp4 --output "D:\Exports\result.mp4" --overwrite | Out-Host
```

目录扫描按常见视频扩展名筛选文件，添加 `--recursive` 可扫描子目录。通配符使用引号包围，`**` 配合 `--recursive` 使用。目录扫描保留相对的子目录；通配符以匹配文件共同父目录为根；显式文件按文件名输出。程序在转换前检查重复的输出路径，发生冲突时返回错误。

默认在源文件旁生成 `文件名_SDR或HDR_编码_用途.mp4或mkv`。单文件输出使用 `--output`，批量输出使用 `--output-dir`，两者互斥。输出目录可自动创建。交互运行时，已有输出会询问是否覆盖，默认取消。脚本可用 `--skip-existing` 跳过，或用 `--overwrite` 直接授权覆盖。覆盖在临时文件编码及校验成功后原子完成。输出路径需与源文件分开；失败或取消时清理临时文件。

任务按顺序执行，默认遇错停止。`--continue-on-error` 继续处理其余输入，批次退出码反映其中的失败。

`plan` 中的 `argv` 显示最终输出路径。转换时先写入临时目录，校验后移动到目标位置；`--overwrite` 使用原子替换。

## 脚本与进度输出

`convert --json` 和 `plan --json` 在 stdout 输出逐行 JSON，schema_version 为 1。转换事件包括 `start`、`log`、`status`、`decoder`、`progress`、`done`、`skipped`、`error`、`cancelled`、`summary`；plan 使用 `plan`。任务事件带 `job`、`total_jobs`、`input`、`output`；`data` 为事件内容。

`decoder.data` 为 `software` 或 `qsv`，表示当前编码尝试的解码方式。

进度包含 `fps`、`percent`、`position_seconds`、`duration_seconds`、`elapsed_seconds`、`eta_seconds`，未知的百分比或剩余时间为 null。错误包含稳定的 `code` 和可读的 `message`。`--log FILE` 向文件追加相同的 JSON Lines 事件，`--verbose` 在文本模式下显示 FFmpeg 日志。

```powershell
./LumaRecode.exe convert clip.mp4 --json --log job.jsonl | ForEach-Object { $_ | ConvertFrom-Json }
$LASTEXITCODE
```

| 退出码 | 含义 |
| --- | --- |
| 0 | 成功，包括按要求跳过已有输出 |
| 1 | 探测、编码、输出校验失败，或批量任务部分失败 |
| 2 | 参数、配置或选项组合错误 |
| 3 | 缺少 / 无效的运行时或 FFmpeg 工具 |
| 4 | 输入不存在、没有匹配文件，或色彩/Log 曲线需人工确认 |
| 5 | 输出、日志路径或文件冲突 |
| 6 | 尚未接受第三方组件条款 |
| 130 | CLI 协作取消 |

脚本通过进程退出码或 `$LASTEXITCODE` 读取结果，通过 JSON 的 `code` 字段处理错误。

交互式 Ctrl+C 同时中断 PowerShell 管道和转换任务，并清理临时输出。需要读取取消事件和退出码时，使用进程 API 发送控制台取消信号并等待程序退出；协作取消返回 130。
