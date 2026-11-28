# 运行配置

本文件说明 0.1.0 基线的桌面与 Rust worker。命令语法、NDJSON 和退出码见 [verdant-worker(1)](README.md)；编辑操作见[桌面手册](../frontend/README.md)。

## 包布局与启动

| 平台 | 桌面入口 | worker／效果宿主 | ONNX Runtime 1.23.2 |
| --- | --- | --- | --- |
| Linux x64 | `verdant-launcher` 或 `bin/verdant gui` | `bin/verdant-worker`；`libexec/verdant/verdant-plughost` | `lib/libonnxruntime.so.1.23.2` |
| Windows x64／MSVC | `verdant-launcher.exe` 或 `bin/verdant.exe gui` | 同目录 `verdant-worker.exe`、`verdant-plughost.exe` | 同目录 `onnxruntime.dll`、`onnxruntime_providers_shared.dll` |
| macOS Apple Silicon | `Verdant.app` | `Contents/MacOS/{verdant-worker,verdant-plughost}` | `Contents/Frameworks/libonnxruntime.1.23.2.dylib` |

构建与打包命令由 [RELEASE.md](../RELEASE.md)维护。Windows 与 macOS 的原生验收待完成；macOS 基线声明最低 13.0，临时签名不表示公证或目标机验收。[平台验收](../frontend/docs/PLATFORM-ACCEPTANCE.md)

```sh
/path/to/package/bin/verdant gui /path/to/song.verp
/path/to/package/bin/verdant gui --import /path/to/source.ufdata
```

`verdant` 无参数显示帮助并退出；`verdant gui` 启动独立 GUI。包根 launcher 可直接启动 GUI。`verdant worker --stdio` 为 Agent 的顺序工作流协议，独立 `verdant-worker` 保持音声合成协议；参见 [Agent 自动化手册](../frontend/docs/AGENT-AUTOMATION.md)。

桌面默认启动真实 worker。`--worker-binary FILE` 优先于 `VERDANT_WORKER_BINARY`，随后选择桌面二进制同目录的 worker；显式路径不存在时启动失败。`--silent-audio` 只改播放时钟，合成仍执行真实模型。

worker 的 ONNX 库选择为 `--onnxruntime-library` → `VERDANT_ONNXRUNTIME_LIBRARY` → 包布局库 → worker 同目录库；指定的文件不存在时拒绝。要求 1.23.2／API 23，不根据任意同名库推定兼容。

常规编辑和声库推理可离线运行，无需音频设备或 GPU。

## 文件与目录

覆盖变量非空时优先；下表中的默认值由 [platform.rs](../frontend/crates/verdant-backend/src/platform.rs)维护。

| 用途 | 环境覆盖 | Linux | Windows | macOS |
| --- | --- | --- | --- | --- |
| state | `VERDANT_STATE_DIR` | `$XDG_STATE_HOME/verdant`，否则 `~/.local/state/verdant` | `%LOCALAPPDATA%/Verdant/state` | `~/Library/Application Support/Verdant/state` |
| 用户 data | `VERDANT_USER_DATA_DIR` | `$XDG_DATA_HOME/verdant`，否则 `~/.local/share/verdant` | `%LOCALAPPDATA%/Verdant/data` | `~/Library/Application Support/Verdant/data` |
| cache | `VERDANT_CACHE_DIR` | `$XDG_CACHE_HOME/verdant`，否则 `~/.cache/verdant` | `%LOCALAPPDATA%/Verdant/cache` | `~/Library/Caches/Verdant` |

Windows 缺少 `LOCALAPPDATA` 时用用户目录的 `AppData/Local`。无法解析持久目录时程序明确诊断临时回退；需要长期保存时显式设置覆盖变量。

| 文件／目录 | 默认与覆盖 |
| --- | --- |
| 桌面偏好 | `state/prefs.json`。 |
| 桌面注册表 | `state/voices.json`；`--voice-registry FILE` 优先于 `VERDANT_VOICE_REGISTRY`。 |
| 独立 worker 注册表 | Linux 需显式参数或 `VERDANT_VOICE_REGISTRY`；Windows／macOS 默认 `state/voices.json`。 |
| 独立 worker workspace | `cache/render`；`--workspace DIR` 覆盖。 |
| 桌面 worker workspace | Linux `state/worker`，Windows／macOS `cache/worker`；`--workspace DIR` 覆盖。 |
| 桌面已验证缓存 | Linux `state/render-cache`，Windows／macOS `cache/render-cache`。 |
| 桌面试听目录 | `state/auditions`；`--audition-dir DIR` 优先于 `VERDANT_AUDITION_DIR`。 |
| 工程恢复日志 | 完整工程文件名后加 `.journal`，例如 `song.verp.journal`。 |
| 只读包资源 | `VERDANT_DATA_DIR`，否则包内 `share/verdant`；与用户声库目录分开。 |

