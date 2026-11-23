# Proto-UI / GPUI 修复验收记录

> 本文保留首轮实际结果及当时未完成范围。后续完成的 headless 解耦、原生交互与 22 项 OS 验收见 [最终完成记录](VALIDATION-COMPLETION-2026-11-22.md)；不要将本文历史状态当作当前实现状态。

日期：2026-11-22。实现提交：`5cf8fd6998e2d6ea20d80872343e8b63c53de356`；审查基线：`dd0db139656322061fa51daf12cbc8a992c0cd8c`。

本记录区分源码重编译、依赖图、Rust 执行、GPUI 框架事件和 OS 窗口输入。下面的结果来自本次修复环境的实际执行，不表示 GitHub Actions 或所有平台验收已经完成。重现入口与设计边界见 [README.md](README.md)。

## 版本与制品

| 项目 | 实际使用的版本或身份 |
|---|---|
| Proto-UI compiler | `e56b2d1a3d6b688a1cbc6930dab1d6757304d733` |
| workspace GPUI | `https://github.com/cyjin-yl/z3rm`，`e5f2f9db75e97613232b5c1946333e61fa3828ab`，`gpui 0.2.2` |
| fresh compiler 执行环境 | Node `v24.22.3`，pnpm `10.32.1`，TypeScript `5.9.3`，tsx `4.21.0` |
| 上游依赖安装 | 完整 `pnpm install --frozen-lockfile --ignore-scripts`；实际安装的锁文件及工具版本由 producer 检查 |
| 上游锁文件 SHA-256 | `e374ba03f76079fd191d63c2da1f62861b7e90bbeba64a0b310027fc2c15b7f8` |
| Rust 原生验证 | `rustc 1.98.1 (48a229cea 2026-09-01)`，`cargo 1.98.1 (797e8a9bc 2026-08-05)`，Linux x86_64 |
| 共享 SDK | `proto-ui-gpui-native 0.2.0`，`gpui-modern-element-api`；唯一 owner 为 `pitch-gesture-surface` |
| SDK 内容 SHA-256 | `379b97bad8d0082097c7944506323fb58936eb880856eb5707e52a7244d483ed` |
| 最终 Cargo.lock SHA-256 | `6e9e15ffb719eba2467602de4a12f558a031e7dda1a1ac88991a98dc427de773` |

最终 fresh check 重新编译了四个单元，共 **35 个 compiler artifacts**，与保留的输出逐字节一致；未手改生成 Rust。Pitch 单元含 SDK，共 20 个 artifacts；note、control、knob 各 5 个。最终 knob package 为 `generated-gpui-1d08ae6cfd66f22afc399345f5522b6f`。

## 自动验证

| 层次 | 执行结果 | 证明范围 |
|---|---|---|
| TOML / Cargo 准入 fixture | **82 项通过** | alias、实际 package 名、普通/build/dev/target 表、子表、多行、注释、非 GPUI rev、workspace 来源及 Cargo 图等诊断 |
| Node 测试 | **40 通过，0 跳过** | 14 producer、12 publication、14 authored knob Runtime 测试；设置了 `PROTO_UI_UPSTREAM` |
| fresh compilation | **4 单元 / 35 artifacts 通过** | 使用固定 compiler、实际锁定依赖及源码闭包重新发射 |
| 完整 Cargo resolve graph | **通过** | `cargo metadata --locked --all-features --offline`；所有 workspace members 可达图中只有一个 GPUI package identity 和一个共享 SDK |
| Rust 测试 | **49 通过，0 ignored** | session 27、typed protocol 4、capture 3、knob 7、devices 5、roll 3 |
| 最终原生 app build | **通过** | 用项目固定 Rust 工具链构建最终源码及保留生成物 |
| TS 来源变异 → Rust consumer | **通过** | baseline `0.55`，variant `0.6`，variant 的旧期望以 assertion failure / exit 101 被拒绝 |

关键重现命令（仓库根；上游和依赖准备见 README）：

```sh
PROTO_UI_UPSTREAM="$PWD/../Proto-UI" corepack pnpm@10.32.1 --dir frontend/proto-ui test
node frontend/proto-ui/tools/build-proto-ui.mjs --check --upstream ../Proto-UI
node frontend/proto-ui/tools/build-proto-ui.mjs --check-retained
node frontend/proto-ui/tools/check-cargo-gpui.mjs --offline

cd frontend
cargo test --offline --locked -p verdant-ui-protocol -p verdant-session
cargo test --offline --locked -p verdant-app --bin verdant host_pointer::tests
cargo test --offline --locked -p verdant-app --bin verdant proto_knob::tests
cargo test --offline --locked -p verdant-app --bin verdant devices::tests
cargo test --offline --locked -p verdant-app --bin verdant roll::other_track_tests
cargo build --offline --locked -p verdant-app --bin verdant
```

Rust 测试包括最终 pointer-up sample、不同 pointer 的隔离、取消和 capture 所有权、旧 generation 清理、真实 GPUI knob instance 的键盘/指针、受控值、失焦、detach 与 disposal。7 个 knob 测试中，5 个经过 GPUI 测试上下文，2 个是纯宿主协议/接触代次检查。JavaScript Runtime 的 14 个测试另行覆盖 props 同步、重入、宿主拒绝/接受、公开状态、键盘和 a11y；它们没有经过 GPUI。

