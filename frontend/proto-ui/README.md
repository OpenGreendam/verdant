# Proto-UI 与 GPUI 集成

Verdant 使用固定版本的实验性 Proto-UI compiler：`gpui-source-v1` 生成原生组件，`rust-data-v1` 生成只依赖 Rust 标准库的类型化决策函数。保留的 Rust 源码全部由上游 compiler 发射；不要手改 `generated/`。版本、完整 GPUI 来源、工具链要求、共享 SDK 内容和各编译单元的目标由 [compiler.pin.json](compiler.pin.json) 绑定。

## 2026-11-25 RIIP：时间线按压策略

编曲区与钢琴卷帘的 `TimelineScrollbar` 使用同一作者源码 `ContinuousControl.beginTrack`。宿主只给 thumb 命中、槽位候选和位移换算；源码的 `trackPress` 决定立即跳转或等待拖动，并发布独立的 `dragOrigin`。取消仍回到原接受值，预览重绘不会重新选择拖动起点。详细职责、原生事件回归与剩余边界见 [本批记录](RIIP-2026-11-25.md)。

本批基于主线 `93ff60c1420850c6db7f3248d234faee9cc655fa`。原清单中的主总线 insert bypass、布尔命令按钮和 checked menu 选择值消费已经存在于该基线，不重复算作本批迁移。继续使用下述 compiler pin 与共享 SDK；生成、平台包和输入验收以本批准确提交的 CI 为准。

## 2026-11-24 RCIP 历史记录

