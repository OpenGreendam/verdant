//! Runtime UI language and native accessibility names.
//!
//! Catalogue entries require both translations at compile time. Keep license and
//! third-party NOTICE bytes in their original language; only application prose is
//! translated. `ui_locale` uses Prefs' existing flattened extension map, preserving
//! preferences written by other versions. Save before publishing a locale change,
//! and always pass the live editor preferences to `set_locale`.
//!
//! Surface names describe editor regions, not synthetic note grids. Naming a
//! canvas does not make its painted notes or automation points accessible. Native
//! AT delivery and keyboard editing still require platform acceptance separately.

use gpui::{App, Global, Role};
use crate::files::Prefs;

pub const LOCALE_PREFERENCE: &str = "ui_locale";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

impl Global for Locale {}

impl Locale {
    pub const ALL: [Self; 2] = [Self::ZhCn, Self::En];

    pub const fn code(self) -> &'static str {
        match self { Self::ZhCn => "zh-CN", Self::En => "en" }
    }

    /// Language names stay in their own language so a misplaced setting is recoverable.
    pub const fn native_name(self) -> &'static str {
        match self { Self::ZhCn => "简体中文", Self::En => "English" }
    }

    pub fn from_preferences(prefs: &Prefs) -> Self {
        match prefs.extra.get(LOCALE_PREFERENCE).and_then(serde_json::Value::as_str) {
            Some("en") => Self::En,
            _ => Self::ZhCn,
        }
    }

    pub fn write_preference(self, prefs: &mut Prefs) {
        prefs.extra.insert(LOCALE_PREFERENCE.into(), self.code().into());
    }

    pub const fn text(self, key: Key) -> &'static str { key.text(self) }
}

pub fn initialize(prefs: &Prefs, cx: &mut App) {
    cx.set_global(Locale::from_preferences(prefs));
}

pub fn current(cx: &App) -> Locale {
    cx.try_global::<Locale>().copied().unwrap_or_default()
}

/// Called with the live editor prefs: later autosave must retain this language.
/// A failed write leaves both the in-memory preferences and UI locale unchanged.
pub fn set_locale(prefs: &mut Prefs, locale: Locale, cx: &mut App) -> Result<(), String> {
    let mut updated = prefs.clone();
    locale.write_preference(&mut updated);
    crate::files::save_prefs(&updated)?;
    *prefs = updated;
    cx.set_global(locale);
    Ok(())
}

macro_rules! catalogue {
    ($( $key:ident => ($zh:literal, $en:literal), )+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum Key { $( $key, )+ }
        impl Key {
            pub const ALL: &'static [Self] = &[$( Self::$key, )+];
            pub const fn text(self, locale: Locale) -> &'static str {
                match (self, locale) {
                    $( (Self::$key, Locale::ZhCn) => $zh,
                       (Self::$key, Locale::En) => $en, )+
                }
            }
        }
    }
}

