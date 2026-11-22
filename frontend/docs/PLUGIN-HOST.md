# verdant-plughost(1) — 音频效果器宿主

`verdant-plugins` 管理扫描、参数、状态和音频桥接；每个插入位由独立的 `verdant-plughost` 子进程执行。插件以当前用户身份运行，进程隔离不构成安全沙箱。实际目标机状态见[平台验收](PLATFORM-ACCEPTANCE.md)，许可见[第三方声明](THIRD_PARTY-PLUGINS.md)。

## 用法

```text
verdant-plughost scan-file PATH
verdant-plughost host --channel=CHANNEL
```

`scan-file` 扫描一个插件文件或包，stdout 输出单行 JSON 描述符报告。`host` 由客户端建立共享音频队列后启动，`CHANNEL` 使用所属客户端提供的 `fd:a,b,c` 或 `name:a,b,c`；它不是独立播放命令。诊断写 stderr。

本程序处理音频效果。DeepSeek Harness 的工具插件通过公开 CLI 接入，范围见 [CLI／DSH 规格](../../spec/CLI-DSH.md)。

## 格式范围

| 格式 | 处理与状态 | 编辑器与限制 |
|---|---|---|
| CLAP | clack 0.2；f32 立体声、参数事件、原生状态流、延迟和可选离线模式 | 一条立体声输入／输出；嵌入或平台浮动 GUI；其他总线形状拒绝 |
| VST3 | vst3-host 0.11.1；组件／控制器状态、延迟、`kOffline` | 激活的立体声主输入／输出；附加输入喂静音；IPlugView，Linux IRunLoop |
| AUv2 | macOS AudioToolbox；`AudioUnitRender`、参数、ClassInfo plist、延迟 | 已注册的立体声 `aufx`，Cocoa NSView；不支持 AUv3 或 Carbon |

扫描成功只说明取得描述符。`has_editor()`／`open_editor()` 返回实际能力或错误；没有 GUI 时可用通用参数面板。编译或 AU 类型检查不能代替外部插件的 PCM、状态和窗口验收。Windows/macOS 原生验收待完成。

## 扫描与路径

宿主解析顺序：应用 `--plughost FILE` → `VERDANT_PLUGIN_HOST` → `VERDANT_PLUGHOST` → 应用同目录 → `../libexec/verdant/verdant-plughost` → `PATH`。Windows 使用 `.exe`。Linux 包启动器提供路径；Windows bin 和 macOS Contents/MacOS 放置同级宿主。

| 平台 | 默认扫描目录 |
|---|---|
| Linux | `~/.clap`、`~/.vst3`、`/usr/{local/,}lib/{clap,vst3}` |
| Windows | `%ProgramFiles%/Common Files/{CLAP,VST3}`、`%LOCALAPPDATA%/Programs/Common/{CLAP,VST3}` |
| macOS | `~/Library/Audio/Plug-Ins/{CLAP,VST3,Components}` 与 `/Library/Audio/Plug-Ins/{CLAP,VST3,Components}` |

单文件诊断：

~~~sh
/path/to/verdant-plughost scan-file /path/to/plugin.clap
~~~

扫描在短命子进程中执行，默认八秒，报告最多 8 MiB。缓存 `plugin-scan.json` 按路径和内容 SHA-256 匹配，成功缓存仍加入本次目录。超时、崩溃和探测错误进入黑名单。包哈希覆盖全部普通文件；文件符号链接绑定目标名和字节，目录符号链接拒绝。指纹失败移除旧描述符，不用旧身份掩盖错误。

## 退出状态与文件

宿主正常结束或扫描完成退出 0；扫描、宿主或协议输出失败退出 1；缺参数、缺 channel 或未知模式退出 2。扫描失败时仍尽量输出带 `error` 的 JSON 报告，并以非零状态结束。调用方同时检查退出状态及报告。

