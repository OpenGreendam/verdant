# Verdant 控件的 Proto-UI Compiler 迁移

本记录描述本分支对原审计 **18 类交互**的实际源码迁移。应用使用 **12 个 `gpui-source-v1` 原生编译单元和 2 个 `rust-data-v1` 决策单元，共 14 个编译单元**。这两个数字分别统计交互类别和编译入口；不能换算成界面面积、代码量或功能完成百分比。

采用率不是缺陷率；一个共享控件接入多个界面，也不等于其所有平台和输入设备已经验收。本轮的源码、编译产物、依赖图和原生框架行为验证已完成；最终 [compiler.pin.json](compiler.pin.json) 与 [receipts](receipts/) 一致绑定 14 个单元、81 个编译产物和完整来源闭包。执行范围、不可变来源及历史失败的处理见文末和 [最终机器记录](VALIDATION-2026-11-23.json)。

## 编译入口

原生单元是 `pitch-gesture-surface`、`normalized-knob`、`action-control`、`collection-control`、`continuous-control`、`editor-surface`、`performance-key`、`drag-source`、`overlay-control`、`text-field`、`graph-point` 和 `scroll-region`。每个入口均在 [src/](src/)，职责和主要调用点见 [集成手册](README.md#当前边界)。`pitch-gesture-surface` 发射唯一的共享 SDK；另外 11 个原生单元引用该 SDK。

`note-gesture-planner` 和 `control-gesture-planner` 是两个无 Cargo 依赖的 Rust data 单元。它们继续向 `verdant-ui-protocol` / session 提供强类型决策函数；headless 路径不创建原生组件 owner、不链接 GPUI，也不通过 JSON 调用原生 runtime。保留这两个纯决策单元不改变交互控件由有状态原生编译组件消费的事实。

## 18 类交互的实际拥有者

| 原审计中的类别 | 编译原型及实际宿主路径 | 留在宿主的内容 |
|---|---|---|
| 钢琴卷帘编辑 | `PitchGestureSurface`、`EditorSurface`、纯 `note-gesture-planner`；`proto_roll.rs`、`roll.rs`、session | 音乐坐标、命中事实、领域守卫、音符事务与绘图 |
| 设备参数旋钮 | `NormalizedKnob`；`proto_knob.rs`、`devices.rs` | 单位/归一化换算、参数目标校验、项目提交与绘图 |
| 设备图形把手 | 每个 `GraphPoint`；`devices/graph_control.rs` | 图表几何、参数映射、两个参数的原子领域写入 |
| 动作按钮 | `ActionControl`；`proto_controls.rs` 与应用按钮调用点 | 被接受的动作对应哪个应用命令 |
| 布尔与离散选择 | `ActionControl` / `CollectionControl`；`ui_controls.rs`、导出/设备表单 | 当前业务值与候选项 |
| Tabs | `ActionControl` 的选择/遍历协议；设备信号页、主视图与公共 Tab 帮助器 | 选定页面的数据与布局 |
| 菜单 | `CollectionControl`；`menu.rs` | 菜单项描述和已接受命令的领域执行 |
| 文本与 IME | `TextField` 组合上游 `asInputEditor`；`proto_text.rs`、歌词、工作流、插件搜索 | 业务解析、校验消息、提交后的领域事实 |
| 数值字段 | `ContinuousControl` + `TextField`；`proto_scalar.rs`、`value_entry.rs` | 单位格式、参数定义和最终领域校验 |
| 混音推子与声像 | `ContinuousControl`；`mixer.rs` | 音频预览、Track/Master 目标和工程事务 |
| 窗口/窗格调整 | `ContinuousControl` 分隔条与 `ActionControl` 标题栏动作 | 操作系统窗口移动/缩放、非客户区命中和布局尺寸 |
| 滚动/缩放 | `ScrollRegion` / `CollectionControl` / `EditorSurface` / `ContinuousControl` | 原生滚动引擎、布局测量、时间/音高坐标换算 |
| 列表/表格 | `CollectionControl`；音符表、曲线点、轨道、历史与插件/预设浏览器列表 | 虚拟化、实际行数据、测量事实、业务选择是否被接受 |
| 弹层 | `OverlayControl`；`proto_overlay.rs`、模态、导出表单 | 窗口创建/关闭和选定响应的领域操作 |
| 编曲区 | `EditorSurface` + `CollectionControl`；`arrange.rs` | 片段/轨道几何与项目变更 |
| 曲线/自动化 | `EditorSurface` / `GraphPoint` / `ContinuousControl`；lanes、automation、phoneme 与设备图表 | 曲线坐标、采样和领域事务 |
| 虚拟键盘 | `PerformanceKey`；参考钢琴与插件演奏键盘 | 音高、声源、实际音频 note-on/off |
| 拖放映射 | `DragSource`；`proto_drag.rs`、`devices/modulation.rs` | 操作系统拖放运输和映射目标的数据合法性 |

`frontend/crates/verdant-app/src/generated/controls.rs` 仍是应用检入的命令/控制定义。其目录名不是 Proto 编译来源证明。真正生成的组件只来自本目录的 producer。

## 滚动接入的明确统计口径

应用共有 **21 处 `proto_scroll::viewport` 源码调用点**。这里统计的是创建视口的位置，不是当前窗口中的实例数；同一模板可渲染多个设备或轨道。

| 应用区域 | 调用点 | 实际内容 |
|---|---:|---|
| 主视图 | 7 | 左侧检查器、右侧属性、卷帘工具栏、泳道标签、曲线工具栏、编曲区、参数泳道 |
| 混音器 | 4 | Track/Master 插入槽、Track/Master 参数面板 |
| 设备与编辑器辅助区域 | 6 | 参考频谱、调制来源、音素时长、快捷键说明、原生插件内容、设备链 |
| 预设工作流 | 4 | 实例列表、预设类别、保存表单、导入报告 |

此外，应用保留 **5 处原生物理滚动容器**。它们都绑定编译组件声明的同一 ScrollSurface 引擎，不是独立的原生滚动策略：

| 原生容器 | 实际路径 | 编译拥有者 |
|---|---|---|
| `preset-list` | `devices/workflow/library.rs` | CollectionControl |
| `mixer-strips` | `mixer.rs` | CollectionControl |
| `context-menu` | `menu.rs` | CollectionControl |
| `plugin-browser-rows` | `plugin_ui.rs` | CollectionControl |
| 模态正文 | `dialogs.rs` | ScrollRegion |

上述 5 处计数排除 `proto_*.rs` 桥接实现和测试。整个 `verdant-app/src` **没有生产代码直接调用 `scroll_to_item`**。音符表、控制点表和历史列表从 UniformList 的真实测量提供可见及未实例化行的几何；参数泳道从真实 ScrollHandle 提供子项几何。最近边缘揭示由源组件检查目标/revision 并发出滚动请求，宿主不再选择 `ScrollStrategy::Nearest`。原生测试分别核对实际像素 offset、可见行的真实 bounds 和源组件的归一化 `position`，不混淆这两种单位。

## 需要保持的边界

- 编译实例拥有控件状态、输入阶段、取消、键盘和辅助功能提案。原生回调运输坐标、窗口事件或原生引擎观测；不能在 Rust 中再保存第二套同一控件状态机。
- 受控数值、选择和编辑提案包含来源身份；同步回调、策略/目标 props、禁用、卸载和工程变更可使旧操作失效。Action 的命令 authority 与实时辅助功能名称/描述分开：播放位置标签更新保留同一个已接受按压，换轨道/槽位/工程或改变禁用策略仍取消旧操作。领域层在写入前继续验证实时目标与 revision。
- `proto_parent.rs` 为宿主内容提供逻辑父 owner。所有惰性构造的编译子根在实际 render 时绑定，原生 slot 在 layout/prepaint/paint 期间保持该身份。
- 虚拟列表使用 `UniformListScrollHandle.0.borrow().base_handle` 的实际引擎。上游 `attach_scroll_engine` lease 让源组件观察和操作同一 handle，关闭重复编译根滚动；不复制两个视口的 offset。
- 滚动引擎 lease 拥有绑定身份，不等于永久接受任意旧输入。通知检查当前绑定、挂载和窗口；原生视图负责它实际注册的事件生命周期。丢弃 lease 不能影响后来重建的绑定。
- TextField 使用上游共享 Input 编辑实现及生成的 GPUI 文本引擎；不是在应用中重新实现 IME、剪贴板、选区和撤销。完整 InputRoot 的独立 ControlLabel 关联不在这些直接命名字段的声明范围内。
- 对窗口边缘和标题栏的系统操作保留系统非客户区机制；应用动作的激活、焦点和辅助功能仍由编译组件发出。

Adwaita 颜色、排版、图标、曲线和音符绘图继续由应用提供。音符命中、音乐坐标、单位换算、音频预览、工程存储和撤销事务属于领域或绘图边界。原生虚拟行回调只传递被命中的索引，连续控件的子部件回调只调用编译源的调整入口；它们不能直接替代源组件的输入准入。

操作系统标题栏移动、缩放、窗口菜单以及 Windows 非客户区命中/贴靠仍使用系统机制。插件窗口的文档撤销/重做属于应用命令分派；演奏键位由宿主运输到 PerformanceKey。活动歌词编辑期间防止无意失焦的窗口协调也保留应用事实。这些边界应与控件自己的按压、键盘、焦点状态和辅助功能提案区分记录。

## 上游回馈

本迁移的通用编译器与原生宿主能力通过 [Proto-UI PR #1730](https://github.com/Proto-UI/Proto-UI/pull/1730) 回馈。上一轮数值辅助功能修复 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710) 已合入上游 main，最终 pin 已整合该主线。下列修复在公共能力或宿主实现层完成，不根据 Verdant 的文件名或组件名分派。

- 条件辅助功能状态、终止分支后的有效类型缩窄、有界标量转换/舍入、UTF-16 长度和 readonly 数据发布。
- 共享 Input 编辑 hook、文本 reset/选区/IME 生命周期，以及 hook 返回 renderer 和回调的身份保留；完整 InputRoot 的 ControlLabel 协议没有被空实现替代。
- 原生 `activate` 辅助功能动作映射，以及原生可支持的 `user-select: none` 样式；其他不支持的值继续拒绝。
- 事件发出、焦点变化和 render 阶段回调的原生通知，避免源已经发出事件而宿主观察器没有收到；应用不用补发业务事件来弥补 SDK。
- 挂载前首次 hover 事实的保留与挂载后交付，按 view epoch、窗口和实际 hitbox 拒绝旧事实；不靠应用伪造 enter 事件。
- 真实原生滚动引擎 lease、Web Component 空渲染和相关跨目标编译回归。
- 文本选区通知分别适配现代 GPUI 的通知 API 与默认固定 GPUI 的字符坐标失效/下一帧查询机制；默认宿主真实读取选区、焦点隔离和过期编辑 lease 的行为有独立回归。

事件发出后抛异常、render 阶段通知、终止 owner、双帧首次 hover、卸载与旧 lease 等回归均已在下文的原生 SDK 运行中执行；当前结果不依靠旧 SDK 小集的成功推断。

`src/upstream-input/` 保留上游版权、许可证和精确来源。生成文件必须由锁定的上游编译器重新生成，不能通过修改 Rust 输出、降低 SDK 身份要求或绕过 source receipt 来修复集成。

## 验证与证据层次

最终完成记录绑定以下身份；执行范围、工具链、来源身份、日志摘要与历史检查点见 [VALIDATION-2026-11-23.json](VALIDATION-2026-11-23.json)。后续文档提交不改变这里被验证的源码快照。

| 身份 | 固定值 |
|---|---|
| Verdant 应用源码快照 | [`f325638dcd7cb4169e52d9eff7796825eccd4ebb`](https://github.com/OpenGreendam/verdant/commit/f325638dcd7cb4169e52d9eff7796825eccd4ebb) |
| Proto-UI 编译器 | [`5912f817581f89785e75c26e324a76c97257dc60`](https://github.com/Proto-UI/Proto-UI/commit/5912f817581f89785e75c26e324a76c97257dc60) |
| 共享 SDK 内容 SHA-256 | `27af6e2995e3c6d9a35ccbc4859a6273ed58b0c613ef4d3247284da1030b4691` |
| Node / package manager | Node `24.22.3` / `corepack pnpm@10.32.1` |

| 已完成的最终检查 | 实际结果与范围 |
|---|---|
| 作者源码 Runtime 与应用工具 | 完整 8 个 Node 测试文件 **173/173**，0 fail、0 skip；其中作者 Runtime **88** 项，publication/headless/producer 工具回归 **85** 项。设置了 `PROTO_UI_UPSTREAM`，fresh producer 用例实际执行；另有 **82** 项 GPUI 规则检查通过 |
| 上游源码与类型 | 聚焦源码回归 **206/206**；聚焦 TypeScript 检查退出码 **0**。涵盖准入、独立 IR 再验证、Web 源码消费者与发射行为，不是整仓或 OS 输入认证 |
| 严格 producer | **14 个单元、81 个编译产物**全部通过 fresh 与 retained 检查，来源闭包、原样生成字节、provenance、SDK/GPUI 身份和 receipts 一致。retained 仍明确表示 `freshCompilation: false`，不冒充 fresh |
| Verdant 现代 GPUI 原生 SDK | **26/26**，0 fail、0 ignored、0 filtered；包含通知、异常、焦点、双帧首次 hover、真实滚动引擎、文本编辑 lease 与选区回归 |
| 完整 Verdant GUI | **237/237**，0 fail、0 ignored、0 filtered。实际挂载编译组件，覆盖鼠标/键盘/辅助功能路径、画布、文本输入、领域提交及真实 viewport/虚拟行几何；不是仅编译成功或只跑旧旋钮筛选集 |
| 完整 Cargo 图 | `cargo metadata --locked --all-features` 的完整 resolve graph 通过；唯一 workspace GPUI 和共享 SDK，session 可达闭包中原生依赖 **0**，两个 data crate 均可达且没有依赖 |
| Headless session / protocol / data | 四个实际 package 的现有本地测试合计 **33** 项通过，0 fail、0 ignored；结合上一行的依赖图证明 headless 边界，不将全 workspace 出现 GPUI 误作 session 依赖 GPUI。本次本地运行不声明基础镜像隔离 |
| 实际 Rust source mutation | baseline 实测 **0.55**、variant 实测 **0.6**，各退出码 0；variant 使用旧 0.55 预期得到所要求的断言失败 exit **101**，实验整体退出码 0。两份产物与最终编译器身份一致，使用按源码摘要区分的 target 目录 |
| 上游默认 GPUI 托管 CI | [run 45071732309](https://github.com/Proto-UI/Proto-UI/actions/runs/45071732309)，head 为上述 `5912f817`：**macOS 15 与 Ubuntu 24.04 各自**通过原生消费者 **39/39**、数值辅助功能 **8/8**、SDK **27/27**，均 0 fail、0 ignored、0 filtered；两边的 `legacy_selection_notifies_the_live_native_input_and_respects_focus` 均实际通过 |

Runtime 88 项由 advanced editors 23、common controls 28、continuous control 9、normalized knob 23 和 text field 5 项组成；工具 85 项由 publication recovery 37、headless graph 27 和 producer 21 项组成。它们是 173 项的划分，不再次相加。上游默认 GPUI `62e5991` 与 Verdant 使用的现代 GPUI 运行不同的宿主 API 分支，SDK 27 项和 26 项分别记录；两种运行都保留了选区、焦点与编辑 lease 的真实行为验证。

### 历史失败及其解决

下面保留开发过程中的失败，用来说明修复依据。它们属于各自的旧来源快照，不是上述最终候选仍有未通过的测试。

| 历史检查点 | 具体原因与最终处理 |
|---|---|
| SDK 25/26 | 双帧 cold-hover 生命周期用例暴露挂载前事实被错误退休。通用宿主修复保留当前 view/window/hitbox 的事实并拒绝旧回调；最终现代 SDK 26/26 和默认 SDK 两平台 27/27 均包含该回归 |
| GUI 234/235 | 表格 fixture 把公开的归一化 `scrollPosition` 当成像素。修正为与同一真实引擎的 offset/max 比例比较，同时继续检查实际像素滚动与可见行 |
| GUI 236/237 | 新增整行几何探针没有 inset，被前面的文本推至静态位置。原生对照实测真实行在 **128–160px**、视口为 **0–160px**，旧探针却在 **154–186px**；实际引擎 offset 为 **−1888px**、最大值 1888px、内容高度 2048px。只为测试探针添加 `inset_0()`，保留整行 32px、完整可见、真实 offset、无原生 ScrollStrategy 和归一化一致性断言；最终完整 GUI **237/237** |
| 表格诊断编译中断 | 首次诊断的 rustc 被 SIGKILL，未执行测试；同一筛选重试通过后移除临时错位探针。该环境中断不记为原生断言失败，也不计入最终通过数 |
| 默认 GPUI 的历史 CI 失败 | [run 45069335331](https://github.com/Proto-UI/Proto-UI/actions/runs/45069335331) 和 [run 45070230468](https://github.com/Proto-UI/Proto-UI/actions/runs/45070230468) 因现代 GPUI 专用的文本选区 API 出现 E0599。最终改为调用默认固定 GPUI 实际支持的焦点/字符坐标失效与下一帧查询路径；run 45071732309 两个平台均实际执行并通过旧版选区通知回归 |
| 中间编译器 `3df32a68` / `5dc02bc` | 中间成功结果保留原身份。最终又修复了默认 GPUI 的文本选区通知兼容路径，SDK 内容身份随之改变；当前表格使用最终 `5912f817` 的重新运行结果，不沿用旧 SDK 的成功状态 |

### 平台范围

上述检查完成了本轮的源码、编译产物、依赖边界和原生框架行为验证。它们不是逐屏视觉或全平台无障碍认证；GPUI 测试、上游默认宿主 CI 与打包后的真实 OS 输入是不同层次。当前候选未执行的打包后 X11/Wayland、物理触摸、系统 IME、读屏和其他平台场景，继续按 [平台验收范围](../docs/PLATFORM-ACCEPTANCE.md) 单独记录。[历史 #79 的 OS 记录](NATIVE-INPUT-2026-11-22.json) 只属于当时的 app/worker 字节与提交，不转移为本候选的成功状态。
