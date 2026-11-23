# Proto-UI 与 GPUI 集成

Verdant 使用 Proto-UI Compiler 实现控件交互，并保留 GPUI 的 Adwaita 绘图和应用领域层。实验性 `gpui-source-v1` 生成原生组件，`rust-data-v1` 生成只依赖 Rust 标准库的类型化决策函数。保留的 Rust 源码必须由上游 compiler 发射；不要手改 `generated/`。发布使用的版本、完整 GPUI 来源、工具链要求、共享 SDK 内容和各编译单元的目标由 [compiler.pin.json](compiler.pin.json) 绑定。

## 当前边界

本迁移的作者源码包含 **12 个 `gpui-source-v1` 原生编译单元和 2 个 `rust-data-v1` 决策单元，共 14 个编译单元**，具有原审计 18 类交互的实际消费路径。编译单元数与交互类别数不是同一种口径；一个共享控件会被多个界面复用。调用点、宿主职责和验收状态见 [完整控件迁移记录](MIGRATION-2026-11-23.md)。最终 pin、来源 receipts、生成目录和 Cargo 依赖必须与这份 14 单元清单一致；来源清单不能代替 producer 的最终 fresh 检查。

| 原生组件 | Proto 源码拥有的职责 | 主要原生消费者 |
|---|---|---|
| `pitch-gesture-surface` | 原生根、尺寸/裁剪、slot 与 pitch 手势入口；共享 SDK owner | `proto_roll.rs` |
| `normalized-knob` | 值/phase、受控提交、复位、取消、键盘、焦点、Slider 辅助功能、精确输入请求 | `proto_knob.rs`、内置设备编辑器 |
| `action-control` | 按压/激活、禁用、选择/展开、上下文请求、键盘与辅助功能操作；目标变化取消旧按压，实时标签更新保留相同命令 | `proto_controls.rs`、按钮、离散控件、Tabs、窗口标题操作 |
| `collection-control` | 游标、选择、遍历、展开、搜索结果导航、上下文请求、滚动/最近边缘揭示与过期 revision 拒绝 | `proto_collection.rs`、菜单、音符表格、点列表、轨道/历史/浏览器列表 |
| `continuous-control` | 数值预览/提交、细调、复位、取消、键盘与数值辅助功能 | `proto_scalar.rs`、推子/声像、数值字段、分隔条、时间线滚动条 |
| `editor-surface` | 手势 phase/阈值/身份、取消、焦点、滚轮与缩放策略 | `proto_editor_surface.rs`、编曲/卷帘/曲线/自动化/标尺/EQ 绘制 |
| `performance-key` | 按键触发、保持/重复、松开与失焦停止、辅助功能激活 | `proto_performance_key.rs`、参考键盘与演奏键盘 |
| `drag-source` | 可消费一次的拖放 lease、revision、取消、键盘/辅助功能目的地选择 | `proto_drag.rs`、调制映射 |
| `overlay-control` | 可见性、默认/取消响应、焦点进入/约束/恢复 | `proto_overlay.rs`、模态与导出窗口 |
| `text-field` | 文本/IME/选区/撤销、焦点、提交/取消/前进与数值文本策略 | `proto_text.rs`、歌词、精确值、工作流、插件搜索 |
| `graph-point` | 每个图形把手的双轴预览/提交、阈值/细调、复位、键盘轴切换与辅助功能 | `devices/graph_control.rs` |
| `scroll-region` | 文档/视口滚动请求、页面/边缘按键、最近边缘揭示与可选焦点范围 | `proto_scroll.rs`、说明、表单、参数泳道及原生滚动引擎 |

`note-gesture-planner` 与 `control-gesture-planner` 继续由 `rust-data-v1` 生成标准库决策函数。`verdant-ui-protocol` 和 headless session 不链接 GPUI、组件 owner 或 JSON runtime；音乐守卫、事务和效果执行留在领域层。

宿主保留 Adwaita 绘图、音乐命中与单位换算、参数/工程事实、音频、撤销事务和操作系统窗口/指针/拖放运输。`proto_parent.rs` 在 native slot 的 layout/prepaint/paint 阶段提供显式逻辑父 owner，编译子根在实际 render 时绑定，避免把视觉嵌套误当成已经成立的焦点树。

