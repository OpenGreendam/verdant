# verdant(1)／verdant-cli(1) 与 DSH

## 用法

`verdant COMMAND [ARGUMENTS] [--json | --events]`（`verdant-cli` 保留为兼容别名）

`verdant-cli request --json < REQUEST.json`

`verdant worker --stdio`（`verdant-cli serve --stdio` 保留为兼容别名）

## 说明

Verdant CLI 在无显示、声卡和 MIDI 设备的环境中编辑、检查和渲染工程。桌面和 CLI 共用工程 reducer、恢复日志、乐句规划与离线 PCM。音频由单独的 worker 进程合成。DSH 在宿主进程内调用这个命令，使用宿主管理长任务和附件。

## 命令

| 命令 | 参数与结果 |
| --- | --- |
| `capabilities`, `schema [OP]`, `doctor` | 枚举操作、领域命令、交换格式、输出设置和实际 worker 能力。 |
| `project new OUTPUT` | 新建工程默认启用元音对拍。可指定 PPQ、初始工程和操作。提供初始工程时沿用其中的时序设置。 |
| `project inspect FILE`, `project validate FILE` | 检查保存快照、可恢复 revision、创建版本和领域结构。 |
| `project apply FILE --expect-revision N --operations JSON` | 一次提交操作数组。对象 ID、字段及操作名由 `schema project.apply` 枚举。 |
| `project save-as`, `project recover`, `project migrate` | 保留领域内容，显式选择版本或恢复。写入以 revision 为条件。 |
| `history list|undo|redo FILE` | 读取、撤销或重做持久化操作。 |
| `import SOURCE --output FILE`, `export project FILE --output SCORE` | UST、USTX、UFDATA、MIDI、SVP、VSQX、VPR。格式能力和精确参数由命令查询。 |
| `project pack FILE OUTPUT.zip`, `project unpack ARCHIVE DIRECTORY` | 携带原生工程与素材，校验大小、SHA-256 及相对路径。 |
| `media import|relink|inspect|verify`, `effect scan|catalog|status` | 导入、重连和校验素材，扫描或查询本机效果。 |
| `render.plan`, `render`, `export audio` | 查看实际乐句计划，合成和导出 WAV，返回音频与渲染报告。 |
| `job list|inspect|verify|cancel|retry`, `cache list|verify|clean` | 按 workspace 查找可重验的作业、取消、重试或管理乐句缓存。 |
| `voice list\|verify\|install\|remove\|set-default\|import-originals\|convert` | 查询、校验和管理本地声库。装包、原版解锁与转换共用 `verdant-voice` 的实现。[声库手册](../docs/verdant-voice.md) |
| `batch --requests FILE` | 顺序执行有界请求列表。每个工程事务独立，`--stop-on-error` 停止提交后续项。 |

工程操作覆盖轨道、parts、音符、歌词、读音、字典、tempo、meter、十项表达曲线、混音、素材、效果和自动化。`project apply` 使用稳定对象 ID 和单一原子批次。`--dry-run` 检查批次，返回预演结果。批次中没有有效变化时 revision 不递增。

## 输入和输出

路径既可为命令参数，也可随 JSON 提供。`--params FILE` 读取参数对象。`--json` 输出一个结构化结果；`--events` 输出逐行 JSON，末行是终态。音频保存到指定文件。

`request` 从 stdin 读取一个 UTF-8 JSON 对象，读取到 EOF 后执行。`worker --stdio`／`serve --stdio` 逐行读取并顺序执行，每次最多一个活动请求。独立 `verdant-worker` 继续使用声库合成协议。公开请求最大 2 MiB。

```json
{"protocol":"verdant-cli/1","request_id":"inspect-1","op":"project.inspect","params":{"path":"song.verp"}}
```

响应回显 `protocol` 和 `request_id`，含 `ok`、`result` 或 `error`，以及 `diagnostics`。错误含 `code`、`message`、`retryable` 和可选的 `details`。事件另含 `event`、从 1 递增的 `sequence` 和适用时的 `job_id`。进度事件的 `data` 来自已发生的阶段。

## 工程与恢复

只读检查和预演不打开写日志。桌面和 CLI 写入同一个工程时，文件锁选择唯一写者。编辑请求以 `expect_revision` 比较恢复链的最新 revision，可用 `expect_project_id` 约束工程身份。恢复分支需要 `recovery=saved` 或 `recovery=recovered`。创建版本超出当前版本时，用 `allowed_creator` 提供看到的完整版本字符串。

交换操作报告保留、近似及丢失项。`loss_policy=strict` 拒绝有损写入，`loss_policy=allow` 接受具体损失。输出默认拒绝覆盖。保留上传的原文件和原工程。

工程包包含 `.verp`、相对素材和校验清单。解包到新目录后打开其中的 `.verp`。相对素材从工程目录解析并重新校验；原生绝对素材需要调用方授予的读取范围。

## 渲染

```sh
verdant-cli render song.verp --output mix.wav --report render.json --events
verdant-cli export audio song.verp --output stem.wav --scope Stems --encoding Pcm24
verdant-cli job verify JOB_ID --json
```

WAV 支持 44100、48000、88200、96000、192000 Hz，单声道与双声道，Float32、PCM16 与 PCM24。`--scope` 选择 `Mix`、`ActiveTrack` 或 `Stems`；`ActiveTrack` 需要 `--track`。`--start-tick` 与 `--end-tick` 一起给出范围。`max_duration_seconds` 在合成前约束时间线。效果处理与桌面共用离线宿主，包含自动化、延迟补偿和有界尾音。

