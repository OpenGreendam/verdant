# Android 平板适配准备

本文记录 Verdant 在 Android 平板上运行前已经具备的条件、仍然缺少的部分，以及建议的实施顺序。结论基于 2026-11-26 的 main 和工作区固定的 GPUI 修订 `28f0728`。

## 已经具备

| 方面 | 现状 |
| --- | --- |
| 响应式布局 | `WidthClass`（Phone < 760 px、Compact < 1100 px、Medium、Wide）决定侧栏是停靠还是以抽屉弹出；窄屏把视图切换收进主菜单。 |
| 触控布局 | `touch_ui` 在首次触摸时自动开启，也可在「显示 → 触控布局（平板）」手动开关并持久保存，或用 `VERDANT_TOUCH=1` 强制开启。开启后顶栏加高，rem 从 16 px 增至 18 px，所有按 rem 计尺寸的控件随之放大，向 44 px 触控目标靠拢；按 px 绘制的乐谱和曲线画布保持原有音乐几何。 |
| 触控手势 | 卷帘、曲线、旋钮和效果器图形使用 GPUI 的 touch arena，不冒充鼠标捕获（见 `devices.rs` 的触控测试）；拖动越过控件仍由各自的 move lease 管理。 |
| 音频输出 | `verdant-audio` 通过 cpal 0.18 输出；cpal 在 Android 上走 AAudio。 |
| 声音处理 | 内置效果器和合成器是纯 Rust（`verdant-plugins::builtin_dsp`），不依赖桌面插件格式。 |
| 字体 | Inter 和 Noto Sans CJK SC 子集随应用嵌入，不依赖系统字体。 |

## 仍然阻塞

1. **GPUI 平台后端（初版已完成，尚未上真机）。** 固定的 GPUI fork 没有 Android 平台层，而且 fork 仓库已归档，所以后端放在 Verdant 自己的 `crates/gpui_android` 里，不修改 GPUI 本体：
   - 平台层：用 `android-activity`（NativeActivity）驱动事件循环，处理窗口创建与销毁、旋转、尺寸和配置变化、系统栏安全区、前后台和内存告警；
   - 窗口层：通过 `gpui_wgpu`（Vulkan，退回 GLES）渲染，进入后台时只卸下绘图表面，保留设备和图集；
   - 调度器：主线程任务在 looper 线程执行，后台任务走线程池，定时器使用独立线程；
   - 输入：多指 MotionEvent 转为 `PlatformInput::Touch`，由 GPUI 的通用手势识别出点击、平移、长按和捏合；返回键接到 `set_back_handler`。

   `examples/android.rs` 和 `script/build-apk.sh` 可以打出 arm64 调试 APK。尚未实现：输入法与软键盘文本输入、返回键以外的硬件键、剪贴板、文件选择（SAF）、打开链接、保持亮屏。
2. **后端进程模型。** 桌面版把合成 worker（`verdant-worker`）和插件宿主（`verdant-plughost`）作为独立进程启动。Android 应用不能可靠地执行随包附带的可执行文件，需要让 worker 以库的形式在应用进程内运行，或者放进一个独立的 Android Service 进程，通过现有的消息协议通信。
3. **ONNX Runtime。** 桌面包从 PyPI wheel 中提取原生库。Android 需要改用官方 `onnxruntime-android` AAR 中的 `libonnxruntime.so`，按 ABI（`arm64-v8a` 为主）放进 APK，并由 `verdant-synthesis` 通过 `libloading` 从应用的 native 库目录加载。
4. **第三方插件。** VST3 和 CLAP 只面向桌面系统。Android 构建应关闭 `verdant-plughost` 的 `vst3`/`clap` 特性，只提供内置效果器和内置合成器；浏览插件的入口在该平台隐藏。
5. **声库安装。** 当前 GUI 的导入流程要求先选目录再选来源文件。Android 上应改用 SAF 选取单个声库 ZIP，复制到应用私有目录后安装，等价于 `verdant-voice install ZIP --set-default`。

## 建议顺序

1. 在真机上验证 `gpui_android` 示例 APK（渲染、点按、旋转、切后台再回来），再补输入法和剪贴板。
2. Verdant 增加 `target_os = "android"` 构建：关闭桌面插件特性，worker 改为进程内运行，接入 Android 版 ONNX Runtime。
3. 仿照 `crates/gpui_android/script/build-apk.sh` 打出 Verdant 的调试 APK，在 Android 平板（横屏，约 1280×800 dp 以上）上验证：
   - 卷帘编辑、曲线绘制和旋钮拖动；
   - 播放与渲染；
   - 声库安装。
