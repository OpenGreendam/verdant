# Proto-UI 与 GPUI 集成

Verdant 使用固定版本的实验性 Proto-UI `gpui-source-v1` compiler。保留的 Rust 源码全部由该 compiler 发射；不要手改 `generated/`。版本、完整 GPUI 来源、工具链要求、共享 SDK 内容和编译单元由 [compiler.pin.json](compiler.pin.json) 绑定。

## 当前边界

| 单元 | 原型拥有的职责 | Rust 宿主拥有的职责 |
|---|---|---|
| `pitch-gesture-surface` | 原生根、尺寸/裁剪和 slot；共享 SDK 的 artifact owner | 卷帘绘图、音乐坐标、音高采样、领域预览与提交；上游 MoveGesture lease 的宿主绑定 |
| `note-gesture-planner` | 移动、复制、resize、框选的纯决策函数和有序效果 | pointer identity、不可变守卫事实、revision、领域数据、效果执行和原生 capture |
| `control-gesture-planner` | 曲线/连续参数的纯决策函数和归一化拖动计算 | 图形命中、坐标与参数单位、preview、最终采样和工程提交 |
| `normalized-knob` | 每实例的值/phase、props 同步、取消、键盘、焦点、Slider 辅助功能语义和提交事件 | Adwaita 绘图 slot、移动距离、参数单位、当前目标/工程 revision 校验和领域提交 |

前三项属于 **Proto 编译的 gesture planner + native GPUI surface**。纯数据方法是上游正式支持的 detached call，不要求创建组件实例。`normalized-knob` 是真实有状态组件实例：它复用 control planner，原型对交互和公开语义负责，宿主不再为该旋钮另存一份 phase 状态机。slot 和自绘是合法组成方式；是否由原型承担交互契约，不能由有无 slot 判断。

旋钮的 `props.value` 是宿主接受的归一化值；`defaultValue` 只表示重置目标。拖动时公开 `value` 可以显示预览，提交前恢复到已接受值，再发出 `{ value, revision }` 提案。宿主接受后通过 props 发布新值，拒绝时不会留下未接受的显示值。`revision` 是不透明的来源标识，宿主可将工程、store 与参数身份纳入其中。值、revision 或 disabled 改变会取消当前交互；键盘接管先取消指针预览，再从已接受值步进，Shift 使用十分之一的步长。公开状态、焦点和事件回调后的 epoch 检查防止重入更新被旧操作覆盖。

`verdant-ui-protocol` 隔离生成 crate 名、owner handle、动态 JSON 和字符串协议，向 app/session 提供枚举、完整结果校验和显式错误。**当前 pinned target 仍让 planner 传递依赖 GPUI。这个 adapter 没有实现无 GPUI 的 data-only backend，也没有消除 session 的平台构建闭包。** 真正的 data-only artifact 需要上游提供相应编译目标，不能另写一套 Rust 状态机代替 TS 真相。

普通 roll 和曲线的原生 capture 由带 window/hitbox/generation 的 `OwnedPointerCapture` 管理。领域变更、取消、失焦和 owner 释放不能丢弃 Release 效果；旧延迟清理不能释放新会话的 capture。终止事件先处理最终坐标/modifier，再决定提交。Pitch 和旋钮复用上游 MoveGesture lease，并区分 mouse/touch；实际 OS 路由仍需要各平台验收。

旋钮的捕获拖动可以继续越过控件 hitbox。上游 press adapter 因 hover 离开而发出的 `press.cancel` 不结束这个独立的 move lease；失焦、detach、失去所有权等真正的 lease 取消通过公开 `cancel` 方法清理原型。

## 再生成与两类检查

使用 Node 24.x、pnpm 10.32.1，将 Proto-UI 检出到 pin 的完整 commit。以下命令从仓库根执行：

```sh
corepack pnpm@10.32.1 --dir frontend/proto-ui install --frozen-lockfile --ignore-scripts
git clone https://github.com/Proto-UI/Proto-UI ../Proto-UI
git -C ../Proto-UI checkout --detach e56b2d1a3d6b688a1cbc6930dab1d6757304d733
corepack pnpm@10.32.1 --dir ../Proto-UI install --frozen-lockfile --ignore-scripts

node frontend/proto-ui/tools/build-proto-ui.mjs --upstream ../Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --check --upstream ../Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --check-retained
```

`--check` 必须新鲜编译，缺失上游 checkout 是失败（exit 10）。`--check-retained` 只核对当前 workspace、来源闭包、保留制品、provenance 和 receipt，明确返回 `freshCompilation: false`。所有离线可核对项先于 upstream 判缺；改错实际 workspace GPUI pin，即使没有上游，也会失败。

生产脚本 `tools/check_frontend.sh` 默认要求 fresh 检查。单独的 Rust 构建 lane 可显式设置 `VERDANT_PROTO_UI_MODE=retained`；CI 的 Linux/Windows/macOS 构建均依赖单独的强制 fresh lane，不能将 retained 成功当成新鲜编译证明。

工具链核验包含实际执行 Node/pnpm、实际加载的 TypeScript/tsx 版本、安装所用 package manager、完整已安装锁文件和上游锁文件内容。成功输出记录这次实际执行版本；receipt 记录可复现要求，不冒充某次机器执行证明。

