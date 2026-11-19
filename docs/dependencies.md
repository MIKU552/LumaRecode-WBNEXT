# 开发与分发 / Development and distribution

[简体中文](#简体中文) · [English](#english)

## 简体中文

### 开发环境

| 组件 | 版本 | 用途 |
| --- | --- | --- |
| .NET SDK | 11.0.100 | 由 `global.json` 固定 |
| .NET runtime | 11.0.0 | 应用、转换核心与 JIT |
| Windows App SDK | 2.6.5 | WinUI 与 Windows App Runtime |
| Windows SDK BuildTools | 10.0.28000.2957 | Windows 资源与清单 |
| Windows API target | 10.0.26100.0 | Windows API 编译合约 |
| Visual Studio | 2026 18.12+ | 解决方案、调试与打包 |
| MSVC linker | 14.52.37029+ | 可选 Native AOT 构建 |
| libultrahdr / libjpeg-turbo | 2.0.2 / 3.2.1 | Ultra HDR JPEG 编码 |
| PySide6 | 6.12.0 | 可选图标生成工具 |

默认 JIT 构建需要 .NET SDK、PowerShell 7，以及 WinUI 构建所需的 Windows 工具：

```powershell
./build.ps1
```

可选 AOT 构建需要 [Visual Studio C++ 工具链](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/)。自行配置原生工具 `PATH` 和 `LIB` 时可加 `-UseEnvironmentalNativeTools`。

NuGet 按架构和编译方式使用锁文件：JIT 使用 `packages.lock.json` 或 `packages.arm64.lock.json`，AOT 使用 `packages.aot.lock.json` 或 `packages.arm64.aot.lock.json`。运行时支持见 [.NET 支持周期](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core)和 [Windows App SDK 发布渠道](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels)。

### 编译目标

| 命令 | 编译方式 | 输出目录 |
| --- | --- | --- |
| `./build.ps1` / `-Target Native` | JIT，自动选择 X64 或 ARM64 | `dist/SelfContained/` |
| `./build.ps1 -Target X64` | JIT，`win-x64` | `dist/SelfContained-X64/` |
| `./build.ps1 -Target Arm64` | JIT，`win-arm64` | `dist/SelfContained-Arm64/` |
| `./build.ps1 -Compilation Aot` | Native AOT，本机指令集 | `dist/Aot-SelfContained/` |
| `./build.ps1 -Target X64 -Compilation Aot` | Native AOT，x86-64-v2 | `dist/Aot-SelfContained-X64/` |
| `./build.ps1 -Target Apx -Compilation Aot` | Native AOT，APX | `dist/Aot-SelfContained-Apx/` |
| `./build.ps1 -Target Arm64 -Compilation Aot` | Native AOT，ARMv8-A | `dist/Aot-SelfContained-Arm64/` |

每个目录生成一个 `LumaRecode.exe`。正式发布提供 X64 和 ARM64 两种 JIT 单文件。Native 根据 Windows 的实际系统架构选择目标，包括仿真终端下的系统架构。

JIT 在运行设备上选择指令集。x64 的 CPU 与 Windows 均支持 APX，且 `DOTNET_EnableAPX` / `COMPlus_EnableAPX` 未显式设置时，启动阶段以子进程环境启用 APX 后重新进入应用。ARM64 使用运行时的特性检测和指令选择。

AOT 的 Native 目标按构建 CPU 优化，需要与系统架构相同的 .NET SDK。显式 X64、Apx、Arm64 使用固定指令集，支持交叉编译。APX AOT 入口在加载主体前检查 CPU 和 Windows 支持。ARM64 默认选择软件归档。

JIT 交叉发布无需 C++ 链接器；AOT 交叉编译需要对应架构的 [C++ 构建工具](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/cross-compile)。中间目录为 `obj/<Compilation>/<Target>/` 和 `bin/<Compilation>/<Target>/`，分发许可清单位于 `build-cache/licenses/<Compilation>/<Target>/`，AOT 载荷位于 `dist/NativeAot/<Target>/`。

### 部署方式

| 命令 | 产物 | 运行时 |
| --- | --- | --- |
| `./build.ps1` | JIT 单文件 EXE | 内置所需运行时 |
| `./build.ps1 -Compilation Aot` | AOT 单文件 EXE | 内置所需运行组件 |
| `./build.ps1 -Deployment FrameworkDependent` | JIT ZIP | 用户安装 .NET 11 Runtime、Windows App Runtime 2.6 |

`-Target` 可与部署选项组合；Apx 使用 SelfContained / Aot。发布包有两种形态：普通包要求用户提供 FFmpeg 与 ffprobe；`-full` 包内嵌 FFmpeg（Windows 取 BtbN 的 GPL 构建并按仓库内的 SHA-256 校验，macOS 用 `Packaging/relocate-brew-tools.py` 重定位 Homebrew 构建及其依赖库）。

Ultra HDR 原生库按架构放置在 `App/Native/x64/uhdr.dll` 与 `App/Native/arm64/uhdr.dll`，首次导出时加载。重新编译需要 MSVC、CMake 和 Ninja；x64 还需要 NASM：

```powershell
./tools/Build-UltraHdr.ps1 -Architecture x64
./tools/Build-UltraHdr.ps1 -Architecture arm64
```

ARM64 使用 NEON，x64 使用 libjpeg-turbo 的 x86 SIMD。源码版本、构建配置和二进制哈希见[原生组件清单](../App/Native/manifest.json)。

### Visual Studio

使用 Visual Studio 2026 18.12 或更新版本，安装 WinUI / C# 桌面开发工具、.NET SDK 11.0.100，并将 PowerShell 7 的 `pwsh` 加入 `PATH`。AOT 另需对应架构的 C++ 构建工具。

打开 `LumaRecode.sln` 或 `LumaRecode.slnx`，将 `LumaRecode.App` 设为启动项目。Debug / x64 和 Debug / ARM64 用于调试。

| 解决方案配置 | 打包目标 |
| --- | --- |
| Release / Native | JIT，自动选择系统架构 |
| Release / x64 | JIT X64 |
| Release / ARM64 | JIT ARM64 |
| Release-AOT / Native | AOT，本机构建 CPU |
| Release-AOT / x64 | AOT X64 |
| Release-AOT / ARM64 | AOT ARM64 |
| Release-APX / x64 | AOT APX |

“生成解决方案”通过 `Packaging` 调用 `build.ps1`，完成依赖还原、许可收集与单文件发布。也可单独生成 `Packaging` 项目。

```powershell
dotnet build LumaRecode.sln -c Release -p:Platform=Native
dotnet build LumaRecode.sln -c Release -p:Platform=x64
dotnet build LumaRecode.sln -c Release -p:Platform=ARM64
dotnet build LumaRecode.sln -c Release-AOT -p:Platform=x64
dotnet build LumaRecode.sln -c Release-APX -p:Platform=x64
```

主视图位于 `App/MainView.xaml`。修改翻译后运行 `python tools/Update-ViewText.py` 更新控件映射。提交前运行 `./tools/Check-Source.ps1`；Python 工具可使用 `ruff check qa tools Brand` 和 `ruff format --check qa tools Brand`。检查命令见 [QA 指南](../qa/README.md)，模块职责见[代码结构](architecture.md)，图标生成见 [Brand/README.md](../Brand/README.md)。

### 字体

界面使用 Source Sans 3、思源黑体中文与日文字体；日志使用 Source Code Pro，并回退到对应的思源黑体。字体随程序提供。

字体位于 `App/Assets/Fonts/`，版权和 SIL OFL 1.1 原文位于 `licenses/Fonts/`。提交、下载地址和 SHA-256 固定在[字体来源清单](../licenses/Fonts/manifest.json)。构建时校验字体与许可文件的哈希。

### 单文件结构

JIT 默认直接打包应用、中间代码、.NET 运行时、Windows App SDK、字体、LUT 与许可文件。WinUI 资源和运行依赖提取到临时缓存后，GUI 主体在当前进程中运行。CLI 入口通过同一 EXE 启动工作子进程，保留管道取消后的清理。

可选 AOT 构建通过 `Bundle/` 打包 AOT 主程序和依赖。GUI 入口启动主体后退出；CLI 入口保留标准句柄和工作目录，等待任务结束并返回退出码。APX JIT 启动引导也采用同样的句柄与退出码传递。

单文件使用 `IncludeAllContentForSelfExtract` 保留磁盘文件兼容性。缓存与进程的性能测量见[性能优化](performance.md)，命令行说明见 [CLI 手册](cli/zh-CN.md)。

### 依赖收集

1. 按锁文件还原当前架构与编译方式的依赖。
2. 收集应用依赖的许可、声明和运行时记录；AOT 另合并 Bundle 入口的依赖图。
3. 校验字体、LUT、原生图片编解码库和许可哈希。
4. 随单文件发布完整组件许可与来源清单。

项目许可见 [LICENSE](../LICENSE)、[NOTICE](../NOTICE)和[第三方组件](../Assets/THIRD_PARTY.md)。

## English

### Development environment

| Component | Version | Purpose |
| --- | --- | --- |
| .NET SDK | 11.0.100 | Pinned by `global.json` |
| .NET runtime | 11.0.0 | Application, conversion core and JIT |
| Windows App SDK | 2.6.5 | WinUI and Windows App Runtime |
| Windows SDK BuildTools | 10.0.28000.2957 | Windows resources and manifests |
| Windows API target | 10.0.26100.0 | Windows API contracts |
| Visual Studio | 2026 18.12+ | Solution, debugging and packaging |
| MSVC linker | 14.52.37029+ | Optional Native AOT builds |
| libultrahdr / libjpeg-turbo | 2.0.2 / 3.2.1 | Ultra HDR JPEG encoding |
| PySide6 | 6.12.0 | Optional icon-generation tool |

Default JIT builds require the .NET SDK, PowerShell 7 and Windows tools for WinUI compilation:

```powershell
./build.ps1
```

Optional AOT builds require the [Visual Studio C++ toolchain](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/). Add `-UseEnvironmentalNativeTools` when configuring the native tools through `PATH` and `LIB`.

NuGet lock files cover each architecture and compilation mode. JIT uses `packages.lock.json` or `packages.arm64.lock.json`; AOT uses `packages.aot.lock.json` or `packages.arm64.aot.lock.json`. Runtime support is listed in the [.NET support policy](https://dotnet.microsoft.com/en-us/platform/support/policy/dotnet-core) and [Windows App SDK release channels](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/release-channels).

### Build targets

| Command | Compilation | Output directory |
| --- | --- | --- |
| `./build.ps1` / `-Target Native` | JIT, automatic X64 or ARM64 | `dist/SelfContained/` |
| `./build.ps1 -Target X64` | JIT, `win-x64` | `dist/SelfContained-X64/` |
| `./build.ps1 -Target Arm64` | JIT, `win-arm64` | `dist/SelfContained-Arm64/` |
| `./build.ps1 -Compilation Aot` | Native AOT, build CPU instructions | `dist/Aot-SelfContained/` |
| `./build.ps1 -Target X64 -Compilation Aot` | Native AOT, x86-64-v2 | `dist/Aot-SelfContained-X64/` |
| `./build.ps1 -Target Apx -Compilation Aot` | Native AOT, APX | `dist/Aot-SelfContained-Apx/` |
| `./build.ps1 -Target Arm64 -Compilation Aot` | Native AOT, ARMv8-A | `dist/Aot-SelfContained-Arm64/` |

Each directory contains one `LumaRecode.exe`. Releases provide JIT single EXEs for X64 and ARM64. Native reads the real Windows system architecture, including from emulated terminals.

The JIT selects instructions on the running device. When the x64 CPU and Windows support APX and neither `DOTNET_EnableAPX` nor `COMPlus_EnableAPX` is explicitly set, startup restarts the application with APX enabled in the child environment. ARM64 uses runtime feature detection and instruction selection.

AOT Native targets the build CPU and requires a .NET SDK matching the system architecture. Explicit X64, Apx and Arm64 targets use fixed instruction profiles and support cross-compilation. The APX AOT entry checks CPU and Windows support before loading the application. ARM64 defaults to Software archive.

JIT cross-publishing does not need a C++ linker. AOT cross-compilation requires the corresponding [C++ build tools](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/cross-compile). Intermediate files use `obj/<Compilation>/<Target>/` and `bin/<Compilation>/<Target>/`. Distribution licenses use `build-cache/licenses/<Compilation>/<Target>/`; AOT payloads use `dist/NativeAot/<Target>/`.

### Deployment options

| Command | Artifact | Runtime |
| --- | --- | --- |
| `./build.ps1` | JIT single EXE | Required runtimes included |
| `./build.ps1 -Compilation Aot` | AOT single EXE | Required runtime components included |
| `./build.ps1 -Deployment FrameworkDependent` | JIT ZIP | Install .NET 11 Runtime and Windows App Runtime 2.6 |

Combine `-Target` with deployment options. Apx uses SelfContained / Aot. Packages come in two shapes: a plain one asks the user for FFmpeg and ffprobe, while a `-full` one embeds them (Windows takes BtbN's GPL build and checks it against the SHA-256 recorded in the repository; macOS relocates the Homebrew build and its libraries with `Packaging/relocate-brew-tools.py`).

Ultra HDR native libraries are stored at `App/Native/x64/uhdr.dll` and `App/Native/arm64/uhdr.dll` and loaded on first export. Rebuilding requires MSVC, CMake and Ninja; x64 also needs NASM:

```powershell
./tools/Build-UltraHdr.ps1 -Architecture x64
./tools/Build-UltraHdr.ps1 -Architecture arm64
```

ARM64 uses NEON; x64 uses libjpeg-turbo x86 SIMD. Source versions, build settings and binary hashes are recorded in the [native-component manifest](../App/Native/manifest.json).

### Visual Studio

Use Visual Studio 2026 18.12 or later with WinUI / C# desktop tools and .NET SDK 11.0.100. Add PowerShell 7's `pwsh` to `PATH`. AOT also requires C++ build tools for the target architecture.

Open `LumaRecode.sln` or `LumaRecode.slnx` and set `LumaRecode.App` as the startup project. Debug / x64 and Debug / ARM64 support debugging.

| Solution configuration | Packaging target |
| --- | --- |
| Release / Native | JIT, automatic system architecture |
| Release / x64 | JIT X64 |
| Release / ARM64 | JIT ARM64 |
| Release-AOT / Native | AOT, build CPU |
| Release-AOT / x64 | AOT X64 |
| Release-AOT / ARM64 | AOT ARM64 |
| Release-APX / x64 | AOT APX |

Build Solution runs `build.ps1` through `Packaging` to restore dependencies, collect licenses and publish the single EXE. You can also build `Packaging` separately.

```powershell
dotnet build LumaRecode.sln -c Release -p:Platform=Native
dotnet build LumaRecode.sln -c Release -p:Platform=x64
dotnet build LumaRecode.sln -c Release -p:Platform=ARM64
dotnet build LumaRecode.sln -c Release-AOT -p:Platform=x64
dotnet build LumaRecode.sln -c Release-APX -p:Platform=x64
```

The main view is `App/MainView.xaml`. Run `python tools/Update-ViewText.py` after translation edits. Run `./tools/Check-Source.ps1` before committing; Python tools support `ruff check qa tools Brand` and `ruff format --check qa tools Brand`. See the [QA guide](../qa/README.md), [code structure](architecture.md) and [icon-generation guide](../Brand/README.md).

### Fonts

The interface uses Source Sans 3 and Source Han Sans for Chinese and Japanese. Logs use Source Code Pro with the corresponding Source Han Sans fallback. Fonts are included with the application.

Fonts are in `App/Assets/Fonts/`; copyright and SIL OFL 1.1 texts are in `licenses/Fonts/`. Commits, download URLs and SHA-256 values are pinned in the [font manifest](../licenses/Fonts/manifest.json). Builds verify font and license hashes.

### Single-file layout

JIT publication directly bundles the application, IL, .NET runtime, Windows App SDK, fonts, LUTs and licenses. WinUI resources and runtime dependencies are extracted to a temporary cache before the GUI runs in the current process. The CLI entry starts a worker through the same EXE so pipeline cancellation can finish cleanup.

Optional AOT publication uses `Bundle/` to package the AOT application and dependencies. The GUI entry exits after starting the application. The CLI entry preserves standard handles and the working directory, waits for completion and returns the exit code. APX JIT startup uses the same handle and exit-code forwarding.

Single-file publication uses `IncludeAllContentForSelfExtract` for disk-file compatibility. Cache and process measurements are described in [Performance](performance.md); commands are in the [CLI guide](cli/en.md).

### Dependency collection

1. Restore locked dependencies for the current architecture and compilation mode.
2. Collect application licenses, notices and runtime records; AOT also merges the Bundle entry's dependency graph.
3. Verify font, LUT, native image-codec and license hashes.
4. Include complete component licenses and provenance with the single EXE.

See [LICENSE](../LICENSE), [NOTICE](../NOTICE) and [third-party components](../Assets/THIRD_PARTY.md).