滚动接入包含 **21 处应用 `proto_scroll::viewport` 调用点**，并保留 **5 处显式绑定真实引擎的原生滚动容器**：预设结果、混音轨道条、上下文菜单、插件结果和模态正文。数字按应用源码构造点计数，不是运行时实例数。`verdant-app/src` 已没有生产代码直接调用 `scroll_to_item`；音符、控制点、历史和参数泳道的显示位置由编译源的滚动策略决定。

虚拟列表通过上游 `attach_scroll_engine` lease 直接绑定 `UniformListScrollHandle` 的真实 `base_handle`。源组件的快照和请求使用同一引擎；编译根不再额外创建一个滚动视口。宿主滚轮回调只报告真实输入，原生布局提供可见及未实例化行的测量事实；不能在两个不同 ScrollHandle 之间复制位置来伪造接入。ScrollSurface 的公开 `position` 是 `[0, 1]` 归一化比例，原生像素 offset 另行核对。

文本使用上游共享 `asInputEditor`，并保留[精确来源](src/upstream-input/PROVENANCE.md)与 [MIT 许可证](src/upstream-input/LICENSE)。它具有实际 Input 编辑语义；应用使用直接辅助功能命名。需要独立 Label 关联的完整 `asInputRoot` 仍保留真实 ControlLabel 协议，本文不宣称 GPUI 已支持该独立能力。

临时预览与最终提交由编译源区分，宿主写入前再核对实际工程、目标和 revision；策略或目标变化、取消、卸载和同步回调可使旧操作失效。实时辅助功能名称或描述的更新不会单独替换命令身份。拖动中的临时值、列表选择和键盘/辅助功能动作都经过相应编译实例。复用旧编译单元的历史 OS 验收，不能替代这些新增控件的当前提交验证。

曲线图在 prepaint 时另外保存这一帧对应的权威参数快照。若参数或 revision 在绘制后、按下前改变，旧帧的拖动、右键菜单和双击写入都会被拒绝；重绘后可以正常编辑。动画和预览值只决定绘制及命中位置，拖动起点必须来自已接受的权威参数，避免把动画中的旧值写回工程。

## 再生成与两类检查

当前主线把 GUI binary 命名为 `verdant-gui`，`verdant` 是 CLI；下面的 app 测试命令使用 GUI target。后文 #79 的结果只属于上一轮旋钮集成的原始 checkout 与包格式，不代表本次完整控件迁移的最终候选。

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

`native-input` CI job 依赖同一 run 的 Linux 打包结果，下载经过发布门禁的 app/worker。它用包内 `verdant-release` 检查 archive、解包后的内容和当前 checkout 的 source selection，再执行 X11 与 windowed Wayland 各 11 项真实输入场景。完整性门禁核对二进制字节、全部场景和原始记录，缺项或 skip 都失败；失败时仍保留原始证据。历史 #79 的这条链实际通过 22/22，见后文历史验收表和 [原生输入记录](NATIVE-INPUT-2026-11-22.json)；新增控件不能继承该结果。

## 来源闭包和共享 SDK

producer 按各单元的 target 调用上游 `compileFile` 或 `compileRustDataFile`，共用 `FileSourceGraph`，记录仓库边界内的入口、导入 helper、包解析输入以及未命中的候选路径；已有的仓库外来源会被拒绝。编译前后核对来源字节与解析图，避免只哈希入口却遗漏 helper 或 exports 变化。

`pitch-gesture-surface` 发射唯一的 `.proto-ui/gpui-native`；其余 11 个原生组件通过上游 `nativeSdkPath` 引用它。两个 data 单元不使用 SDK。pin 绑定 owner、路径、package/version、ABI feature 和 SDK 内容摘要；data target 则绑定稳定 crate 名、公开方法和空依赖集合。增加原生单元时必须复用 SDK，不能复制同名 package，也不能改生成 Rust 来修 Cargo 冲突。

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

