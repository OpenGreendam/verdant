# 全部应用交互类别的 Compiler 迁移

本清单按产品中的交互职责分组，不把源码调用次数、组件数量或测试数量换算成界面采用率。当前编译清单是 `compiler.pin.json`：12 个 `gpui-source-v1` 组件和两个 `rust-data-v1` 决策单元。它们共享一份由上游编译器生成的 native SDK，应用不加载 JavaScript runtime。

## 交互类别与实际入口

| 类别 | Proto 来源 | 生产调用入口 / 宿主边界 |
|---|---|---|
| 钢琴卷帘与音符手势 | `pitch-gesture-surface`、`editor-surface`、`note-gesture-planner` | `proto_roll.rs`、`roll.rs`、session 强类型绑定；音乐坐标、音符命中及事务留在领域层 |
| 参数旋钮 | `normalized-knob`、`control-gesture-planner` | `proto_knob.rs`；Adwaita 绘图、参数单位与权威值校验 |
| 设备图形把手 | `graph-point` | `devices/graph_control.rs`；图形几何及一次性多参数事务 |
| 动作按钮 | `action-control` | `proto_controls.rs`，工具栏、窗口按钮、插件操作、文件操作 |
| 布尔与离散选择 | `action-control` | Switch、CheckBox、RadioButton 与受控 selected 提议；checked/pressed 按实际 role 输出 |
| Tabs | `action-control` | 工作区、属性面板、插件筛选等；方向键、Home/End 与 roving Tab |
| 菜单及右键入口 | `collection-control`、`action-control`、`overlay-control` | `menu.rs`；禁用项、子菜单拓扑、修饰键、ContextMenu/Shift+F10；菜单业务动作由 Rust 执行 |
| 文本、歌词与 IME | `text-field` 和有来源记录的 Base Input | `proto_text.rs`、`lyric_view.rs`、工作流字段；宿主 SDK 投影原生编辑、IME、剪贴板与 UTF-16 选区 |
| 数值字段与精确输入 | `continuous-control`、`text-field`、`overlay-control` | `ui_controls.rs`、参数值窗口、导出参数；单位解析与领域范围验证留在 Rust |
| 混音推子与声像 | `continuous-control` | `proto_scalar.rs`、`mixer.rs`；宿主执行实时预览与受控提交 |
| 窗口／窗格调整 | `continuous-control`、`action-control` | pane splitter 与窗口动作；操作系统的原生 resize/move/non-client hit testing 保留宿主实现 |
| 滚动与缩放 | `scroll-region`、`continuous-control`、`editor-surface` | `proto_scroll.rs`、时间线条与编辑画布；原生滚动句柄、viewport 和物理事件提供事实 |
| 列表与表格 | `collection-control`、`action-control`、`scroll-region` | `table.rs`、预设库、历史、实例列表；宿主保留虚拟化、数据投影及 item reveal |
| 弹层、对话框与抽屉 | `overlay-control`、`text-field`、`action-control` | `proto_overlay.rs`、`dialogs.rs`、提示／导出／数值窗口；源拥有焦点范围和 response，业务校验保留宿主 |
| 编曲区 | `editor-surface` | `arrange.rs`、`audio_edit_ui.rs`；源拥有工具与命中事实到选择、范围、移动、边界、淡化、分割等操作意图的决策，以及 contact、取消和键盘策略；领域层执行音频操作 |
| 参数曲线与自动化 | `editor-surface`、`continuous-control` | `lanes.rs`、`automation_lane.rs`、`phoneme_lane.rs`、EQ sketch；源拥有创建／移动、选择修饰键与锁轴策略；音乐坐标、曲线采样与事务留在宿主 |
| 虚拟演奏键盘 | `performance-key` | `proto_performance_key.rs`、原生合成器窗口；源拥有按下／松开和取消，宿主发送 note-on/note-off |
| 调制拖放映射 | `drag-source`、`action-control` | `proto_drag.rs`、`devices/modulation.rs`；GPUI 运送 payload 和拖动图像，源控制单次 lease 接受、取消和键盘选择 |

