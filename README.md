# Verdant 翠绿

Verdant 是歌声编辑器。用钢琴卷帘编写音符、歌词和表达曲线，预听后导出 WAV；工程保存为 `.verp`。桌面使用 GPUI，合成由独立的 Rust worker 执行。

## 用法

```text
verdant                         # 帮助后退出
verdant gui [工程.verp | --import 乐谱文件] [选项]
verdant capabilities --json
verdant skill list --json
```

`verdant` 无参数只显示帮助，不打开或等待 GUI。`verdant gui` 启动桌面；未指定文件时打开本账户的未命名工作副本，或建立示例工程。`verdant gui --import` 创建新的工作副本；在文件菜单查看损失报告，再保存为 `.verp`。

## 安装与启动

发行入口为 [OpenGreendam/verdant Releases](https://github.com/OpenGreendam/verdant/releases)。选择对应平台的归档，核对随附 SHA-256 后解包。已有候选包、正式发行条件与各平台验收分别见[构建与发行](RELEASE.md)和[平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md)。

| 平台 | 入口 |
| --- | --- |
| Linux | 包根目录 `verdant-launcher [工程.verp]`；CLI 用 `bin/verdant` |
| Windows x64 | 双击包根目录 `verdant-launcher.exe`；CLI 用 `bin\verdant.exe` |
| macOS Apple Silicon | 打开 `Verdant.app` 或包根目录 `verdant-launcher` |

软件、worker 和桌面声库共用版本 `0.1.0`。本地声库安装需要原始来源和兼容模型包；安装器检查版本与文件身份，再更新本账户注册表。构建、测试、打包、安装和推理无需 Python。[声库安装与回滚](docs/verdant-voice.md)

桌面和 CLI 默认通过 `verdant-voice` 合成。旧版原始引擎仅供显式选择经 `verdant-voice import-originals` 双 SHA-256 校验登记的原始声库使用。

## 文件与运行

打开、保存和另存为使用 `.verp`。可导入的乐谱格式包括 UFDATA、MIDI、UST、USTX、SVP、VSQX 和 VPR。备份时请同时保留 `.verp` 工程和对应的 `.verp.journal`。编辑器会提示较新的创建版本和待恢复内容。

配置、注册表、缓存和模型各有独立目录。路径、覆盖变量、worker 参数及错误边界见[运行手册](backend/RUNTIME.md)；编辑、读音、播放、导出及桌面退出码见[编辑器手册](frontend/README.md)。

## 命令行与机器人

```sh
verdant capabilities --json
verdant project inspect song.verp --json
verdant render song.verp --output song.wav --events
verdant worker --stdio
verdant skill install --all --agent generic --scope project
```

CLI 与桌面共用工程和音频服务；插件、混音、预设、音频片段和分析均使用可发现的原子操作。`verdant-cli` 保留为兼容别名，`verdant worker --stdio` 是顺序执行的工作流 worker，与独立声库进程 `verdant-worker` 区分。离线技能随包提供，安装和更新保护用户修改。[入口、工作流与技能安装](frontend/docs/AGENT-AUTOMATION.md)。DeepSeek Harness 插件调用 `verdant-cli/1`，在受管会话中交付试听和工程。[命令与协议](spec/CLI-DSH.md) · [插件源码](plugins/dsh/)

已有专用工具分别见 [UST／USTX 导入](frontend/crates/verdant-import/README.md)、[乐谱交换](frontend/crates/verdant-exchange/README.md)和 [worker 协议](backend/README.md)。DeepSeek Harness 插件与音频效果宿主 `verdant-plughost` 分别维护。

## 开发与来源

代码、问题和后续发行使用 [OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。[参与开发](CONTRIBUTING.md)说明补丁约定；[构建与发行](RELEASE.md)维护工具链、依赖和打包命令。

没有审批门禁：改完、验证过，就可以开 PR 并合并。把下面这段交给你的 Agent，它会自己读手册、改代码、跑检查：

```text
在 OpenGreendam/verdant 仓库里工作（没有就 git clone 一份）。先读 README.md、SPEC.md、
docs/README.md 和 RELEASE.md，确认这次改动涉及的模块、手册和构建命令。然后直接实现目标：
<在这里写你想改的东西>。
改完本地验证：cargo check、相关测试各跑一次，再从真实入口（verdant gui、verdant 或 verdant-cli）
跑一个实际工程确认行为。最后提交到新分支，写清动机、执行的命令和结果，开一个 Pull Request。
```

[手册索引](docs/README.md) · [产品规格](SPEC.md) · [开发计划](DEV_PLAN.md) · [待验收事项](TODO.md)

复现使用报告对应的源码版本及原输入，保留原哈希与失败记录。

源码采用 AGPL-3.0-or-later；第三方资料保留各自条款。[许可证](LICENSE) · [归属](NOTICE.md) · [合成来源与验证](SYNTHESIS-FIDELITY.md)