作业记录工程 revision 与 SHA-256、有效设置、声库、素材和效果身份。缓存按乐句内容查找，命中后重验。完成结果记录 WAV 与报告的路径、字节数和 SHA-256。`job verify` 重新验证文件。相同幂等键只重放输入一致且制品通过验证的作业。

作业状态为 `queued`、`running`、`cancelling`、`succeeded`、`failed`、`cancelled` 或 `interrupted`。取消等待 worker 终态或受控退出。重试分配新 ID 并关联旧 ID，已校验缓存可复用。中断的推理按完整输入重新调度。

## DSH

插件由宿主 Cordis 加载，调用固定 `verdant-cli` 和参数数组。宿主版本和插件服务见[上游接口记录](../docs/research/cli-dsh-research-20261120.md)。管理配置指定程序、workspace、声库和资源上限，上传文件写入受管目录并绑定源哈希。任务、领取和取消绑定可验证的请求身份。未经授权的本地主机路径和原生效果不能由附件选定。

插件接受原生工程、支持的交换格式或明确的音符、节拍和歌词。歌词请求需要旋律。作业返回已校验试听和可编辑工程，传输包可携带素材。宿主关闭或卸载时终止受监督转换。后台 result 可先领取；实际通道的阶段以发送收据为准。

| 通道 | 交付 |
| --- | --- |
| Web / Desktop | 受管产物入口。 |
| 腾讯 QQ 插件 | 是否支持原生音频、后台上下文与送达，以部署和实测为准。 |
| 社区 dsh-im | 活跃 turn 可登记产物。queued 须等待通道结果；后台沿许可的会话上下文交付。 |

投递单独记录 `not_requested`、`pending`、`queued`、`sent`、`failed` 或 `unknown`。超时且无收据可查询时保留结果供领取。聊天平台命令需在该通道配置后使用。

## 退出状态

| 值 | 含义 |
| --- | --- |
| 0 | 请求成功。 |
| 2 | 参数、工程、损失或版本决策未通过。 |
| 3 | 能力、声库、素材或预算不可用。 |
| 4 | 工程锁或 revision 冲突。 |
| 5 | I/O、worker 或产物校验失败。 |
| 70 | 程序内部错误。 |
| 130 | 前台任务取消。 |

## 验收

每项记录源码、worker、声库与宿主版本、输入摘要、平台和结果。听感、PCM 校验和真实 IM 回执分别验收。

| ID | 场景 | 通过标准 |
| --- | --- | --- |
| A01 | 无 DISPLAY/Wayland、无声卡/MIDI，断网导入→编辑→合成→多轨导出 | 无 GUI／设备初始化；产生完整、可验证文件和报告。 |
| A02a | 同一黄金工程经桌面与 CLI | 领域内容、诊断、计划、缓存身份一致；同环境确定性 PCM 一致。非确定第三方效果单列容差和原因。 |
| A02b | dsh 调用同一黄金工程与操作 | 对照 A02a 的领域内容、诊断和最终 PCM；插件不改变有效输入或重解释参数。 |
| A03 | 全功能矩阵逐项映射 | 每个持久化桌面动作有公开操作与用例；只支持 render 不得标为全功能。 |
| A04 | 非 960 PPQ、tempo/meter 变化、七格式往返 | 音符位置和时间映射正确；所有损失定位并报告。 |
| A05 | 十项表达、音素时长、手工读音、自动化与效果 | 真正进入同一有效映射或明确拒绝；不静默丢弃。 |
| A06 | 中日英、重复元音（含 `ああ`）、连续与休止、短末音、移调边界 | 复用发声覆盖 benchmark；预期有声区域满足门控，合法闭塞／休止不误报；不可评估不能当通过。 |
| A07 | 长歌跨段、重采样、多轨、延迟补偿和尾音 | 无新增重复 ramp、漏音／叠音、样本漂移；最终混音与桌面一致。 |
| A08 | 只读检查、批量原子性、撤销／重做、恢复、桌面与 CLI 同时打开 | 只读不写文件；冲突不覆盖；日志和 revision 合同成立。 |
| A09 | 取消竞争、EOF、磁盘满、worker 崩溃、重启、重复消息 | 单一终态、无孤儿进程／损坏 ready；去重与 interrupted／重试可解释。 |
| A10 | 缺声库、版本／指纹不符、缺素材、不可离线效果 | 可定位诊断；不自动换默认、补权重或 bypass。 |
| A11 | dsh 安装、工具 schema、宿主 jobs、热卸载 | 可在干净安装中启动，版本范围明确，所有任务被监督。 |
| A12 | 已选 IM 通道真实发工程→异步渲染→收试听与工程 | 分开核对渲染、queued、sent；实际播放并记录通道限制，不能用本地 mock 代替。 |
| A13 | 投递失败／超时、原 turn 结束、跨会话并发、上传恶意路径 | 不串会话、不盲目重发，结果可领取；输入限定在受管范围。 |
| A14 | bot 工程包在另一台桌面解包、重开、继续编辑再渲染 | 素材可解析、声库选择显式、工程可编辑；不能依赖 bot 主机路径。 |
| A15 | Linux、Windows x64、Apple Silicon 发行包 | 各自核验无编译器安装、CLI／worker／库定位；平台结果分别登记。 |
| A16 | 安装／文档／包元数据 | 指向 `OpenGreendam/verdant`，模型不混入软件／npm／工程包。 |

[工程规格](../SPEC.md) · [发行](../RELEASE.md)
