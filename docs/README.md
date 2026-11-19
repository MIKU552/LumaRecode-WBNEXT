# 文档 / Documentation

[简体中文](#简体中文) · [English](#english)

## 简体中文

### 使用指南

| 指南 | 简体中文 | English | 日本語 | Español |
| --- | --- | --- | --- | --- |
| 项目与安装 | [README](../README.md) | [README](readme/en.md) | [README](readme/ja.md) | [README](readme/es.md) |
| 命令行（含 macOS 入口） | [CLI](cli/zh-CN.md) | [CLI](cli/en.md) | [CLI](cli/ja.md) | [CLI](cli/es.md) |

### 专题与开发

以下文档均提供完整的中文和英文部分，可从页面顶部切换。

| 文档 | 内容 |
| --- | --- |
| [色彩转换](color-conversion.md) | 相机曲线、动态 HDR 与 RAW 显影 |
| [开发与分发](dependencies.md) | 构建、依赖、字体、Visual Studio 与单文件打包 |
| [代码结构](architecture.md) | 模块、任务生命周期、线程与缓存 |
| [性能优化](performance.md) | 预览、内存与运行开销测量 |
| [QSV 兼容性](qsv-compatibility.md) | 硬件处理策略与上游报告 |
| [重构实施记录](refactor/README.md) | 共享核心、C ABI、迁移台账与阶段进度 |
| [编码预设测试](benchmarks/encoding-benchmark.md) | 编码参数、画质、体积与转码速度 |
| [测试数据](benchmarks/data/README.md) | 测量环境、参数、字段与原始结果 |
| [开发检查](../qa/README.md) | 自动检查、集成测试与设备核对 |
| [图标资源](../Brand/README.md) | 图标文件与生成命令 |
| [第三方组件](../Assets/THIRD_PARTY.md) | 许可、分发依据与资源来源 |
| [更新记录](../CHANGELOG.md) | 各版本变更 |

第三方许可原文与来源清单位于 [`licenses/`](../licenses/)。

### 文档目录

```text
README.md                         简体中文项目首页
docs/readme/{en,ja,es}.md          英语、日语、西语项目首页
docs/cli/{zh-CN,en,ja,es}.md       四种语言的命令行指南
docs/*.md                         中英双语专题与开发指南
docs/benchmarks/                   中英双语测试报告与原始数据
qa/README.md                      中英双语开发检查指南
Brand/README.md                   中英双语图标说明
Assets/THIRD_PARTY.md              中英双语第三方组件说明
CHANGELOG.md                      中英双语更新记录
```

## English

### User guides

| Guide | 简体中文 | English | 日本語 | Español |
| --- | --- | --- | --- | --- |
| Project and setup | [README](../README.md) | [README](readme/en.md) | [README](readme/ja.md) | [README](readme/es.md) |
| Command line | [CLI](cli/zh-CN.md) | [CLI](cli/en.md) | [CLI](cli/ja.md) | [CLI](cli/es.md) |

### Topics and development

Each page below provides complete Chinese and English sections, selectable at the top.

| Document | Contents |
| --- | --- |
| [Color conversion](color-conversion.md#english) | Camera curves, dynamic HDR and RAW development |
| [Development and distribution](dependencies.md#english) | Builds, dependencies, fonts, Visual Studio and single-file packaging |
| [Architecture](architecture.md#english) | Modules, job lifetimes, threads and caches |
| [Performance](performance.md#english) | Preview, memory and runtime measurements |
| [QSV compatibility](qsv-compatibility.md#english) | Hardware policies and upstream reports |
| [Refactor record](refactor/README.md#english) | Shared core, C ABI, migration ledger and phase status |
| [Encoding preset benchmark](benchmarks/encoding-benchmark.md#english) | Encoding parameters, quality, size and throughput |
| [Benchmark data](benchmarks/data/README.md#english) | Environment, parameters, fields and raw results |
| [Development checks](../qa/README.md#english) | Automated checks, integration tests and device verification |
| [Icon assets](../Brand/README.md#english) | Artwork files and generation commands |
| [Third-party components](../Assets/THIRD_PARTY.md#english) | Licenses, distribution terms and resource provenance |
| [Changelog](../CHANGELOG.md#english) | Changes by version |

Original third-party license texts and provenance are in [`licenses/`](../licenses/).

### Documentation layout

```text
README.md                         Simplified Chinese project home
docs/readme/{en,ja,es}.md          English, Japanese and Spanish project home
docs/cli/{zh-CN,en,ja,es}.md       Command-line guides in four languages
docs/*.md                         Chinese/English topic and development guides
docs/benchmarks/                   Chinese/English benchmark report and raw data
qa/README.md                      Chinese/English development checks
Brand/README.md                   Chinese/English icon documentation
Assets/THIRD_PARTY.md              Chinese/English third-party component guide
CHANGELOG.md                      Chinese/English changelog
```
