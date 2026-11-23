# Proto-UI / GPUI 修复完成与验收

日期：2026-11-22。原始审查基线为 `dd0db139656322061fa51daf12cbc8a992c0cd8c`。本记录的修复实现 head 为 **`a55bdcd4cc73e3a48cdfc956151e9934cc865f48`**，已包含与 main `91a775546ebd21a87d76d76568195a6513437719` 的语义合并。

**[Frontend #79](https://github.com/OpenGreendam/verdant/actions/runs/45026004598) 的七个 job 全部成功；[DSH](https://github.com/OpenGreendam/verdant/actions/runs/45026004593) 也成功。** 实际被检出、测试并写入三个发布包的 PR synthetic merge commit 是 `9e6cd34715b9da4b733466b95d749764e5c410b3`。分支 head 与实际 checkout 分别记录。后续文档提交不改变这批证据的归属；最新 PR 检查见 [Verdant #5](https://github.com/OpenGreendam/verdant/pull/5)。

## F1–F7 的完成范围

| 审查项 | 实现及结果 |
|---|---|
| F1：单 GPUI 准入漏检 | 使用真正的 TOML parser，按实际 package 身份处理 alias、normal/dev/build、target-specific、子表、多行和注释。无关依赖的 rev 不再冒充 GPUI pin。声明检查之外，完整 Cargo resolve graph 核对唯一 GPUI package ID、source/rev、唯一 SDK 及所有 workspace roots 的可达依赖。 |
| F2：缺 checkout 跳过实际 pin | 所有离线 workspace/source/receipt/manifest 检查前移；fresh 检查缺固定上游即失败。retained lane 明确声明没有新鲜编译，不能作为 fresh 的替代。Linux/macOS/Windows 均依赖 fresh 与独立 headless gate。 |
| F3：多单元和 SDK 耦合 | producer 支持上游 nativeSdkPath；pitch 与 normalized knob 共用唯一 SDK，pin 绑定其 owner、路径、ABI 和内容。note/control 使用独立 data target。未复制同名 SDK，也未手改生成 Rust。 |
| F4：headless session 反向依赖 GPUI | **完成依赖层解耦。** 两个 planner 均从同一 TS 真相由上游生成稳定命名、零 Cargo 依赖的 Rust crate；稳定 typed protocol 隔离生成 API，不含 GPUI/SDK/native owner/serde/JSON。session 的完整可达图无原生 UI 依赖，并在未安装 GPUI 原生库的独立 Rust 镜像中实际构建、测试。 |
| F5：输入和生命周期分散 | 所有权明确的 capture 与 lease 取消、终止前最终 sample、窗口边界和非有限坐标检查、touch/mouse 竞争、跨窗口隔离和实时目标校验；control/note 边界使用类型化结果与完整资源契约校验。新增真实有状态 normalized-knob，承担公开值、取消、键盘、焦点与数值辅助功能契约。 |
| F6：测试无法支持保证 | 82 项声明/依赖图 fixture、真实固定编译器、无原生 UI 的 session 构建、TS 来源变异消费者、完整 Linux Rust workspace、三平台构建/发布，以及 X11/Wayland 真实输入和 AT-SPI 协议验收形成分层证据。 |
| F7：工具链、来源和发布边界 | 核验实际 Node/pnpm/已加载 TS/tsx 和安装 lock；记录完整来源解析闭包。发布只接受完整 inventory，使用唯一 staging、owner 互斥和可恢复 journal。完整 owner 与初始 journal 原子公开，真实子进程 SIGKILL、竞争获取和旧恢复/新 owner 交接回归均通过。 |

没有另写 Rust planner 造成双真相，也没有把整个 DAW canvas 迁进 TS。音乐坐标、绘图、命中索引、工程 revision 和领域写入保留在 Rust；`normalized-knob` 是本次完成的有状态 Proto 组件切片。其他原生自定义 surface 的边界如实公开。slot、自绘与 detached data method 本身均不是违规或伪编译的证据。

## 固定版本和实际产物

| 项目 | 身份 |
|---|---|
| Proto-UI compiler | `5712266648fdbce2f6662636cb83f01eb849adb1` |
| Rust data target | 上游 [PR #1694](https://github.com/Proto-UI/Proto-UI/pull/1694)，复用其 backend |
| 数值辅助功能扩展 | 上游 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710)，固定上述 commit |
| GPUI | `cyjin-yl/z3rm@e5f2f9db75e97613232b5c1946333e61fa3828ab`，版本 0.2.2 |
| SDK content SHA-256 | `f83fcd1dbf8bee0a3de1c8f1f8e298d90c60b7544191b69d6dd83ab3a160e774` |
| Cargo.lock SHA-256 | `c903aad1d309fa1a26f3b842cf7f28a1f26b9dadd319cc3dbb130cba892690ee` |
| 编译执行工具链 | Node 24.22.3、pnpm 10.32.1、TypeScript 5.9.3、tsx 4.21.0；Rust/Cargo 1.98.1 |
| 本次 OS 实际 app SHA-256 | `273e3ea6fa8015d436ea885ccc7eaf721c4e80d71950ae60d1659d412793cf20` |
| 同次构建的 worker SHA-256 | `c4542e8a6f151c7b2f9556e0008547646c4a55ce180eda3ffdf8e40963765033` |

完整 frozen installation 后，固定 compiler 新鲜发射 **31 个 artifacts**，逐字节匹配保留制品：pitch/native SDK 20、normalized knob 5、note data 3、control data 3。SDK owner 为 pitch-gesture-surface；两个 data crate 的 manifest 均无 dependencies。这里证明来源发射一致性，不把保留源码数量或其哈希称为最终机器码的可复现构建证明。

## 自动验证：最终 #79

| 检查 | 实际结果 |
|---|---|
| TOML / Cargo admission fixture | **82 通过** |
| Node / producer / authored Runtime | **90 通过，0 失败、0 跳过**；使用真实固定上游。publication 和 producer 定向检查各为 21/21，是完整测试中的组成部分 |
| fresh / retained | 两者通过；fresh 的 31 个制品一致，retained 明确报告 freshCompilation: false |
| 完整 Cargo 图 | 一个 GPUI package ID、一个 native SDK；session 完整可达图 68 个 package IDs，原生 UI 依赖为 0；两个 data 节点各为 0 依赖 |
| [独立 headless image](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219335550) | 未安装 GPUI 原生库，独立 target；session 27、protocol 6，**33 passed / 0 failed / 0 ignored** |
| [Linux workspace](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956596) | **1118 passed / 0 failed / 15 个既有 ignored**；94 个单元/集成测试 suite，另有 21 个零项 doctest suite |
| Linux app / domain / session / protocol / release | **206 / 108 / 27 / 6 / 13 项通过**，均包含于上述 1118；app 零 ignored |
| [macOS ARM64](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956666) | **102 passed / 0 failed / 2 ignored**：选定 app 53、session 27、protocol 6、worker unit 12、voice_package 4 |
| [Windows x86-64](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137219956587) | **106 passed / 0 failed / 2 ignored**：选定 app 53、session 27、protocol 6、worker unit 11、voice_package 4、Windows path 5 |
| 离线 Python helper | **11 / 11**；选择器、scene/contact、CI 完整性和身份检查；不启动 OS 窗口 |
| [Linux 真实窗口与 AT-SPI](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/job/137223338246) | **22 / 22，0 skip**；X11 与 windowed Wayland 各 11，详见下节 |

这些数字不重复相加。独立 headless lane 是相同 session/protocol 测试在无原生 UI 环境中的复验；macOS/Windows 编译了 app 测试 binary，但只执行 workflow 明列的 53 项：theme 9、capture 3、knob 16、devices 8、roll 8、pointer boundary 9，不能写成它们各执行了完整 206 项 app 测试。

Linux 15 个既有 ignored 分布为 plugins 7、service 2、synthesis 4、worker 2。源码前提涉及已构建的 plughost、真实 effect/test CLAP bundle、商业 factory preset corpus、用户提供的原始 bank/token/model，以及一项要求 native ORT 和匹配 worker pin 的测试。没有源码标注以物理硬件缺失为忽略原因，不自行改称“硬件测试”。macOS/Windows 的两项 ignored 为同样的原始 bank/ORT worker 测试。

三个最新宿主回归在 Linux 完整 app 测试中均明确通过：空白 graph 双击尊重 live touch 会话；toolbar close 在 detach 前取消 held knob、晚到 up 不提交；toolbar activation 不取消另一个窗口或后来替换的 lease。

### 三平台发布

三平台均构建真实 native executables，并完成 package 与 source-offer 检查，发布判定均为 **allowed**。每次检查含一个 package 和一个 source offer；共同 source commit 为 `9e6cd34715b9da4b733466b95d749764e5c410b3`。

| 包 | Payload / 校验成员数 | Archive SHA-256 |
|---|---:|---|
| verdant-0.1.0-linux-x86_64.tar.gz | 42 / 735 | `3ebb0439398289309d509170700ba5a51a3662d79afd840267ed86f5896e14a1` |
| verdant-0.1.0-macos-aarch64.tar.gz | 40 / 733 | `f8b0b0c22706e30a506b55893e23f8773d7b76685a3a40762ffe34a5366476a6` |
| verdant-0.1.0-windows-x86_64.zip | 39 / 732 | `d9324e7f2b4226eb03b3ed1c4e7a7fb290498bffd18ed296dbe502f49842be3d` |

### TS 来源变异确实改变 Rust 消费者

同一固定 compiler 将 fine sensitivity 从 0.1 改为 0.2 后，实际编译 consumer 的结果从 **0.55** 变为 **0.6**。variant 若仍断言旧值 0.55，实际 **exit 101**，负例按预期失败。两个 data artifacts 均声明 dependencies: []。

| 变体 | 生成 lib.rs SHA-256 | 实测 |
|---|---|---|
| baseline | `6deeb7570506d5f0fb5bac51662e444f44e71ebf446b1828e64418ea6f1fae2a` | 0.55 |
| variant | `d0ad540782c1c508164c24a79f80fb20a153d58167de6759094ca22a52717973` | 0.6 |

实验使用生成源码摘要区分 Cargo target 子目录。最初共用 target 的尝试暴露了稳定 package 名与相同解包时间戳可能复用 baseline 缓存的问题；已保留失败，修复隔离后重新执行两变体和负断言。没有靠删除共享缓存掩盖问题。

## 真正窗口、辅助功能与持久化：22 项通过

Ubuntu 24.04 上，X11 **11/11**，windowed Wayland（Weston **13.0.0**）**11/11**，无 skip，分别用时 **111.849 秒 / 114.044 秒**。两个后端运行上述相同 app/worker 字节，各保留 **40 个观测步骤**，总计 80 个 step JSON、截图与保存工程观测。每个后端最终工程均为 **−8 dB / revision 12**。

X11 的输入链为 XTest → X11 → Verdant；Wayland 为 XTest → 独立嵌套 Weston → 真实 Wayland 输入 → Verdant。Wayland app 移除 DISPLAY，并保留实际 wl_pointer/wl_keyboard 消息。脚本不调用 Editor 方法、Proto exposed 方法或 GPUI 测试注入。AT-SPI 使用实际 Value、Focus 和 advertised Action 接口。

| 每个后端的场景 | 实际观测 |
|---|---|
| Keyboard | 保存约 −7.7 dB / r1；一次 Undo 恢复 −8 / r2 |
| Drag / final release，54 px | X11 1.0001424153645821 dB、Wayland 0.9999999999999964 / r3；一次 Undo 到 −8 / r4 |
| Escape | held preview 取消，晚到 up 后仍 −8 / r4，保存字节不变 |
| Focus | 真实辅助功能焦点转移取消预览；失谐节点实际 focused，音量节点非 focused，revision/hash 不变 |
| Exit | 真正窗口外 motion 与 release；取消，revision/hash 不变 |
| Detach | 真实 toolbar Action 移除输入子树，晚到 up 不写入，恢复后仍 r4 |
| SetValue | 实际 Value.CurrentValue = 0.63，回读 0.63；保存 −5.100000000000001 / r5；一次 Undo 到 r6 |
| SetValue during drag | 在 up 前已保存 −5.1 / r7；额外 move 和晚到 up 后修订及文件 hash 不变；一次 Undo 到 r8 |
| Lease reuse | 18 px 重新编辑约 −5 dB / r9；一次 Undo 到 −8 / r10 |
| Native window close | 真实 native window 的关闭 push button 取消 held preview；晚到 up 仍 r10、同 hash。重开后真实拖动提交 r11，一次 Undo 到 r12 |
| Native preset cancellation | 真实自定义 combo box 打开 preset menu，先恢复 preview；held Escape、晚到 up、关闭窗口后仍 −8 / r12、同 hash |

对位移拖动采用原先明确限定的 **0.001 dB** 像素容差；键盘/辅助功能写值、取消、revision 和保存 hash 仍严格检查。每次成功编辑要求一次领域修订、一次 undo；取消要求值、修订及完整工程字节不变。每次保存还要求发生实际文件替换，不能拿旧文件冒充新保存。

独立读回核验下载 ZIP SHA 与 GitHub artifact digest、日志标记内摘要与 artifact summary、两后端全部 step JSON/PNG/verp 的 hash、manifest/source-selection/program/script 身份。未修改的 ci.validate_run 在本地重建结果，与 CI result.json 严格相等。每后端另外直接比较六个取消路径及一次 takeover late-up 的前后完整文档 hash，全部相同。mtimeNs 超过 JavaScript 安全整数范围，机器记录保留原始日志 JSON 的数字 token，未经 parse→stringify 重写。

当前原始 [artifact 14756497529](https://github.com/OpenGreendam/verdant/actions/runs/45026004598/artifacts/14756497529) 为 **17,443,110 bytes**，ZIP SHA-256 为 `588dd0087e554d621b40f0af2e860815a0c4175b5a5b7da8ccabed7feb967c5b`，按 CI 策略保留至 2026-12-06。完整当前观测与失败过程见 [NATIVE-INPUT-2026-11-22.json](NATIVE-INPUT-2026-11-22.json)。原始成功/失败 ZIP 与完整日志另已保存，未只保留最终截图。

## 关键实现边界

### 类型化协议和宿主所有权

protocol 先完整解码，再检查 Begin/Release 与活动所有权对应、Idle 不产生已占有的 preview/commit、note 清理/释放顺序，以及 control 终止恰好 commit 或 discard 后释放。它验证资源契约，决策仍来自生成函数。

普通 roll/曲线使用带 window/hitbox/generation 的 OwnedPointerCapture；pitch/knob 使用上游 MoveGesture lease。终止事件在 capture phase 接收最终 sample，非有限或真实 viewport 外坐标取消。touch 使用实际 touch arena，与 mouse/lease 通过 contact 所有权协调。lane、automation、mixer 和数值 scrub 同样检查窗口边界与目标，但仍使用原生 capture API；没有宣称全部迁入同一 capture 类型。

Wayland implicit grab 会延迟 leave 到 MouseUp，却继续投递窗口外坐标。现在以实际 viewport 检查 move 与 terminal sample；SDK 目标局部坐标先换回窗口坐标。控件 hitbox 外但窗口内仍可拖动。实际 Wayland 日志记录了窗口外点 `(-20, 460)` 和释放，不拿被 compositor 输出钳制的坐标当作成功。

每个 contact 记录来源窗口及 lease 的弱引用，命令执行前同步取消同窗口 lease；窗口 A 的晚到 blur/cancel 不能清掉窗口 B 或新 generation 的会话。领域/项目变更按其全局语义取消。参数写入前重新检查 live target、revision、权威值及当前 menu/modal/prompt/plugin-busy 等 host policy，不只信上一帧 props。

### 有状态旋钮及真实语义命令

normalized-knob 作者源码承担 props、状态、值、取消、键盘、焦点、Slider 范围和有限数值 setValue。提交是受控提案，宿主接受才发布新值；拒绝和重入不会留下未接受的 preview。SDK 数值动作修复在上游模板完成，再由 pin 重生成。

GPUI 默认辅助功能 Click 可能合成 MouseDown/MouseUp，误把 toolbar 坐标当活动拖动的最终 sample。四个视图切换按钮、原生插件 toolbar 的 11 个按钮和相关 dropdown/参数命令现在显式执行共享语义动作；先取消同窗口 live lease/raw graph preview，再执行原命令。新 Rust 回归使用真实生产路径；新增 native-close/preset OS 场景另验证实际 AT-SPI transport。

与 main 的合并保留 Track/Master 效果链、独立插件窗口、精确输入、物理步长/范围、重置、调制拖放和 Adwaita 绘图。独立窗口可编辑未选中的轨道，docked editor 仍要求当前目标；instrument 身份纳入 document generation。没有以回退到旧原生旋钮状态机解决冲突。

### 发布初始化、恢复与来源选择

完整 owner 先写入唯一 UUID 私有 state，再以 create-only hard link 原子公开锁；完整初始 journal 同样先写入再原子公开，早于任何 staging 写入。release/recovery 核对捕获的 owner identity，旧操作不能删除新 owner 的锁。完整旧目录锁可恢复；缺 owner、未知内容、伪造路径和符号链接仍拒绝。

21 项 publication 回归包含真实子进程 SIGKILL 初始化窗口、并发 acquire、迟到 release/recovery 和 legacy lock handoff。未公开残留只孤立保留，不猜测扫描删除。此保证针对进程中断和并发，不声称 fsync 断电持久性或跨目录原子快照；Cargo 等未持 producer 锁的读取方不得与再生成并行。

发布来源精确允许 8 个验证 Python 路径（a11y 两个、native-input 六个），没有放宽整个 tools 或一般 .py。真实临时 Git 仓库回归经过 select/load、source archive 和既有 archive guard；邻接/嵌套未知 Python、未跟踪或变动来源继续拒绝。release crate 13 项测试和三平台实际包检查均通过。

## 保留的失败与历史基线

| 尝试 | 实际结果与后续处理 |
|---|---|
| 历史 `8fc5f4e` | 344 项选定 Rust 测试通过（app 203、domain 108、session/protocol 33），真实 X11/Wayland 各 9，共 18/18。旧二进制身份单独保留于机器记录，不冒充本次 22 项 |
| [#68 Linux](https://github.com/OpenGreendam/verdant/actions/runs/45022399294/job/137208708007) | 1115 passed / 0 failed / 15 ignored、app 206 通过；随后 tools/a11y/observe-atspi.py 未分类使打包失败，OS job 因依赖失败 skipped，没有执行 OS 场景 |
| 来源分类修复 | 增加三项真实 Git/source archive 回归；release 13/13。#72 和 #79 均完成实际打包 |
| [#72 OS 首次 CI](https://github.com/OpenGreendam/verdant/actions/runs/45023915036/job/137216128900) | 原九项 X11 通过；新增 native close 因一按钮与两个 radio choice 同名而选择不唯一。Wayland 在 app 启动前因 Weston 13 输出标题发现失败而中止；没有把它写成产品场景失败 |
| 两项 harness 修复 | Close 限定实际 push-button role；从自建 Weston 日志读实际输出 ID，并与 X11 几何核对。保留回归后，#79 重新完整通过 22 项 |
| 本地 Rust 替代环境 | domain/session/protocol 141 项通过；app test/check 分别在 x11rb-protocol / ash 编译时 SIGKILL，原因未证实为 OOM，不能算本地 app 通过。后来真实 CI 成功另行记录 |
| 本地 OS 替代环境 | 官方 Debian D-Bus/GI 依赖可加载，但 socket(AF_UNIX, SOCK_STREAM) 返回 EPERM，未启动 app。没有绕过限制；独立 Ubuntu CI 完成真实窗口验收 |

#72 原始失败 [artifact 14754927877](https://github.com/OpenGreendam/verdant/actions/runs/45023915036/artifacts/14754927877) 为 6,634,808 bytes，SHA-256 `074a9ff4e7f4bdebb7736ce465bd49409bb30b6ddf3ce39be7e26a9cb04226eb`。失败和成功的原始日志均保留。

更早 OS 调试中，真实树缺 Value interface 的 SDK numeric fields、toolbar 默认 Click 导致误提交、Wayland implicit grab 下窗口外取消已修复。另两项属于 harness：held Escape 不应使用先释放鼠标的 --clearmodifiers；D-Bus setter 必须为 ssv/Double variant。未把 harness 错误误修成产品行为。ProjectLease 的并行 Busy 另以显式 unlock 后 close 和保留克隆描述符的确定性回归处理，未声称已定位某个继承 fd 的子进程。

## 证据边界与上游状态

当前 22 项证明的是 Linux 的实际窗口/协议输入、AT-SPI 数值/焦点/动作、持久化和单次 undo。它不等于物理触摸设备、实体读屏器可用性、IME、真实音频硬件，或 macOS/Windows 的 OS 输入验收。其他平台/设备范围仍见 [PLATFORM-ACCEPTANCE.md](../docs/PLATFORM-ACCEPTANCE.md)。

固定 GPUI 的 SCREEN/frame extents 仍可能没有有效全局位置；probe 保留原始 SCREEN/WINDOW 值，并用标准 WINDOW 坐标加实际 X11 window origin。Wayland 从实际 Weston scene graph 按 app PID/app_id/buffer 识别 origin 和 configure 尺寸，外部点在 compositor 输出内、应用矩形外。这是可审计的坐标换算，不声称修复 GPUI 全局几何。

上游 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710) 保持 draft。最终 pin `5712266` 的[轻量 CI](https://github.com/Proto-UI/Proto-UI/actions/runs/45017285566) 成功，但该触发跳过重型 browser/native jobs。此前 `2685caf` 的[完整手动矩阵](https://github.com/Proto-UI/Proto-UI/actions/runs/45012891567) 中，类型检查、Linux native 和 macOS native 成功；test-general 和多个 browser/visual jobs 失败。

已确认属于本次数值修复的诊断注册和 pointer alias 断言已在最终 pin 修复，相关 87 项测试通过；另两个 compiler 失败已在真实 merge-base `16393dd` 的隔离 frozen 环境重现。其他失败未逐项归因，不能统称无关，也不能宣称上游整仓全绿。Verdant 的固定来源 fresh 编译、Rust/native 和 OS 验收各自记录，不相互替代。实验性 compiler profile 不被宣传为稳定公开 compiler 或零运行时保证。

脚本、依赖准备和失败保留规则见 [native-input](../../tools/native-input/README.md) 与 [a11y](../../tools/a11y/README.md)。首轮历史记录保留在 [VALIDATION-2026-11-22.md](VALIDATION-2026-11-22.md)。