## 来源闭包和共享 SDK

producer 使用上游 `compileFile` 与 `FileSourceGraph`，记录仓库边界内的入口、导入 helper、包解析输入以及未命中的候选路径；已有的仓库外来源会被拒绝。编译前后核对来源字节与解析图，避免只哈希入口却遗漏 helper 或 exports 变化。

`pitch-gesture-surface` 发射唯一的 `.proto-ui/gpui-native`；另外三个单元通过上游 `nativeSdkPath` 引用它。pin 绑定 owner、路径、package/version、ABI feature 和 SDK 内容摘要。增加单元时必须复用该 SDK，不能复制同名 package，也不能改生成 Rust 来修 Cargo 冲突。

生成物替换先验证 receipt 的完整 inventory 和每个文件内容。额外文件、人工改动、符号链接和不完整 inventory 会被拒绝；它们不会被“目录里有 provenance”这一条件授权删除。发布使用互斥锁、唯一 staging、备份和恢复 journal。它是可恢复的多目录替换，只有遵守 producer 锁的读写方才具备批次一致性；不要在再生成过程中同时运行 Cargo。

中断后，先确认记录的 producer 已停止，再执行：

```sh
node frontend/proto-ui/tools/build-proto-ui.mjs --recover-publication
```

恢复入口检查同机 owner PID 并独占恢复所有权；活跃 producer 不能被恢复操作覆盖。恢复本身被中断后，也可在其 owner 停止后再次执行。不要手动删除活跃锁或备份。

## 准入与证据

本次修复的实际工具链、测试结果、OS 输入观察和剩余范围见 [2026-11-22 验收记录](VALIDATION-2026-11-22.md)。

TOML admission 使用 parser，按实际 `package` 名检查普通/build/dev 和 target-specific 的所有依赖表，支持 alias、子表、多行和注释。它检查声明；另一个门禁调用完整 `cargo metadata --locked --all-features`，沿全部 workspace members 的 resolve graph 检查 GPUI package ID/source/rev 和唯一 SDK。`--no-deps` 或扫描 Cargo.lock 字符串不能替代这一检查。

```sh
corepack pnpm@10.32.1 --dir frontend/proto-ui test
PROTO_UI_UPSTREAM="$PWD/../Proto-UI" corepack pnpm@10.32.1 --dir frontend/proto-ui test
(cd frontend && cargo fetch --locked)
node frontend/proto-ui/tools/check-cargo-gpui.mjs --offline

(cd frontend && cargo test --locked -p verdant-ui-protocol -p verdant-session)
(cd frontend && cargo test --locked -p verdant-app --bin verdant host_pointer::tests)
(cd frontend && cargo test --locked -p verdant-app --bin verdant proto_knob::tests)
```

离线 Cargo graph 检查前，需要在 `frontend/` 中执行过 `cargo fetch --locked`。Rust 测试进入该目录执行，使 rustup 采用其中的固定工具链。

设置 `PROTO_UI_UPSTREAM` 的测试覆盖真实 compiler 发射和来源变异，也将 authored knob 放进 pinned `createRuntimeSession`，通过受控 host capability 检查拒绝更新、同步接受、重入取消和生命周期。这些 JavaScript Runtime 测试不经过 GPUI，也不代表 OS 输入测试；无 checkout 时会明确跳过。上面的 Rust 测试另外覆盖类型化协议、宿主 capture 和 GPUI 组件实例。

CI 还准备 baseline/variant 两套完整编译产物，用同一 Rust consumer 执行：TS 中 fine sensitivity 从 0.1 改到 0.2，Rust 函数的实测结果必须从 0.55 改到 0.6，旧期望必须失败。这验证 TS 来源确实影响已编译的消费者行为，不涉及窗口输入。

```sh
node frontend/proto-ui/tools/check-native-source-mutation.mjs --prepare .ci/proto-ui-source-mutation --upstream ../Proto-UI
node frontend/proto-ui/tools/check-native-source-mutation.mjs --run .ci/proto-ui-source-mutation
```

保留字节一致、Cargo graph、Rust trace、GPUI 框架事件、OS 窗口输入和物理设备分别提供不同证据。哈希或截图不能代替取消/捕获正确性，GPUI 测试不能替代物理触摸、IME 或屏幕阅读器验收。跨平台剩余范围仍见 [PLATFORM-ACCEPTANCE.md](../docs/PLATFORM-ACCEPTANCE.md)。

## 上游依据

以下链接固定到本次 compiler pin；实验性目标的能力不能从当前 main 或旧 source audit 推断。

- [Compiler README：宿主组成、workspace GPUI、共享 SDK 和证据边界](https://github.com/Proto-UI/Proto-UI/blob/e56b2d1a3d6b688a1cbc6930dab1d6757304d733/packages/compiler/README.md)
- [compile.ts：compileFile、来源图和 nativeSdkPath](https://github.com/Proto-UI/Proto-UI/blob/e56b2d1a3d6b688a1cbc6930dab1d6757304d733/packages/compiler/src/compile.ts)
- [Base Slider Root：承载语义的 slot 原型](https://github.com/Proto-UI/Proto-UI/blob/e56b2d1a3d6b688a1cbc6930dab1d6757304d733/packages/prototypes/base/src/slider/root.proto.ts)
