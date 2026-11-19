# 第三方组件与许可 / Third-party components

[简体中文](#简体中文) · [English](#english)

## 简体中文

项目自主内容采用 [MIT License](../LICENSE)。本页列出第三方组件的许可与来源，完整条款保存在 [licenses](../licenses)。版本、来源和文件哈希见[运行时清单](../licenses/manifest.json)与 [LUT 清单](license-manifest.json)。

### 运行时与 LUT

| 组件 | 许可与分发条款 | 原文 |
| --- | --- | --- |
| .NET 11.0.0、apphost、ASP.NET 运行时 | MIT；Windows `coreclr.dll`、单文件内运行时及指定 WPF 等二进制适用 .NET Library License，个别文件适用 Windows SDK 条款 | [Windows 逐组件说明](https://github.com/dotnet/core/blob/main/license-information-windows.md)、[.NET Library License](../licenses/DotNet-Library-LICENSE.html) |
| Native AOT runtime 11.0.0 | MIT 及组件声明；单文件入口中的 Windows 运行时按上一行条款分发 | [LICENSE](../licenses/microsoft.netcore.app.runtime.nativeaot.win-x64-11.0.0-LICENSE.TXT)、[声明](../licenses/microsoft.netcore.app.runtime.nativeaot.win-x64-11.0.0-THIRD-PARTY-NOTICES.TXT) |
| Windows SDK .NET 投影 10.0.26100.87 | Windows SDK 条款；REDIST 的 `Microsoft.Windows.SDK.NET.Ref` 章节允许随应用分发未修改文件 | [许可](../licenses/Windows-SDK-Net-Ref-LICENSE.rtf)、[REDIST](https://learn.microsoft.com/en-us/legal/windows-sdk/redist#microsoftwindowssdknetref) |
| Windows App SDK / WinUI 2.6.5 | 正式包条款第 3 节，适用于 NuGet 放入应用输出目录的代码 | [许可](../licenses/Microsoft.WindowsAppSDK.Base-2.6.5-license.txt) |
| WebView2 SDK | WinUI 的传递依赖，BSD-3-Clause | [LICENSE](../licenses/Microsoft.Web.WebView2-1.0.3719.77-LICENSE.txt)、[NOTICE](../licenses/Microsoft.Web.WebView2-1.0.3719.77-NOTICE.txt) |
| Pocket 3 LUT | OpenOSV v0.6.4，Apache-2.0 | [LICENSE](OpenOSV/LICENSE)、[NOTICE](OpenOSV/NOTICE) |

完整条款可从首次许可窗口和“关于”打开。重新分发应用时，需保留组件许可与声明，并向接收者提供适用的微软组件条款。

.NET 二进制分发要求见 .NET Library License 第 3 节。

### LUT 来源

- **Pocket 3 D-Log M → HLG**：来自 [OpenOSV v0.6.4](https://github.com/Kemerd/OpenOSV/releases/tag/v0.6.4) 的 `luts/Rec2100_HLG/DJI_Osmo_DLogM_to_Rec2100_HLG_BT2408_Neutral.cube`，是第三方拟合转换，采用 Apache-2.0。原始 [LICENSE](OpenOSV/LICENSE) 和 [NOTICE](OpenOSV/NOTICE) 随文件保留。
- **Mi-Log / Samsung Log → HLG**：由 [generate_hlg_luts.py](../tools/generate_hlg_luts.py) 根据公开数值参数与项目高光映射生成，采用 MIT。技术来源：[小米资料](https://web.vip.miui.com/page/info/mio/mio/detail?fromPathname=mioSingleBoard&postId=47863325)、[Samsung Log 文档](https://developer.samsung.com/mobile/samsung-log-video.html)。

- **Apple Log / Apple Log 2 → HLG**：由同一生成工具独立实现，采用 MIT。Apple Log 使用 BT.2020；Apple Log 2 使用 Apple Wide Gamut，两者共用 Apple Log 传递曲线。数值定义见 [Apple AVFoundation](https://developer.apple.com/documentation/avfoundation/avcapturecolorspace/applelog2) 和 [OpenColorIO Apple transforms](https://opencolorio.readthedocs.io/projects/config-aces/en/latest/_modules/opencolorio_config_aces/clf/transforms/apple/generate.html)。

### 字体

Source Sans 3、思源黑体 CN / JP 和 Source Code Pro 以原始文件随应用分发，采用 SIL OFL 1.1。原始版权、许可和固定版本来源见 [licenses/Fonts](../licenses/Fonts/manifest.json)。

### Ultra HDR 编码

| 组件 | 许可 | 分发内容 |
| --- | --- | --- |
| [libultrahdr 2.0.2](https://github.com/google/libultrahdr/tree/v2.0.2) | MIT、Apache-2.0 | `uhdr.dll`，包含 XMP 与 ISO 21496-1 增益图元数据支持 |
| image_io / modp_b64 | Apache-2.0 / BSD-3-Clause | 随 libultrahdr 编译的内部组件 |
| [libjpeg-turbo 3.2.1](https://github.com/libjpeg-turbo/libjpeg-turbo/tree/3.2.1) | IJG、BSD-3-Clause | 静态链接到 `uhdr.dll` 的 JPEG 编解码器 |
| MSVC STL | Apache-2.0 WITH LLVM-exception | C++ 模板代码；微软运行库按现有运行时条款分发 |

原始许可、版权及 Adobe Gain Map 声明保存在 [licenses/UltraHDR](../licenses/UltraHDR)。源码下载哈希、构建选项和二进制哈希见 [Native/manifest.json](../App/Native/manifest.json)。构建脚本为 [Build-UltraHdr.ps1](../tools/Build-UltraHdr.ps1)。

This product includes Gain Map technology under license by Adobe.

This software is based in part on the work of the Independent JPEG Group.

### 外部工具与资源

| 项目 | 使用方式 |
| --- | --- |
| FFmpeg / ffprobe | 普通包由用户提供，以独立子进程调用；`-full` 包内嵌一版 FFmpeg（GPL-3.0-or-later，版本钉在 `Packaging/ffmpeg-windows.json`），许可证与源码清单随包放在 `Licenses/` 与 `SOURCES.txt`。各构建的许可见 [FFmpeg 法律说明](https://ffmpeg.org/legal.html) |
| PySide6 / Qt | 可选图标生成工具；版本见 [requirements.txt](../Brand/requirements.txt)，许可见 [Qt for Python](https://doc.qt.io/qtforpython-6/licenses.html) |
| SDK / BuildTools | 开发构建工具；随应用输出的运行时文件保留对应许可 |
| 品牌图形 | 项目绘制的 SVG / PNG / ICO，采用 MIT |
| 系统字体 | 通过系统 FontIcon 显示图标 |

## English

Original project work uses the [MIT License](../LICENSE). This page lists third-party licenses and sources; full terms are retained in [licenses](../licenses/). Versions, sources and hashes are in the [runtime manifest](../licenses/manifest.json) and [LUT manifest](license-manifest.json).

### Runtimes and LUTs

| Component | License and distribution terms | Original text |
| --- | --- | --- |
| .NET 11.0.0, apphost, ASP.NET runtime | MIT; Windows `coreclr.dll`, the runtime inside the single-file bundle and designated binaries such as WPF use the .NET Library License; some files use Windows SDK terms | [Windows component guidance](https://github.com/dotnet/core/blob/main/license-information-windows.md), [.NET Library License](../licenses/DotNet-Library-LICENSE.html) |
| Native AOT runtime 11.0.0 | MIT and component notices; the Windows runtime in the single-file entry follows the preceding row | [LICENSE](../licenses/microsoft.netcore.app.runtime.nativeaot.win-x64-11.0.0-LICENSE.TXT), [notices](../licenses/microsoft.netcore.app.runtime.nativeaot.win-x64-11.0.0-THIRD-PARTY-NOTICES.TXT) |
| Windows SDK .NET projection 10.0.26100.87 | Windows SDK terms; the `Microsoft.Windows.SDK.NET.Ref` REDIST section permits distribution of unmodified files with the application | [License](../licenses/Windows-SDK-Net-Ref-LICENSE.rtf), [REDIST](https://learn.microsoft.com/en-us/legal/windows-sdk/redist#microsoftwindowssdknetref) |
| Windows App SDK / WinUI 2.6.5 | Production-package terms, section 3, for code placed in application output by NuGet | [License](../licenses/Microsoft.WindowsAppSDK.Base-2.6.5-license.txt) |
| WebView2 SDK | Transitive WinUI dependency, BSD-3-Clause | [LICENSE](../licenses/Microsoft.Web.WebView2-1.0.3719.77-LICENSE.txt), [NOTICE](../licenses/Microsoft.Web.WebView2-1.0.3719.77-NOTICE.txt) |
| Pocket 3 LUT | OpenOSV v0.6.4, Apache-2.0 | [LICENSE](OpenOSV/LICENSE), [NOTICE](OpenOSV/NOTICE) |

Full terms can be opened from the first-run license dialog and About. Redistribution must retain component licenses and notices and provide recipients with the applicable Microsoft component terms.

Section 3 of the .NET Library License sets out .NET binary redistribution requirements.

### LUT sources

- **Pocket 3 D-Log M → HLG**: the unmodified `luts/Rec2100_HLG/DJI_Osmo_DLogM_to_Rec2100_HLG_BT2408_Neutral.cube` from [OpenOSV v0.6.4](https://github.com/Kemerd/OpenOSV/releases/tag/v0.6.4), a third-party fitted transform under Apache-2.0. Its original [LICENSE](OpenOSV/LICENSE) and [NOTICE](OpenOSV/NOTICE) are retained.
- **Mi-Log / Samsung Log → HLG**: generated by [generate_hlg_luts.py](../tools/generate_hlg_luts.py) from public numeric definitions and the project's highlight mapping, under MIT. Technical sources: [Xiaomi material](https://web.vip.miui.com/page/info/mio/mio/detail?fromPathname=mioSingleBoard&postId=47863325), [Samsung Log documentation](https://developer.samsung.com/mobile/samsung-log-video.html).
- **Apple Log / Apple Log 2 → HLG**: independently implemented by the same generator under MIT. Apple Log uses BT.2020; Apple Log 2 uses Apple Wide Gamut, with a shared Apple Log transfer curve. Numeric definitions are linked from [Apple AVFoundation](https://developer.apple.com/documentation/avfoundation/avcapturecolorspace/applelog2) and [OpenColorIO Apple transforms](https://opencolorio.readthedocs.io/projects/config-aces/en/latest/_modules/opencolorio_config_aces/clf/transforms/apple/generate.html).

### Fonts

Source Sans 3, Source Han Sans CN / JP and Source Code Pro ship as original font files under SIL OFL 1.1. Original copyrights, license texts and pinned upstream sources are in [licenses/Fonts](../licenses/Fonts/manifest.json).

### Ultra HDR encoding

| Component | License | Distributed content |
| --- | --- | --- |
| [libultrahdr 2.0.2](https://github.com/google/libultrahdr/tree/v2.0.2) | MIT, Apache-2.0 | `uhdr.dll`, with XMP and ISO 21496-1 gain-map metadata |
| image_io / modp_b64 | Apache-2.0 / BSD-3-Clause | Internal components compiled with libultrahdr |
| [libjpeg-turbo 3.2.1](https://github.com/libjpeg-turbo/libjpeg-turbo/tree/3.2.1) | IJG, BSD-3-Clause | JPEG codec statically linked into `uhdr.dll` |
| MSVC STL | Apache-2.0 WITH LLVM-exception | C++ template code; Microsoft runtime libraries follow the existing runtime terms |

Original licenses, copyrights and the Adobe Gain Map notice are in [licenses/UltraHDR](../licenses/UltraHDR/). Source archive hashes, build options and binary hashes are in [Native/manifest.json](../App/Native/manifest.json). The build script is [Build-UltraHdr.ps1](../tools/Build-UltraHdr.ps1).

This product includes Gain Map technology under license by Adobe.

This software is based in part on the work of the Independent JPEG Group.

### External tools and resources

| Item | Use |
| --- | --- |
| FFmpeg / ffprobe | A plain package expects user-supplied tools invoked as separate processes; a `-full` package embeds an FFmpeg build (GPL-3.0-or-later, pinned in `Packaging/ffmpeg-windows.json`) with its licence and source list in `Licenses/` and `SOURCES.txt`. Licensing depends on the build, as described in [FFmpeg legal information](https://ffmpeg.org/legal.html) |
| PySide6 / Qt | Optional icon-generation tools; versions are in [requirements.txt](../Brand/requirements.txt), licensing in [Qt for Python](https://doc.qt.io/qtforpython-6/licenses.html) |
| SDK / BuildTools | Development tools; runtime files copied into application output retain their applicable licenses |
| Brand graphics | Project-created SVG / PNG / ICO artwork under MIT |
| System fonts | FontIcon uses system fonts for icons |