4. 在平板上调整触控布局：横屏宽度通常落在 Medium，两侧面板同时停靠时卷帘偏窄，可考虑在触控布局下把右侧检查器默认收成抽屉。

## 构建示例 APK

```sh
export ANDROID_NDK_HOME=/path/to/android-ndk-r27c
export ANDROID_BUILD_TOOLS=/path/to/sdk/build-tools/35.0.0
export ANDROID_JAR=/path/to/sdk/platforms/android-35/android.jar
rustup target add aarch64-linux-android
crates/gpui_android/script/build-apk.sh
adb install target/gpui-android-apk/gpui-android-example.apk
adb logcat -s gpui
```

## 桌面上预览平板布局

```sh
VERDANT_TOUCH=1 target/debug/verdant-gui
```

把窗口调到 1280×800（横屏平板）或 800×1280（竖屏，走 Compact 抽屉布局）即可检查触控尺寸和面板行为。

## 迁移到官方 GPUI 仓库

GPUI 的官方仓库发布后，Verdant 从 `cyjin-yl/z3rm@28f0728` 切换过去，按下面顺序进行。

### 1. 切换依赖来源

GPUI 的来源只在两处声明，其余检查都从这两处读取：

- `frontend/Cargo.toml` 中的 `gpui`、`gpui_platform`、`gpui_wgpu` 三项必须使用同一个 `git` 和 `rev`；
- `frontend/proto-ui/compiler.pin.json` 中的 `gpu.workspace.git`、`gpu.workspace.rev` 和 `gpu.workspaceGpuiRevision`。

两处都改好后：

在仓库根目录运行（`../Proto-UI` 是与 README 相同的上游编译器 checkout）：

```sh
cargo update --manifest-path frontend/Cargo.toml -p gpui -p gpui_platform -p gpui_wgpu
node frontend/proto-ui/tools/build-proto-ui.mjs --upstream ../Proto-UI          # 重新生成 14 个单元和 receipt
node frontend/proto-ui/tools/build-proto-ui.mjs --check --upstream ../Proto-UI
node frontend/proto-ui/tools/check-proto-ui-rules.mjs
node frontend/proto-ui/tools/check-cargo-gpui.mjs
cargo test --manifest-path frontend/Cargo.toml --workspace
```

`check-proto-ui-rules.mjs` 里出现的 `cyjin-yl/z3rm` 只是固定的测试样例，不需要改。`P0-DECISIONS.md` 中锁定 GPUI 的那条记录需要更新为新的仓库和修订。

### 2. 核对 fork 独有的接口

官方仓库不一定包含这个 fork 新增的接口。`crates/gpui_android` 依赖下面这些接口，切换前应逐项确认：

| 类别 | 接口 |
| --- | --- |
| 平台信息 | `PlatformInfo`、`OperatingSystem::Android`、`PlatformTarget` |
| 触控与手势 | `PlatformInput::Touch`、`TouchEvent`、`TouchId`、`TouchPhase`、`PlatformGestures`、`GestureTuning`、`ScrollPhysics::android()` |
| 移动端窗口 | `WindowInsets`、`on_insets_changed`、`set_back_handler`、`set_back_enabled`、`show_soft_keyboard`、`hide_soft_keyboard`、`WindowVisibility`、`on_visibility_change`、`on_memory_warning` |
| 帧调度 | `RequestFrameOptions`、`frame_waker` |
| 渲染（`gpui_wgpu`） | `WgpuRenderer::new`、`replace_surface`、`unconfigure_surface`、`needs_redraw`、`WgpuSurfaceConfig::preferred_present_mode`、`GpuContext`、`CosmicTextSystem` |

Verdant 桌面端自身也依赖其中的触控接口（卷帘、曲线、旋钮的 touch arena），以及无障碍角色、客户端装饰和窗口按钮布局等 fork 接口。

### 3. 安卓后端的去向

- 如果官方仓库自带 Android 平台，就删掉 `crates/gpui_android`，改用官方的入口函数，示例和 `build-apk.sh` 跟着迁移。
- 如果没有，可以继续保留在 Verdant 中，并按上表调整接口。也可以把它贡献到官方仓库，但这个 crate 目前沿用 Verdant 的 AGPL-3.0-or-later 许可，而 GPUI 是 Apache-2.0，提交上游前需要权利人决定是否以 Apache-2.0 授权。这个 crate 只依赖 `gpui`、`gpui_wgpu` 和 Android 相关的 crate，不引用任何 Verdant 代码，可以整体搬过去。
