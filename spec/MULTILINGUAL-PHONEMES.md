# 多语言读音

## 输入

歌词保留显示文字。`/` 后可写拼音或显式音标；空间分隔音素，点分隔音节。语言选择声学映射，音标体系决定符号含义。中文、英语和日语按当前声库声明的库存与映射预检。

| 输入 | 行为 |
| --- | --- |
| `上/shang4` | 明确拼音。 |
| `上海/[zh:ʂ ɑŋ . x aɪ]` | 中文 IPA。 |
| `hello/[en:h ə . ˈl oʊ]` | 英文 IPA、音节和重音。 |
| `red/[en/xsampa:r\ E d]` | 标准 X-SAMPA。 |
| `word/[en~:ɚ]` | 允许有解释的近似音值。 |
| `さくら/[ja/kana:さくら]` | 假名经跨语言配置渲染。 |
| `学校/[ja/kana-common:がっこう]` | 在有限假名规则中启用常见长音合并。 |
| `さくら/[ja!:s a . k ɯ . ɾ a]` | 要求严格日语映射，当前近似声库会给出诊断。 |

库存和规范 IPA／X-SAMPA 对照由同一数据表提供。VOCALOID 方言须显式选 `vocaloid-xsampa`。解析保留大小写、反斜杠、长音、重音和边界。未知符号、路线上不可用的映射和未获准近似均返回定位诊断。[Rust 音素表](../frontend/crates/verdant-backend/src/data/phoneme_inventory.json)

## 工程

`Pronunciation` 可存 `language`、`alphabet`、`phonemes`、`allow_approximation`、`stress`、`mora_weights`、`reading_provenance` 和 `phoneme_durations_permille`。缺少 `alphabet` 的旧工程按旧读音合同解释。换成新歌词或拼音会清理被替换的音标与假名来源；重新确认同一读音保留编辑的比例。CLI 与桌面均通过工程事务保存这些字段。[工程规格](../SPEC.md) · [编辑器](../frontend/README.md)

## 时长

时长按模型展开后的片段分配。`phoneme_durations_permille` 有效时，各项为正整数并合计 1000。自动分配保留元音核与必要辅音预算。旧五状态路线逐片段至少 25 ms；声明 compact phone states 的声库可使用 5 ms 状态时间。超出音符预算时返回诊断，路径不改变已写音符长度。

日语 `ー` 延长元音，`っ` 把额外一拍分给对应辅音的首片段。显式策略只合并长音符号；`kana-common` 还处理声明的重复元音与 `ei/ou`。同一假名词内，`ん` 可在指定辅音前做经来源验证的有限同化。报告保存原符号、近似原因和实际模型键。重新编辑音标后按新读音计算；合成时按累计绝对边界量化。[日语时长夹具](../frontend/crates/verdant-backend/tests/fixtures/japanese_prosody_v1.json)

## 诊断和验证

工程检查器提供规范音标、模型键和片段时长。worker 在 `native_score.phoneme_frontend` 声明路线支持，调度输入绑定音素表、有效参数及声库身份。旧路线和新声库的范围以实际 capability 为准。

验收记录音值、时长、掉音、F0、清浊和独立听评。复验使用对应报告的对照和固定输入。[worker 运行](../backend/RUNTIME.md)