本次迁移的源码、编译产物、依赖图和原生框架行为验证已完成，绑定应用源码快照 [`f325638d`](https://github.com/OpenGreendam/verdant/commit/f325638dcd7cb4169e52d9eff7796825eccd4ebb) 与编译器 [`5912f817`](https://github.com/Proto-UI/Proto-UI/commit/5912f817581f89785e75c26e324a76c97257dc60)。执行范围、来源身份和日志摘要见 [最终机器记录](VALIDATION-2026-11-23.json)，职责清单、证据层次和历史修复见 [迁移记录](MIGRATION-2026-11-23.md#验证与证据层次)。

| 最终检查范围 | 实际结果 |
|---|---|
| 严格 producer | 14 个单元、81 个编译产物的 fresh 与 retained 检查均通过；保留完整来源 receipts、唯一 SDK 和 GPUI 身份 |
| 应用工具与作者 Runtime | 82 项规则检查通过；完整 Node 测试 173/173，通过且 0 skip，其中作者 Runtime 88 项、工具回归 85 项 |
| 上游源码与类型 | 聚焦源码回归 206/206；TypeScript 检查退出码 0 |
| Verdant 使用的现代 GPUI | 原生 SDK 26/26；完整 `verdant-gui` suite 237/237，均无失败、忽略或筛选掉的测试 |
| Cargo 图与 headless | 完整 locked/all-features resolve graph 通过，唯一 GPUI/SDK，session 原生依赖为 0；session、protocol 和两个 data crate 的本地测试共 33 项通过，此次本地运行不声明基础镜像隔离 |
| 实际 Rust source mutation | baseline 实测 0.55、variant 实测 0.6；variant 使用旧 0.55 预期时按要求断言失败，实验整体通过 |
| 上游默认 GPUI 的托管 CI | [run 45071732309](https://github.com/Proto-UI/Proto-UI/actions/runs/45071732309) 在 macOS 15 和 Ubuntu 24.04 上分别通过原生消费者 39/39、数值辅助功能 8/8、SDK 27/27；两边均执行了旧版宿主的真实选区通知回归 |

这些结果分别证明源码语义、生成 Rust、依赖边界和框架输入行为，不相加为“总验收项”。默认 GPUI 的 27 项 SDK 回归与 Verdant 现代 GPUI 的 26 项使用不同宿主 API 分支；后者不能代替前者。当前候选的逐屏视觉、打包后 OS 输入、物理触摸、系统 IME 与读屏验收仍按平台单独记录，不继承下面 #79 的历史成功状态。

上一轮旋钮集成的历史版本见 [首轮验收记录](VALIDATION-2026-11-22.md) 和 [后续修复验收](VALIDATION-COMPLETION-2026-11-22.md)。后文历史表固定到实现 head `a55bdcd4cc73e3a48cdfc956151e9934cc865f48` 和 [Frontend #79](https://github.com/OpenGreendam/verdant/actions/runs/45026004598)；实际被检出、测试和打包的 PR merge commit 是 `9e6cd34715b9da4b733466b95d749764e5c410b3`。关联讨论位于 [PR #5](https://github.com/OpenGreendam/verdant/pull/5)。这些记录不随当前分支和后续文档提交改写。

TOML admission 使用 parser，按实际 `package` 名检查普通/build/dev 和 target-specific 的所有依赖表，支持 alias、子表、多行和注释。它检查声明；另一个门禁调用完整 `cargo metadata --locked --all-features`，沿全部 workspace members 的 resolve graph 检查 GPUI package ID/source/rev 和唯一 SDK，同时按实际 workspace/manifest 身份追踪 session 的所有可达边，拒绝原生组件、GPUI、SDK 和图像支持。两个 data crate 必须可达且没有依赖；不能改个包名躲过路径核验。`--no-deps` 或扫描 Cargo.lock 字符串不能替代这一检查。

```sh
corepack pnpm@10.32.1 --dir frontend/proto-ui test
PROTO_UI_UPSTREAM="$PWD/../Proto-UI" corepack pnpm@10.32.1 --dir frontend/proto-ui test
(cd frontend && cargo fetch --locked)
node frontend/proto-ui/tools/check-cargo-gpui.mjs --offline

(cd frontend && cargo test --locked -p verdant-ui-protocol -p verdant-session)
(cd frontend && cargo test --locked -p verdant-app --bin verdant-gui)
```

离线 Cargo graph 检查前，需要在 `frontend/` 中执行过 `cargo fetch --locked`。Rust 测试进入该目录执行，使 rustup 采用其中的固定工具链。

设置 `PROTO_UI_UPSTREAM` 的测试覆盖真实 compiler 发射和来源变异，并把作者原型放进 pinned `createRuntimeSession`，通过受控 host capability 检查动作/集合、连续值、文本/IME、编辑表面、图形、拖放、滚动、拒绝更新、同步接受、重入取消和生命周期。这些 JavaScript Runtime 测试不经过 GPUI，也不代表 OS 输入测试；无 checkout 时相关用例会明确跳过，不能把跳过计作通过。完整 GUI Rust suite 另外覆盖类型化协议、宿主 capture 和实际挂载的生成组件；仅运行旧旋钮筛选集不足以验收本次迁移。

CI 还准备 baseline/variant 两套完整 data-target 编译产物，用同一类型化 Rust consumer 执行：TS 中 fine sensitivity 从 0.1 改到 0.2，Rust 函数的实测结果必须从 0.55 改到 0.6，旧期望必须失败。每个变体使用由生成源码摘要区分的 Cargo target 子目录，避免相同 package ID 和解包时间戳错误复用另一变体的构建缓存。这验证 TS 来源确实影响已编译的消费者行为，不涉及窗口输入。

```sh
node frontend/proto-ui/tools/check-native-source-mutation.mjs --prepare .ci/proto-ui-source-mutation --upstream ../Proto-UI
node frontend/proto-ui/tools/check-native-source-mutation.mjs --run .ci/proto-ui-source-mutation
```

保留字节一致、Cargo graph、Rust trace、GPUI 框架事件、OS 窗口输入和物理设备分别提供不同证据。哈希或截图不能代替取消/捕获正确性，GPUI 测试不能替代物理触摸、IME 或屏幕阅读器验收。跨平台剩余范围仍见 [PLATFORM-ACCEPTANCE.md](../docs/PLATFORM-ACCEPTANCE.md)。

## 历史验收：上一轮旋钮集成 #79

| 检查 | #79 的实际结果 |
|---|---|
| Proto 工具与再编译 | 82 项准入 fixture；完整 Node 测试 90/90；31 个保留制品与固定 compiler 新鲜输出一致。90 项包含 21 项 publication 回归，覆盖真实子进程 SIGKILL、竞争获取和恢复交接 |
| Linux Rust | [job 137219956596](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956596)：1118 passed、0 failed、15 项既有 ignored；94 个单元/集成测试套件，另有 21 个零测试 doctest 结果。包含 app 206、domain 108、session 27、protocol 6、release 13 |
| macOS Rust 与打包 | [job 137219956666](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956666)：102 passed、0 failed、2 项可选声库测试 ignored；其中选定 app 测试 53 项。40 个 payload 文件、733 个 archive members，发布检查 allowed |
| Windows Rust 与打包 | [job 137219956587](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956587)：106 passed、0 failed、2 项可选声库测试 ignored；其中选定 app 测试 53 项、Windows path 5 项。39 个 payload 文件、732 个 archive members，发布检查 allowed |
| Linux 打包与来源变异 | 42 个 payload 文件、735 个 archive members，package/source-offer 检查 allowed；真实 Rust consumer 的 baseline 为 0.55、variant 为 0.6，以旧期望执行 variant 得到预期断言失败 exit 101 |
| Linux 真实窗口与辅助功能 | [job 137223338246](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137223338246)：X11 11/11、windowed Wayland 11/11，共 22/22、0 skip，每个 backend 40 个观测步骤；实际包含 native editor 关闭/重开和 preset-menu 取消路径 |

Rust 各 crate 的数量是 Linux 总数中的子集，不重复相加。macOS/Windows 运行 workflow 明列的测试范围，不代表各自执行了 Linux 的全部 206 项 app 测试。15 项 Linux ignored 涉及外部插件/宿主或用户自备声库等既有条件；不能当作它们已经通过。11 项离线 Python helper 回归与 22 项实际 OS 场景分别计数。

该历史 OS 验收复用同一 run 的 Linux 包：archive SHA-256 为 `3ebb0439398289309d509170700ba5a51a3662d79afd840267ed86f5896e14a1`，实际 app 为 `273e3ea6fa8015d436ea885ccc7eaf721c4e80d71950ae60d1659d412793cf20`，worker 为 `c4542e8a6f151c7b2f9556e0008547646c4a55ce180eda3ffdf8e40963765033`。输入经过 XTest→X11 或 XTest→Weston→真实 Wayland；AT-SPI 写值经过实际 `Value.CurrentValue` setter。辅助功能数值读取、持久化、revision、单次撤销、捕获接管和取消都由原始观测核对；没有以直接调用 Editor/Proto 方法代替 OS 输入。

#68 的发布来源分类失败、#72 的原生关闭按钮消歧和 Weston 输出发现失败均保留在完成记录中；#79 在修复后重新运行了完整链。更早 `8fc5f4e` 的 344 项 Rust 和 18 项 OS 结果也保留为历史基线；两批历史结果均不能替代本分支最终 app/worker 的验证。原始 OS artifact 与摘要的独立读回、摘要和源身份校验见 [机器记录](NATIVE-INPUT-2026-11-22.json)；[CI 原始证据](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/artifacts/14756497529) 当时按 14 天策略保留至 2026-12-06。

该历史真实窗口验收发生在 Linux；它不等于物理触摸设备、IME、屏幕阅读器或 macOS/Windows 的 OS 输入验收。当时的上游 PR #1710 记录验证了相关测试和真实数值链，同时记录了整仓 CI 尚未解决的失败和 draft 状态；该历史快照保持不变。#1710 现已合入上游 main，当前迁移的上游回馈和具体 CI 范围见下一节。Verdant 的通过结果不能证明上游整仓全绿。

## 上游来源与历史依据

上一轮数值辅助功能修复 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710) 已合入上游 main；本次迁移的通用能力与修复通过 [PR #1730](https://github.com/Proto-UI/Proto-UI/pull/1730) 回馈。它们包括条件辅助功能、共享 Input 编辑、原生通知/焦点/初次 hover 生命周期、真实滚动引擎绑定，以及同时兼容默认与现代 GPUI 的文本选区通知，详见 [迁移记录](MIGRATION-2026-11-23.md#上游回馈)。最终 `compiler.pin.json` 将这些能力锁定到 `5912f817581f89785e75c26e324a76c97257dc60`，14 个单元均经严格 fresh 检查；不能用旧 source audit 替代该来源身份。

以下链接固定到**历史 pin `5712266`**，用于定位上一轮集成的设计与实现，不能作为本次新增能力的完整依据：

- [Compiler README：宿主组成、workspace GPUI、共享 SDK 和证据边界](https://github.com/Proto-UI/Proto-UI/blob/5712266648fdbce2f6662636cb83f01eb849adb1/packages/compiler/README.md)
- [compile.ts：compileFile、来源图和 nativeSdkPath](https://github.com/Proto-UI/Proto-UI/blob/5712266648fdbce2f6662636cb83f01eb849adb1/packages/compiler/src/compile.ts)
- [rust-data.ts：类型化标准库决策函数与来源图](https://github.com/Proto-UI/Proto-UI/blob/5712266648fdbce2f6662636cb83f01eb849adb1/packages/compiler/src/rust-data.ts)
- [GPUI 数值辅助功能契约与验证范围](https://github.com/Proto-UI/Proto-UI/blob/5712266648fdbce2f6662636cb83f01eb849adb1/internal/compiler/gpui-accessibility-value.md)
- [Base Slider Root：承载语义的 slot 原型](https://github.com/Proto-UI/Proto-UI/blob/5712266648fdbce2f6662636cb83f01eb849adb1/packages/prototypes/base/src/slider/root.proto.ts)
