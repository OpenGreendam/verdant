# Adwaita 原生插件与分析工具验收

日期：2026-11-22。14 类真实 GPUI 原生编辑器，使用 `spec/examples/native-suite-advanced.verp` 的隔离副本。所有单张 PNG 是 1600×1100 的 Xvfb 根窗口截屏；没有用网页或静态 mockup 代替应用。两张 contact JPG 仅用于浏览，原始 PNG 保留完整画面。

## 结果与索引

- [14 类浅色总览](contact.jpg)，窗口标题／尺寸见 [inventory.json](inventory.json)。
- [深色 EQ](eq-dark.png)；[640 像素宽主画布](eq-narrow-640-top.png) 与 [滚动后的全部主控](eq-narrow-640-controls.png)。
- [预设浏览与另存选择](preset-browser-saved.png)；[640×480 保存表单](preset-save-640.png)。
- [实际 EQ Match](eq-match-applied.png)、[实际绘制与拟合](eq-sketch-applied.png)、[真实 WAV 导入的 IR 拟合](reverb-ir-applied.png)，合览见 [analysis-contact.jpg](analysis-contact.jpg)。
- [原工程播放完成](playback-complete.png)、[字段／检查／图片哈希回执](receipt.json)、[原生字形解析](font-resolution.txt)、[7 条新量化测试日志](analysis-fit-quality-tests.log)。

| 家族 | 原生截图 | 家族 | 原生截图 |
| --- | --- | --- | --- |
| EQ | [EQ](eq-light.png) | Compressor | [Compressor](compressor-light.png) |
| Gate | [Gate](gate-light.png) | De-esser | [De-esser](deesser-light.png) |
| Limiter | [Limiter](limiter-light.png) | Reverb | [Reverb](reverb-light.png) |
| Delay | [Delay](delay-light.png) | Saturation | [Saturation](saturation-light.png) |
| Multiband | [Multiband](multiband-light.png) | Filter | [Filter](filter-light.png) |
| Dual Filter | [Dual Filter](simplon-light.png) | Micro Filter | [Micro Filter](micro-light.png) |
| Synth | [Synth](synth-light.png) | One | [One](one-light.png) |

## 播放、主题和字体

GUI 使用无硬件输出的软件音频时钟，DSP 宿主使用优化后的 `target/release/verdant-plughost`。同一原始工程从头播放到实际 GUI 终点 **7.896 秒**，总观察 35 秒，未出现原生队列故障；随后 14 类捕获过程也没有故障。其他离线导出回执中的 23.98 秒包含有界尾音，不是这个 GUI 时间线的播放终点。未优化宿主曾在 Limiter 的 32×处理下积压输入队列；诊断图保留在构建目录，已用正常优化宿主的截图替换本目录的主证据。

谱线、波形历史和电平来自真实处理数据。无可用音频时显示等待，停止后显示上次数据；这些状态不替换成装饰曲线。物理声卡、外部 MIDI、多显示器与其他操作系统未在本回执中验收。

本机没有 Adwaita Sans／Cantarell，系统选择 Noto Sans CJK SC。为避免凭图片猜测字体，临时原生诊断程序调用与插件相同的字体选择和回退函数，再用 GPUI CosmicTextSystem 生成包含中文的 27 字形行：实际 shaped run 对应 **Noto Sans CJK SC**，没有落入 Noto Serif CJK SC。完整结果在 `font-resolution.txt`；诊断程序已移出产品源码。

## 预设操作

实际完成搜索、上下方向键、Enter 载入、稳定 ID 收藏、试听还原、另存及小窗口检查。EQ 载入 Intimate Lead 后返回原音，保存工程验证原 `b1_freq=120`、连续陡度开启、`phase=2`、动态段状态均恢复。另存用户文件包含 **601 个有效 EQ 参数**，名称、类别、作者、原厂来源 ID 保留，异步刷新后仍选中新文件。One 载入 Glass Pluck 后保存的工程 `instrument.params` 为 **248 个 Synth＋共享字段**，兼容已有完整状态；One 的有效描述符范围仍由其设备规范限制。

保存模式独立使用可滚动内容区，640×480 下名称、类别、作者、标签、用途与保存动作均可访问。搜索／列表键盘操作与保存字段输入分开，IME 组合状态不被浏览快捷键截获；本回执没有宣称完成物理中文输入法测试。

## 三项分析入口

- **Match：** 输入 100–10000 Hz、50% 强度、12 段上限、1/3 octave 平滑，实际捕获源输入与参考输出频谱并提交。界面报告 RMS **0.43 dB**、最大 **1.01 dB**、移除电平差 **−18.8 dB**。
- **Sketch：** 鼠标绘制连续目标曲线，后台拟合后显示实际叠加曲线，提交 **11 段**，界面报告 RMS **0.04 dB**、最大 **0.12 dB**。
- **IR：** 通过真实 GTK 文件选择器导入自生成的 48 kHz、16 bit 单声道 WAV，已知 1.2 秒尾、20 ms 起始静音和稳定噪底。界面报告 **1.20 s、r² 1.000、8 个频带、模型误差 0.4%**。随后保存工程，确认预延迟 20 ms、阻尼 20 kHz、衰减／后置节点实际写入。基础衰减旋钮约 1.278 s 与输入全带测量不同，因为它参与频率相关 FDN 拟合；两者没有混称。

量化算法、定义、采样率条件与边界见 [原生分析说明](../../native-plugin-analysis.md)。IR 是可编辑算法模型近似，不是卷积播放；匹配误差针对处理后的静态目标，不能当成任意音频的等价保证。

## 复现要点

构建 GUI、worker 与优化宿主，在独立 XDG 配置／数据／运行目录和 Xvfb 中启动。实际验收使用 `verdant-gui PROJECT --silent-audio --appearance light|dark --worker-binary WORKER --plughost RELEASE_PLUGHOST --workspace PRIVATE_WORKSPACE --repo REPO`。面向用户的入口现在为 `verdant gui`，完整启动和打包规则由主入口文档定义。

捕获应使用根窗口，例如 `import -window root capture.png`；Vulkan 窗口的单独 backing pixmap 在该环境会陈旧。GTK 位置栏获得焦点后再输入完整文件路径，避免自动化把前几个字符发给旧控件。

本目录同时记录早先的独立预设文件操作与最终优化宿主的主界面／分析操作。后续仅聚焦字段显示选区的绘制修复已进入完整工作区构建；该变化不改变已验证的保存布局和参数结果。
