# verdant(1) — 桌面歌声编辑器

## 用法

```text
verdant [工程.verp | --import FILE] [选项]
verdant --help
verdant --runtime-features
```

在卷帘编辑音符、歌词、读音与表达，经真实 Rust worker 合成后播放或导出 WAV。不指定文件时打开本账户的未命名工作副本；无已有工作副本时使用试听目录或诊断示例。

## 启动与选项

| 平台 | 包入口 | 基线验收范围 |
| --- | --- | --- |
| Linux x64 | `bin/verdant` | 已有真实本地模型、离线合成、48 kHz 和 Pride 外观记录。 |
| Windows x64／MSVC | `bin/verdant.exe` | 打包代码已准备，原生验收待完成。 |
| macOS Apple Silicon | `Verdant.app` 或 `Contents/MacOS/verdant` | 打包代码已准备，原生验收待完成。 |

使用包内启动入口；声库准备见 [verdant-voice](../docs/verdant-voice.md)。当前本地声库输入及正式随包声库的条件分别见[发行政策](../RELEASE.md)。

```sh
/path/to/package/bin/verdant song.verp
/path/to/package/bin/verdant --import source.ufdata
```

Windows PowerShell：`& 'C:\path\bin\verdant.exe' 'C:\songs\song.verp'`。macOS 可从 Finder 启动后打开工程，或执行 `/path/to/Verdant.app/Contents/MacOS/verdant song.verp`。