该轮把 compiler 从 `4e499ae25e825e130e740f16af66d70b23a743cf` 升到公开提交 [`e9340df6279e063cf372080bf4cc97caea3fd901`](https://github.com/Proto-UI/Proto-UI/commit/e9340df6279e063cf372080bf4cc97caea3fd901)，基于上游主线 `88fc0883bec70abbc0d7505d88370657c279639e`，通用兼容修复见 [Proto-UI PR #1791](https://github.com/Proto-UI/Proto-UI/pull/1791)。共享 SDK 内容摘要为 `832777a017c27e8969ed37ce941ea1f7f5ee21d532fb5f9412ff930074e94b3b`；SDK 版本仍为 `0.2.0`，14 个单元和 receipt 全部通过固定干净编译器重新生成。

本轮补齐连续输入的阈值／步进、受控 `selected` 的真实宿主消费，以及弹窗和乐谱导出表单的默认响应、子输入提交与关闭退役。逐控件对应和仍需继续迁移的项目见 [RCIP 清单](RCIP-2026-11-24.md)，准确执行范围见 [本轮验证记录](VALIDATION-RCIP-2026-11-24.json)。这些实现和 compiler pin 已包含在上述 2026-11-25 主线基线中；下列旧执行数量仍只对应原记录中的候选，后续修复和本批改动各自验证。

本轮本地验证已通过：14 个单元／81 个产物的 fresh 与 retained 检查、236 项源与 producer 测试、298 项应用原生测试、SDK 两组配置的 63／62 项、33 项 session／protocol 测试，以及最终实际 all-features 依赖图。上游较广编译器套件保留 1 项已复现的基线失败；本轮跨平台包和实际 OS 窗口结果仍以对应 PR 的新 CI 为准。

## 2026-11-23 已合入主线的整合记录

Verdant [PR #12](https://github.com/OpenGreendam/verdant/pull/12) 已于 2026-11-23 合入 `main`，合并提交为 `afd3eda361874f275422263473334daba7575b59`。该轮保留 18 类交互对应的 14 个编译入口、Adwaita 绘图和音乐领域层，并将编辑操作选择、连续值精度和输入生命周期放进作者源码。

**最终候选 `5e19b7de231894c921a6dd69f16e8820b35b8ec3` 已完成 fresh producer、完整 Node、三平台 GUI/SDK、打包以及全部 28 项 Linux 真实窗口输入验收。** [Frontend #122](https://github.com/OpenGreendam/verdant/actions/runs/45212699291) 实际测试 PR merge checkout `96691e1abdf722d573465ee2d32a4a1588a9191b`；结果固定到该来源，后续提交仍需各自验证。

| 已执行范围 | #122 的实际结果 |
|---|---|
| 固定编译器再生成 | 14 个单元、81 个制品；82 项 GPUI 规则 fixture；完整 Node 224/224、0 skip |
| 独立 retained lane | 97 passed、127 个依赖上游的用例明确 skipped；不能替代上面的 fresh 结果 |
| 无原生 UI 依赖的 session lane | 33 项 Rust 测试通过 |
| Linux、Windows x64、macOS Apple Silicon | 每平台完整 GUI 291/291；SDK 两个 feature 配置分别 53/53、52/52；各自打包和来源检查通过 |
| Linux X11 与 windowed Wayland | 原有 11 + 11、迁移控件 3 + 3，共 **28/28**；严格 finalizer 的 `errors = []` |

完整来源、job、原始 artifact 摘要和未覆盖范围见[主线验收记录](VALIDATION-MAINLINE-2026-11-23.json)。上游固定到 `4e499ae25e825e130e740f16af66d70b23a743cf`，共享 SDK 内容摘要为 `e7bade58e60d8e4addfb42ca1228c0c38f066d0093f148eae3101cdcad5aca35`。这些数字来自各自执行层，不能相加为总验收数量；较广 Cargo workspace 中既有资源依赖的 ignored/filtered 用例仍按原日志保留。

[恢复期记录](VALIDATION-INTEGRATION-2026-11-23.json)、此前失败和历史通过记录保持原样，不把它们重标为最终通过。`gpui-source-v1` 仍是实验性 profile；系统 IME、物理触摸／触笔、完整读屏、物理音频和 Windows/macOS 实际窗口操作仍按[平台验收](../docs/PLATFORM-ACCEPTANCE.md)分别完成。CI 包未随附完整声库，不能据此宣称正式发行或新用户断网合成已经完成。

## 当前边界

目前共有 **12 个原生编译组件 + 2 个标准库决策单元**；完整的 18 类映射、宿主边界与验证范围见 [CONTROL-MIGRATION.md](CONTROL-MIGRATION.md)。这些入口在本轮基线已经存在；本轮继续补齐实际消费路径，没有用新建同名包装组件增加计数。下表各单元共用生成、pin、receipt 和 CI 门禁。

| 单元 | 原型拥有的职责 | Rust 宿主拥有的职责 |
|---|---|---|
| `pitch-gesture-surface` | 原生根、尺寸/裁剪、slot 与 pitch 手势入口；共享 SDK owner | `proto_roll.rs` |
| `note-gesture-planner` | 移动、复制、resize、框选的纯决策函数和有序效果 | pointer identity、不可变守卫事实、revision、领域数据、效果执行和原生 capture |
| `control-gesture-planner` | 曲线/连续参数的纯决策函数和归一化拖动计算 | 图形命中、坐标与参数单位、preview、最终采样和工程提交 |
| `normalized-knob` | 值/phase、受控提交、复位、取消、键盘、焦点、Slider 辅助功能、精确输入请求 | `proto_knob.rs`、内置设备编辑器 |
| `action-control` | 按压/激活、禁用、选择/展开、上下文请求、键盘与辅助功能操作；目标变化取消旧按压，实时标签更新保留相同命令 | `proto_controls.rs`、按钮、离散控件、Tabs、窗口标题操作 |
| `collection-control` | 游标、选择、遍历、展开、搜索结果导航、上下文请求、滚动/最近边缘揭示与过期 revision 拒绝 | `proto_collection.rs`、菜单、音符表格、点列表、轨道/历史/浏览器列表 |
| `continuous-control` | 数值预览/提交、精度与量化、受控值接受、细调、复位、取消、键盘与数值辅助功能 | `proto_scalar.rs`、推子/声像、数值字段、分隔条、时间线滚动条 |
| `editor-surface` | 操作选择、lease/phase/阈值、吸附、Shift 轴约束、边缘滚动、取消、焦点和统一缩放导航 | `proto_editor_surface.rs`、编曲/卷帘/曲线/自动化/标尺/EQ 绘制 |
| `performance-key` | 按键触发、保持/重复、松开与失焦停止、辅助功能激活 | `proto_performance_key.rs`、参考键盘与演奏键盘 |
| `drag-source` | 可消费一次的拖放 lease、revision、取消、键盘/辅助功能目的地选择 | `proto_drag.rs`、调制映射 |
| `overlay-control` | 可见性、默认/取消响应、焦点进入/约束/恢复 | `proto_overlay.rs`、模态与导出窗口 |
| `text-field` | 文本/IME/选区/撤销、焦点、提交/取消/前进与数值文本策略 | `proto_text.rs`、歌词、精确值、工作流、插件搜索 |
| `graph-point` | 每个图形把手的双轴预览/提交、阈值/细调、复位、键盘轴切换与辅助功能 | `devices/graph_control.rs` |
| `scroll-region` | 文档/视口滚动请求、页面/边缘按键、最近边缘揭示与可选焦点范围 | `proto_scroll.rs`、说明、表单、参数泳道及原生滚动引擎 |

前三项属于 **Proto 编译的 gesture planner + native GPUI surface**。note/control 由上游 data target 生成独立的 Rust 函数，不创建组件实例、不执行组件 setup，也不链接 GPUI。`normalized-knob` 是真实有状态组件实例：它复用 control planner 的同一份 TS 来源，原型对交互和公开语义负责，宿主不再为该旋钮另存一份 phase 状态机。slot 和自绘是合法组成方式；是否由原型承担交互契约，不能由有无 slot 判断。

旋钮的 `props.value` 是宿主接受的归一化值；`defaultValue` 只表示重置目标。拖动时公开 `value` 可以显示预览，提交前恢复到已接受值，再发出 `{ value, revision }` 提案。宿主接受后通过 props 发布新值，拒绝时不会留下未接受的显示值。`revision` 是不透明的来源标识，宿主可将工程、store 与参数身份纳入其中。值、revision 或 disabled 改变会取消当前交互；键盘接管先取消指针预览，再从已接受值步进，Shift 使用十分之一的步长。公开状态、焦点和事件回调后的 epoch 检查防止重入更新被旧操作覆盖。

滚动采用编译源声明的 ScrollSurface：一般内容通过 `proto_scroll::viewport`，列表、菜单等通过显式 lease 绑定真实原生引擎。具体构造点会随窗口接入改变，本文不把上一轮的 21/5 统计当作当前总数；原统计保留在历史机器记录中。音符、控制点、历史和参数泳道的显示位置继续由编译源的滚动/最近边缘揭示策略决定。

辅助功能除了 Slider 的名称、范围、值和 disabled 状态，还声明 `increment`、`decrement`、`setValue` 动作。SDK 将 AccessKit `NumericValue` 解码为原型事件的有限数值；缺失、错类型或非有限数据不分发。原型中的写值动作也先取消旧预览，再走同一受控提交事件。Linux AT-SPI 当前通过 `Value.CurrentValue` setter 请求 SetValue；不能把检查到了 Slider role 或模拟一个 host event 计作实际平台写值通过。

`text-field` 使用共享 SDK 的同一个宿主编辑器。其辅助功能投影提供真实 `TextRun`、规范化文本、Unicode 字符长度和 anchor/focus 选区，辅助技术的选区请求回到当前编辑 lease；旧 owner、旧帧、detach 或替换后的请求不能修改新编辑器。字符几何没有用控件外框伪造；字符边界、软换行、IME 和屏幕阅读器的完整平台体验仍须分别验收。真实 AT-SPI `Text` 读写与选区操作由 OS 场景核对，SDK 内部回调测试不能替代它。

`verdant-ui-protocol` 只依赖稳定命名的 `verdant-note-gesture-data` 和 `verdant-control-gesture-data`；这两个生成 crate 均没有 Cargo 依赖。adapter 直接构造编译器生成的 guard 类型、调用类型化函数，并将完整结果预解码为枚举，检查未知效果、次序和所有权后再交给业务层执行。它不依赖 GPUI、native owner、serde 或 JSON，也没有另写 Rust 状态机。session 的完整可达 Cargo 图和实际构建都单独验收；原生组件的 props/event 编码留在 app 边界内。

普通 roll 和曲线的原生 capture 由带 window/hitbox/generation 的 `OwnedPointerCapture` 管理。领域变更、取消、失焦、窗口离开和输入子树移除都会结束对应的会话；旧延迟清理不能释放新会话的 capture。touch 使用 GPUI touch arena，不冒充 mouse capture。终止事件在 capture phase 接收最终坐标/modifier，非有限或窗口 viewport 外的坐标取消，不能沿用过期 preview 提交。Pitch 和旋钮复用上游 MoveGesture lease，并与普通 roll/曲线按 contact 所有权互斥。lane、automation、mixer 和数值 scrub 的既有宿主路径也按各自 editor/target/pointer 所有权检查窗口边界、撤销预览并释放对应捕获；这些路径没有全部改成 `OwnedPointerCapture`。参数提交在写入前重新核对工程、revision、当前 track/master 目标、slot、parameter key 和权威参数值，不能只信任上一帧的 props。

旋钮的捕获拖动可以继续越过控件 hitbox。上游 press adapter 因 hover 离开而发出的 `press.cancel` 不结束这个独立的 move lease；失焦、detach、失去所有权等真正的 lease 取消通过公开 `cancel` 方法清理原型。

曲线图在 prepaint 时另外保存这一帧对应的权威参数快照。若参数或 revision 在绘制后、按下前改变，旧帧的拖动、右键菜单和双击写入都会被拒绝；重绘后可以正常编辑。动画和预览值只决定绘制及命中位置，拖动起点必须来自已接受的权威参数，避免把动画中的旧值写回工程。

### 本轮进一步统一的源码语义

- `EditorSurface` 以同一 lease 决定区域/音符/曲线/音素/标尺操作、选择方式、吸附、Shift 轴约束、边缘滚动，以及滚轮、pinch、工具栏和快捷键的导航请求。宿主只提供命中/几何事实并执行音乐事务；旧取消携带旧 pointer、revision 和 lease，不能取消下一次接触。保留编曲的三像素阈值和原有音频快捷键/焦点协议。
- `PerformanceKey` 保留短促参考键触发、持续音符、最高 C 键和旧 note-off 所有者校验，并补强重复按键、指针与键盘重叠、窗口停用、隐藏、卸载与延迟 note-on 退役。触摸不借用鼠标 capture；丢失的鼠标 capture 不能完成新 owner 的接触。
- 连续值与图形把手由源决定精度、量化、细调和提交阶段；应用不能把渲染动画或临时预览当作新的接受基准。图形背景双击也核对帧快照和 live surface，发出源动作后再次核对实际 authority。
- TextField 继续组合上游 `asInputEditor`。宿主镜像源接受的规范化文本，并按 draft generation、view epoch 与挂载状态截断旧事件批次；搜索导航先由源发出，再直接交给既有 Collection `filterKey` 协议。IME 组合期间保留本地编辑语义，列表 reveal 仍使用同一真实引擎。歌词/重命名提示以新的实例身份拒绝同名重开窗口的旧提交。
- Overlay 尊重子字段已阻止的默认键动作；每次排队响应交付前核对 present/mounted/disposed、view epoch、revision、open 与 disabled。一个响应导致关闭、替换或重开后，后续旧响应不能继续执行。
- 配置 `submitOnEnter` 的 Action 对一次物理 Enter 只发出 submit。即使提交回调同步替换 props，随后原生 `press.start`/`press.commit` 也不能再激活新命令；Space、指针和普通 Enter 保持既有激活行为。接收端保留原始位置、修饰键、输入来源与命令 authority。

## 再生成与两类检查

当前主线把 GUI binary 命名为 `verdant-gui`，`verdant` 是 CLI；下面的 app 测试命令使用 GUI target。#79 的实际结果仍绑定其原始 checkout 与包格式，后续主线集成的检查另见 PR Checks。

使用 Node 24.x、pnpm 10.32.1，将 Proto-UI 检出到 pin 的完整 commit。以下命令从仓库根执行：

```sh
corepack pnpm@10.32.1 --dir frontend/proto-ui install --frozen-lockfile --ignore-scripts
git clone https://github.com/Proto-UI/Proto-UI ../Proto-UI
compiler_revision=$(node -p "JSON.parse(require('fs').readFileSync('frontend/proto-ui/compiler.pin.json','utf8')).upstream.revision")
git -C ../Proto-UI checkout --detach "$compiler_revision"
corepack pnpm@10.32.1 --dir ../Proto-UI install --frozen-lockfile --ignore-scripts

node frontend/proto-ui/tools/build-proto-ui.mjs --upstream ../Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --check --upstream ../Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --check-retained
```

`--check` 必须新鲜编译，缺失上游 checkout 是失败（exit 10）。`--check-retained` 只核对当前 workspace、来源闭包、保留制品、provenance 和 receipt，明确返回 `freshCompilation: false`。所有离线可核对项先于 upstream 判缺；改错实际 workspace GPUI pin，即使没有上游，也会失败。

生产脚本 `tools/check_frontend.sh` 默认要求 fresh 检查。单独的 Rust 构建 lane 可显式设置 `VERDANT_PROTO_UI_MODE=retained`；CI 的 Linux/Windows/macOS 构建均依赖强制 fresh lane 和独立 headless-session lane，不能将 retained 成功当成新鲜编译证明。headless lane 使用固定的、未经 Verdant 扩展的 Rust 基础镜像，不安装 GPUI 的原生依赖；它解析完整 workspace 图，再在独立 target 目录中编译和测试真实 session、protocol 与两个 data crate。

工具链核验包含实际执行 Node/pnpm、实际加载的 TypeScript/tsx 版本、安装所用 package manager、完整已安装锁文件和上游锁文件内容。成功输出记录这次实际执行版本；receipt 记录可复现要求，不冒充某次机器执行证明。

`native-input` CI job 依赖同一 run 的 Linux 打包结果，下载经过发布门禁的 app/worker。它用包内 `verdant-release` 检查 archive、解包后的内容和当前 checkout 的 source selection，再执行 X11 与 windowed Wayland 各 11 项既有场景及各 3 项 authored-controls 场景，共 28 项。新增场景覆盖文本框的 Unicode 选区、提交/取消和撤销，预设列表的键盘导航与滚动，以及 mixer pan 的最终松手采样、取消和撤销。完整性门禁核对二进制字节、全部场景、原始辅助功能节点与实际输入记录，缺项或 skip 都失败；失败时仍保留原始证据。实际执行结果绑定相应提交的 PR Checks，不能由脚本中的场景数量推断通过。#79 的历史链只有既有 22 项且已通过 22/22，见下方验收表和 [原生输入记录](NATIVE-INPUT-2026-11-22.json)。

## 来源闭包和共享 SDK

producer 按各单元的 target 调用上游 `compileFile` 或 `compileRustDataFile`，共用 `FileSourceGraph`，记录仓库边界内的入口、导入 helper、包解析输入以及未命中的候选路径；已有的仓库外来源会被拒绝。编译前后核对来源字节与解析图，避免只哈希入口却遗漏 helper 或 exports 变化。

`pitch-gesture-surface` 发射唯一的 `.proto-ui/gpui-native`；其他 11 个原生单元通过上游 `nativeSdkPath` 引用它。两个 data 单元不使用 SDK。pin 绑定 owner、路径、package/version、ABI feature 和 SDK 内容摘要；data target 则绑定稳定 crate 名、公开方法和空依赖集合。增加原生单元时必须复用 SDK，不能复制同名 package，也不能改生成 Rust 来修 Cargo 冲突。

生成物替换先验证 receipt 的完整 inventory 和每个文件内容。额外文件、人工改动、符号链接和不完整 inventory 会被拒绝；它们不会被“目录里有 provenance”这一条件授权删除。发布使用互斥锁、唯一 staging、备份和恢复 journal。

公开锁是完整 owner 记录的原子、仅创建 hard link；owner 先在唯一私有 state 目录完整写入，再公开锁，竞争获取不能覆盖已有锁。初始 journal 同样先完整写入，再原子公开，且先于任何 staging 写入。进程在这两个初始化窗口被杀，不会暴露半份 owner/journal；尚未公开的私有残留不会阻塞新 publisher，也不会通过通配扫描删除。释放和恢复绑定 owner identity，旧 owner 不得移除后来 owner 的锁。没有发布 journal 的完整旧目录锁仍可回收；缺 owner、包含未知内容的旧锁，以及没有所有权快照的 v1 发布 journal 都保留并明确拒绝自动恢复。

v2 journal 在首次 staging 写入前记录每个替换目标的旧、新所有权快照，包含目录、空目录、文件内容摘要和 receipt 字节。恢复旧批次和清理已提交批次都先核对全部 target、staging、backup，再执行任何删除或还原；每次删除前再次核对对应路径。崩溃后添加的文件、人工修改、未知目录或截断的写入都会使恢复失败，锁、journal 和所有产物保留供检查。staging 只有在内容是计划库存中已完整写入的子集时才允许自动清理；路径名称本身不能授权递归删除。

这是针对进程中断的可恢复多目录替换，不保证断电持久性，也不是跨目录的原子快照。只有遵守 producer 锁的读写方才具备批次一致性；不要在再生成过程中同时运行 Cargo。

中断后，先确认记录的 producer 已停止，再执行：

```sh
node frontend/proto-ui/tools/build-proto-ui.mjs --recover-publication
```

恢复入口检查同机 owner PID，并用原子公开的编号 claim 独占恢复所有权；活跃 producer 不能被恢复操作覆盖。恢复本身被中断后，也可在其 owner 停止后再次执行。恢复路径不会猜测删除未知文件、伪造身份或符号链接。不要手动删除活跃锁或备份。

## 准入与证据

本轮最终执行与合入状态见[主线验收记录](VALIDATION-MAINLINE-2026-11-23.json)；[恢复期机器记录](VALIDATION-INTEGRATION-2026-11-23.json)保持其当时的 pending 状态。以下表格是**上一轮已执行的历史快照**，绑定应用源码快照 [`f325638d`](https://github.com/OpenGreendam/verdant/commit/f325638dcd7cb4169e52d9eff7796825eccd4ebb) 与编译器 [`5912f817`](https://github.com/Proto-UI/Proto-UI/commit/5912f817581f89785e75c26e324a76c97257dc60)。执行范围、来源身份和日志摘要见 [上一轮机器记录](VALIDATION-2026-11-23.json)，职责清单、证据层次和历史修复见 [迁移记录](MIGRATION-2026-11-23.md#验证与证据层次)。

| 上一轮检查范围 | 该快照的实际结果 |
|---|---|
| 严格 producer | 14 个单元、81 个编译产物的 fresh 与 retained 检查均通过；保留完整来源 receipts、唯一 SDK 和 GPUI 身份 |
| 应用工具与作者 Runtime | 82 项规则检查通过；完整 Node 测试 173/173，通过且 0 skip，其中作者 Runtime 88 项、工具回归 85 项 |
| 上游源码与类型 | 聚焦源码回归 206/206；TypeScript 检查退出码 0 |
| Verdant 使用的现代 GPUI | 原生 SDK 26/26；完整 `verdant-gui` suite 237/237，均无失败、忽略或筛选掉的测试 |
| Cargo 图与 headless | 完整 locked/all-features resolve graph 通过，唯一 GPUI/SDK，session 原生依赖为 0；session、protocol 和两个 data crate 的本地测试共 33 项通过，此次本地运行不声明基础镜像隔离 |
| 实际 Rust source mutation | baseline 实测 0.55、variant 实测 0.6；variant 使用旧 0.55 预期时按要求断言失败，实验整体通过 |
| 上游默认 GPUI 的托管 CI | [run 45071732309](https://github.com/Proto-UI/Proto-UI/actions/runs/45071732309) 在 macOS 15 和 Ubuntu 24.04 上分别通过原生消费者 39/39、数值辅助功能 8/8、SDK 27/27；两边均执行了旧版宿主的真实选区通知回归 |

这些结果分别证明源码语义、生成 Rust、依赖边界和框架输入行为，不相加为“总验收项”。默认 GPUI 的 27 项 SDK 回归与 Verdant 现代 GPUI 的 26 项使用不同宿主 API 分支；后者不能代替前者。本轮已执行的 Linux 打包后 28 项 OS 输入结果见文首 #122；逐屏视觉、物理触摸、系统 IME、完整读屏和其他平台窗口操作仍分别记录，不继承下面 #79 的历史成功状态。

上一轮旋钮集成的历史版本见 [首轮验收记录](VALIDATION-2026-11-22.md) 和 [后续修复验收](VALIDATION-COMPLETION-2026-11-22.md)。后文历史表固定到实现 head `a55bdcd4cc73e3a48cdfc956151e9934cc865f48` 和 [Frontend #79](https://github.com/OpenGreendam/verdant/actions/runs/45026004598)；实际被检出、测试和打包的 PR merge commit 是 `9e6cd34715b9da4b733466b95d749764e5c410b3`。关联讨论位于 [PR #5](https://github.com/OpenGreendam/verdant/pull/5)。这些记录不随当前分支和后续文档提交改写。

全部控件迁移的入口与职责同时列于 [迁移清单](CONTROL-MIGRATION.md)；当前完成状态以整合记录和对应提交的实际 CI 执行为准。

TOML admission 使用 parser，按实际 `package` 名检查普通/build/dev 和 target-specific 的所有依赖表，支持 alias、子表、多行和注释。它检查声明；另一个门禁调用完整 `cargo metadata --locked --all-features`，沿全部 workspace members 的 resolve graph 检查 GPUI package ID/source/rev 和唯一 SDK，同时按实际 workspace/manifest 身份追踪 session 的所有可达边，拒绝原生组件、GPUI、SDK 和图像支持。两个 data crate 必须可达且没有依赖；不能改个包名躲过路径核验。`--no-deps` 或扫描 Cargo.lock 字符串不能替代这一检查。

```sh
NODE_OPTIONS=--max-old-space-size=512 PROTO_UI_UPSTREAM="$PWD/../Proto-UI" node --test --test-concurrency=1 frontend/proto-ui/tools/test/*.test.mjs
(cd frontend && cargo fetch --locked)
node frontend/proto-ui/tools/check-cargo-gpui.mjs --offline

(cd frontend && cargo test --locked -p verdant-ui-protocol -p verdant-session)
(cd frontend && cargo test --locked -p verdant-app --bin verdant-gui -- --test-threads=1)
(cd frontend && cargo test --locked -p proto-ui-gpui-native --features test-support,gpui/test-support --lib -- --test-threads=1)
```

离线 Cargo graph 检查前，需要在 `frontend/` 中执行过 `cargo fetch --locked`。Rust 测试进入该目录执行，使 rustup 采用其中的固定工具链。

设置 `PROTO_UI_UPSTREAM` 的测试覆盖真实 compiler 发射和来源变异，也将 authored knob 放进 pinned `createRuntimeSession`，通过受控 host capability 检查拒绝更新、同步接受、重入取消和生命周期。这些 JavaScript Runtime 测试不经过 GPUI，也不代表 OS 输入测试；无 checkout 时会明确跳过。上面的 Rust 测试另外覆盖类型化协议、宿主 capture 和 GPUI 组件实例。

CI 还准备 baseline/variant 两套完整 data-target 编译产物，用同一类型化 Rust consumer 执行：TS 中 fine sensitivity 从 0.1 改到 0.2，Rust 函数的实测结果必须从 0.55 改到 0.6，旧期望必须失败。每个变体使用由生成源码摘要区分的 Cargo target 子目录，避免相同 package ID 和解包时间戳错误复用另一变体的构建缓存。这验证 TS 来源确实影响已编译的消费者行为，不涉及窗口输入。

```sh
node frontend/proto-ui/tools/check-native-source-mutation.mjs --prepare .ci/proto-ui-source-mutation --upstream ../Proto-UI
node frontend/proto-ui/tools/check-native-source-mutation.mjs --run .ci/proto-ui-source-mutation
```

保留字节一致、Cargo graph、Rust trace、GPUI 框架事件、OS 窗口输入和物理设备分别提供不同证据。哈希或截图不能代替取消/捕获正确性，GPUI 测试不能替代物理触摸、IME 或屏幕阅读器验收。跨平台剩余范围仍见 [PLATFORM-ACCEPTANCE.md](../docs/PLATFORM-ACCEPTANCE.md)。

## 迁移前基线验收（#79）

| 检查 | #79 的实际结果 |
|---|---|
| Proto 工具与再编译 | 82 项准入 fixture；完整 Node 测试 90/90；31 个保留制品与固定 compiler 新鲜输出一致。90 项包含 21 项 publication 回归，覆盖真实子进程 SIGKILL、竞争获取和恢复交接 |
| Linux Rust | [job 137219956596](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956596)：1118 passed、0 failed、15 项既有 ignored；94 个单元/集成测试套件，另有 21 个零测试 doctest 结果。包含 app 206、domain 108、session 27、protocol 6、release 13 |
| macOS Rust 与打包 | [job 137219956666](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956666)：102 passed、0 failed、2 项可选声库测试 ignored；其中选定 app 测试 53 项。40 个 payload 文件、733 个 archive members，发布检查 allowed |
| Windows Rust 与打包 | [job 137219956587](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956587)：106 passed、0 failed、2 项可选声库测试 ignored；其中选定 app 测试 53 项、Windows path 5 项。39 个 payload 文件、732 个 archive members，发布检查 allowed |
| Linux 打包与来源变异 | 42 个 payload 文件、735 个 archive members，package/source-offer 检查 allowed；真实 Rust consumer 的 baseline 为 0.55、variant 为 0.6，以旧期望执行 variant 得到预期断言失败 exit 101 |
| Linux 真实窗口与辅助功能 | [job 137223338246](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137223338246)：X11 11/11、windowed Wayland 11/11，共 22/22、0 skip，每个 backend 40 个观测步骤；实际包含 native editor 关闭/重开和 preset-menu 取消路径 |

Rust 各 crate 的数量是 Linux 总数中的子集，不重复相加。macOS/Windows 运行 workflow 明列的测试范围，不代表各自执行了 Linux 的全部 206 项 app 测试。15 项 Linux ignored 涉及外部插件/宿主或用户自备声库等既有条件；不能当作它们已经通过。11 项离线 Python helper 回归与 22 项实际 OS 场景分别计数。

本次 OS 验收复用同一 run 的 Linux 包：archive SHA-256 为 `3ebb0439398289309d509170700ba5a51a3662d79afd840267ed86f5896e14a1`，实际 app 为 `273e3ea6fa8015d436ea885ccc7eaf721c4e80d71950ae60d1659d412793cf20`，worker 为 `c4542e8a6f151c7b2f9556e0008547646c4a55ce180eda3ffdf8e40963765033`。输入经过 XTest→X11 或 XTest→Weston→真实 Wayland；AT-SPI 写值经过实际 `Value.CurrentValue` setter。辅助功能数值读取、持久化、revision、单次撤销、捕获接管和取消都由原始观测核对；没有以直接调用 Editor/Proto 方法代替 OS 输入。

#68 的发布来源分类失败、#72 的原生关闭按钮消歧和 Weston 输出发现失败均保留在完成记录中；#79 在修复后重新运行了完整链。早先 `8fc5f4e` 的 344 项 Rust 和 18 项 OS 结果保留为历史基线，不用来替代本次 app/worker 的验证。原始 OS artifact 与摘要的独立读回、摘要和源身份校验见 [机器记录](NATIVE-INPUT-2026-11-22.json)；[CI 原始证据](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/artifacts/14756497529) 按 14 天策略保留至 2026-12-06。

本次真实窗口验收发生在 Linux；它不等于物理触摸设备、IME、屏幕阅读器或 macOS/Windows 的 OS 输入验收。上游 PR #1710 的相关测试和真实数值链已经验证，该段记录仅对应 #79 当时的提交与运行；后续上游状态和本轮迁移验收应查看各自 PR 的最新 Checks，不能以历史结果替代。

## 上游依据

上一轮数值辅助功能修复 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710) 已合入上游 main；本次迁移的通用能力与修复通过 [PR #1730](https://github.com/Proto-UI/Proto-UI/pull/1730) 回馈。它们包括条件辅助功能、共享 Input 编辑、原生通知/焦点/初次 hover 生命周期、真实滚动引擎绑定，以及同时兼容默认与现代 GPUI 的文本选区通知，详见 [迁移记录](MIGRATION-2026-11-23.md#上游回馈)。上一轮 pin 将这些能力锁定到 `5912f817581f89785e75c26e324a76c97257dc60` 并完成当时的 fresh 检查。已合入的 #12 使用 [PR #1731](https://github.com/Proto-UI/Proto-UI/pull/1731) 中适用的修复，并固定到 `4e499ae25e825e130e740f16af66d70b23a743cf` 完成 fresh 重新生成。后续 compiler pin 更新仍需新的 receipts 与消费者验收，不能用旧 source audit 或历史运行替代。

- [Compiler README：宿主组成、workspace GPUI、共享 SDK 和证据边界](https://github.com/Proto-UI/Proto-UI/blob/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/packages/compiler/README.md)
- [compile.ts：compileFile、来源图和 nativeSdkPath](https://github.com/Proto-UI/Proto-UI/blob/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/packages/compiler/src/compile.ts)
- [rust-data.ts：类型化标准库决策函数与来源图](https://github.com/Proto-UI/Proto-UI/blob/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/packages/compiler/src/rust-data.ts)
- [GPUI 数值辅助功能契约与验证范围](https://github.com/Proto-UI/Proto-UI/blob/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/internal/compiler/gpui-accessibility-value.md)
- [Base Slider Root：承载语义的 slot 原型](https://github.com/Proto-UI/Proto-UI/blob/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/packages/prototypes/base/src/slider/root.proto.ts)
