# 更新记录 / Changelog

[简体中文](#简体中文) · [English](#english)

## 简体中文

### WBNEXT 5.0.0（首个发布）

- 版本号 5.0.0；产品名统一为「映码 · LumaRecode WBNEXT」（窗口标题、终端横幅、DMG 名）。
- Windows 每个架构两个包：不内嵌 FFmpeg 的，与内嵌 FFmpeg n9.0（GPL）的；内嵌包按仓库内钉死的
  SHA-256 校验，随包附许可证与 NOTICE，应用本身仍为 MIT。macOS 仍只发一个包：Homebrew 的 FFmpeg
  是动态链接的，只拷可执行文件在别的机器上起不来，静态构建另行决定。
- `release` 工作流由标签触发，产出 4 个 Windows ZIP 与 2 个 macOS DMG，并生成 SHA256SUMS。

- 由上游 4.4.3 派生并重命名为 WBNEXT 重构版，位于独立分支 `next`，非正式发布。
- 重构分支已就绪（三条流水线全绿），等待合并到 `main`；标签与正式 Release 待签名就绪后执行。
- 共享核心迁移到 Rust，Windows 与 macOS 使用同一 Rust 1.100.0 工具链；新增 `native`、`windows` 两条 Actions 工作流。
- 开发协助：WorkBuddy（DeepSeek-V4.1-Flash-1017，极高推理，1M 上下文）；此后的提交不再添加协作者记录。
- 补充 Windows / macOS 签名政策与隐私说明，为免费签名申请准备公开条款。
- `presets` 与预设面板显示编码后端；`doctor` 增加核心状态、后端统计与只读缓存报告。
- 校验流程新增 Windows ARM64 应用构建与 x64 单文件发布检查。
- 新增 `cache` 命令：查看缓存清单与回收计划，`--purge` 在范围保护下真正删除。
- 编码器的 GOP 间隔改由共享核心计算（`plan.gop`）；托管回退沿用同一规则，两条路径的 `-g` 由对照检查逐帧率比较。
- HDR 元数据参数组合改由共享核心生成（`plan.hdr_metadata`）：Dolby Vision 码率下限、HDR10+ 负载与 x265 / SVT-AV1 的静态元数据 token 统一在核心拼装。
- macOS 新增 `convert` 命令：编码器、GOP、流映射与参数串全部由共享核心决定，流水线里真实转出 SDR 与 10 位 HDR 各一条。
- 软件色彩转换段（zscale / format / setparams）改由共享核心拼装（`plan.color_filters`）；对照检查比较完整滤镜串。
- 探测摘要带出侧数据，macOS 的 `convert` 因此能把 mastering display 与亮度声明原样带给编码器。
- 曲线表增加一致性守卫：宿主的回退表与核心的 `plan.log_profile` 逐条比较 LUT 文件、矩阵与范围。
- 色彩标签改由共享核心派生（`plan.color_tags`）：缺省补齐、RGB 与 profile 5 的表面对应、SDR/HDR 目标标签各只写一次。
- macOS 的 `convert` 已能生成 HDR→SDR 的完整滤镜链与 BT.709 标签；渲染需要带 libplacebo 的 FFmpeg，构建缺少该滤镜时会给出明确提示而不是原始报错。
- macOS 支持相机曲线：`--log-profile` 配合 `--lut-dir`，文件名、矩阵与范围取自核心的曲线表（`plan.log_profile`），转出 HLG；缺少 `zscale`/`lut3d`/`libplacebo` 时会指名哪个滤镜缺失。
- LUT 路径含逗号、冒号等字符时会自动加引号，避免滤镜链被截断（此前 FFmpeg 会报 `No option name near …`）。
- macOS 的 `convert` 支持 `--dry-run`：只打印计划不执行，便于支持与流水线断言。
- macOS 窗口里可以直接转换打开的文件：进度取自 FFmpeg 的 `-progress` 流，可取消，失败与拒绝都有明确文字。
- 预览画布显示真实画面，时间轴可拖动定位（抽帧走 AVFoundation，时间映射有单测）。
- 导出面板：可选目标目录、改文件名、覆盖前明确确认；命名与去重规则有单测。
- macOS 支持共享档：`Assets/recommended-presets-macos.json` 提供 VideoToolbox 行，`-q:v`/`-realtime`/`-allow_sw` 由宿主拼装（数值待 M 系列实测校准）。
- macOS 的音频跟随预设表的决定：归档复制全部音轨，共享按 `presets.audio` 重新编码（AAC 256k / Opus 192k）。
- macOS 共享档在 CI 里用 VideoToolbox 真机编码（HEVC + AAC）并检查产物，不再是仅计划层断言。
- 执行层补上类型化进度事件（任务 ID、序号、步号、比例）；失败步会自行上报，比例只在全部成功时到 1。
- 新增 `luma_set_progress_sink` 与 `execution.run`：宿主给出命令序列，核心按步执行并把事件送回回调。
- macOS 新增 `doctor`：逐步探测 FFmpeg 的编码器与滤镜并列出缺项，是进度回调的第一个真实消费者。
- macOS 新增 `motionphoto`：从动态照片里取出内嵌视频（按声明区间复制，短读即报错）。
- macOS 新增 `hdr10plus`：从片段导出 `dhdr10-info` 文档；CI 用 x265 造片段做往返回读验证。
- CI 用 `ffmpeg-full` 真渲染曲线链（`lut3d` + `zscale`，断言输出 HLG/10 位）；色调映射仍需 Vulkan，托管 runner 不可用。
- 按决定取消 Windows 代码签名：不再申请签名证书，四语文档改为「不签名 + 发布 SHA-256」，P8 收口。
- P6 起步：macOS 能识别动态照片与 Ultra HDR（合成字节单测）；核心新增 HDR10+ SEI 读取、`dhdr10-info` 文档生成与 RAW 显影滤镜链（逐位/逐帧/顺序测试）。宿主侧的扫描与 RAW 解包仍待做。
- 曲线 LUT、色调映射、Dolby 渲染、动态元数据删除与 QSV 的 VPP 段也由核心拼装（`plan.render_filters`）；对照检查扩到 10 种素材。

### 4.4.3

- 默认构建与正式发布改用自包含 JIT 单文件，提供 X64 和 ARM64 两种 EXE。
- 保留 Native AOT 可选构建及 APX 固定目标；按编译方式隔离缓存、许可清单和输出。
- 在支持 APX 的 x64 设备上按启动条件启用 APX，尊重用户显式配置。
- JIT 命令行使用同一 EXE 的工作子进程，保留 PowerShell 管道取消后的任务清理。
- 添加 Visual Studio Release-AOT 配置，更新四语 README 与中英文开发文档。

### 4.4.2

- 为 FFmpeg / ffprobe 文本捕获和逐行元数据读取加入资源上限，超限后停止并回收子进程。
- 时间轴与 HDR10+ 索引采用内存预算，计入数组扩容和快照副本，保留长片流式元数据处理。
- 时间轴加载只生成一次最终快照；遇到缺失时间戳时提前切换到解码索引。
- 添加 UTF-8、管道回收、索引边界，以及长片索引和大体量元数据流的回归检查。

### 4.4.1

- Native 自动选择本机的 x64、APX 或 ARM64 目标，并启用本机指令集优化。
- 添加显式 X64 目标，与 APX、ARM64 一起支持固定目标和交叉编译。
- 添加 Visual Studio Release / Native 配置；自动选择 APX 时保留启动兼容性检查。
- 在构建前检查 Native 优化所需的 SDK 架构，支持识别仿真环境下的实际系统架构。

### 4.4.0

- 添加 Native、Intel APX 与 ARM64 构建目标，默认使用本机指令集的 Native AOT。
- 添加 Visual Studio 解决方案、GUI 启动配置和 Packaging 项目，Release 构建可直接生成单文件发行包。
- 按目标隔离依赖还原、编译缓存、许可清单和输出目录，并提供 ARM64 锁文件。
- 内置 ARM64 Ultra HDR 编解码库，使用 ARM64 NEON；ARM64 默认软件编码，并按架构选择 RAW 解码参数。
- APX 启动入口在加载应用前检查 CPU 与 Windows 支持，提供兼容性提示。

### 4.3.1

- 预览改为后台单解码器调度，合并拖动请求，并缓存最近访问的帧。
- 预览在色彩处理前缩小画面，通过有界 PNG 管道返回；复用缓冲并限制解码线程。
- 拖动时间轴只更新相关控件，布局只在断点或页面结构改变时重排。
- 修复直接输入帧号的同步，切换工具后重新识别素材，关闭窗口前等待预览进程退出。
- 精简逐帧索引分配，统一诊断尾部缓冲、代码格式与资源生命周期注释。
- 添加解决方案入口、源码检查命令和代码结构说明，整理四语文档及测试目录。

### 4.3.0

- 添加“图片与动态照片”页面、逐帧预览、拖动时间轴与封面帧选择。
- 添加视频开始 / 结束帧裁剪，并保持可变帧率时间轴与音频边界。
- 添加 HDR AVIF、Ultra HDR JPEG、SDR JPEG / PNG，以及 Google 格式的 Android 动态照片。
- 内置 libultrahdr 编码组件；动态视频可选择 CPU 或 Intel 编码。
- 命令行添加 `frames`、`image` 与帧 / 时间范围参数。
- 软件解码勾选框同步必需解码路径和运行时兼容回退。

### 4.2.0

- 添加 Apple Log 与 Apple Log 2 的 HLG / SDR 转换，Log 列表按 DJI、三星、小米、Apple 排列。
- 添加单层 Dolby Vision 与 HDR10+ 识别、标准 HDR / SDR 转换，以及软件编码保留逐帧动态元数据。
- 内置 C# HDR10+ 元数据解析与 x265 JSON 导出，HEVC 通过码流扫描提取元数据。
- 添加 ProRes RAW 相机参数显影、裁切与方向处理，编码前检查所选 FFmpeg 的相机兼容性。
- 修复 ffprobe HDR10+ 重复字段导致的素材读取错误。

### 4.1.4

- 修复 QSV 快速分享丢失旋转或镜像方向的问题，并校验方向变换后的输出尺寸。
- AV1 HDR 的重复元数据、QSV 解码或硬件帧格式错误自动切换软件解码，继续使用硬件编码。
- 低功耗模式不受支持时自动选择编码模式；兼容重试最多三次，保留质量档和已有输出。
- 快速分享 GOP 上限设为 240 帧，规避部分 Arc 驱动的长 GOP 故障。
- 补充 QSV 兼容策略、上游问题索引和实际编码回归检查。

### 4.1.3

- 修复部分可变帧率素材在 QSV H.264、HEVC 和 AV1 快速分享中的时间戳兼容问题。
- 绕开硬件滤镜的时间戳量化，保留原片帧数和播放节奏；H.264 和 HEVC 同时关闭 B 帧重排。
- 遇到未提前识别的时间戳冲突时，自动使用兼容路径重试一次。

### 4.1.2

- 日志区域独立处理滚轮和触控板滚动，到达边界时保持主界面位置。
- 在“转换日志”标题右侧添加复制按钮，复制当前全部日志并显示复制状态。

### 4.1.1

- 修正“结束后通知”的垂直对齐，移除居中布局中的顶部内边距和整组偏移。

### 4.1.0

- 添加 Windows 任务通知，成功时显示文件名、耗时、输出大小、时长和封装／编码格式，失败时显示文件名、耗时和原因。
- 在“打开目录”左侧添加默认开启的“结束后通知”开关，并保存选择；失败弹窗继续保留。
- 已有输出支持确认覆盖，GUI 默认突出“取消”，CLI 支持交互确认及脚本授权。
- 使用 Windows 原生通知与系统提示音设置，点击通知可返回程序。
- 任务结束时冻结耗时，避免将查看失败弹窗的时间计入转换耗时。

### 4.0.5

- GUI 直接传递进度快照，限制事件队列容量，并合并日志刷新。
- 本地化模板按需创建匹配器，批量转换复用 FFmpeg 能力查询。
- 清空素材时重置输入解释和输出默认值，关闭窗口时解除系统事件订阅。
- 主窗口按职责拆分文件，统一 C# 与开发脚本格式，补充异步事件和缓存检查。

### 4.0.4

- 路径文字、占位提示和相邻按钮共用光学偏移，保持同一行的文字位置一致。
- 底部操作按钮同步调整，开始按钮的图标和文字一起对齐。

### 4.0.3

- 路径输入框的实际文字和占位文字垂直居中，与相邻按钮统一行高和内边距。
- 按钮文字上移 1 个逻辑像素，校准字形的视觉中心；输出路径行同步调整。
- 保留首次选择视频的强调色，选入素材后切换普通按钮样式。

### 4.0.2

- 修复“更换视频”按钮丢失圆角的问题，两种状态均使用 WinUI 按钮模板。
- 视频选择按钮改用显式文本控件，统一字体回退与常规字重。

### 4.0.1

- 修复带 `.exe` 的工具名称查找、缺失命令行选项值，以及无效配置和日志路径的错误分类。
- 修复通配符中的方括号匹配，按匹配深度扫描目录，并优先识别包含方括号的现有文件路径。
- 修复数值解析溢出和截取时长超过原片时的进度总时长。
- GUI 与命令行共用编码选项和位深规则，移除旧进程通信格式，补充边界回归检查。
- 整理代码格式，补充核心流程、界面状态、字体、构建和测试注释。

### 4.0.0

- 转换核心与命令行迁至 C# / .NET，GUI 直接调用核心，发布包移除 Python 运行时。
- 保留素材识别、Log / HDR 转换、采样与位深限制、音轨策略、批量任务、进度、取消及输出校验。
- 移除 Python 路径设置和 `--python` 参数；`doctor` 改为报告原生后端。
- 调整项目目录、构建流程、第三方声明与四种语言的文档。

### 3.8.0

- 项目更名为映码 · LumaRecode，程序与命令行入口统一为 `LumaRecode.exe`。
- 更新应用图标、四种语言的界面名称与文档。
- 首次运行自动导入 FrameFlow 的语言、外观、工具路径与许可确认设置。
- 命令行环境变量使用 `LUMARECODE_` 前缀，同时兼容原有 `FRAMEFLOW_` 前缀。

### 3.7.4

- 界面统一使用 Source Sans 3 和思源黑体，日志使用 Source Code Pro，并提供中日文字体回退。
- 调整中日文标题字重与说明行距，统一下拉菜单的字体设置。
- 增加日语、西班牙语 README 和命令行指南，补齐四种语言的文档导航。

### 3.7.3

- 修复字体初始化过早访问应用资源导致的启动崩溃。

### 3.7.2

- 统一 Windows 原生字体与中日文字体回退，修正英语和西班牙语界面中的中文字形。
- 调整日语界面的标题字重、说明行距和文案长度，语言菜单按各语言显示字形。

### 3.7.1

- 增加日语和西班牙语，覆盖主界面、许可证窗口、提示信息和命令行帮助。
- 系统语言识别支持日语及各地区西班牙语；四种语言可即时切换。

### 3.7.0

- 增加 Limited / Full 色彩范围控制，归档默认保持原范围，分享默认转换为 Limited。
- 软件 HEVC / H.264 归档保持原采样，RGB 默认选择 4:4:4，AV1 与快速分享选择 4:2:0；支持手动指定采样。
- 增加位深选择，归档按原片与编码器上限输出，采样和位深只允许保持或降低；软件 AVC 支持 10 位 SDR。
- 增加 FLAC 音轨选项，按音轨检查无损整数输入并保留采样率、声道与精度，使用 MKV 封装。
- 修正手动输入类型的默认动态范围与转换选项，添加素材前禁用任务设置。
- 设置卡片全部展开，并排卡片等宽、等高，色彩转换集中 LUT、采样和范围设置，格式与音频提供封装、音轨与解码设置；窄窗口切换为单列。命令行同步支持新选项。

### 3.6.4

- WinUI 前端改用 Native AOT，单文件入口在图形界面启动后退出。
- 编译主视图 XAML，生成本地化映射与 JSON 序列化代码，缓存控件引用，禁用路径和日志的拼写检查与文字预测。
- 本机启动私有工作集均值由 122.3 MiB 降至 116.6 MiB，测量方法和结果见[性能优化](docs/performance.md)。
- 增加内存采样工具、设置和 JSON 兼容性检查，以及 AOT / JIT 独立锁文件；许可收集合并应用与打包入口的依赖图。

### 3.6.3

- 首次许可窗口的语言入口及辅助功能名称统一为“界面语言 / Language”。
- 修复配色弹出层的主题资源缓存，切回系统强调色时同步刷新控件颜色。

### 3.6.2

- 移除“工具路径”窗口的完整许可入口，可从“关于”及首次许可窗口查看条款。
- 配色面板和 ColorPicker 改为首次使用时创建。

### 3.6.1

- 首次许可窗口增加系统 / 简体中文 / English 语言选择，保存选择并同步主界面语言。
- 组件说明支持中英文，CLI 的 `license --text` 使用所选语言。
- 右上角新增原生“关于”窗口，显示项目图标、版本、开发与核验说明、GitHub 主页及完整许可入口。
- 中英文 README 增加项目图标；Release 使用 GitHub 附件自带的 SHA-256 校验值。

### 3.6.0

- 增加完整命令行入口：probe、convert、plan、presets、profiles、doctor、license，支持全部 GUI 编码/色彩选项、自动默认值、配置文件和路径覆盖。
- 支持多文件、目录、递归通配符、JSON Lines 进度/日志、fps 与剩余时间、批量跳过/继续、稳定退出码，以及 Ctrl+C / Ctrl+Break 取消。
- 单个 FrameFlow.exe 提供 GUI 和 CLI：无参数打开 GUI，带命令参数运行 CLI；支持 PowerShell 7 的管道输出、退出码和取消。
- 输出覆盖使用临时文件编码、校验与原子替换，源文件受写入保护；失败或取消时清理临时输出。
- 增加中英文命令行文档及终端、编码集成检查。

### 3.5.1

- 自主代码、文档、原创图形及项目生成的 Mi-Log / Samsung Log LUT 从 Apache-2.0 改为 MIT。
- 更新中英文 README、LUT 生成器 SPDX、许可清单与程序内说明。

### 3.5.0

- 开发与发布环境升级到 .NET SDK 11.0.100 / .NET 11.0.0、Windows App SDK 2.6.5、Windows SDK BuildTools 10.0.28000.2957、官方 Python 3.15.2；固定 SDK 和 NuGet 锁文件。
- 移除未使用的 AI、ML、Search、Widgets 依赖。
- 使用官方嵌入式 Python 替代 PyInstaller，提供包含运行时的单文件 EXE。
- 可选构建不含运行时的 framework-dependent ZIP，并在工具设置中支持选择外部 Python。
- 整理第三方许可、来源与哈希，增加首次运行组件条款窗口及全文入口。
- 增加 worker 集成检查，覆盖 SDR、10 位 HLG、三组 Log LUT、4:2:2、音频、进度与取消清理。

### 3.4.7

- 统一 WinUI 3 按钮和分组图标，调整素材导入及底部操作区布局。
- 修复工具栏图标裁切，窄窗口下输入解释和操作按钮自动换行。
- 语言菜单改为向下展开，与选择框等宽、右侧对齐；英文系统选项改为 System。

### 3.4.6

- 修复调整窗口宽度时内容右移、左侧留白及右侧控件裁切的问题。
- 滚动容器按视口宽度伸展，内层内容在宽屏居中、窄窗口切换为单列。

### 3.4.5

- 配色菜单改用独立原生弹出层与不透明主题底色，修复 HDR 下的亮带和关闭滞后。

### 3.4.4

- 升级 Windows App SDK 2.0.1，HDR 模式使用原生 SystemBackdropElement 显示 Mica。

### 3.4.3

- HDR 模式改用不透明主背景。

### 3.4.2

- 调整弹出面板的阴影绘制方式。

### 3.4.1

- 将跟随系统 / 浅色 / 深色集中到界面配色面板。

### 3.4.0

- 支持中英文即时切换及跟随系统语言，其他系统语言默认使用英文。
- 增加翻译资源检查。

### 3.3.0

- 更新矢量图标，提供 PNG、SVG 和多尺寸 ICO。

### 3.2.0

- 更名为映流 · FrameFlow，更新窗口、EXE、任务栏与品牌资源。

### 3.1.0

- 支持独立选择 MP4 / MKV，并检查音轨与封装的兼容性。
- 素材信息增加位深、采样与音轨，编码信息增加编码器额外参数。

### 3.0.0

- 迁移到原生 WinUI 3，加入 Mica、主题与系统强调色。
- 调整窗口布局，增加编码帧率、进度和剩余时间。

### 2.0.0

- 增加 PySide6 / Qt 界面、分类编码预设、动态范围与采样选项。
- 支持素材自动识别与三组色彩转换。

## English

### WBNEXT (refactor preview)

- Derived from upstream 4.4.3 and renamed to the WBNEXT refactor preview on the independent `next` branch; not an official release.
- The refactor branch is ready and green, awaiting the merge to `main`; the tag and the release wait for signing.
- The shared core moves to Rust; Windows and macOS use the same Rust 1.100.0 toolchain, with new `native` and `windows` Actions workflows.
- Development assistance: WorkBuddy (DeepSeek-V4.1-Flash-1017, high reasoning, 1M context); later commits carry no co-author trailer.
- Add the Windows / macOS code signing policy and privacy statement for the free signing application.
- Presets and the preset panel name the encoder backend; `doctor` reports the core state, backend counts and a read-only cache inventory.
- The pipeline now builds the ARM64 application and verifies the x64 single-file publish.
- New `cache` command: inspect the inventory and retention plan; `--purge` deletes inside the guarded root.
- The encoder GOP interval is computed by the shared core (`plan.gop`); the managed fallback keeps the same rule and the harness compares the `-g` value both paths produce for every frame rate.
- HDR metadata parameters are composed by the shared core (`plan.hdr_metadata`): the Dolby Vision rate-control floor, the HDR10+ payload and the x265 / SVT-AV1 static tokens are spelled in one place.
- macOS gains a `convert` command: encoder, GOP, stream mapping and the parameter string all come from the shared core, and the pipeline converts one SDR and one 10-bit HDR clip for real.
- The software colour tail (zscale / format / setparams) is composed by the shared core (`plan.color_filters`); the harness compares the whole filter chain.
- The probe summary now carries side data, so the macOS `convert` hands the mastering display and light levels back to the encoder.
- The curve table gains a consistency guard: the host's fallback copy is compared with `plan.log_profile` entry by entry.
- Color tags are derived by the shared core (`plan.color_tags`): gap filling, the RGB and profile 5 surface names, and the SDR/HDR target tags are written once.
- The macOS `convert` command now builds the whole HDR to SDR chain and the BT.709 tags; rendering needs an FFmpeg with libplacebo, and a build without it gets a clear message instead of a raw filter error.
- macOS develops camera curves: `--log-profile` with `--lut-dir` takes the file name, matrix and range from the core's curve table (`plan.log_profile`) and delivers HLG; when `zscale`, `lut3d` or `libplacebo` is missing, the message names it.
- A LUT path containing a comma or colon is quoted, so the filter chain is no longer cut short (`No option name near ...`).
- The macOS `convert` command gains `--dry-run`: it prints the plan and stops, which support and pipelines can assert on.
- The macOS window converts the open file directly: progress comes from FFmpeg's `-progress` stream, the run can be cancelled, and failures arrive as readable text.
- The preview canvas shows a real frame and the timeline scrubs; stills come from AVFoundation and the time mapping is unit tested.
- The export sheet picks the folder, edits the name and confirms a replacement; the naming and de-duplication rules are unit tested.
- macOS shares through VideoToolbox: `Assets/recommended-presets-macos.json` carries the rows and the host builds `-q:v` / `-realtime` / `-allow_sw` (the values await M-series measurement).
- macOS audio follows the table: an archive copies every track, a share re-encodes to the named preset (AAC 256k or Opus 192k).
- The macOS share path encodes with VideoToolbox in CI (HEVC plus AAC) and the artifacts are checked, not just the plan.
- The execution layer now has typed progress events (task id, sequence, step, fraction); a failing step reports itself and the fraction reaches one only on success.
- `luma_set_progress_sink` and `execution.run` let a host hand over a sequence of commands and hear every finished step.
- macOS gains `doctor`: it probes the FFmpeg encoders and filters step by step and lists what is missing, as the first real user of the progress sink.
- P6 begins: macOS recognises motion photos and Ultra HDR (tested on composed bytes), and the core gains the HDR10+ SEI reader, the `dhdr10-info` document builder and the RAW development chain (bit-level, frame-by-frame and ordering tests). The host-side scan and the RAW demux remain.
- Curve LUTs, tone mapping, the Dolby render pass, the dynamic metadata drop and the QSV VPP stage are composed by the core too (`plan.render_filters`); the harness now covers ten sources.

### 4.4.3

- Switch default builds and releases to self-contained JIT single EXEs for X64 and ARM64.
- Retain optional Native AOT and the fixed APX target; isolate caches, license records and outputs by compilation mode.
- Opt supported x64 devices into APX during startup while respecting explicit user settings.
- Run JIT commands in a worker launched through the same EXE to preserve cleanup after PowerShell pipeline cancellation.
- Add Visual Studio Release-AOT configurations and update all four READMEs and Chinese/English development guides.

### 4.4.2

- Bound FFmpeg / ffprobe text capture and streaming metadata lines, stopping and joining child processes when a limit is exceeded.
- Apply memory budgets to timeline and HDR10+ indexes, including array growth and snapshots, while retaining streamed metadata processing for long recordings.
- Create one final timeline snapshot and switch to decoded indexing early when packet timestamps are missing.
- Add regressions for UTF-8, pipe cleanup, index boundaries, long-recording indexes and large metadata streams.

### 4.4.1

- Native automatically selects the local X64, APX or ARM64 target and enables build-CPU instruction optimization.
- Add an explicit X64 target alongside APX and ARM64 for fixed-target builds and cross-compilation.
- Add the Visual Studio Release / Native configuration and retain the APX launch check when APX is selected automatically.
- Check the SDK architecture before Native optimization and detect the actual OS architecture under emulation.

### 4.4.0

- Add Native, Intel APX and ARM64 build targets, defaulting to Native AOT with the build CPU's instruction sets.
- Add a Visual Studio solution, GUI launch profile and Packaging project that produces a single-file distribution from Release builds.
- Isolate restore graphs, compilation caches, license manifests and output directories by target; add ARM64 lock files.
- Include an ARM64 NEON Ultra HDR codec; default ARM64 to software encoding and select RAW decoder arguments by architecture.
- Check CPU and Windows support in the APX launcher before loading the application.

### 4.3.1

- Use one background preview renderer, coalesce pending scrub requests and cache recently visited frames.
- Scale previews before color processing; return PNG through a bounded pipe with pooled buffers and limited decoder threads.
- Update frame-related controls while scrubbing and rearrange layout at structural breakpoints.
- Fix frame-number synchronization, source recognition after changing tools and preview-process cleanup on window close.
- Reduce frame-index allocations; share diagnostic tail buffers and unify code formatting and lifetime comments.
- Add solution navigation, source checks and architecture documentation; organize the four-language guides, development notes, measurements and QA directory.

### 4.3.0

- Add Images & Motion Photos, frame preview, timeline scrubbing and cover-frame selection.
- Add start/end frame trimming while retaining VFR timing and audio boundaries.
- Add HDR AVIF, Ultra HDR JPEG, SDR JPEG / PNG and Google-format Android Motion Photos.
- Include libultrahdr; offer CPU or Intel encoding for motion video.
- Add CLI `frames`, `image`, frame-range and time-range options.
- Keep the software-decoding checkbox synchronized with required decode paths and runtime fallback.

### 4.2.0

- Add Apple Log and Apple Log 2 to HLG / SDR; order Log profiles as DJI, Samsung, Xiaomi and Apple.
- Add single-layer Dolby Vision / HDR10+ detection, standard HDR / SDR conversion and per-frame metadata preservation with software encoding.
- Include a C# HDR10+ parser and x265 JSON export, with HEVC bitstream scanning.
- Add ProRes RAW camera-calibrated development, cropping and orientation, checking the selected FFmpeg's camera compatibility before encoding.
- Fix source inspection when ffprobe repeats HDR10+ fields.

### 4.1.4

- Fix rotation/reflection loss in QSV Quick share and verify dimensions after orientation changes.
- Fall back to software decoding after duplicate AV1 HDR metadata, QSV decoding or hardware-frame errors while retaining hardware encoding.
- Select encoding mode automatically when LowPower is unsupported; cap compatibility retries at three and preserve quality settings and existing outputs.
- Cap Quick share GOP length at 240 frames to avoid long-GOP failures on some Arc drivers.
- Document QSV policies, upstream reports and encoding regression checks.

### 4.1.3

- Fix timestamp compatibility for some VFR sources in QSV H.264, HEVC and AV1 Quick share.
- Bypass hardware-filter timestamp quantization to preserve source frame counts and pacing; disable B-frame reordering for H.264 and HEVC.
- Retry once through the compatibility path when an undetected timestamp conflict occurs.

### 4.1.2

- Keep wheel and touchpad scrolling inside the log region at its boundaries.
- Add a copy button beside Conversion log to copy the full log and show copy status.

### 4.1.1

- Align Notify when finished vertically by removing top padding and group offsets from the centered layout.

### 4.1.0

- Add Windows job notifications: success includes filename, elapsed time, size, duration and container/codec; failure includes filename, elapsed time and reason.
- Add a saved, enabled-by-default Notify when finished checkbox beside Open folder; retain failure dialogs.
- Confirm output replacement, emphasizing Cancel in the GUI; support CLI interactive confirmation and scripted overwrite permission.
- Use native Windows notifications and system sound settings; clicking a notification returns to the application.
- Freeze elapsed time at job completion so time spent viewing an error dialog is excluded.

### 4.0.5

- Pass progress snapshots directly to the GUI, bound the event queue and coalesce log refreshes.
- Create localization matchers on demand and reuse FFmpeg capability queries in batches.
- Reset input interpretation and output defaults when clearing a source; unsubscribe from system events on close.
- Split MainWindow by responsibility, unify C# / development-script formatting and add asynchronous-event and cache checks.

### 4.0.4

- Share optical offsets between path text, placeholders and adjacent buttons to align text on each row.
- Apply the same alignment to footer actions and the Start icon/text group.

### 4.0.3

- Vertically center path text and placeholders with consistent row height and padding beside buttons.
- Move button text up by 1 logical pixel for optical centering; align the output-path row the same way.
- Retain the accent style for initial source selection and use the standard button style after a source is selected.

### 4.0.2

- Restore rounded corners on Change video using WinUI button templates for both states.
- Use explicit text controls for source selection with consistent font fallback and regular weight.

### 4.0.1

- Fix tool lookup with `.exe` suffixes, missing option values and error classification for invalid config/log paths.
- Fix wildcard bracket matching, scan directories to the required depth and prefer existing paths containing brackets.
- Fix numeric overflow and progress duration when the requested sample exceeds the source length.
- Share encoding options and bit-depth policy between GUI and CLI, remove the old process protocol and add boundary checks.
- Unify code formatting and document core flows, UI state, fonts, builds and tests.

### 4.0.0

- Move the conversion core and CLI to C# / .NET; call the core directly from the GUI and remove the Python runtime from distribution.
- Retain source detection, Log / HDR conversion, sampling/depth limits, audio policy, batches, progress, cancellation and output verification.
- Remove Python path settings and `--python`; make `doctor` report the native backend.
- Update the project layout, build pipeline, third-party notices and four-language documentation.

### 3.8.0

- Rename the project to 映码 · LumaRecode and use `LumaRecode.exe` for GUI and CLI.
- Update the application icon, names and documentation in all four languages.
- Import FrameFlow language, appearance, tool paths and accepted component terms on first launch.
- Use the `LUMARECODE_` environment-variable prefix while accepting the previous `FRAMEFLOW_` prefix.

### 3.7.4

- Use Source Sans 3 and Source Han Sans throughout the interface, with Source Code Pro and Chinese/Japanese fallback in logs.
- Adjust Chinese/Japanese heading weights and description line spacing; unify dropdown fonts.
- Add Japanese and Spanish README / CLI guides and complete four-language navigation.

### 3.7.3

- Fix a startup crash caused by accessing application resources too early during font initialization.

### 3.7.2

- Unify Windows native fonts and Chinese/Japanese fallback, fixing Chinese glyph selection in English and Spanish interfaces.
- Adjust Japanese heading weight, description spacing and text length; display language-menu items with their corresponding glyphs.

### 3.7.1

- Add Japanese and Spanish across the main view, license dialog, messages and CLI help.
- Recognize Japanese and regional Spanish system languages and switch among four languages immediately.

### 3.7.0

- Add Limited / Full range: archive retains source range and sharing converts to Limited by default.
- Preserve sampling for software HEVC / H.264 archive, use 4:4:4 for RGB and 4:2:0 for AV1 / Quick share, with manual sampling selection.
- Add bit-depth selection within source and encoder limits; sampling/depth can be retained or reduced. Software AVC supports 10-bit SDR.
- Add FLAC audio with per-track lossless-integer checks, retaining sample rates, channels and precision in MKV.
- Fix dynamic-range defaults and conversion choices for manual input interpretation; disable job options before source selection.
- Expand settings cards, match side-by-side card dimensions, group LUT/sampling/range under Color conversion and container/audio/decoding under Format & audio; use a single column in narrow windows. Add matching CLI options.

### 3.6.4

- Move the WinUI frontend to Native AOT and exit the single-file entry after GUI startup.
- Compile main-view XAML, generate localization mappings and JSON serializers, cache control references and disable path/log spelling and text prediction.
- Reduce the measured mean startup private working set from 122.3 MiB to 116.6 MiB; see [Performance](docs/performance.md#english) for methods and results.
- Add memory sampling, settings/JSON compatibility checks and separate AOT / JIT lock files; collect licenses from both application and launcher dependency graphs.

### 3.6.3

- Use “界面语言 / Language” for the first-run language entry and accessibility name.
- Fix popup theme-resource caching and refresh controls when returning to the system accent color.

### 3.6.2

- Remove the full-license link from Tool paths; keep access in About and the first-run dialog.
- Create the appearance panel and ColorPicker on first use.

### 3.6.1

- Add System / 简体中文 / English selection to the first-run dialog, save the choice and synchronize the main interface.
- Provide Chinese/English component descriptions and use the selected language for CLI `license --text`.
- Add a native About dialog with icon, version, development/verification credits, GitHub home and full-license access.
- Add the icon to Chinese/English READMEs and use GitHub release-asset SHA-256 digests.

### 3.6.0

- Add CLI commands probe, convert, plan, presets, profiles, doctor and license, covering GUI encoding/color options, automatic defaults, config files and path overrides.
- Support multiple files, directories, recursive wildcards, JSON Lines progress/logs, fps/ETA, batch skipping/continuation, stable exit codes and Ctrl+C / Ctrl+Break cancellation.
- Use one FrameFlow.exe for GUI and CLI, with PowerShell 7 output pipelines, exit codes and cancellation.
- Encode and verify temporary output before atomic replacement, protect source files and clean up after failure or cancellation.
- Add Chinese/English CLI documentation and terminal/encoding integration checks.

### 3.5.1

- Change original code, documentation, artwork and generated Mi-Log / Samsung Log LUTs from Apache-2.0 to MIT.
- Update Chinese/English READMEs, generator SPDX identifiers, license manifests and in-application text.

### 3.5.0

- Upgrade to .NET SDK 11.0.100 / .NET 11.0.0, Windows App SDK 2.6.5, Windows SDK BuildTools 10.0.28000.2957 and official Python 3.15.2; pin SDK and NuGet dependencies.
- Remove unused AI, ML, Search and Widgets dependencies.
- Replace PyInstaller with official embedded Python in the runtime-included single EXE.
- Offer a framework-dependent ZIP and external Python selection in tool settings.
- Collect third-party licenses, provenance and hashes; add a first-run component-terms dialog and full-text access.
- Add worker integration checks for SDR, 10-bit HLG, three Log LUTs, 4:2:2, audio, progress and cancellation cleanup.

### 3.4.7

- Unify WinUI 3 buttons and group icons; adjust source import and footer actions.
- Fix toolbar-icon clipping; wrap input interpretation and actions in narrow windows.
- Open the language menu downward, matching the control width and right edge; shorten the English system choice to System.

### 3.4.6

- Fix content shifting right, left-side gaps and right-side clipping at certain window widths.
- Stretch the scroll container to the viewport; center wide layouts and switch narrow windows to one column.

### 3.4.5

- Use a separate native popup and opaque theme background for appearance settings, fixing HDR bright bands and delayed closing.

### 3.4.4

- Upgrade Windows App SDK to 2.0.1 and use native SystemBackdropElement Mica in HDR mode.

### 3.4.3

- Use an opaque main background in HDR mode.

### 3.4.2

- Adjust popup shadow rendering.

### 3.4.1

- Group System / Light / Dark choices in the appearance panel.

### 3.4.0

- Add live Chinese/English switching and system-language selection, with English for other system languages.
- Add translation-resource checks.

### 3.3.0

- Update the vector icon and provide PNG, SVG and multi-resolution ICO assets.

### 3.2.0

- Rename the application to 映流 · FrameFlow and update the window, EXE, taskbar and brand assets.

### 3.1.0

- Add independent MP4 / MKV selection and audio/container compatibility checks.
- Show bit depth, sampling and audio in source information, plus additional encoder parameters in preset details.

### 3.0.0

- Move to native WinUI 3 with Mica, themes and the Windows accent color.
- Revise the layout and add encoding fps, progress and remaining time.

### 2.0.0

- Add a PySide6 / Qt GUI, categorized presets, dynamic-range and sampling controls.
- Add automatic source detection and three color-conversion groups.
