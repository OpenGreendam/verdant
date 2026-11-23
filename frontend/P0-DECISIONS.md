# 前端设计决定

本文件维护当前依赖针和所有权边界。早期 SPEC 审阅保留在 [spec](../spec/README.md)，运行状态见[平台验收](docs/PLATFORM-ACCEPTANCE.md)。

## 依赖与生成

- GPUI 与 `gpui_platform` 锁定到 [cyjin-yl/z3rm](https://github.com/cyjin-yl/z3rm/tree/e5f2f9db75e97613232b5c1946333e61fa3828ab) 的 `e5f2f9db75e97613232b5c1946333e61fa3828ab`，见 [Cargo.toml](Cargo.toml) 与 [Cargo.lock](Cargo.lock)。它继承最初 Zed `48741fd` 审阅基线并加入 Wayland 触摸支持；不能将原始审阅当成后续提交的验收。
- Linux 显式启用 `font-kit`、`x11`、`wayland`。缺少窗口后端特性会进入 headless，不能用进程存活判断窗口成功。
- 编译工具链由 [rust-toolchain.toml](rust-toolchain.toml) 固定。Cargo 工作区版本为 0.1.0；编辑器、worker、后端及 `verdant-voice` 共用版本。
- 控件与命令表保留 `frontend/crates/verdant-app/src/generated/controls.rs` 的检入源码。`frontend/proto-ui/` 将 pitch slot、note/control planner 和有状态旋钮分为四个编译单元：两个原生组件经上游 `nativeSdkPath` 共用一个 SDK，两个 planner 经 `rust-data-v1` 生成无 Cargo 依赖的稳定 Rust crate。Rust 会话通过类型化 `verdant-ui-protocol` 调用编译出的决策函数，处理守卫、capture 与领域副作用；旋钮实例由原型持有值、phase、取消、键盘/焦点和 Slider 辅助功能语义，宿主提供 Adwaita 绘图与经 revision 校验的提交。见 [集成边界与检查命令](proto-ui/README.md) 和 [compiler.pin.json](proto-ui/compiler.pin.json)。

## 状态与进程

工程格式为 `greendam-project` v0.1，原生扩展名为 `.verp`，未知字段保留。所有持久编辑经过 `ProjectStore`：一个命令批对应一个 revision 和撤销项；草稿与取消不改工程。导入创建独立工作副本，创建版本与恢复规则见[用户手册](README.md#工程与恢复)。

GPUI 负责窗口、输入和视图；Rust worker 负责模型准入与合成；音频引擎负责混音和设备时钟。后台消息按任务、revision 和内容身份匹配。未知能力视为不支持，失败不改用其他合成路线。播放与导出共用引擎处理和已校验产物。

## 复用边界

工程领域不通过旧 Proto UI wire 协议运行。pitch/旋钮使用 pinned native SDK 的 MoveGesture lease；note/control planner 是独立类型化纯数据函数，不能称整个 DAW 的输入、状态和辅助功能都由原型统一承载。`verdant-ui-protocol` 只依赖两个标准库 data crate；session 的完整依赖图排除 GPUI、native SDK 和原生组件，并在未经扩展的 Rust 基础镜像中独立构建/测试。宿主能力逐项记录在平台验收收据中。规范示例中的 `not_executed` 保持原义。[平台验收](docs/PLATFORM-ACCEPTANCE.md)

读音、音素展开和连续 F0 按真实 worker 能力生成，不再受早期仅元音代理的限制。日语跨语言近似和未恢复的原引擎行为仍须明确展示，见[多语言合同](../spec/MULTILINGUAL-PHONEMES.md)。

## 证据

领域／协议测试、静态编译、原生窗口、真实模型和物理设备分别证明各自范围。Linux 的本地真实模型运行不能代替 Windows/macOS 验收；静音时钟不能代替声卡。平台 IME、完整读屏、物理触摸、音频热插拔及 AU 的剩余范围集中记录于[平台验收](docs/PLATFORM-ACCEPTANCE.md)。