| 选项 | 行为／默认 |
| --- | --- |
| `--worker real` | 默认且唯一 worker 模式。 |
| `--worker-binary FILE` | 覆盖 `VERDANT_WORKER_BINARY` 及桌面同目录 worker。 |
| `--voice-registry FILE` | 覆盖 `VERDANT_VOICE_REGISTRY`；默认 state 目录的 `voices.json`。 |
| `--voice-package DIR` | 显式版本化声库来源；对应 `VERDANT_VOICE_PACKAGE_DIR`。 |
| `--workspace DIR` | worker 任务目录；Linux 默认 `state/worker`，Windows／macOS 默认 `cache/worker`。 |
| `--worker-arg ARG` | 追加一个 worker 参数，可重复。 |
| `--plughost FILE` | 指定音频效果器宿主；完整解析顺序见[宿主手册](docs/PLUGIN-HOST.md)。 |
| `--silent-audio` | 使用静音播放时钟，真实合成照常执行。 |
| `--appearance system|dark|light` | 默认跟随系统。 |
| `--audition-dir DIR` | 覆盖 `VERDANT_AUDITION_DIR`；默认 `state/auditions`。 |
| `--repo DIR` | 覆盖 `VERDANT_REPO` 指定工作根；平台默认见[运行配置](../backend/RUNTIME.md#文件与目录)。 |
| `--runtime-features` | stdout 输出版本、资源目录和声库支持的 JSON 后退出。 |

ONNX 库由包布局或 `VERDANT_ONNXRUNTIME_LIBRARY` 提供。开发诊断时可逐项传入 `--worker-arg --onnxruntime-library --worker-arg /path/to/library`。来源选项、平台目录及独立 worker 参数见[运行配置](../backend/RUNTIME.md)。

## 工程与恢复

打开、保存和另存为只接受 `.verp`；无扩展名的保存目标补 `.verp`。内容为 JSON，格式 `greendam-project`、schema `0.1`，保留未知扩展字段。

新建及导入写 `extensions["verdant.created_with"]`；保存及另存为保留创建版本。较新或无法识别的创建版本在写恢复日志前询问，可取消或继续。无创建标记的早期 `.verp` 仍可打开。

UFDATA、MIDI、UST、USTX、SVP、VSQX 和 VPR 使用「导入」或 `--import FILE`，生成独立的 `.verp` 工作副本，保留原始乐谱。导入和导出报告可在文件菜单查看。UFDATA 固定 480 PPQ，UST 导出需选择一条演唱轨，VSQX 需真实歌手元数据。各格式的损失报告约定见 [UST／USTX 导入](crates/verdant-import/README.md)与[乐谱交换](crates/verdant-exchange/README.md)。

恢复日志追加在完整工程名后，如 `song.verp.journal`。备份时保留当前工程与当前日志。启动发现较新的自动保存时，由用户选择恢复或打开已保存版本；选择前不把待恢复版本当作当前已保存文件。一次编辑对应一次命令批、revision 和撤销；拖动先预览，松手提交，Esc 或失焦取消草稿。「编辑 → 打开撤销历史窗口」可跳转历史状态。

## 可移植工程交接

以下为正在集成的 [CLI／DSH 规格](../spec/CLI-DSH.md)接口，字段与边界可对照当前 [pack.rs](crates/verdant-cli/src/pack.rs)；是否可用以最终集成版本的 capability 和验收为准。0.1.0 RC 的发行行为仍按其归档手册解释。

| 操作 | 参数与结果 |
| --- | --- |
| `project.pack` | 必填 `path`（工程）、`output`（新 ZIP）；可选 `recovery=saved|recovered`、`allowed_creator`、`allow_external_media`（默认 false）。无扩展名的输出补 `.zip`。 |
| `project.unpack` | 必填 `path`（ZIP）、`output`（新目录）；可选正整数字节上限 `max_expanded_bytes`、`max_archive_bytes`，只允许降低原生上限。返回可打开的 `<output>/project.verp` 及实际 `limits`。 |

二者以原子发布建立新目标，已有文件或目录拒绝覆盖。绝对路径素材须显式允许打包；相对素材按工程所在目录解析。封装与解包在发布前通过原生导入器核对素材 SHA-256、字节数、采样率、声道数及帧数。交付保留工程 ID、revision 与音频片段 ID。

运输包采用 `verdant-portable-project/1`，含 `project.verp`、`manifest.json` 和 `assets/<sha256>.<音频扩展名>`。素材原文件名后缀不在已知音频集合时，运输名使用 `.audio`，内容仍须通过原生解码与校验。最多 4,096 个成员，工程 64 MiB、清单 1 MiB、单素材 512 MiB、展开总量 2 GiB、归档 2 GiB + 8 MiB、中央目录 2 MiB，最大展开比 200 倍；不接受 ZIP64、加密、符号链接或特殊成员。

解包默认采用上述原生上限。`max_expanded_bytes` 接受 1–2,147,483,648，`max_archive_bytes` 接受 1–2,155,872,256；在解压成员内容或建立暂存目录前检查。展开预算计成员逻辑字节，不含文件系统块开销。桌面打开解包后的 `.verp`，声库另行准入。

整合版本跨目录执行原生另存为时，按原相对路径复制已验证素材，包括撤销／重做历史仍引用的音频；同目录另存为不复制。目标已有素材须通过完整原生身份校验才复用，不能覆盖。当前工程副本导出只收集当前文档可达的相对素材；素材复制保持工程 ID、revision、历史与来源引用不变。

CLI 的 `dry_run` 只核验素材，不创建输出目录、暂存文件或工程。素材发布后若工程文件发布失败，目标可能留下已验证但未被工程引用的素材；失败不改写源工程或来源文件。

## 编辑与渲染

轨道区管理歌手、语言、路线、音量、声像、静音和独奏。卷帘支持选择、绘制、移动、缩放音符及歌词、颤音、滑音编辑；时间尺定位播放头，触控板横滑平移。参数泳道支持点列、插值和精确输入；轨道、主总线及插件自动化写入工程。

音频片段位置使用工程 tick，裁剪使用源样本帧。工程 PPQ 与 tempo／meter maps 决定坐标，不固定按 960 PPQ 解释导入工程。

| 参数 | 有效映射 |
| --- | --- |
| PIT | 连续作者音高。 |
| VIB／POR | 音符颤音及滑音进入 F0。 |
| DYN | 共用混音器的逐样本增益。 |
| BRE／BRI／CLE／GEN／OPE／GWL | 重建后的气声、亮度、清晰度、性别因子、开口度及嘶吼处理。 |

默认使用已安装的 `verdant-voice` 声库包。用 `verdant-voice import-originals` 导入原始 `.tkn` 和 `.bin` 时，两个固定摘要均匹配才会解锁原版歌手。[声库安装](../docs/verdant-voice.md)

预检阻止含错误的乐句。「未发送 · 含错误」表示尚未提交，检查该乐句的读音、时长和音高诊断。旧 revision 的完成结果不能覆盖新编辑。作者 F0 为 35–2,000 Hz，包含 PIT、VIB 和 POR 的组合峰值；越界拒绝。当前声库音素最短 5 ms，解锁的原版声库为 25 ms；工程可保留更短音符，由诊断定位无法渲染的部分。数值准入与听感接受分别记录。

工程保留已有的合成设置。渲染请求按所选声库的能力提交适用的参数。[默认值与可选处理器](../backend/RUNTIME.md#乐谱时间与默认值)

## 歌词与读音

可输入 `ni3`、`さくら`、`sakura`、`hello` 或词片段。自动解析依赖语言及相邻音符，显式读音优先。`好/hao3` 保留显示歌词并指定拼音；歌词改变时清除旧自动读音。

同一音符的空格分隔多个片段，如 `ni3 さくら hel`，按片段分配时长；不足时延长音符或修改读音。普通话词组读音优先，多音字可更正；用户词典和显式读音先于自动结果。

```text
hello/[en:h ə . ˈl oʊ]
red/[en/xsampa:r\ E d]
さくら/[ja/kana:さくら]
学校/[ja/kana-common:がっこう]
```

音标以空格分隔、点分音节。`kana-common` 显式启用常见长音合并并记录启发式。`ja` 为现有库存上的跨语言近似；`ja!` 要求严格模式，`en~` 显式允许英语近似。日语不自动预测汉字读音、词汇重音或无声化。

「读音器… → 分段映射…」为同轨 `[起始tick,结束tick)` 选择读音器，按音符起点命中，同轨范围不可重叠。输入为 `轨道序号, 起始tick, 结束tick, 读音器id`，轨道序号从 1 起。支持 `mandarin-dictionary-v1`、`explicit-only-v1`、`japanese-kana-v1`。

解析优先级为显式音符读音 → 用户词组 → 用户词条 → 分段映射 → 轨道语言 → 歌手 → 工程默认。分段写入 `verdant.phonemizer` 扩展。[多语言合同](../spec/MULTILINGUAL-PHONEMES.md) · [字典归属](../NOTICE.md)

## 播放与导出

播放与 WAV 导出使用相同已验证渲染结果、混音、自动化和插入处理。WAV 设置为 44.1／48／88.2／96／192 kHz、单／双声道、Float32／PCM16／PCM24；可选完整混音、当前轨或逐轨，以及完整、循环或选区范围。效果器用独立离线实例，失败中止导出；延迟与十秒尾音合同见[插件宿主](docs/PLUGIN-HOST.md)。

设备声明缓冲范围时默认请求 1,024 帧，并限制在设备及预分配范围内；未知时交给系统。48 kHz 的 1,024 帧约 21.3 ms，只是缓冲长度。检查器显示设备、采样率、请求缓冲及已报告欠载；静音时钟与零欠载计数不证明物理设备健康。

## 配置与文件

偏好在 `state/prefs.json`；未命名工作副本在 `state/untitled/project.verp`，导入副本在 `state/documents/`。注册表、worker workspace 和已验证缓存分别保存，平台路径见[运行手册](../backend/RUNTIME.md#文件与目录)。

试听目录的 `manifest.json` 使用 `verdant-audition-catalog/1`，`projects` 项含 `title` 与目录内相对 `path`。只为首次未命名会话建立第一首的工作副本，已有工程或恢复不被替换。没有目录时使用诊断旋律。

外观默认跟随系统。Pride 默认为「自动」，在本地 5 月 17 日、6 月 28 日、10 月 11 日启用，可任意日期手动开关；仅改配色和装饰。

## 退出状态

正常关闭、帮助与运行特性查询退出 0；未知参数、互斥输入、工程启动失败、工作根或 worker 路径无法解析退出 2。运行中的渲染失败由界面与 worker 终态报告，不能只用桌面退出码判断声音结果。

## 参见

[项目入口](../README.md) · [构建与发行](../RELEASE.md) · [平台验收](docs/PLATFORM-ACCEPTANCE.md) · [路线图](ROADMAP.md) · [规格](../SPEC.md) · [CLI／DSH 设计](../spec/CLI-DSH.md)

开发使用现有开发容器与锁定工具链；Compiler 源、固定身份及生成物位于 `proto-ui/`，说明见 [P0-DECISIONS.md](P0-DECISIONS.md) 和 [spec](../spec/README.md)。