完整 Cargo 图中 GPUI 的 source 为：

```text
git+https://github.com/cyjin-yl/z3rm?rev=e5f2f9db75e97613232b5c1946333e61fa3828ab#e5f2f9db75e97613232b5c1946333e61fa3828ab
```

唯一 SDK 指向 `frontend/proto-ui/generated/pitch-gesture-surface/.proto-ui/gpui-native`。这次检查使用完整 resolve graph，没有使用 `--no-deps`，也没有以 Cargo.lock 字符串计数代替解析图。

### 原生来源变异

实验由 `check-native-source-mutation.mjs --prepare/--run` 完成。使用同一固定 compiler 将 control planner 的 fine sensitivity 从 `0.1` 改为 `0.2`，再编译、运行相同 Rust consumer。baseline 和 variant 分别得到 `0.55` 与 `0.6`；对 variant 断言旧值 `0.55` 的消费者实际失败。两套输出共享同一个 SDK 内容身份。

| 输出 | generated SHA-256 |
|---|---|
| baseline | `172e0f756ea6e0e1f7bd120ef868c5bc9316e621b915006129556619fbee9843` |
| variant | `8d526d74d17dabfe387a8d8ea96a9b465575330acfab37c21d947151501d5726` |

这项实验验证 control planner 的 TS 变化能改变已编译 Rust 行为，不验证窗口输入。最终 knob 生命周期修正没有改变该 control planner 或共享 SDK 的字节。

## 实际 X11 窗口和工程持久化

启动最终 Verdant debug 可执行文件，在独立 Xvfb X11 桌面、Lavapipe 软件渲染、静默音频和独立用户状态目录下执行虚拟键鼠输入。使用带内置 synth 的测试工程，检查保存后的参数及 revision，并实际执行 undo；判定不依赖截图。

可执行文件 SHA-256：`3bdcd3489c93a8e01fc6a0457f7effead36f2878d88d31790fa91807c6754a12`。

| 操作 | 实际观察 |
|---|---|
| Gain 旋钮获得焦点后 ArrowUp，保存 | `-8 dB → -7.7 dB`，工程 revision 增加一次 |
| 拖出旋钮 hitbox，在窗口内释放，保存 | 得到约 `1.300137 dB`；一次 undo 恢复 `-7.7 dB`，没有重复提交 |
| 拖动预览后 Escape，再释放 | 已保存值与 revision 不变 |
| 拖动预览期间 undo 改变工程，再释放 | 保存为 undo 后的 `-8 dB`；旧预览没有覆盖新工程 |
| 双击 Gain | 恢复默认 `-8 dB`，正常保存 |
| 拖动中将 X11 焦点移出窗口，再释放 | 没有提交预览，值与 revision 不变 |
| 拖到应用窗口外释放，再返回保存 | 没有遗留提交，值与 revision 不变 |
| 活跃预览期间退出应用 | 进程 exit 0；未写入预览，没有 disposal 回调错误 |

物理坐标到参数的转换存在浮点误差；54 px 拖动的工程值用 `0.001 dB` 容差核对，并独立检查一次 undo 的原子性。未将显示文本的舍入结果当成精确领域值。

验证期间还修复了应用对固定 native SDK 三个边界的适配：不能用动态 Owner 相等性判断 run；离开 hover hitbox 的 `press.cancel` 不能取消独立 move lease；`beforeDispose` 不能再写 state 或调用 run。最终原型通过 epoch 判断、lease 取消入口，以及 unmount 清理/纯 lexical dispose retirement 适配这些边界。

## 明确保留的边界

- **F4 只完成了短期 adapter 隔离。** Session 不再直接引用 fingerprint package/owner/JSON 协议；当前 planner 仍传递依赖 GPUI。无 GPUI 的 data-only artifact/backend 尚未实现，也没有另写 Rust 状态机制造第二份真相。
- 只有 normalized knob 完成了原型拥有状态、取消、键盘、焦点与辅助功能语义的完整组件切片。DAW canvas、音乐坐标和领域提交继续由原生 Rust 负责；其余 planner 不宣称已经成为完整 Proto-UI 控件。
- OS 输入证据仅覆盖本次 Linux X11 虚拟桌面。Windows/macOS、Wayland、物理触摸、多指竞争、IME、屏幕阅读器和真实音频设备仍需各自验收。GPUI touch/mouse 测试不能替代物理设备测试。
- subtree detach 已有 GPUI 框架事件测试；没有取得 OS 窗口中实际切换视图导致 detach 的独立证据。设备视图中按 `E` 不触发该切换，不能计作 detach 通过。
- 初次软件渲染进程在受限虚拟内存环境中退出 139；限制软件渲染线程与 CPU affinity 后，最终二进制完成上述输入、保存及退出测试。未据此宣称任意 GPU/驱动或资源配置都通过。
- 本地未执行完整发布打包、全部 workspace 测试或跨平台 CI。新增 CI 会分别要求 retained、fresh、Cargo graph、Rust/native consumer 和平台构建；Actions 的实际状态应以对应提交的运行结果为准。

后续平台验收仍按 [PLATFORM-ACCEPTANCE.md](../docs/PLATFORM-ACCEPTANCE.md) 记录。保留制品一致、编译成功和实际输入正确性是分别验收的结果。
