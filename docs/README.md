# 手册

本目录维护当前使用说明与接口来源。命令语法中的 `[]` 表示可选，`FILE`／`DIR` 为需替换的路径；相对路径按相应命令的目录规则解析。

## 使用

| 手册 | 内容 |
| --- | --- |
| [项目入口](../README.md) | 安装入口、工程和产品范围。 |
| [桌面编辑器](../frontend/README.md) | 启动、编辑、读音、播放、导出、恢复、文件和退出码。 |
| [声库](verdant-voice.md) | 本地输入、预检、安装、默认选择与回滚。 |
| [运行配置](../backend/RUNTIME.md) | 平台目录、资源、worker 参数、路由与产物。 |
| [worker 协议](../backend/README.md) | `ogd-worker/1` 请求、能力、任务终态与取消。 |
| [效果器宿主](../frontend/docs/PLUGIN-HOST.md) | CLAP／VST3／AU、扫描、状态、离线与窗口限制。 |

CLI 的命令、协议、工程交付与宿主验收见命令手册。独立的格式工具保留原入口。[CLI／DSH](../spec/CLI-DSH.md) · [UST／USTX](../frontend/crates/verdant-import/README.md) · [乐谱交换](../frontend/crates/verdant-exchange/README.md)

## 开发与验收

[参与开发](../CONTRIBUTING.md) · [构建与发行](../RELEASE.md) · [产品规格](../SPEC.md) · [Compiler](../spec/README.md) · [开发计划](../DEV_PLAN.md) · [待办](../TODO.md)

[平台验收](../frontend/docs/PLATFORM-ACCEPTANCE.md)列出目标机义务；[合成来源与验证](../SYNTHESIS-FIDELITY.md)规定数值、声音和来源的判断边界。[DSH 上游来源](research/cli-dsh-research-20261120.md)固定接口审阅版本；源码审阅不能代替真实宿主或通道验收。

## 复现

源码、问题和发行见 [OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。

重放须使用报告对应的源码、输入和环境，不修改冻结字段或重标历史结果。保留在源码中的证据供准入、测试或明确工具消费者使用；它们的原始哈希与来源范围保持不变。模型、录音、原谱及完整私有输入依各自权利保留在本地。[分发政策](../RELEASE.md) · [归属](../NOTICE.md)
