# 编译规格

Verdant 0.1.0 的完整合同见[主规格](../SPEC.md)，源码入口为
[OpenGreendam/verdant](https://github.com/OpenGreendam/verdant)。本文件规定编译输入、生成物与宿主准入；
工程持久化、事务、worker、音频、性能目标与强制 Pride 条款均适用主规格。

## 输入与所有权

| 路径 | 用途 |
|---|---|
| `frontend/crates/verdant-app/src/generated/controls.rs` | 检入的控件与命令表 Rust 源码。 |
| `frontend/crates/verdant-session/` | Rust 会话、手势守卫与领域副作用。 |
| `frontend/proto-ui/src/pitch-gesture-surface.proto.ts` | 原生根、尺寸/裁剪和 slot；共享 SDK artifact owner。 |
| `frontend/proto-ui/src/note-gesture-planner.proto.ts` | 移动、复制、调整长度和框选的纯决策函数。 |
| `frontend/proto-ui/src/control-gesture-planner.proto.ts` | 连续控制的纯决策函数和归一化拖动计算。 |
| `frontend/proto-ui/src/normalized-knob.proto.ts` | 每实例值/phase、props 同步、取消、键盘、焦点、辅助功能和提交事件。 |
| `frontend/proto-ui/src/action-control.proto.ts` | 按压、激活、布尔/离散选择、Tabs、上下文请求和命令 authority。 |
| `frontend/proto-ui/src/collection-control.proto.ts` | 菜单、列表/表格的游标、遍历、选择、搜索结果导航和滚动策略。 |
| `frontend/proto-ui/src/continuous-control.proto.ts` | 推子、声像、数值字段、分隔条与时间线滚动条的连续输入语义。 |
| `frontend/proto-ui/src/editor-surface.proto.ts` | 编辑表面的输入阶段、阈值、焦点、取消、滚轮与缩放策略。 |
| `frontend/proto-ui/src/graph-point.proto.ts` | 图形参数把手的双轴值、提交/取消、键盘与数值辅助功能。 |
| `frontend/proto-ui/src/text-field.proto.ts` | 组合上游 Input 编辑 hook，拥有文本/IME、选区、提交/取消和数值文本策略。 |
| `frontend/proto-ui/src/overlay-control.proto.ts` | 弹层可见性、默认/取消响应、焦点约束与恢复。 |
| `frontend/proto-ui/src/scroll-region.proto.ts` | 视口滚动、页面/边缘按键、最近边缘揭示与可选焦点范围。 |
| `frontend/proto-ui/src/performance-key.proto.ts` | 虚拟按键的按下、保持、松开、失焦和辅助功能激活。 |
| `frontend/proto-ui/src/drag-source.proto.ts` | 拖放 lease、单次接受、取消与键盘/辅助功能目的地选择。 |
| `frontend/crates/verdant-ui-protocol/` | 生成 API 到 app/session 的强类型边界；只依赖两个无 GPUI 的 data crate。 |
| `frontend/proto-ui/compiler.pin.json` | 上游提交、工具链、目标 profile 和 GPUI 版本绑定。 |
| `frontend/proto-ui/generated/` | 两个标准库 data crate 与十二个原生组件；后者共用一份 native SDK，附 source map。每个单元均含 provenance。 |
| `frontend/proto-ui/receipts/` | 仓库边界内完整来源/解析输入闭包与生成制品的哈希记录。 |

桌面编译上游 compiler 发射的 Rust 控件源码。本迁移有 **12 个 `gpui-source-v1` 原生单元和 2 个 `rust-data-v1` 决策单元，共 14 个入口**；版本、工具链和目标由最终 `compiler.pin.json` 绑定。14 个入口具有原审计 18 类交互的实际消费路径；交互类别数不是编译单元数，也不是功能完成百分比。

卷帘的编译根、EditorSurface 和纯 note planner 分别承担原生入口、输入阶段和音乐会话决策。Rust 会话通过强类型 adapter 调用 data 函数，提供音乐守卫并执行领域副作用；GPUI bridge 运输原生事件、捕获和命中事实。旋钮及其他控件的状态、取消、键盘、焦点和辅助功能由相应有状态编译实例消费。应用保留 Adwaita 绘图 slot、音乐坐标、单位换算、音频预览及经目标/revision 校验的领域提交。覆盖分类与验证范围见 [完整控件迁移记录](../frontend/proto-ui/MIGRATION-2026-11-23.md)；源码接入不等于所有平台已经运行验收。[具体边界](../frontend/proto-ui/README.md)

## 当前版本

上游 Proto UI 提交、包版本、Node 和 pnpm 版本及各单元编译目标由版本锁定文件记录。最终 pin、14 单元来源 receipts、生成目录和 Cargo manifests 必须相互一致；逐单元开发探针不能替代最终 producer fresh 检查。生成的原生组件与 SDK 共用 workspace GPUI 依赖；note/control data crate 无 Cargo 依赖，稳定 crate 名和导出方法也绑定在 pin 中。实际 Rust 依赖版本见 Cargo.lock。[compiler.pin.json](../frontend/proto-ui/compiler.pin.json)

本轮完成验证的应用源码快照为 `f325638dcd7cb4169e52d9eff7796825eccd4ebb`，编译器为 `5912f817581f89785e75c26e324a76c97257dc60`，共享 SDK 内容 SHA-256 为 `27af6e2995e3c6d9a35ccbc4859a6273ed58b0c613ef4d3247284da1030b4691`。14 个编译单元共 81 个编译产物已通过严格 fresh/retained 核验；结果及来源绑定见 [最终机器记录](../frontend/proto-ui/VALIDATION-2026-11-23.json)。上游数值辅助功能 [PR #1710](https://github.com/Proto-UI/Proto-UI/pull/1710) 已合入 main，本次控件能力与宿主修复回馈见 [PR #1730](https://github.com/Proto-UI/Proto-UI/pull/1730)。

历史来源审阅固定 Proto-UI `35195a6e6f1ba6a7d800da0029f2683013abd18b`，
其实验 Adapter 的 GPUI 为 `62e5991dd0f0c8a3af8d5e7e9c4652490d468db8`；
独立 API 快照为 `48741fd17d0dec28be6ad664b0a06dc7202086be`。
这些来源和哈希保留于 [SOURCES.md](SOURCES.md)，不随当前工作区版本改写。

## 编译产物与宿主接口

上游编译产物包含 Rust 源码与来源记录；原生组件另有 source map 和共享 SDK。投影通过 native slot 调用卷帘宿主。data target 生成的类型化函数负责状态转换，Rust 会话将输入与守卫结果直接传给这些函数，预解码全部效果后执行对应的领域操作，不创建 owner 或经 JSON 调用。模型、词典、音频设备和推理实现由各自 crate 提供。

普通按钮、开关、数值与选择控件经显式 target 分派。数值控件由持有实体的 render owner 提供受控当前值；步进、细调、范围与取消由编译源产生提案，原生点击回调不能另算第二套步进结果。宿主在写入前核对实际目标与 revision，不能以过期 render 捕获覆盖新值。同一用户意图经一个 `CommandBatch` 提交；preview／cancel 不修改工程。

命令 authority 与展示文本分开：换工程、换参数目标、策略改变或禁用使旧按压失效，实时名称/描述更新保留同一命令的已接受输入。`proto_parent.rs` 在原生 slot 的 layout/prepaint/paint 阶段传递逻辑父 owner；惰性编译子根在实际 render 时绑定，不能从 GPUI 视觉嵌套推断 Proto 焦点树已经建立。

应用有 21 处 `proto_scroll::viewport` 源码调用点。其余 5 处原生物理滚动容器通过 `attach_scroll_engine` lease 使用编译 ScrollSurface 的同一引擎；虚拟列表绑定实际 UniformList base handle。`verdant-app/src` 不保留生产 `scroll_to_item` 策略。宿主提供实际行/视口测量，编译源决定最近边缘揭示；公开 `position` 为 `[0, 1]` 归一化比例，原生像素 offset 是另一项物理事实。具体 21/5 清单见迁移记录。

原生虚拟化、Adwaita 绘图、操作系统窗口/非客户区机制及领域命令仍由对应宿主负责。系统文件对话框、物理 IME 和平台辅助技术的验收范围独立记录，不因为编译组件挂载成功而自动通过。

相同输入、编译器和 profile 必须生成相同字节。生成物带输入／编译器／profile 身份与该 target 的来源记录，
定制逻辑留在宿主接口中；禁止手改生成文件。

## 宿主 profile 与诊断

上游编译目标由 `frontend/proto-ui/compiler.pin.json` 固定。编译产物核验、模型 trace、原生输入与目标平台验收分别记录证据。[平台验收](../frontend/docs/PLATFORM-ACCEPTANCE.md)

动态 JS、DOM／selector／React hook、无界迭代、未实现 primitive、写入只读工程投影及实时域推理均须拒绝。
错误须含源位置、失败类型／义务和目标 capability，不能生成貌似工作的空实现。

mount 建立订阅和焦点，unmount 结束临时手势并移除监听。后台回调以弱引用／会话 token 拒绝过期目标。
paint 只消费布局；worker 任务和文件处理不能阻塞 Render；音频回调不能执行编译、推理或 I/O。

## 再生成

在开发环境准备锁定的上游检出与依赖后，从仓库根目录执行：

```sh
node frontend/proto-ui/tools/build-proto-ui.mjs --upstream /path/to/Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --upstream /path/to/Proto-UI --check
```

第一条命令按 pin 声明的 target 重新编译所有入口；本迁移要求完整收录上述 14 个单元。第二条新鲜编译并核对产物字节与来源记录；缺失上游时必须失败，漏列迁移入口的旧 pin 不能证明本次全部编译成功。显式 `--check-retained` 只证明保留制品自洽，不证明新鲜编译。workspace GPUI 的来源、rev 和 feature 在检查上游存在之前核验，完整 Cargo resolve graph 另行准入。CI 的 Rust 构建依赖强制 fresh lane 和独立的 session 无原生依赖构建 lane。窗口、IME 与辅助技术按目标平台独立验收。
构建方法见[前端手册](../frontend/README.md)。

上述最终快照已通过 82 项规则检查、173/173 项完整 Node 测试（其中作者 Runtime 88 项，0 skip）、上游聚焦源码回归 206/206 与 TypeScript 检查、现代 GPUI 原生 SDK 26/26、完整 GUI 237/237、headless 本地测试 33 项，以及完整 Cargo 图和实际 Rust source-mutation 门禁。headless 本地运行与完整依赖图共同验证 session 的无原生依赖边界，不声明该次运行是基础镜像隔离实验。来源变更实验实测 baseline 0.55、variant 0.6，并拒绝 variant 的旧 0.55 预期；不是仅比较生成源码字符串。

上游默认 GPUI `62e5991` 的 [CI run 45071732309](https://github.com/Proto-UI/Proto-UI/actions/runs/45071732309) 在 macOS 15 和 Ubuntu 24.04 上各通过原生消费者 39/39、数值辅助功能 8/8、SDK 27/27，其中旧版宿主的选区通知回归实际执行。默认 GPUI 和 Verdant 现代 GPUI 的 SDK 数量分别记录，不互相替代，也不代表物理 OS 输入。当前候选的打包后 X11/Wayland、物理触摸、系统 IME 和读屏验证仍按平台范围单独验收；上一轮 #79 的 OS 与打包记录继续作为历史证据。

## 来源与历史材料

2026-11-13 的上游确有 Rust host、style、session/epoch/lease 与有限 AccessKit，实现由仓库内 TS Runtime peer 驱动。
其 `private:true` 是包发布标记，源码公开；它与当前编译器和纯 Rust 成品分别归属。
保留原许可与归属，不复制 Prototype 私有业务逻辑到编译器以制造符合性。
声音协议 `ogd-worker/1` 与上游 host protocol 不共享消息或 framing。

[examples/](examples/) 中的协议、IR、host profile、工程和 trace 是早期设计样例；保留原状态和路径。
样例复验以记录的来源及哈希为准。实际编译、平台输入、读屏和声音分别验收。
后续控制与发音扩展见[目录索引](README.md)。当前工程入口为 `.verp`，旧 `.gdmproj`／JSON 示例只用于显式导入与历史参考。
