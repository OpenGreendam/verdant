# 开发计划

本文件维护实现顺序和完成条件；具体待验收项目见 [TODO.md](TODO.md)，用户命令见[手册索引](docs/README.md)。

## 共享服务与 CLI

按照 [CLI／DSH 规格](spec/CLI-DSH.md)完成桌面可持久化操作到公开机器接口的映射。桌面与 CLI 共用领域事务、格式库、读音规划、worker 调度、制品验证及混音导出；默认 CLI 无需显示、声卡或 MIDI 设备。

交付次序为共享服务、工程和领域操作、七格式交换、真实合成与导出、任务与恢复、声库及环境诊断。每个操作公布 schema、默认值、文件副作用和错误；只读操作不创建恢复日志，写操作遵守锁和 revision 检查。以完整能力矩阵与同工程桌面对照决定是否完成。

## DeepSeek Harness

插件只映射公开 CLI、宿主 jobs、附件与通道交付。固定受测宿主及通道版本，分别记录渲染成功、文件排队、平台发送结果和未知结果。交付试听、可编辑 `.verp` 及允许迁移的素材，在另一台桌面继续编辑再渲染。[接口与验收矩阵](spec/CLI-DSH.md) · [固定上游来源](docs/research/cli-dsh-research-20261120.md)

## Rust 工具与源码边界

普通生产运行及安装、发行工具采用 Rust。运行数据随所属模块维护，源码引用和打包清单均指向该模块。测试夹具、第三方通知和固定来源身份独立保留。

数值重放使用对应报告记录的源码与输入。来源和复现义务见 [SYNTHESIS-FIDELITY.md](SYNTHESIS-FIDELITY.md)。

## 发行与平台

软件、worker 与声库共用 workspace 版本。每个平台用待发行归档检查安装、首启、工程、真实声音、声库准入与回滚；Windows x64、Apple Silicon 和 Linux 的结果分别登记。正式发行还须具备完整声库的分发依据及干净环境断网首启结果。[发行政策](RELEASE.md) · [平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md)

读音、衔接、表达和音域按固定源输入及独立对照继续核验。IME、读屏、触摸、设备恢复及插件原生窗口按目标机观测判断。手势改动保留 Compiler 源、生成物和实际状态转换。[产品规格](SPEC.md) · [Compiler 规格](spec/README.md)

工作在 [OpenGreendam/verdant Issues](https://github.com/OpenGreendam/verdant/issues)跟踪。本文中的交付条件不预填为已通过。