独立 worker 的 `--project-root DIR` 默认 `VERDANT_REPO`，否则当前目录。桌面可用 `--repo DIR`／`VERDANT_REPO` 覆盖；Linux 默认当前目录，Windows／macOS 默认显式输入文件的父目录，或 `state/documents`。

## 来源与准入

| 来源 | 参数／环境 | 用户 data 下的默认位置 |
| --- | --- | --- |
| 版本化声库 | `--voice-package`／`VERDANT_VOICE_PACKAGE_DIR` | 已准入注册表选择的包 |

常规生产路线为 `verdant-voice`，版本绑定 workspace `0.1.0`。每次以 `capabilities` 的实际状态为准。`verdant-voice import-originals` 独立核验原始 `.tkn/.bin` 文件对的两个固定 SHA-256，登记 `verdant-original-sn16k`。受限 `C0` 乐谱合成须显式选择该已准入引用。旧工程和收据可保留 C0、C1、A 来源记录，已退役的路线选择会得到诊断。

请求中的 `voice_ref` 先解析注册表，未知引用拒绝。省略时使用已准入默认或命令行来源；未注册的历史 `voice-sn` 别名保留命令行来源。显式工程引用与 pin 不因默认声库改变而重解释。

`admit_voice` 包含 `reference`、`display_name` 和 `assets`。版本化来源使用 `voice_package`，另需 `source_evidence` 的原始 `token`、`model` 及用户显式 `rights_attested`；内容匹配不替代用户声明。版本化包的原始来源路径不成为准入后收据的运行依赖。

完整验证与原子保存后才返回 `voice_admitted`。同路径、同角色和同字节的重复准入可复用；搬移同一引用仍须保持角色、摘要及来源收据一致。资产变更使用新引用，失败保留原注册表。仅传 `--voice-package` 不生成来源收据。渲染前后校验所选资产的身份，缺失或变化均失败。

`--set-default-voice REF` 明确选择已准入来源；`--upgrade-default-voice REF` 只更新空默认或安装器管理的默认。显式选择结束自动升级，包括选择不变时；旧注册表无管理标记时视为用户选择。二者都不修改工程轨道。安装与精确回滚见[声库手册](../docs/verdant-voice.md)。

## 乐谱、时间与默认值

[请求 schema](worker-request.schema.json)固定字段，实际 capability 决定可用组合；未知能力视为不支持。

| 项目 | 合同 |
| --- | --- |
| 作者 F0 | 35–2,000 Hz，含 PIT、VIB、POR 的合成峰值；超出拒绝，不截断。 |
| 控制轨 | `frame_times_seconds`、`f0_hz`、`uv` 等长，至少两点，时间严格递增；右连续区间，末点为结束哨兵；`uv=true` 时 F0 为 0。原生辅音 UV 仍生效。 |
| 音素最短时长 | 受限 C0 为五状态帧 25 ms；协商 `compact_phone_states` 后，`verdant-voice` 接受一帧 5 ms。不能容纳时拒绝。 |
| 普通任务 | 最长 30 秒。 |
| 连续段 | 协商 `continuous-runs-v1` 及 `vowel-anchor-v1` 后，最长 120 秒、2,048 段、32,768 控制点；单个音素仍最长 30 秒，休止最短 5 ms。 |

`phoneme_timing.notes` 逐音符记录谱面拍点、声学片段范围和实际提前量 `prefix_seconds`。元音能对拍时，元音核的首段从拍点开始。提前空间不足时，`nucleus_delay_seconds` 表示元音核起点比拍点晚多少秒，省略表示零。没有元音核的读音保留其原生片段，`nucleus_segment_index` 为 `null`。

连接段共用一份帧计划、参数生成和状态，前端不在任意传输边界切断。普通话拼音库存含 418 个拼写；IPA、X-SAMPA 和日语假名保留目标及实际选用音素。日语为明确的跨语言近似。文本读音与声学准入分别判断。[多语言合同](../spec/MULTILINGUAL-PHONEMES.md)