catalogue! {
    Language => ("界面语言", "Interface language"),
    LanguageHelp => ("选择后立即生效，并保存至应用设置。", "Applies immediately and is saved in application preferences."),
    LanguageSaveFailed => ("语言设置保存失败", "Could not save the interface language"),
    Roll => ("钢琴卷帘", "Piano roll"),
    RollHelp => ("音符编辑区域；可使用键盘音符表编辑音符与歌词。", "Note editing region. Use the keyboard note table to edit notes and lyrics."),
    Lanes => ("参数曲线", "Automation lanes"),
    LanesHelp => ("音高与表现参数曲线编辑区域。", "Editing region for pitch and expression curves."),
    Mixer => ("混音器", "Mixer"),
    MixerHelp => ("轨道音量、声像、静音、独奏与插入效果。", "Track volume, pan, mute, solo and insert effects."),
    Dialog => ("编辑器对话框", "Editor dialog"),
    DialogHelp => ("编辑器输入与确认操作。", "Editor input and confirmation controls."),
    Arrangement => ("轨道编排", "Track arrangement"),
    ArrangementHelp => ("项目轨道、音符组与音频片段的时间排列。", "Tracks and clips arranged on the project timeline."),
    NoteTable => ("键盘音符表", "Keyboard note table"),
    NoteTableHelp => ("用键盘浏览和编辑音符、歌词与读音。", "Browse and edit notes, lyrics and readings with the keyboard."),
    Inspector => ("检查器", "Inspector"),
    Open => ("打开…", "Open…"),
    Save => ("保存", "Save"),
    SaveAs => ("另存为…", "Save as…"),
    Undo => ("撤销", "Undo"),
    Redo => ("重做", "Redo"),
    PlayPause => ("播放 / 暂停", "Play / pause"),
    Stop => ("停止", "Stop"),
    GoToStart => ("回到开头", "Go to start"),
    AddTrack => ("+ 轨道", "+ Track"),
    Mute => ("静音", "Mute"),
    Solo => ("独奏", "Solo"),
    Close => ("关闭", "Close"),
    Cancel => ("取消", "Cancel"),
    Confirm => ("确定", "OK"),
    Pointer => ("指针", "Pointer"),
    Pencil => ("铅笔", "Pencil"),
    Eraser => ("橡皮", "Eraser"),
    PitchCurve => ("音高曲线", "Pitch curve"),
    FitNotes => ("适合音符", "Fit notes"),
    Freehand => ("自由绘制", "Freehand"),
    Line => ("直线", "Line"),
    Smooth => ("平滑", "Smooth"),
    Erase => ("擦除", "Erase"),
    SelectPoints => ("框选点", "Select points"),
    CopyCurve => ("复制曲线", "Copy curve"),
    PasteCurve => ("粘贴曲线", "Paste curve"),
    DisclosuresTitle => ("开源软件与研究引用", "Open-source software and research references"),
    DisclosuresWindowTitle => ("开源软件与研究引用 — Verdant", "Open-source software and research references — Verdant"),
    DisclosuresIntro => (
        "Verdant 翠绿：开源歌唱编辑器（AGPL-3.0-or-later）。以下区分本仓库软件、随包数据/权重与研究引用；材料归属以其原许可为准，离线可用，不依赖模型或运行时。",
        "Verdant: an open-source singing editor (AGPL-3.0-or-later). Repository software, bundled data/weights and research references are listed separately below. Original licenses govern attribution. This information is available offline without models or a backend runtime."),
    SoftwareHeading => ("软件、数据与权重", "Software, data and weights"),
    ResearchHeading => ("研究引用（引用而非集成，除非另有标注）", "Research references (cited, not integrated unless stated)"),
    LicensesHeading => ("许可与通知原文", "Original licenses and notices"),
    NoticeLicense => ("根目录 NOTICE（第三方材料与归属，含 mozillazg pinyin MIT 原文）", "Repository NOTICE (third-party attribution, including the mozillazg pinyin MIT license)"),
    LucideLicense => ("Lucide 图标许可（assets/icons/LICENSE-lucide.txt）", "Lucide icon license (assets/icons/LICENSE-lucide.txt)"),
    ProtoLicense => ("Proto-UI 许可（spec/licenses/Proto-UI-LICENSE.txt）", "Proto-UI license (spec/licenses/Proto-UI-LICENSE.txt)"),
    GpuiLicense => ("GPUI 许可（spec/licenses/GPUI-LICENSE-APACHE.txt）", "GPUI license (spec/licenses/GPUI-LICENSE-APACHE.txt)"),
    DiffSingerLicense => ("DiffSinger Apache-2.0（spec/licenses/DiffSinger-LICENSE.txt）", "DiffSinger Apache-2.0 (spec/licenses/DiffSinger-LICENSE.txt)"),
    OpenVpiLicense => ("OpenVPI 声码器官方 NOTICE（原文副本，assets/licenses/OpenVPI-NOTICE.md）", "Official OpenVPI vocoder NOTICE (original copy, assets/licenses/OpenVPI-NOTICE.md)"),
    VerdantLicense => ("Verdant 许可（LICENSE, AGPL-3.0-or-later）", "Verdant license (LICENSE, AGPL-3.0-or-later)"),
    VerdantName => ("Verdant 翠绿（本程序与仓库）", "Verdant (this application and repository)"),
    VerdantNote => (
        "源码采用 AGPL-3.0-or-later，完整许可见下文。发行包附 verdant-source.tar.gz。声库从用户已有文件离线安装，参见 RELEASE.md。源码仓库 https://github.com/OpenGreendam/verdant",
        "Source is licensed under AGPL-3.0-or-later. See the license below and verdant-source.tar.gz in the distribution. Install the voice from local files as described in RELEASE.md. https://github.com/OpenGreendam/verdant"),
    ProtoName => ("Proto-UI 与 GPUI", "Proto-UI and GPUI"),
    ProtoNote => (
        "Proto-UI Contributors（MIT）与 Zed Industries / GPUI（Apache-2.0），原文许可见下文；本项目应用与控件实现见 frontend/crates/verdant-app。上游参考与本项目实现分别保留来源。",
        "Proto-UI Contributors (MIT) and Zed Industries / GPUI (Apache-2.0); original licenses below. This project's application and controls are in frontend/crates/verdant-app. Upstream references and this implementation retain their separate provenance."),
    LucideName => ("Lucide 图标（assets/icons）", "Lucide icons (assets/icons)"),
    LucideNote => ("Lucide ISC License，随包原文见下。", "Lucide ISC License; bundled original text below."),
    OriginalModelName => ("早期朗读软件内置女童声的原候选模型（sn16k.tkn / sn16k.bin）", "Original candidate model of an early Chinese reading-software girl voice (sn16k.tkn / sn16k.bin)"),
    OriginalModelNote => (
        "研究用取得的商业软件资产，不入 Git、不在 Release 中分发；仅以来源与 SHA-256 引用，见根目录 NOTICE.md 与 models/manifest.json。",
        "Commercial software assets obtained for research, excluded from Git and Releases. Referenced only by source and SHA-256; see repository NOTICE.md and models/manifest.json."),
    PinyinName => ("pinyin 0.11.0 + pinyin-data（歌词汉字→拼音候选）", "pinyin 0.11.0 + pinyin-data (Chinese lyrics to pinyin candidates)"),
    PinyinNote => (
        "mozillazg/rust-pinyin 0.11.0 与 mozillazg/pinyin-data，MIT，Copyright (c) 2016 mozillazg；两者共用 MIT 原文见下（取自根目录 NOTICE.md）。字典只提供单字读音候选，不表示原声学模型恢复或支持这些读音。",
        "mozillazg/rust-pinyin 0.11.0 and mozillazg/pinyin-data, MIT, Copyright (c) 2016 mozillazg. Their shared MIT license appears below (from repository NOTICE.md). The dictionary supplies individual-character reading candidates; it does not establish recovery of the original acoustic model or support for those readings."),
    OpenVpiName => ("OpenVPI PC-NSF-HiFiGAN（历史声码器研究）", "OpenVPI PC-NSF-HiFiGAN (historical vocoder research)"),
    OpenVpiNote => (
        "OpenVPI Team 发布（pc-nsf-hifigan-44.1k-hop512-128bin-2025.02），CC BY-NC-SA 4.0。仅用于已退役合成路径的历史研究；当前合成不加载这些权重。官方 NOTICE 原文见下。",
        "Released by OpenVPI Team (pc-nsf-hifigan-44.1k-hop512-128bin-2025.02), CC BY-NC-SA 4.0. Used in historical research for retired synthesis paths; current synthesis does not load these weights. Original official NOTICE below."),
    DiffSingerName => ("DiffSinger 接口参考", "DiffSinger interface reference"),
    DiffSingerNote => (
        "DiffSinger 接口对照固定到 commit cb7869b8733a3882679c328994ee162845092cff。许可原文见 spec/licenses/DiffSinger-LICENSE.txt 及下文。ONNX 发行版的准确构建提交尚未核实。",
        "The DiffSinger interface reference is pinned to commit cb7869b8733a3882679c328994ee162845092cff. The original license is in spec/licenses/DiffSinger-LICENSE.txt and below. The exact source commit used to build the ONNX release remains unverified."),
    EnvironmentName => ("原生运行依赖", "Native runtime dependencies"),
    EnvironmentNote => (
        "产品使用原生 ONNX Runtime CPU 库与 WORLD 源码；发行包保留 ONNX Runtime 的 MIT 许可、第三方通知及 WORLD 许可原文。完整归属见根目录 NOTICE.md。",
        "The product uses the native ONNX Runtime CPU library and WORLD source. Distributions retain ONNX Runtime's MIT license, third-party notices and the original WORLD license. See repository NOTICE.md for attribution."),
    DurationName => ("时长分配目标函数（已集成，数学参考）", "Duration allocation objective (integrated mathematical reference)"),
    DurationNote => (
        "Keijiro Saino、Heiga Zen、Yoshihiko Nankaku、Akinobu Lee、Keiichi Tokuda：An HMM-based Singing Voice Synthesis System，Interspeech 2006 §2.3，DOI 10.21437/Interspeech.2006-584（https://www.isca-archive.org/interspeech_2006/saino06_interspeech.pdf）。Native 0.4.0 的整数 Gaussian 时长分配为本项目新实现：固定乐谱边界下最小化状态时长的 Gaussian 负对数似然；未复制论文代码、未训练其 time-lag 模型、未声称恢复原商业引擎时长语义。",
        "Keijiro Saino, Heiga Zen, Yoshihiko Nankaku, Akinobu Lee, Keiichi Tokuda: An HMM-based Singing Voice Synthesis System, Interspeech 2006 §2.3, DOI 10.21437/Interspeech.2006-584 (https://www.isca-archive.org/interspeech_2006/saino06_interspeech.pdf). Native 0.4.0 integer Gaussian duration allocation is this project's new implementation: minimizing Gaussian negative log-likelihood of state durations at fixed score boundaries. No paper code was copied, no time-lag model was trained, and original commercial-engine duration semantics are not claimed to be recovered."),
    BaselineName => ("复现与对照基线", "Reproduction and comparison baseline"),
    BaselineNote => (
        "感谢所有者提供的 GPT 6 Pro 复现包：本项目规格与旧版合成对照基于该工作。冻结来源与边界登记：https://github.com/OpenGreendam/docs/tree/main/greendam-reproduction-2026-11-13；基线 render_native.py，研究记录见 SYNTHESIS-FIDELITY.md。原商业模型及第三方权重的归属独立保留。",
        "Thanks to the owner for the GPT 6 Pro reproduction package, which underlies the project specification and historical synthesis comparisons. Frozen provenance and scope: https://github.com/OpenGreendam/docs/tree/main/greendam-reproduction-2026-11-13; baseline render_native.py; research records in SYNTHESIS-FIDELITY.md. Original commercial models and third-party weights retain separate attribution."),
    SifiName => ("SiFi-GAN（调研，未集成）", "SiFi-GAN (surveyed, not integrated)"),
    SifiNote => (
        "Reo Yoneyama、Yi-Chiao Wu、Tomoki Toda：Source-Filter HiFi-GAN: Fast and Pitch Controllable High-Fidelity Neural Vocoder（ICASSP 2023，DOI 10.1109/ICASSP49357.2023.10095298）。作者代码 https://github.com/chomeyama/SiFiGAN；近期延伸文献 https://doi.org/10.1587/transinf.2025EDP7142。源/滤波结构候选，未集成。",
        "Reo Yoneyama, Yi-Chiao Wu, Tomoki Toda: Source-Filter HiFi-GAN: Fast and Pitch Controllable High-Fidelity Neural Vocoder (ICASSP 2023, DOI 10.1109/ICASSP49357.2023.10095298). Author code: https://github.com/chomeyama/SiFiGAN; recent extension: https://doi.org/10.1587/transinf.2025EDP7142. Candidate source/filter architecture, not integrated."),
    ApbweName => ("AP-BWE（调研，未集成）", "AP-BWE (surveyed, not integrated)"),
    ApbweNote => (
        "Ye-Xin Lu、Yang Ai、Hui-Peng Du、Zhen-Hua Ling：Towards High-Quality and Efficient Speech Bandwidth Extension with Parallel Amplitude and Phase Prediction，https://arxiv.org/abs/2401.06387；作者实现 https://github.com/yxlu-0102/AP-BWE。宽带增强候选，未集成。",
        "Ye-Xin Lu, Yang Ai, Hui-Peng Du, Zhen-Hua Ling: Towards High-Quality and Efficient Speech Bandwidth Extension with Parallel Amplitude and Phase Prediction, https://arxiv.org/abs/2401.06387; author implementation: https://github.com/yxlu-0102/AP-BWE. Bandwidth enhancement candidate, not integrated."),
    LseName => ("LSE + Vocos2D（调研，未集成）", "LSE + Vocos2D (surveyed, not integrated)"),
    LseNote => (
        "Runxuan Yang、Kai Li、Guo Chen、Xiaolin Hu：Enhancing Spectrogram Realism in Singing Voice Synthesis via Explicit Bandwidth Extension Prior to Vocoder，https://arxiv.org/abs/2508.01796；试听页 https://lse-vocos2d.github.io/acmmm2025-demo/。调研，未集成。",
        "Runxuan Yang, Kai Li, Guo Chen, Xiaolin Hu: Enhancing Spectrogram Realism in Singing Voice Synthesis via Explicit Bandwidth Extension Prior to Vocoder, https://arxiv.org/abs/2508.01796; samples: https://lse-vocos2d.github.io/acmmm2025-demo/. Surveyed, not integrated."),
    DitName => ("DiTSinger（调研，未集成）", "DiTSinger (surveyed, not integrated)"),
    DitNote => (
        "Zongcai Du、Guilin Deng、Xiaofeng Guo、Xin Gao、Linke Li、Kaichang Cheng、Fubo Han、Siyu Yang、Peng Liu、Pan Zhong、Qiang Fu：DiTSinger: Scaling Singing Voice Synthesis with Diffusion Transformer and Implicit Alignment，https://arxiv.org/abs/2510.09016。字/音素时间范围与局部隐式对齐研究，未集成。",
        "Zongcai Du, Guilin Deng, Xiaofeng Guo, Xin Gao, Linke Li, Kaichang Cheng, Fubo Han, Siyu Yang, Peng Liu, Pan Zhong, Qiang Fu: DiTSinger: Scaling Singing Voice Synthesis with Diffusion Transformer and Implicit Alignment, https://arxiv.org/abs/2510.09016. Research into character/phoneme time ranges and local implicit alignment, not integrated."),
    ExcitationName => ("混合激励（研究参考）", "Mixed excitation (research reference)"),
    ExcitationNote => (
        "Takayoshi Yoshimura、Keiichi Tokuda、Takashi Masuko、Takao Kobayashi、Tadashi Kitamura：Mixed Excitation for HMM-based Speech Synthesis，Eurospeech 2001，DOI 10.21437/Eurospeech.2001-539，https://www.isca-archive.org/eurospeech_2001/yoshimura01_eurospeech.pdf。用于激励结构分析；原引擎各频带清浊参数尚未恢复，不据此宣称原混合激励已复现。",
        "Takayoshi Yoshimura, Keiichi Tokuda, Takashi Masuko, Takao Kobayashi, Tadashi Kitamura: Mixed Excitation for HMM-based Speech Synthesis, Eurospeech 2001, DOI 10.21437/Eurospeech.2001-539, https://www.isca-archive.org/eurospeech_2001/yoshimura01_eurospeech.pdf. Reference for excitation analysis; the original engine's per-band voicing parameters remain unrecovered, so original mixed excitation is not claimed to be reproduced."),
    VotName => ("塞音起声时间 VOT（分析参考）", "Stop-consonant voice onset time, VOT (analysis reference)"),
    VotNote => (
        "Kuan-Yi Chao、Li-mei Chen：A Cross-Linguistic Study of Voice Onset Time in Stop Consonant Productions，CLCLP 13(2)，2008，pp.215–232，https://aclanthology.org/O08-4005.pdf。用于跨语言 VOT 分析；没有直接采用论文中的 VOT 常数作为合成参数。",
        "Kuan-Yi Chao, Li-mei Chen: A Cross-Linguistic Study of Voice Onset Time in Stop Consonant Productions, CLCLP 13(2), 2008, pp.215–232, https://aclanthology.org/O08-4005.pdf. Reference for cross-language VOT analysis; paper VOT constants are not directly used as synthesis parameters."),
    MlpgName => ("MLPG 参数生成（数学参考）", "MLPG parameter generation (mathematical reference)"),
    MlpgNote => (
        "Keiichi Tokuda、Takayoshi Yoshimura、Takashi Masuko、Takao Kobayashi、Tadashi Kitamura：Speech parameter generation algorithms for HMM-based speech synthesis，ICASSP 2000，DOI 10.1109/ICASSP.2000.861820，https://doi.org/10.1109/ICASSP.2000.861820。现有 MLPG 参数轨迹求解的数学参考；原引擎历史动态窗尚未核实。",
        "Keiichi Tokuda, Takayoshi Yoshimura, Takashi Masuko, Takao Kobayashi, Tadashi Kitamura: Speech parameter generation algorithms for HMM-based speech synthesis, ICASSP 2000, DOI 10.1109/ICASSP.2000.861820, https://doi.org/10.1109/ICASSP.2000.861820. Mathematical reference for the current MLPG trajectory solver; the original engine's historical dynamic windows remain unverified."),
    HifiName => ("HiFi-GAN（声码器背景参考）", "HiFi-GAN (vocoder background reference)"),
    HifiNote => (
        "Jungil Kong、Jaehyeon Kim、Jaekyoung Bae：HiFi-GAN: Generative Adversarial Networks for Efficient and High Fidelity Speech Synthesis，2020，https://arxiv.org/abs/2010.05646。作为历史声码器架构背景；不声称使用该论文的权重。历史 OpenVPI 研究所用权重的许可与官方 NOTICE 另列。",
        "Jungil Kong, Jaehyeon Kim, Jaekyoung Bae: HiFi-GAN: Generative Adversarial Networks for Efficient and High Fidelity Speech Synthesis, 2020, https://arxiv.org/abs/2010.05646. Historical vocoder architecture background; use of this paper's weights is not claimed. The weights used in historical OpenVPI research have separate licensing and official NOTICE."),
    WorldName => ("WORLD（分析框架参考）", "WORLD (analysis framework reference)"),
    WorldNote => (
        "Masanori Morise、Fumiya Yokomori、Kenji Ozawa：WORLD: A Vocoder-Based High-Quality Speech Synthesis System for Real-Time Applications，IEICE E99-D(7)，2016，pp.1877–1884，DOI 10.1587/transinf.2015EDP7457，https://doi.org/10.1587/transinf.2015EDP7457。pyworld 用于分析诊断；不声称原商业引擎采用 WORLD 合成。",
        "Masanori Morise, Fumiya Yokomori, Kenji Ozawa: WORLD: A Vocoder-Based High-Quality Speech Synthesis System for Real-Time Applications, IEICE E99-D(7), 2016, pp.1877–1884, DOI 10.1587/transinf.2015EDP7457, https://doi.org/10.1587/transinf.2015EDP7457. pyworld is used for analysis and diagnostics; the original commercial engine is not claimed to use WORLD synthesis."),
    AuditoryName => ("Auditory Toolbox（mel 表示参考）", "Auditory Toolbox (mel representation reference)"),
    AuditoryNote => (
        "Malcolm Slaney：Auditory Toolbox Version 2，Technical Report 1998-010，https://engineering.purdue.edu/~malcolm/interval/1998-010/。作为 mel 表示的参考；未随程序打包该工具箱 MATLAB 代码。",
        "Malcolm Slaney: Auditory Toolbox Version 2, Technical Report 1998-010, https://engineering.purdue.edu/~malcolm/interval/1998-010/. Reference for mel representation; the toolbox's MATLAB code is not bundled."),
    RepresentationName => ("歌唱参数表示比较（研究参考）", "Singing parameter representation comparison (research reference)"),
    RepresentationNote => (
        "Onur Babacan、Thomas Drugman、Tuomo Raitio、Daniel Erro、Thierry Dutoit：Parametric Representation for Singing Voice Synthesis: A Comparative Evaluation，预印本 https://arxiv.org/abs/2006.04142。用于比较歌唱参数表示与激励的研究背景；不是本程序音质或可懂度的验收结果。",
        "Onur Babacan, Thomas Drugman, Tuomo Raitio, Daniel Erro, Thierry Dutoit: Parametric Representation for Singing Voice Synthesis: A Comparative Evaluation, preprint https://arxiv.org/abs/2006.04142. Research background for comparing singing parameter representations and excitation; not an acceptance result for this application's sound quality or intelligibility."),
    VocosName => ("Vocos 带宽扩展（调研，未集成）", "Vocos bandwidth extension (surveyed, not integrated)"),
    VocosNote => (
        "Yatharth Sharma：Fast and Flexible Audio Bandwidth Extension via Vocos，2026 预印本，https://arxiv.org/abs/2603.07285。宽带增强候选，未集成；论文速度数字不代表本机实测性能，也不证明能恢复缺失音素。",
        "Yatharth Sharma: Fast and Flexible Audio Bandwidth Extension via Vocos, 2026 preprint, https://arxiv.org/abs/2603.07285. Bandwidth enhancement candidate, not integrated. Paper speed figures are not measurements on this machine and do not establish recovery of missing phonemes."),
    DdspName => ("DDSP（verdant-voice 的方法基础）", "DDSP (methodological basis of verdant-voice)"),
    DdspNote => (
        "Jesse Engel、Lamtharn Hantrakul、Chenjie Gu、Adam Roberts，DDSP: Differentiable Digital Signal Processing，ICLR 2020，https://arxiv.org/abs/2001.04643 。verdant-voice 独立实现条件控制、带限周期源与可微声谱滤波、独立滤波噪声及多尺度波形谱损失，并保留谐波包络对照；没有复制 Magenta 代码或使用其预训练权重。",
        "Jesse Engel, Lamtharn Hantrakul, Chenjie Gu and Adam Roberts, DDSP: Differentiable Digital Signal Processing, ICLR 2020, https://arxiv.org/abs/2001.04643 . verdant-voice independently implements conditioning, a band-limited periodic source, differentiable spectral filtering, independent filtered noise and multiscale waveform losses, retaining a harmonic-envelope comparator; no Magenta code or pretrained weights are copied."),
    PcDdspName => ("pc-ddsp / DDSP-SVC（设计研究）", "pc-ddsp / DDSP-SVC (design research)"),
    PcDdspNote => (
        "yxlllc 及贡献者，Pitch Controllable DDSP Vocoders，https://github.com/yxlllc/pc-ddsp ；以及 https://github.com/yxlllc/DDSP-SVC 。参考其可控音高、清浊分离与源滤波比较；未接入其权重、语音内容编码器或声线转换管线，不把研究引用说成完整集成。",
        "yxlllc and contributors, Pitch Controllable DDSP Vocoders, https://github.com/yxlllc/pc-ddsp , and https://github.com/yxlllc/DDSP-SVC . References for pitch control, voicing separation and source/filter comparisons. Their weights, speech-content encoders and conversion pipelines are not integrated."),
    GolfName => ("GOLF（声门源与声道参数化研究）", "GOLF (glottal-source and vocal-tract parameterization research)"),
    GolfNote => (
        "Chin-Yun Yu、György Fazekas，Singing Voice Synthesis Using Differentiable LPC and Glottal-Flow-Inspired Wavetables，ISMIR 2023，https://arxiv.org/abs/2306.17252 。作为候选分析依据；本轮简化源滤波对照不是 GOLF 的复现或官方模型评测。",
        "Chin-Yun Yu and György Fazekas, Singing Voice Synthesis Using Differentiable LPC and Glottal-Flow-Inspired Wavetables, ISMIR 2023, https://arxiv.org/abs/2306.17252 . Cited for candidate analysis; our simplified source/filter control is not a GOLF reproduction or an evaluation of its official model."),
    L8nName => ("L8N DEMO 参考素材", "L8N DEMO reference recordings"),
    L8nNote => (
        "用户提供的 L8N DEMO.zip，包内作者署名黄亦天，说明素材为 2010 年某朗读软件的女童声并做过限幅处理。verdant-voice 训练清单逐文件绑定指纹和用途；原始素材及其衍生声库不因本项目代码许可而成为自由分发数据。",
        "User-supplied L8N DEMO.zip credits 黄亦天 and identifies processed girl-voice recordings from a 2010 reading-software package with limiting applied. verdant-voice records per-file hashes and roles. The code license does not grant redistribution rights for these recordings or derived voice assets."),
    RosenbergName => ("Rosenberg 式激励（数学背景，近似实现）", "Rosenberg-style excitation (mathematical background, approximate implementation)"),
    RosenbergNote => (
        "A. E. Rosenberg：Effect of Glottal Pulse Shape on the Quality of Natural Vowels，The Journal of the Acoustical Society of America 49(2)，1971，pp.583–590，DOI 10.1121/1.1912389，https://pubmed.ncbi.nlm.nih.gov/5541751/。当前声门流导数激励为近似重建，开闭相参数是本项目选择；原 pulse 表尚未恢复，不声称精确复现论文或原商业引擎。",
        "A. E. Rosenberg: Effect of Glottal Pulse Shape on the Quality of Natural Vowels, The Journal of the Acoustical Society of America 49(2), 1971, pp.583–590, DOI 10.1121/1.1912389, https://pubmed.ncbi.nlm.nih.gov/5541751/. Current glottal-flow derivative excitation is an approximate reconstruction; open/closed phase parameters were chosen by this project. The original pulse table remains unrecovered; exact reproduction of the paper or original commercial engine is not claimed."),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface { Roll, Lanes, Mixer, Dialog, Arrangement, NoteTable, Disclosures }

pub struct SurfaceSemantics {
    pub role: Role,
    pub label: &'static str,
    pub description: &'static str,
}

impl Surface {
    pub const ALL: [Self; 7] = [Self::Roll, Self::Lanes, Self::Mixer, Self::Dialog, Self::Arrangement, Self::NoteTable, Self::Disclosures];

    pub fn semantics(self, locale: Locale) -> SurfaceSemantics {
        let (role, label, description) = match self {
            Self::Roll => (Role::Group, Key::Roll, Key::RollHelp),
            Self::Lanes => (Role::Group, Key::Lanes, Key::LanesHelp),
            Self::Mixer => (Role::Group, Key::Mixer, Key::MixerHelp),
            Self::Dialog => (Role::Dialog, Key::Dialog, Key::DialogHelp),
            Self::Arrangement => (Role::Group, Key::Arrangement, Key::ArrangementHelp),
            Self::NoteTable => (Role::Table, Key::NoteTable, Key::NoteTableHelp),
            Self::Disclosures => (Role::Window, Key::DisclosuresTitle, Key::DisclosuresIntro),
        };
        SurfaceSemantics { role, label: locale.text(label), description: locale.text(description) }
    }
}

#[cfg(test)]
#[path = "i18n_tests.rs"]
mod tests;