工程保存插入身份和状态，扫描缓存保存插件指纹与失败；共享音频队列由所属客户端创建及回收。`external_plugin_proof` 的独立输出目录和退出码见[外部证明](#外部证明)。

## 音频边界

源音频、混音、插件和离线处理固定为 **48,000 Hz**；设备输出另行重采样。换设备保持现有实例、插入链和状态。

控制协议为 v5 逐行 JSON；音频为 v3 mmap 双向队列，各预分配 64 槽。Unix 使用非阻塞管道门铃，Windows 使用命名事件。实时 DSP 量子为 256 帧，固定传输延迟为 **9,472 帧（197⅓ ms）**。启动期间为静音；串联插入的延迟相加，轨道由预分配 DelayPad 对齐。

实时设备最低 44,100 Hz，单个完整物理回调最多 8,192 帧。低于最低速率的设备配置被拒绝；超大回调返回 `CallbackBufferError`、写静音，不推进混音器或暗中拆回调。显式静音时钟使用自己的源时钟，不证明物理设备支持相应速率。

回调不等待子进程、不加互斥锁、不分配或释放缓冲。序列号及原子槽所有权隔离读写。队列满、过期或未及时完成的结果产生可见 xrun／直通；拒绝期间增长的时间序号不永久占用队列。插件退出、控制断开、非有限 PCM 或处理错误使实例失败并旁通。控制请求超时，或有待处理输入且两秒无输出进展时，监督线程终止宿主。

内置界面通过 `GetTelemetry` 读取每实例的实际输入／输出：固定容量采样环在宿主处理成功后更新，JSON 序列化在控制路径，频谱 FFT 在客户端后台执行。可见原生窗口每秒最多请求十次，绘制只读缓存。读数包括采样峰值、未加权 RMS、4× 重建 true peak、400 ms／3 s／综合 LUFS、响度范围与长时最大值，动态效果器另提供真实内部增益变化历史。响度采用立体声 BS.1770 K 加权与门限计算，直方图量化为 0.01 LU；这是实现与校准测试范围，不表示标准认证。停播后保留上次采样时间；不足完整窗口的综合响度保持未测量语义。ResetMeters 清除测量历史，不伪造音频帧数、不改插件参数。控制协议 v5 在握手时拒绝不匹配的旧宿主。

内置效果器的同步 LFO／XLFO，以及延迟、混响、动态节拍触发，接收工程速度图的私有样本偏移事件，宿主节拍同步支持 **20–1,000 BPM**。事件按每一级的上游实际延迟对齐：实时播放计算上游 DSP、桥接传输及轨道对齐延迟，离线导出计算上游 DSP 及相应轨道对齐延迟，不计实时桥接延迟。稳定循环播放在循环范围内对速度曲线作循环移位，使事件与延迟后的音频来源一致。速度是临时传输状态，不写入插件预设，不作为第三方插件参数发送；保存的参考速度保留各设备原生范围，只有启用同步参数（含会打开同步的自动化）的设备才限制不受支持的工程速度。纯自由时间处理继续可用。

跳转定位不会清空已有插件、反馈、轨道对齐和桥接缓冲。定位后的残留音频仍受原有缓冲历史及效果尾音影响；速度事件对齐不表示任意跳转已经消除这些瞬态。

`plugin_latency_samples()` 仅报告插件延迟；`transport_latency_samples()` 实时为 9,472、离线为 0；`latency_samples()` 为两者之和。失败旁通会改变有效延迟，该区间不能视为正常补偿音频。

## 参数、所有权与工程

`PluginClient::insert` 返回 `Arc<InsertHandle>`，注册表保留强引用；实时 `processor()` 只能取一次。处理器经音频控制线程退役并确认回收后才释放宿主。无关编辑和设备切换保留原链。必须重建时先保存状态，再创建新实例；拓扑／时间线均确认后才恢复播放意图。

控制请求在句柄内完整串行化。值必须有限且符合插件范围：CLAP/AU 使用普通值，VST3 使用 0–1 归一化值。VST3 在确认修改、恢复和保存前执行零采样参数处理，不推进音频。CLAP 一般参数使用事件／flush；`REQUIRES_PROCESS` 参数经停止、停用、非激活 flush、重新激活后确认，可能重置尾音与内部历史，不是采样精确自动化。

工程自动化在控制线程编译参数目标，实时经 `process_automated`、导出经 `process_offline_events` 提交采样偏移事件；音频与事件由同一共享槽持有。离散参数使用原生步长，变化的线性曲线须拒绝，不能静默量化。无效事件或超过量子事件上限时，整次调用在发布前拒绝。通用参数控制与采样事件是不同接口，保存状态不等于自动化已处理。

插入链存于轨道 `inserts` 字段（领域类型 `Track.unknown["inserts"]`）：

~~~json
{"inserts":[{"format":"clap","id":"com.github.free-audio.clap.gain","name":"Gain","path":"/installed/plugin.clap","version":"0.1","state_b64":"","bypass":false}]}
~~~

未知字段保留。恢复按格式与 ID 查目录，保存路径可辅助定位。缺失格式／插件或加载失败保留为 `InsertSlot::Missing`，身份、状态和未知字段不丢失。保存前通过 `get_state_b64()` 更新快照及旁通值；原生 `.verp` 的版本与恢复语义见[用户手册](../README.md#工程与恢复)。

## 内置 MIDI 与实时乐器

v5 的 MidiEvents 控制请求接收每批最多 1,024 条已验证的音符／CC／弯音／通道压力／逐音压力消息。音频块中的同类消息保留样本偏移，共用每量子 4,096 事件的容量和整批拒绝合同。通道范围为 0–15，未定义的位与非有限数值在修改状态前拒绝。

桌面先同步排入所属 InsertHandle 的有界 MIDI 队列，再安排后台投递；参数读取、状态保存和其他控制请求在同一请求锁内先排空已经排入的消息。保存或导出不会越过尚未启动的后台 MIDI 任务。控制侧队列满会明确释放发声；停止监听向原生效果器发送通道停止／复位，不留下触发包络。CC Learn 写入实际参数，导出与保存读取实际宿主状态。

Synth／One 的现场输入由预分配的原生合成器直接注入所属轨道插入前，经过现有轨道／主总线处理；没有内置效果插入时不经过子进程音频桥。已经添加的效果器仍按上述桥接与 DSP 延迟计算，不能把直接声源的路径当作整条链的端到端延迟保证。

实时乐器和工程音符播放／导出复用同一振荡器、滤波器、包络、琶音器和效果引擎。控制参数用独立事件命名空间原地更新，不重建正在发声的音符；整批参数先校验原生范围、离散网格和本乐器目标。音频事件队列按实例世代归属，轨道移动保留声部，旧实例事件不能写入新实例。预览键盘与外接 MIDI 的音符归属分离，关闭预览会释放自己的保持／琶音集合。

暂停播放时仍可演奏。输出速率转换为暂停中的实时监听保留独立时钟，主时间线已经预读的 PCM 留待继续播放。两个乐器的实时测量按每样本累计、至多约 30 Hz 原子发布，GUI 后台按 10 Hz 读取；绘制不计算 FFT。录制的力度、通道、弯音、CC 和压力保存在工程音符元数据中，由同一乐器引擎回放。外部乐器的通道／表情播放能力按其现有宿主合同验证，不能由内置乐器支持推导。

## 离线导出

`offline_insert` 创建独占实例，不进入实时注册表、不共享播放环。按顺序用 `process_offline` 提交等长立体声缓冲并等同一块的结果，支持短尾块。失败或超时中止导出。

CLAP 使用可选 render 扩展，明确要求实时硬件的插件拒绝离线；VST3 使用 `kOffline`；AU 使用可用的 OfflineRender 提示或正常原生 DSP 同步执行。离线 PCM 不含实时传输延迟，只补齐／裁掉插件自身延迟，不能再裁 9,472 帧。

每条含启用插件的轨道，在最后源音频后继续输入 **10 秒静音**再独立截止；无启用插件的轨道不追加。文件长度包含实际尾音样本。这是有界尾音策略，不能表示无限反馈已经自然衰减。

## 原生窗口

编辑器在宿主主线程运行：Linux X11 父窗口、Windows HWND、macOS NSView。Linux Wayland 桌面须提供 XWayland／`DISPLAY`，目前无原生 Wayland 插件编辑器协议。Windows 和 AppKit 消息队列独立服务；CLAP 主线程回调、POSIX FD、定时器和 GUI 请求单独服务。

先执行插件 destroy／removed／移除子视图，再释放父窗口。CLAP `gui.suggest_title` 仅用于插件自有浮动窗口，嵌入窗口由宿主设置标题。实际验收须覆盖打开、修改参数、尺寸变化、关闭、重开与实例释放。

## 外部证明

[external_plugin_proof.rs](../crates/verdant-plugins/examples/external_plugin_proof.rs) 使用公开 API，拒绝 builtin，适用于确定性增益／纯延迟夹具。每次 `--out` 必须是新目录。已有编译产物的调用示例：

~~~sh
/path/to/external_plugin_proof --host /path/to/verdant-plughost \
  --module /path/to/clap-plugins.clap --format clap \
  --id com.github.free-audio.clap.gain --param 0 --a -12 --b 6 \
  --expected-latency 0 --stall --out /path/to/new-proof
~~~

增益检查在任何音频前设置／保存 A、B，原位恢复并重存 A，再用新子进程比较非零 PCM 和短尾块。延迟夹具用 `--latency-only`，不声称参数／状态覆盖。回执包含 f32 WAV、状态／PCM 哈希和 `report.json`（`verdant-external-plugin-proof/2`）。

实时证明覆盖完整回调、非整量子、8,192 帧突发和 44.1／48／96 kHz 输出重采样，并分别核对插件延迟与额外传输延迟；还检查旁通、真实子进程崩溃、POSIX 挂起及回收。回调内存计数不包含控制线程。证明进程按 48 kHz 节拍驱动，不能代替物理声卡或桌面验收。

程序在扫描／实例化前建立进程树所有权：Unix 独占进程组和 EOF 守护，Windows KILL_ON_JOB_CLOSE Job Object。工作期限 120 秒，强制退出宽限最多三秒。正常退出回收自有子进程；不依赖可能复用的裸 PID 集合作为监督边界。

退出码：0 全部选定检查通过；1 操作／断言失败；2 参数或目录错误；77 能力不可用；124 总期限超时。检查分别记录 `pass/fail/unavailable/not_run`，缺能力不得写成通过。AU、原生 GUI 和真实设备另按[平台验收](PLATFORM-ACCEPTANCE.md)执行。第三方测试插件和模型不进入发行包；许可证保留于[第三方声明](THIRD_PARTY-PLUGINS.md)。