`verdant-voice` 原生解码为 48 kHz／240 样本 hop。控制器单独使用 45–800 Hz 的统计参考；作者目标仍进入解码器。`payload.voice_music={"method":"verdant-voice-music-v1","sample_rate":48000}` 选择仅衰减的输出余量处理，可输出 44,100 或 48,000 Hz；省略保留原生 48 kHz。完整对象进入缓存身份，原生 NPZ／控制时钟不变。

旧基础设置在工程中保留，版本化请求省略不适用项。worker 拒绝直接提交的不兼容参数。

## 可选处理器

| 处理器 | 显式启用合同 |
| --- | --- |
| `score.synthesis.phonetic_correction` | `articulatory-v1`，strength 0–1；省略／null 关闭，0 保持原数值路径，未映射音素记录为未矫正。 |
| 受限 C0 `vocal_tract` | `bilinear-v1` 或 `anchored-warp-v1`；ratio 和 anchor ratio 为有限的 0.8–1.25，默认 1。 |
| 受限 C0 `model_dynamics` | `voiced-envelope-v1`，strength 0–1、最大衰减 0–12 dB；不能与 `gain_mode=relative` 共用。 |
| 受限 C0 `consonant_dynamics` | `unvoiced-model-envelope-v1`，strength 0–1、最大衰减 0–12 dB；不能与 `gain_mode=relative` 共用。 |
| `output_transients` | `phoneme-output-envelope-v1`，strength 0–1、最大衰减 0–12 dB；重建后作用于适用清音样本，省略保留原路径。 |

版本化路线拒绝旧谐波补偿、声道变形、生成高带及模型／辅音增益处理器。BRE／BRI／CLE／GEN／OPE／GWL 使用 `vocaloid-controls-v1`，范围 0–127，默认依次为 0／64／0／64／127／0。POR 默认 64，并与 PIT／VIB 编译到 F0；DYN 由共用混音器执行。它们采用 Verdant 的有效映射，保留各自单位。

协商后的 `source_note_id` 使同一作者音符的辅助音素共享来源；它是追踪字段，不是声学条件、身份认证或启用元音锚点的开关。缺失时不因局部索引相同而合并音素。

## 任务与产物

并发读取 `capabilities.max_active_jobs`，不固定假设一个槽。job ID 一旦建立目录便不可复用，新 revision 使用新 ID。取消确认与唯一终态分开；部分输出不进入有效缓存。

Unix 用独占进程组，先 SIGTERM，取消等待超过两秒后 SIGKILL；Windows 用原生进程 API 终止所属子进程。Linux 子进程登记父进程死亡信号，其他平台的子进程用所属 stdin 的 EOF 监测父进程。EOF、shutdown 和受处理的中断回收活动工作。

任务目录为 `workspace/jobs/<job_id>/`，包含 `request.json`、`worker-config.json`、`commands.json`、日志、执行与终态记录，以及：

| 路线／阶段 | 主要制品 |
| --- | --- |
| 受限 C0 原始文件对 | `native.wav`、`native.npz`、`native.json`。 |
| 版本化声库 | `verdant-voice.wav`、`verdant-voice.npz`、`verdant-voice.json`、`verdant-voice-conditions.npz`、`native-reference-plan.json`、`native-reference-conditions.npz`、`native.npz`。 |

最终适配顺序为 expression → local headroom → output transients → 制品枚举。保留的上游 F0 估计标为表达处理前。历史任务目录继续记录各自的请求、制品和来源图。

客户端校验 workspace 边界、字节数、SHA-256、有限 WAV、NPZ 音高及时间图。缓存绑定协议、版本、路线、有效请求、编译源码／锁／schema／库存、worker 及实际资产和原生库摘要。旧 revision 可填充内容缓存，但不能替换当前编辑；播放和导出使用相同的已验证结果。

## 来源与验收

模型报告 `quality_admission:false`；数值准入、实际声音和主观接受分别判断。原始 bank、派生数据和 ONNX 图按各自的发行授权处理。[合成来源与验证](../SYNTHESIS-FIDELITY.md) · [作者与许可](../NOTICE.md) · [发行政策](../RELEASE.md)

复验时长、表达、高带及其他数值对照时，使用对应报告记录的源码与固定输入。
