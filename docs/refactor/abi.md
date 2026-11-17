# 共享核心 C ABI / Shared core C ABI

[简体中文](#简体中文) · [English](#english)

## 简体中文

头文件是唯一权威定义：[`native/include/luma_core.h`](../../native/include/luma_core.h)。
Windows 的 `Interop/LumaCoreNative.cs` 与 macOS 的 `LumaKit` 都只依赖该头文件导出的四个符号。

### 符号与所有权

| 符号 | 作用 | 所有权 |
| --- | --- | --- |
| `luma_abi_version()` | 返回已加载库实现的 ABI 修订号 | 无 |
| `luma_core_version()` | 返回静态存储的版本字符串 | 静态，**不可释放** |
| `luma_call_json(op, request)` | 执行一次操作 | 返回值由调用方用 `luma_string_free` 释放 |
| `luma_string_free(value)` | 释放上一个符号的返回值 | `NULL` 安全 |

调用约定为 `cdecl`。符号名不做修饰，Windows x64/ARM64 与 macOS ARM64 一致。

### 信封

每次调用都返回一个 JSON 对象，失败不是空指针：

```json
{"ok":true,"data":{"bit_depth":10}}
{"ok":false,"error":{"code":"policy.hdr_depth","detail":null}}
```

`NULL` 只表示分配失败，宿主应按致命错误处理。错误码是稳定标识，界面负责本地化；
`detail` 只携带被拒绝的值，例如 `"420"` 或 `"3:pcm_f32le"`。

### 操作

| 操作 | 请求 | 数据 |
| --- | --- | --- |
| `core.info` | `{}` | `abi`、`core_version`、`target_arch`、`target_os`、`operations` |
| `policy.describe` | `source`、可选 `options` | 源事实与可选采样、位深、默认值、FLAC 可用性 |
| `policy.resolve` | `source`、可选 `options` | 具体采样、范围、位深、像素格式、容器、后缀 |
| `policy.validate` | `source`、可选 `options` | 同上，但先执行完整校验 |
| `timeline.resolve` | `frames`、`time_base`、`packet_indexed`、`range` | 帧数、时长、`seek` 与起止秒 |
| `presets.resolve` | `document`、`purpose`、`codec`、`variant` | 编码器、预设、质量值、参数、音频默认 |
| `presets.validate` | `document` | 预设清单与数量 |
| `capability.select` | `devices`、`requirement` | 选中的设备、后端、得分、理由、是否回退 |
| `capability.summary` | `devices`、`codec` | 可用后端、最高位深、采样、HDR 能力 |
| `plan.build` | `source`、`options`、`backend`、`frame_window`、`motion_photo`、`preview_size` | 解码路径、帧内存位置、有序步骤、诊断标记 |
| `probe.summarize` | `document` | 宽高、旋转、类型、像素格式、位深、Dolby Vision、HDR10+、音轨数、归一化源描述、视频流序号、帧率 |
| `capability.backend` | `encoder` | 后端名，未识别为 null |
| `capability.supports` | `backend`、`codec`、`depth`、`sampling` | 该家族是否支持 |
| `capability.detect` | `encoders` | 每个后端的存在状态与匹配到的编码器 |
| `capability.parameters` | `backend`、`codec` | 参数名、类型与可选值 |
| `cache.plan` | `entries`、`current`、`policy` | 每项处置、保留与可回收字节 |
| `plan.input_map` | `input`、`video_index`、`audio_map`、`preserve_metadata`、`preserve_chapters` | `-i` 至流选择参数 |
| `plan.color_args` | `primaries`、`transfer`、`space`、`range`、`suppress_dv5_tags` | 四个色彩标志 |
| `plan.encoder_args` | `encoder`、`preset`、`quality`、`share`、`codec`、`depth`、`gop`、`auto_low_power`、`timing_compatibility`、`async_depth`、`pixel_format` | `-c:v` 起至 profile 段的参数 |
| `plan.parameter_string` | `values`、`appended` | 冒号连接的参数串 |
| `plan.filter_chain` | `filters` | 逗号连接的滤镜链 |
| `plan.gop` | `frame_rate`、`share`、`motion_photo` | GOP 帧数 |
| `plan.hdr_metadata` | `codec`、`output`、`source_kind`、`preserve_dynamic`、`dolby_vision_profile`、`hdr10_plus`、`side_data` | 编码器参数条目与静态元数据 token |
| `plan.color_filters` | `primaries`、`transfer`、`space`、`range`、`conversion_range`、`pixel_format`、`purpose`、`depth`、`preserve_dynamic` | zscale、format 与 setparams 三条滤镜 |
| `plan.render_filters` | `log`、`tone_map`、`dolby_render`、`drop_dynamic_metadata`、`vpp` | 曲线 LUT、色调映射、Dolby 渲染、元数据删除与 VPP，按需返回 |

`frames` 只传时间戳、时长和关键帧标记；解码后的图像不跨边界。

### 线程、取消与 panic

- 调用彼此独立，无全局可变状态；可在任意线程调用。
- 本修订不含长任务，因此没有取消令牌。执行与进度属于 P2 的类型化接口，
  会以任务 ID、序号和终态保证另行定义，不复用 JSON 信封。
- 所有 panic 在边界内被捕获并转换为 `ffi.panic`，不会终止宿主进程。

### 演进规则

1. 增加操作或可选字段属于兼容变更，不提升 ABI 修订号。
2. 删除操作、改变必需字段、改变错误码含义属于不兼容变更，必须提升
   `LUMA_ABI_VERSION`，并在宿主启动时比较版本后拒绝加载。
3. 错误码一经发布只增不改。新增规则必须有新码，不能挪用旧码表达新含义。

## English

The header is authoritative:
[`native/include/luma_core.h`](../../native/include/luma_core.h). Both hosts —
`Interop/LumaCoreNative.cs` on Windows and `LumaKit` on macOS — depend only on
the four exported symbols.

### Symbols and ownership

| Symbol | Purpose | Ownership |
| --- | --- | --- |
| `luma_abi_version()` | ABI revision implemented by the loaded library | None |
| `luma_core_version()` | Version string in static storage | Static, **never free** |
| `luma_call_json(op, request)` | Runs one operation | Caller frees with `luma_string_free` |
| `luma_string_free(value)` | Releases the previous symbol's result | `NULL` is safe |

Calling convention is `cdecl`; symbols are undecorated on Windows x64/ARM64 and
macOS ARM64 alike.

### Envelope

Every call answers with a JSON object; failure is not a null pointer:

```json
{"ok":true,"data":{"bit_depth":10}}
{"ok":false,"error":{"code":"policy.hdr_depth","detail":null}}
```

`NULL` means allocation failed and must be treated as fatal. Codes are stable
identities localized by the front end; `detail` only carries the rejected value.

### Operations

| Operation | Request | Data |
| --- | --- | --- |
| `core.info` | `{}` | `abi`, `core_version`, `target_arch`, `target_os`, `operations` |
| `policy.describe` | `source`, optional `options` | Source facts, allowed sampling and depth, defaults, FLAC eligibility |
| `policy.resolve` | `source`, optional `options` | Concrete sampling, range, depth, pixel format, container, suffix |
| `policy.validate` | `source`, optional `options` | Same shape, after full validation |
| `timeline.resolve` | `frames`, `time_base`, `packet_indexed`, `range` | Count, duration, `seek` target and start/end seconds |
| `presets.resolve` | `document`, `purpose`, `codec`, `variant` | Encoder, preset, quality, parameters, audio default |
| `presets.validate` | `document` | Preset catalog and count |
| `capability.select` | `devices`, `requirement` | Chosen device, backend, score, reasons, fallback flag |
| `capability.summary` | `devices`, `codec` | Available backends, maximum depth, samplings, HDR capability |
| `plan.build` | `source`, `options`, `backend`, `frame_window`, `motion_photo`, `preview_size` | Decode path, frame transport, ordered steps, diagnostic notes |
| `probe.summarize` | `document` | Size, rotation, kind, pixel format, depth, Dolby Vision, HDR10+, audio count, normalized description, video index, frame rate |
| `capability.backend` | `encoder` | Backend name, or null when unknown |
| `capability.supports` | `backend`, `codec`, `depth`, `sampling` | Whether the family supports it |
| `capability.detect` | `encoders` | Presence state and matched encoders per backend |
| `capability.parameters` | `backend`, `codec` | Parameter names, kinds and choices |
| `cache.plan` | `entries`, `current`, `policy` | Decision per entry, kept and reclaimable bytes |
| `plan.input_map` | `input`, `video_index`, `audio_map`, `preserve_metadata`, `preserve_chapters` | Arguments from `-i` through the stream selection |
| `plan.color_args` | `primaries`, `transfer`, `space`, `range`, `suppress_dv5_tags` | The four color flags |
| `plan.encoder_args` | `encoder`, `preset`, `quality`, `share`, `codec`, `depth`, `gop`, `auto_low_power`, `timing_compatibility`, `async_depth`, `pixel_format` | Arguments from `-c:v` through the profile |
| `plan.parameter_string` | `values`, `appended` | Colon-separated parameter string |
| `plan.filter_chain` | `filters` | Comma-separated filter chain |
| `plan.gop` | `frame_rate`, `share`, `motion_photo` | GOP length in frames |
| `plan.hdr_metadata` | `codec`, `output`, `source_kind`, `preserve_dynamic`, `dolby_vision_profile`, `hdr10_plus`, `side_data` | Parameter entries and static metadata tokens |
| `plan.color_filters` | `primaries`, `transfer`, `space`, `range`, `conversion_range`, `pixel_format`, `purpose`, `depth`, `preserve_dynamic` | The zscale, format and setparams filters |
| `plan.render_filters` | `log`, `tone_map`, `dolby_render`, `drop_dynamic_metadata`, `vpp` | Curve LUT, tone mapping, Dolby render, metadata drop and VPP, only the ones that apply |

`frames` carries timestamps, durations and keyframe flags only; decoded images
never cross the boundary.

### Threads, cancellation, panics

- Calls are independent, keep no global mutable state and may run anywhere.
- This revision has no long-running work, so there is no cancellation token.
  Execution and progress belong to the typed P2 interface with task ids,
  sequence numbers and guaranteed terminal delivery; they will not reuse the
  JSON envelope.
- Every panic is caught at the boundary and reported as `ffi.panic`; a rule bug
  cannot abort the host.

### Evolution rules

1. Adding an operation or an optional field is compatible; the ABI revision stays.
2. Removing an operation, changing a required field or changing a code's meaning
   is incompatible and requires a new `LUMA_ABI_VERSION`; hosts compare the
   revision before use.
3. Published error codes only ever gain members. A new rule needs a new code and
   must not repurpose an existing one.
