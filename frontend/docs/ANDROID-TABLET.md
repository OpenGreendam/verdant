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

1. **GPUI 没有 Android 平台后端。** 固定的 GPUI fork 有 macOS、iOS、Linux（X11/Wayland）和 Windows 平台，以及一个 `gpui_wgpu` 渲染器，但没有 Android 平台层。需要新增 `gpui_android`：
   - 用 `android-activity`（GameActivity）提供窗口、输入和生命周期；
   - 渲染复用 `gpui_wgpu`（Vulkan）；
   - 文本复用现有的 font-kit/cosmic-text 路径；
   - 补上 IME（`InputConnection`）、剪贴板和文件选择（Storage Access Framework）。
   
   这是上游工作，应先在 fork 中完成，再由 Verdant 升级 pin。
2. **后端进程模型。** 桌面版把合成 worker（`verdant-worker`）和插件宿主（`verdant-plughost`）作为独立进程启动。Android 应用不能可靠地执行随包附带的可执行文件，需要让 worker 以库的形式在应用进程内运行，或者放进一个独立的 Android Service 进程，通过现有的消息协议通信。
3. **ONNX Runtime。** 桌面包从 PyPI wheel 中提取原生库。Android 需要改用官方 `onnxruntime-android` AAR 中的 `libonnxruntime.so`，按 ABI（`arm64-v8a` 为主）放进 APK，并由 `verdant-synthesis` 通过 `libloading` 从应用的 native 库目录加载。
4. **第三方插件。** VST3 和 CLAP 只面向桌面系统。Android 构建应关闭 `verdant-plughost` 的 `vst3`/`clap` 特性，只提供内置效果器和内置合成器；浏览插件的入口在该平台隐藏。
5. **声库安装。** 当前 GUI 的导入流程要求先选目录再选来源文件。Android 上应改用 SAF 选取单个声库 ZIP，复制到应用私有目录后安装，等价于 `verdant-voice install ZIP --set-default`。

## 建议顺序

1. 在 GPUI fork 中实现 `gpui_android`，先跑通一个空白窗口、触控输入和 wgpu 渲染。
2. Verdant 增加 `target_os = "android"` 构建：关闭桌面插件特性，worker 改为进程内运行，接入 Android 版 ONNX Runtime。
3. 用 `cargo-apk` 或 `xbuild` 打出调试 APK，在 Android 平板（横屏，约 1280×800 dp 以上）上验证：
   - 卷帘编辑、曲线绘制和旋钮拖动；
   - 播放与渲染；
   - 声库安装。
4. 在平板上调整触控布局：横屏宽度通常落在 Medium，两侧面板同时停靠时卷帘偏窄，可考虑在触控布局下把右侧检查器默认收成抽屉。

## 桌面上预览平板布局

```sh
VERDANT_TOUCH=1 target/debug/verdant-gui
```

把窗口调到 1280×800（横屏平板）或 800×1280（竖屏，走 Compact 抽屉布局）即可检查触控尺寸和面板行为。
