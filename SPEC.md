# Verdant 规格

Verdant 0.1.0 的工程格式、事务、渲染和宿主合同。命令接口见 §21，平台验收见 §17。[源码](https://github.com/OpenGreendam/verdant)

## 1. 范围

### 1.1 已核验的上游边界

桌面编辑器直接编译检入的控件 Rust 源码。卷帘投影与会话手势的状态转换由上游 Proto UI 编译为 Rust，桌面会话实现守卫与领域副作用。上游编译入口、版本和目标见[编译规格](spec/SPEC.md)。
2026-11-13 的上游审阅固定在 Proto-UI `35195a6e6f1ba6a7d800da0029f2683013abd18b`，
其 GPUI Adapter 使用 `62e5991dd0f0c8a3af8d5e7e9c4652490d468db8`；独立 API 审阅使用
Zed `48741fd17d0dec28be6ad664b0a06dc7202086be`。
这些是历史来源，不能替代当前编译版本或平台验收。[来源、哈希与许可](spec/SOURCES.md)保留原样。

### 1.2 与当前合成能力的关系

默认声库为 `verdant-voice`，软件、worker 和声库版本同为 `0.1.0`。
包保留原统计模型，并要求 controller 与 decoder ONNX 输入；文件、接口、版本和来源须全部通过准入。
常规生产路线为 `verdant-voice`。本地 `verdant-voice import-originals` 核验原始 `.tkn/.bin` 文件对的两个固定 SHA-256，并登记保留引用。仅显式选择这一已准入引用时可运行受限 C0 乐谱合成。历史路线字符串和收据继续可读；显式请求已退役路线时须报告错误。

所有者已确认 `sn / 楠楠 / VID 10008` 为「童声01」。身份、数值可运行范围与听感验收分别记录。
原商业引擎上下文、协同发音和韵律的完整复原仍未成立；`quality_admission:false` 不得改写为听感通过。
原统计量、派生数据、ONNX 权重及其分发限制见 [RELEASE.md](RELEASE.md)。

### 1.3 编辑功能

0.1.0 提供多轨编辑、歌词与读音、曲线、撤销、`.verp`、本地合成、缓存播放和导出。
任意 Proto UI TypeScript、完整中文／英文／日文文本 G2P、网络协作、即时神经流式推理和外部工程无损兼容
均不属于已有支持。导入须报告损失；编辑和保存一个字段不表示当前路线能合成它。

## 2. 模块

| 层 | 所有权 |
|---|---|
| `verdant-domain` / `ProjectStore` | 工程、revision、原子命令批、撤销历史；唯一写者。 |
| `verdant-session` | 焦点、选择、视口、手势草稿、文本组合；不复制可写工程。 |
| `verdant-app` 与生成投影 | 原生事件、布局、绘制、领域命令分派。 |
| `verdant-backend` | 预检、不可变请求、排队、取消、内容缓存与制品验证。 |
| `verdant-service` | 桌面与 CLI 共用的 PCM 准备、离线混音、效果处理和文件输出。 |
| `verdant-cli` / `plugins/dsh` | 本地命令和版本化 JSON 接口。DSH 调用 CLI 并管理宿主任务和附件。 |
| `verdant-worker` / `verdant-synthesis` | `ogd-worker/1` 服务、模型准入、Rust／原生合成。 |
| `verdant-audio` | 准备好的 PCM、混音、设备 sample clock 与运输。 |

绘制和音频回调只消费已准备的状态和 PCM。导入、推理与制品处理在后台进行，结果按工程身份和 revision 提交。

## 3. 运行

解压发行包后运行对应程序。桌面、CLI、worker 和安装器使用本地声库目录与原生运行库。[入口](README.md) · [声库安装](docs/verdant-voice.md) · [发行](RELEASE.md)

```sh
bin/verdant song.verp
bin/verdant --import source.ufdata
bin/verdant --worker-binary /path/to/verdant-worker --workspace /path/to/cache
```

开发构建在开发容器中执行，依赖须事先准备：

```sh
cd frontend
cargo build --offline --locked --release -p verdant-app -p verdant-cli -p verdant-worker -p verdant-voice
cargo run --offline --locked --release -p verdant-app -- song.verp
```

新建工程 → 选择已准入声库与语言 → 输入音符、歌词／明确读音 → 查看乐句预检 → 渲染 → 播放 → 保存。
编辑使受影响缓存失效。界面区分待渲染、已接受、运行、失败和可播放；只拦截含错误的乐句，错误须能定位到音符。
键位、面板和具体操作见[前端手册](frontend/README.md)。

## 4. 工程领域模型与持久化

### 4.1 规范数据类型

原生 JSON 为 `format="greendam-project"`、`version="0.1"`。
软件版本 `0.1.0` 与工程 schema `0.1` 分开。字段定义以
[model.rs](frontend/crates/verdant-domain/src/model.rs)为准。

| 数据 | 约定 |
|---|---|
| 对象 ID | 稳定、不透明；复制内容生成新的音符、控制点和实例 ID。 |
| revision | 整数 `0..2^53-1`；有效提交递增一次，溢出拒绝。 |
| tick / PPQ | 整数音乐时间；新工程 PPQ=960，导入保留源 PPQ；音符起点非负，长度至少 1 tick。 |
| 音高 | MIDI 整数和整数 millicents 偏移；曲线相对音符基准。 |
| 增益／声像 | millidecibels／permille；静音独立保存，不写 NaN、Infinity。 |
| 速度 | 正整数 `micro_bpm`；tempo map 与拍号独立。 |
| 文本 | UTF-8；显示歌词、明确读音、规范音素、来源与近似记录分别保存。 |

工程保存声库引用、轨道、音符、曲线、渲染设置和扩展；视口、hover、草稿不参与合成输入。
未知字段通过各对象的扩展映射往返保留。当前 loader 接受 `0.*` schema；这不是任意未来字段可编辑的保证。

### 4.2 时间与音高的精确映射

常速段 `seconds = ticks * 60 / (PPQ * BPM)`，变速按段积分。
音频边界按绝对时间映射后舍入，长度取末端与起端之差，不能逐段独立舍入积累漂移。
`F0 = 440 * 2^((midi_note - 69 + cents/100)/12)`。
PIT、颤音和滑音合成后按路线范围预检；不得钳位超限音高或用 MIDI 存储范围代替声学范围。

### 4.3 工程容器

桌面“打开”“保存”“另存为”和工程副本导出只接受 `.verp`；无后缀的保存目标补 `.verp`。
旧 `.json`、`.verdant.json`、`.gdmproj` 与交换格式走显式导入，生成独立工作副本，保留源文件并显示损失报告。
领域层保留读取历史 JSON／ZIP 的接口，不改变桌面入口政策。

新建和导入时写 `extensions["verdant.created_with"]`；普通保存、另存为保留创建版本。
按 SemVer 比较创建版本；较新或无法解析的版本须先提示“取消／继续”，通过后才能打开或修改 journal。
无此标记的早期 `.verp` 仍可打开。

保存先完整写入并同步同目录的临时文件。Windows 用带 `MOVEFILE_WRITE_THROUGH` 的 `MoveFileExW` 原子替换。目标仍由共享删除句柄打开时，Rust 的 Windows `rename` 可用 `FileRenameInfoEx` 替换。Unix 用 `rename` 替换后同步目录。
rename 前失败保留旧文档；rename 后的目录 sync 失败须报告，不能保证文档仍为旧字节。
文档先保存，再更新恢复 checkpoint；两者不是一个跨文件原子事务。

恢复日志在完整文件名后追加 `.journal`，如 `song.verp.journal`。
首帧含工程与 undo/redo checkpoint，后续为已提交事务；预览、拒绝和无变化编辑不写日志。
恢复校验工程身份、保存内容 digest 与 revision 链；完整事务可重放，撕裂尾帧可修复，较早损坏或外来身份拒绝。
用户选择保留恢复状态或回到最后保存文档；两种选择都不覆盖手动保存文件。
若文档保存成功而 checkpoint 未完成，只能以已验证链上的内容匹配识别该窗口，不能仅比较 revision。
checkpoint 替换失败导致写者身份不确定时，要求重新打开再编辑。

## 5. 钢琴卷帘与坐标体系

音乐坐标是命中和绘制的共同输入：`x=(tick-view_start)*px_per_tick`，纵轴由音高和行高映射。
逻辑像素处理 DPI；缩放不改工程。指针锚定缩放保持锚点对应的 tick，吸附粒度独立于网格显示密度。
按时间／音高区间裁剪可见内容，不为全工程每个音符建立长期 Entity。离屏音符仍须能用键盘访问。

### 5.1 命中与优先级

活动控制点、选中音符边缘、音符主体、背景按工具上下文处理；一次按下只认领一个手势。
鼠标与触控使用各自热区。捕获期间必须处理松手、Escape、失焦与取消，并释放捕获及边缘滚动。
具体阈值以 `verdant-session` 为准，不把早期样例数值当作平台保证。

### 5.2 操作语义

组移动共用 tick／pitch delta；整组边界约束保持相对位置。调整起止保持长度至少 1 tick，穿越时钳位。
复制提交生成新 ID，取消不留下对象。删除连同附属曲线作为一个批处理。
单声部重叠保留原编辑并给出合成诊断，不静默截断或丢音符。

## 6. 事务、撤销和并发 revision

手势执行 `begin → preview* → commit | cancel`。preview 不改工程或 revision；commit 校验 base revision。
一个有效 `CommandBatch` 产生一个新 revision 和一个 undo 项；无变化不产生二者。
撤销／重做仍经 reducer 并生成新的单调 revision。脏状态比较当前内容 digest 与最近保存 digest。
持久化事务须先写日志再发布工程；日志写失败不改变快照、revision 或历史栈。

拖动时发生其他有效提交，取消旧草稿，保留新工程；Escape、失焦或取消同样清理草稿。
IME marked text 不进入历史，一次提交按文本会话形成原子编辑。批量歌词、量化和曲线笔画各作为一个用户意图提交。
渲染制品不是可撤销工程编辑；回到同一内容可以复用缓存，revision 不因此倒退。

## 7. 歌词、拼音、音素与 IME

使用 GPUI 文本输入接口；UTF-8 存储与平台 UTF-16 range 在合法边界间转换，移动／删除遵循字素边界。
IME 优先处理组合中的 Enter、Space、Escape，其次文字框，最后工作区快捷键。
组合期间不得启动播放或触发卷帘命令。候选框必须按实际 shaping 位置定位。

读音优先保留手工／导入值与用户词典；普通话可按词组与单字默认读音选择，须允许更正。
显式音标和假名保留规范 IPA、源文本、时长与近似来源；日语假名是已选择的跨语言近似，不是原生日语声库。
混合片段与各语言的规则见[多语言契约](spec/MULTILINGUAL-PHONEMES.md)。
未知读音和不满足模型库存的音素必须预检报错；不得填任意替代音素。重新生成读音不能无意覆盖手工选择。

## 8. 音高、包络与音素时长

| 控制 | 有效映射 |
|---|---|
| PIT / VIB / POR | 连续音高控制；含每音符颤音与相邻音符滑音。 |
| DYN | 渲染后逐样本增益，播放与导出共用。 |
| BRE / BRI / CLE / GEN / OPE / GWL | `vocaloid-controls-v1` 输出表达处理，范围 0–127；不表示 VOCALOID 二进制等价。 |
| 音素比例／时间 | 在总时值约束内分配实际模型音素；逐项校验时长下限。 |
| 原模型专属处理 | 仅受限原始引擎按实际 capability 启用其支持的参数。 |

曲线点有稳定 ID 与音符相对时间；插值、重复时间归一化及拉伸按领域命令执行。
显示简化不得改保存或合成点列。绘制、拖动和精确输入共用事务路径。
工程保留原模型专属设置。适配器只编译所选声库支持的参数。
直接 worker 请求带不兼容处理时必须拒绝。控件启用条件必须对应实际映射，不展示无效音色旋钮。

## 9. 键盘、焦点与无障碍

菜单、按钮与快捷键调用同一命令路径。默认键表以应用绑定与 F1 窗口为准：Space 播放／暂停，
Ctrl/Cmd+Z 撤销，Shift+Ctrl/Cmd+Z 重做，Escape 取消临时操作。单字母动作只在工作区生效。
Tab／Shift+Tab 提供焦点访问；文本框与 IME 优先消费文本操作。

音符表与控制点应有稳定语义 ID、名称、值、选择和编辑动作；ID 不得使用可见数组下标。
离屏访问须滚入目标；颜色不能成为错误或选择的唯一信号，进度播报只在阶段变化或节流后进行。
AccessKit／AT-SPI 接口和树存在不等于读屏完成音符／控制点操作；平台验收边界见 §17。

## 10. 预听、运输和音频线程

播放头由设备 sample clock 投影；GUI timer 只驱动绘制。无已验证 PCM 时显示等待渲染状态。
播放与导出使用同一准备 PCM 和混音／自动化实现，配置不能通过丢参数或降低质量获得速度。

回调消费预分配数据和有界控制；禁止文件／网络 I/O、模型推理、阻塞等待、日志格式化及大型分配／析构。
stop 不得静默丢失，换入和旧 buffer 回收须遵守实时边界。设备丢失保留工程与缓存并显示错误。
设备报告范围时默认请求 1024 帧，并限制在设备范围与回调容量内；未知范围交给系统。
48 kHz 的 1024 帧约 21.3 ms，仅为请求周期，不是端到端延迟保证。

## 11. 渲染调度、缓存与状态呈现

每次提交捕获不可变工程／声库／路线／有效参数快照，后续编辑不修改该请求。
调度服从 `capabilities.max_active_jobs`；当前 worker 按可用 CPU 数选择 1 或 2，不能固定假设单槽。
取消请求不释放名额，收到终态后才释放；进度只显示真实阶段或后端比例，无比例时使用不定进度。

缓存身份绑定协议、软件／worker／合成源哈希、实际模型与原生库哈希、路线和有效音乐输入／参数。
视口、hover、文件名不是合成身份。结果须校验目录包含关系、字节数、SHA-256、有限 WAV 与 NPZ 音高／时间映射。
旧 revision 结果可留缓存，只有当前内容和请求绑定匹配时才能成为当前可播放结果。
取消、部分产物与损坏图不能进入 ready。进程重启按会话隔离迟到事件；崩溃须收束未完成任务并显示原因。

## 12. 后端能力适配边界

规范入口：[请求 schema](backend/worker-request.schema.json)、[运行合同](backend/RUNTIME.md)、
[Rust worker](frontend/crates/verdant-worker/src/main.rs)。领域模型与 wire 类型分离。
启动等待 `ready`，再请求 capabilities；stdout 仅为 UTF-8 NDJSON，单请求上限 8 MiB，stderr 不是协议。
公共字段为 `protocol/request_id/op`；render 需要 `job_id/revision/route/payload`。
`job_id` 为 1–64 个 ASCII 字母数字、`_` 或 `-`，归档后不可复用；request_id 仅作请求关联。

| 消息／限制 | 合同 |
|---|---|
| 路线 | `verdant-voice`；只有经双 SHA-256 准入并显式引用的原始文件对可使用受限 `C0`。 |
| `native_score` | 普通乐句上限 30 秒；连续运行扩展仅按 capability 声明的 `continuous-runs-v1` 使用，最多 120 秒／2048 段／32768 控制点。 |
| 音素下限 | `verdant-voice` 为 5 ms／音素；受限 `C0` 为 25 ms／音素；不得静默延长不足时值。 |
| 显式 F0 | 35–2000 Hz；不是整个范围的听感保证。 |
| `accepted` | 任务已建立，尚不能播放。 |
| `progress` | 真实阶段与可选比例；不按计时猜测。 |
| `completed.artifacts[]` | workspace 相对路径、bytes、sha256；验证后再绑定。 |
| `rejected` | 请求未接收，包括 BUSY、重复 ID、无效参数。 |
| `cancel_ack` | 取消请求已处理，仍等该任务实际终态。 |
| `completed/failed/cancelled` | 每个接受任务只接收一个终态。 |
| EOF／shutdown／终止 | 监督并清理活动进程，不能遗留运行任务。 |

`verdant-voice` 默认 48 kHz、240 样本步长；`voice_music` 可明确请求 44.1 kHz。
原统计模型的 controller reference 为独立 45–800 Hz 参考；decoder 保留 authored pitch，不能用参考范围裁掉曲谱。
普通工程编辑与已准入声库推理无需安装 Python 或额外下载。
任务目录保留请求、实际命令、日志与音频／条件／来源图。最终输出操作以报告为完成标记，重入须验证全图，不得重复处理。
未知 capability 按不支持处理；协议不兼容时拒绝启动 renderer，工程编辑仍可使用。

## 13. Compiler 的受限输入与阶段

### 13.1 控件源码与卷帘声明

控件和命令表由 `frontend/crates/verdant-app/src/generated/controls.rs` 的检入 Rust 源码实现。
`frontend/proto-ui/src/pitch-gesture-surface.proto.ts` 声明卷帘投影及移动、复制、调整长度与框选的状态转换。上游编译入口位于 `frontend/proto-ui/`，按该目录的 `compiler.pin.json` 编译。Rust 会话调用该声明编译出的数据方法，负责守卫判断与领域副作用。

### 13.2 编译阶段

上游 Proto UI 编译器检查卷帘声明并生成 Rust。生成物保留 source map 与来源记录，Rust 宿主通过具名接口调用生成的投影与数据方法。详细输入和证据范围见[编译规格](spec/SPEC.md)。

### 13.3 上游 GPUI 实验

固定历史上游有真实 Rust GPUI host、style、session/epoch/lease 协议和有限 AccessKit，
并由未发布的仓库内 TS Runtime peer 驱动；`private:true` 是发布标记，源码公开。
它不是已经完成的纯 Rust Compiler。来源代码、借鉴语义与本项目新增代码须分开归属。
成品不带 TS peer／JavaScript 解释器。上游 host protocol 与声音协议 `ogd-worker/1` 各自独立。

## 14. 历史协议与 IR 样例

早期设计样例记录组件身份、状态所有权、事件、守卫、顺序副作用与投影义务。样例及其来源保留在原路径。[协议与 IR 样例](spec/examples/)

## 15. 宿主 profile 与错误语义

上游编译目标见 `frontend/proto-ui/compiler.pin.json`。桌面宿主能力按平台逐项记录实现与证据。[平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md)
指针捕获、焦点、文本组合、绘制、无障碍、命令和音频控制分别准入；未知义务拒绝。
诊断须指明源位置、失败义务／类型、目标 profile 和修正方向；动态 JS、错误所有权与实时域推理不可降级忽略。

## 16. GPUI 代码生成职责

生成 View 消费领域快照并分派事件；音符画布、shaping、IME、音频与高密度索引保留为具名宿主 primitive。
primitive 同样公开身份、事件、动作和边界合同。paint 只消费已准备布局，不修改工程。
mount／unmount 管理订阅与手势；后台回调使用弱引用／会话 token，销毁后不重建窗口。
上游编译的 `PitchGestureSurface` 负责根尺寸与裁剪，native slot 负责卷帘绘制及音乐坐标事件转换。会话经编译后的数据方法取得手势状态转换，并在 Rust 中执行领域副作用。
该组件的通过不表示整个编辑器或任意 TypeScript 已编译。

## 17. 可复验 contract traces

验收按层记录：领域 reducer、生成 handler、协议与制品、原生输入、目标机 IME／读屏／音频。
各层保留源提交、生成输入、profile、OS／窗口系统、设备、观测与失败；截图不能代替操作语义。
必须覆盖拖动一次提交、取消不改工程、复制取消无残留、最小时值、IME 不触发运输、字素边界、
锚点缩放、过时任务、取消竞争、能力拒绝、离屏键盘访问、损坏制品、设备丢失与未知编译义务。

Linux 发行与安静 Pride 已用真实本地模型、断网、48 kHz 原生路径核验。
Windows x86_64/MSVC 与 Apple Silicon 有打包代码，原生验收待完成；不能表述为已发布或已通过。
平台 IME 提交／候选框、完整音符／控制点读屏、物理触摸、音频热插拔和 macOS AU 须分别保留验收结果。
已有模型／AT-SPI 树／冒烟证据不能覆盖这些范围，见[平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md)。

## 18. 性能与资源预算

以下保留为目标，不能当作已测保证。基准：4 物理核、8 GB RAM、集成 GPU、1080p/60 Hz。

| 场景 | 目标 |
|---|---|
| 总音符 100000、可见不超过 2000 | 帧 p95 ≤16.7 ms，UI CPU p95 ≤8 ms。 |
| 命中／选择 | p95 ≤1 ms，避免全工程扫描。 |
| 拖动／缩放 | 输入至显示 p95 ≤32 ms。 |
| 准备 PCM 预听 | 指令至开始播放 ≤50 ms。 |
| 音频回调 | p99 小于块时长 25%，压力下无未解释欠载。 |
| 领域／UI 内存 | 基准工程 ≤512 MB，不含模型与音频缓存。 |

冷启动与暖机分开报告；测量设备／缓冲、分辨率、可见内容和计时终点必须固定。
现有 12000 音符／4000 分段 Linux Wayland 窗口提交测量只覆盖其具体设备与视口，
不能推导本表十万音符或端到端设备延迟已通过。不为帧率降低音质或隐藏过期结果。

## 19. 实施顺序与阶段完成定义

保留 P0–P5 名称供历史引用：P0 版本与所有权；P1 编译纵切片；P2 编辑与持久化；
P3 后端／音频闭环；P4 曲线、无障碍与性能；P5 分发验收。
当前已有各层实现，完成定义是对应合同与目标平台证据，不是重新从 P0 开工，也不是所有阶段统一通过。
原始决定见 [P0-DECISIONS.md](frontend/P0-DECISIONS.md)，其中早期能力状态按历史阅读。

## 20. 交付清单与本目录验收范围

交付源码、锁定依赖、生成记录、schema、构建／启动方法与第三方通知。
`spec/examples/`、`source-audit/` 和 `source-manifest.json` 是保留的设计／来源材料；
早期 `.gdm.json` 和 worker 示例不能代替当前桌面文件政策、schema 或已执行平台验收。
原生交互、合成和性能以对应版本的运行验收为准。[平台验收](frontend/docs/PLATFORM-ACCEPTANCE.md)
用户路径、可写目录、动态库和打包范围以 [运行手册](backend/RUNTIME.md)与 [RELEASE.md](RELEASE.md)为准。
显式 `VERDANT_STATE_DIR`、`VERDANT_USER_DATA_DIR`、`VERDANT_CACHE_DIR` 优先；
`VERDANT_DATA_DIR` 指只读包资源。Linux 使用 XDG，Windows 使用 LOCALAPPDATA/Verdant，
macOS 使用 Application Support/Verdant 与 Caches/Verdant；实现定义见
[platform.rs](frontend/crates/verdant-backend/src/platform.rs)。

### 桌面外观保留要求

Verdant 桌面发行必须保留 Pride 主题、手动选择和按本地日期自动启用能力。
默认自动模式在 5 月 17 日、6 月 28 日和 10 月 11 日启用，其余日期使用原配色。
用户在任意日期可选择自动、开启或关闭。模式只改变界面绘制，不改变工程、编辑、渲染、音频或快捷键。
启用与停用不产生弹窗、通知、日志或遥测。移除主题或自动／手动入口的版本不通过桌面发行验收。
此条属于项目验收规范；软件许可见 [LICENSE](LICENSE)。

## 21. CLI 与 DeepSeek Harness

`verdant-cli capabilities --json` 查询命令与音频格式。`verdant-cli schema` 查询公开操作参数。
桌面和 CLI 使用同一领域命令与工程锁，按已提交的 revision 读取和编辑。合成经同一 worker 调度，以通过校验的 PCM 作为混音输入。

DSH 使用宿主工具和 jobs 调用 CLI 的 `verdant-cli/1` 协议。工程与试听可供领取。通道是否发出文件、发出原生语音以及后台是否取得发送回执，各自记录。

命令、协议与验收条件见 [CLI／DSH](spec/CLI-DSH.md)。

## 来源标记

上游固定引用、SHA-256、Git blob 指纹与原许可见 [SOURCES.md](spec/SOURCES.md)、
[source-manifest.json](spec/source-manifest.json)与 [licenses](spec/licenses/README.md)。
桌面控件 Rust 源码、卷帘声明与 Verdant 特有合同归本项目。Proto UI／GPUI 接口及其实现归相应上游。
字典及算法来源见 [NOTICE.md](NOTICE.md) 和 [合成忠实度](SYNTHESIS-FIDELITY.md)。