复用上述控件的专用插件编辑器、通用参数编辑器和独立原生窗口使用同一实现。第三方插件自己绘制的嵌入窗口不属于 Verdant 编写的交互控件。

## 必须保持的边界

- 控件的 phase、受控提议、键盘／辅助功能入口、焦点与生命周期来自原型或上游 SDK。应用 `Action.on_click` 接收的是源已经接受的 `Activation`，不是第二条原生激活状态机。物理点击元数据、键盘修饰键和辅助功能来源不能互相冒充。
- 项目、store、revision、track/master、插件实例、parameter key 和权威值都由宿主再次校验。原型接受输入不等于领域层接受写入。旧菜单、旧文本草稿或旧参数目标的延迟事件不能写入新的实例。
- 普通鼠标 capture 绑定 window、hitbox 与 generation；源 contact 另外拥有 revision 与单调 lease。即使指针或 hitbox 相同，旧终止事件也不能提交或释放新的捕获。触摸使用真实 touch arena，不能借用鼠标捕获。
- `NativeScroll` 绑定已有 ScrollHandle（虚拟列表为原有 base_handle）。Proto 的翻页、首末定位和滚动事实必须指向它；不能加一个外层空滚动容器冒充原列表已经接入。
- wheel、pinch、缩放按钮和快捷键都向同一个编辑原型提供输入事实，由原型选择滚动方向、缩放意图和修饰键策略；宿主负责 viewport 几何、时间单位换算和实际显示。数值表格单元格只消费数值编辑键，未消费的导航键交给所属集合。
- native slot 明确携带 Proto parent。Adwaita 布局、自绘、音乐时间／单位换算、播放与撤销事务是宿主职责。操作系统窗口移动、边框调整和第三方插件窗口机制仍是原生系统能力。
- 文本字段不通过 Rust 的字符串追加／删除来另写编辑器。程序替换通过编辑会话 reset，清理旧组合与历史；选区按 UTF-16 检查边界。未结束的 IME composition 不应被 Enter 提交或外围快捷键抢走。

## 来源与生成门禁

所有 `src/*.proto.ts` 顶层入口必须登记在 pin 中，所有 `generated/*/Cargo.toml` 消费者也必须属于相同清单；遗漏入口或遗留未登记生成 crate 都会失败。嵌套的 vendored helper 由 source graph 记录。应用 Rust 中依赖的 package 名从真实生成 Cargo.toml 更新，生成文件本身不可手改。

`build-proto-ui.mjs --check` 校验固定、干净的上游提交及实际 Node/pnpm/TypeScript/tsx、来源闭包、SDK 内容和全部产物字节。`--check-retained` 仅证明当前来源与保留产物自洽，两者必须分开报告。session 仍只依赖两个无 GPUI、无 serde 的标准库 data crates。

## 验证范围

源级测试覆盖受控值拒绝、重入更新、revision 替换、卸载、取消、文本组合和输入准入。GPUI 测试使用真实生成组件与事件，覆盖鼠标、键盘、AccessKit 动作、虚拟列表与实际音符表格、导出对话框、图形末次采样、触摸仲裁、持续演奏键和卸载后的迟到事件。图形测试还验证捕获丢失／同 hitbox 被重新取得后不得写入，旧清理不得释放新捕获。测试直接调用 Expose 方法与实际派发原生事件的证据分别保留。

macOS 与 Windows 的 workflow 除原有 pointer/roll/参数链外，也运行 `proto_`、`devices::`、`table::`、`menu::`、`commands::`、`files::` 范围，Linux 运行完整 app 测试。真实 X11／Wayland 输入 job 继续依赖同一次构建的发布包，并绑定源身份与二进制摘要。

每个验证结果以本变更对应提交的 CI Checks 为准；README 中保留的 #79 结果是迁移前基线。框架测试不能代替物理触摸、平台 IME、屏幕阅读器或所有平台的实际窗口验收，未运行的范围不能由覆盖清单推断为通过。
