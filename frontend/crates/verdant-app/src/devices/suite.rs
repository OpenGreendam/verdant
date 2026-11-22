//! Shared native editor shell and dedicated multiband/filter workspaces.
//! Curves use the processor's public response functions; no synthetic audio
//! meter or spectrum is presented as a live measurement.

use super::*;

pub(super) fn values(specs: &[dsp::Spec], params: &[ParamDesc]) -> Vec<f64> {
    specs.iter().map(|spec| param(params, spec.id).map_or(spec.default, |p| p.value)).collect()
}

fn snapshot(params: &[ParamDesc]) -> Vec<(String, f64)> {
    params.iter().map(|p| (p.id.clone(), p.value)).collect()
}

fn switch_comparison(ed: &mut Editor, target: &Target, index: usize, cx: &mut Context<Editor>) {
    if ed.plugin_busy.contains(&target.key)
        || (target.slot != crate::instrument::INSTRUMENT_SLOT && ed.inserts_loading) { return; }
    let Some(params) = current_params(ed, target) else { return };
    let current = snapshot(&params);
    let state = ed.device_anim.entry(target.key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now()));
    if state.comparison.pending.is_some() || state.comparison.active == index { return; }
    let active = state.comparison.active;
    let saved = state.comparison.slots[index].clone().unwrap_or_else(|| current.clone());
    let next = match workflow::comparison_values(&params, &saved) {
        Ok(next) => next,
        Err(error) => { ed.notice(format!("无法恢复比较组：{error}")); cx.notify(); return; }
    };
    let state = ed.device_anim.get_mut(&target.key).unwrap();
    state.comparison.slots[active] = Some(current.clone());
    state.comparison.slots[index] = Some(next.clone());
    if next == current { state.comparison.active = index; }
    else {
        state.comparison.pending = Some(index);
        if !ed.set_device_params(target.track.clone(), target.slot, next, cx) {
            if let Some(state) = ed.device_anim.get_mut(&target.key) { state.comparison.pending = None; }
        }
    }
    workflow::save_comparison(ed, target);
    cx.notify();
}

fn copy_comparison(ed: &mut Editor, target: &Target, cx: &mut Context<Editor>) {
    if ed.plugin_busy.contains(&target.key) { return; }
    let Some(params) = current_params(ed, target) else { return };
    let state = ed.device_anim.entry(target.key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now()));
    if state.comparison.pending.is_some() { return; }
    let active = state.comparison.active;
    let values = snapshot(&params);
    state.comparison.slots[active] = Some(values.clone());
    state.comparison.slots[1 - active] = Some(values);
    workflow::save_comparison(ed, target);
    cx.notify();
}

pub(super) fn toolbar(ed: &Editor, owner: &DeviceTarget, slot: usize, key: InsertKey, effect: &str, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = Target { track: owner.clone(), slot, key };
    let comparison = ed.device_anim.get(&key).map(|s| s.comparison.clone()).unwrap_or_default();
    let busy = ed.plugin_busy.contains(&key) || comparison.pending.is_some();
    let mut compare = div().flex().items_center().gap(gpui::rems(3.0/16.));
    for index in 0..2 {
        let label = if index == 0 { "A" } else { "B" };
        let selected = comparison.active == index;
        let target = target.clone();
        let key_target = target.clone();
        compare = compare.child(crate::ui_controls::selectable_button(format!("compare-{slot}-{index}"), label, selected, th)
            .aria_label(format!("切换到参数组 {label}")).tab_index(0).when(busy, |button| button.opacity(0.5))
            .on_click(cx.listener(move |ed, _, _, cx| switch_comparison(ed, &target, index, cx)))
            .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    switch_comparison(ed, &key_target, index, cx); window.prevent_default(); cx.stop_propagation();
                }
            })));
    }
    let copy_target = target.clone();
    let key_copy_target = target.clone();
    let copy_label = if comparison.active == 0 { "A → B" } else { "B → A" };
    compare = compare.child(crate::ui_controls::selectable_button(format!("compare-copy-{slot}"), copy_label, false, th)
        .aria_label("复制当前参数到另一比较组").tab_index(0).when(busy, |button| button.opacity(0.5))
        .on_click(cx.listener(move |ed, _, _, cx| copy_comparison(ed, &copy_target, cx)))
        .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                copy_comparison(ed, &key_copy_target, cx); window.prevent_default(); cx.stop_propagation();
            }
        })));
    let defaults: Vec<(String, f64)> = params.iter().filter_map(|p| p.default_value.map(|v| (p.id.clone(), v))).collect();
    let preset_params = params.to_vec();
    let preset_owner = owner.clone();
    let effect = effect.to_string();
    let library_effect = effect.clone();
    let wet_lock = workflow::read_workspace(ed, &target).wet_lock;
    let preset = dropdown_button(format!("device-presets-{slot}").into(), "预设".into(), th, move |_| {
        use crate::editor::MenuAction as M;
        use crate::menu::MenuItem as I;
        let mut rows = vec![I::header("起始设置")];
        for (name, overrides) in presets(&effect) {
            let mut values: Vec<(String, f64)> = preset_params.iter().filter_map(|p| p.default_value.map(|v| (p.id.clone(), v))).collect();
            for &(id, value) in overrides {
                if let Some(entry) = values.iter_mut().find(|(key, _)| key == id) { entry.1 = value; }
            }
            if wet_lock {
                if let Some(mix) = param(&preset_params, "mix") {
                    if let Some(value) = values.iter_mut().find(|(id, _)| id == "mix") { value.1 = mix.value; }
                }
            }
            rows.push(I::action(name, M::SetDeviceParams { target: preset_owner.clone(), slot, expected_key: Some(key), values }));
        }
        rows
    }, cx);
    let library_target = target.clone();
    let lock_target = target.clone();
    let reset_target = target.clone();
    let key_defaults = defaults.clone();
    div().flex().flex_wrap().items_center().gap(gpui::rems(8.0/16.)).pb(gpui::rems(8.0/16.)).border_b_1().border_color(th.shade)
        .child(preset)
        .child(crate::ui_controls::selectable_button(format!("device-library-{slot}"), "预设库", false, th).tab_index(0)
            .on_click(cx.listener(move |_, _, _, cx| {
                let editor = cx.entity(); let target = library_target.clone(); let effect = library_effect.clone();
                cx.defer(move |cx| workflow::library::open(editor, target, effect, cx));
            })))
        .when(param(params, "mix").is_some(), |row| row.child(crate::ui_controls::selectable_button(format!("wet-lock-{slot}"), "锁定干湿", wet_lock, th)
            .role(Role::Switch).aria_selected(wet_lock).tab_index(0)
            .on_click(cx.listener(move |ed, _, _, cx| {
                let mut state = workflow::read_workspace(ed, &lock_target); state.wet_lock = !state.wet_lock;
                workflow::write_workspace(ed, &lock_target, &state); cx.notify();
            }))))
        .child(compare).child(div().flex_1())
        .child(crate::ui_controls::selectable_button(format!("device-reset-{slot}"), "重置", false, th).tab_index(0)
            .on_click(cx.listener(move |ed, _, _, cx| write_if_current(ed, &reset_target, defaults.clone(), cx)))
            .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    write_if_current(ed, &target, key_defaults.clone(), cx); window.prevent_default(); cx.stop_propagation();
                }
            })))
        .into_any_element()
}

type Preset = (&'static str, &'static [(&'static str, f64)]);

fn presets(effect: &str) -> Vec<Preset> {
    let mut presets: Vec<Preset> = vec![("初始化", &[])];
    let extra: Vec<Preset> = match effect {
        dsp::EQ_ID => vec![
            ("人声清晰", &[("b1_on", 1.), ("b1_freq", 85.), ("b2_gain", -1.5), ("b3_gain", -2.), ("b4_gain", 1.), ("b5_gain", 2.), ("b6_gain", 1.5)]),
            ("动态去浑浊", &[("b3_freq", 300.), ("b3_q", 1.2), ("b3_dynamic", 1.), ("b3_dyn_range", -4.), ("b3_dyn_threshold", -30.)]),
        ],
        dsp::COMPRESSOR_ID => vec![
            ("柔和人声", &[("threshold", -22.), ("ratio", 2.5), ("attack", 15.), ("release", 140.), ("knee", 12.)]),
            ("总线黏合", &[("threshold", -16.), ("ratio", 2.), ("attack", 30.), ("release", 220.), ("knee", 6.)]),
            ("并行冲击", &[("threshold", -28.), ("ratio", 8.), ("attack", 20.), ("release", 80.), ("mix", 35.)]),
        ],
        dsp::GATE_ID => vec![
            ("柔和降噪", &[("threshold", -45.), ("range", 18.), ("ratio", 3.), ("attack", 2.), ("hold", 80.), ("release", 180.)]),
            ("紧致鼓门", &[("threshold", -30.), ("range", 60.), ("attack", 0.1), ("hold", 45.), ("release", 75.)]),
        ],
        dsp::DEESSER_ID => vec![
            ("自然齿音", &[("freq", 6500.), ("range", 6.), ("threshold", -28.), ("release", 90.)]),
            ("高频柔化", &[("freq", 9000.), ("range", 4.), ("threshold", -32.)]),
        ],
        dsp::LIMITER_ID => vec![
            ("透明峰值保护", &[("gain", 0.), ("ceiling", -1.), ("lookahead", 5.), ("release", 180.)]),
            ("响度提升", &[("gain", 4.), ("ceiling", -1.), ("lookahead", 5.), ("release", 120.)]),
        ],
        dsp::REVERB_ID => vec![
            ("人声小房间", &[("predelay", 18.), ("decay", 1.2), ("size", 35.), ("mix", 16.), ("low_cut", 180.), ("high_cut", 9000.)]),
            ("宽阔大厅", &[("predelay", 35.), ("decay", 4.5), ("size", 90.), ("width", 130.), ("mix", 30.)]),
        ],
        dsp::DELAY_ID => vec![
            ("短拍回声", &[("time", 110.), ("feedback", 15.), ("mix", 18.)]),
            ("立体声乒乓", &[("time", 375.), ("pingpong", 1.), ("feedback", 42.), ("mix", 25.)]),
            ("温暖长回声", &[("time", 500.), ("feedback", 50.), ("drive", 5.), ("high_cut", 4500.), ("mix", 22.)]),
        ],
        dsp::SATURATION_ID => vec![
            ("温暖磁带", &[("style", 0.), ("drive", 9.), ("tone", -2.), ("mix", 50.)]),
            ("柔和电子管", &[("style", 1.), ("drive", 12.), ("mix", 35.)]),
        ],
        dsp::MULTIBAND_ID => vec![
            ("人声频段整理", &[("b2.range", -3.), ("b2.threshold", -28.), ("b3.range", -3.), ("b3.threshold", -26.), ("b5.range", -4.), ("b5.threshold", -30.)]),
            ("低频控制", &[("b1.range", -6.), ("b1.threshold", -24.), ("b1.ratio", 4.), ("b1.attack", 25.)]),
        ],
        dsp::FILTER_ID | dsp::SIMPLON_ID => vec![
            ("电话频带", &[("f1.freq", 3400.), ("f2.enabled", 1.), ("f2.freq", 350.), ("f2.shape", 1.)]),
            ("缓慢扫频", &[("f1.freq", 1200.), ("f1.resonance", 2.), ("lfo_rate", 0.25), ("lfo_depth", 24.)]),
        ],
        dsp::MICRO_ID => vec![
            ("包络哇音", &[("freq", 500.), ("resonance", 3.), ("env_depth", 30.), ("env_release", 180.)]),
            ("低频清理", &[("shape", 1.), ("freq", 100.)]),
        ],
        _ => vec![],
    };
    presets.extend(extra);
    presets
}

/// Controls not present in an effect's primary graph section stay in the
/// same native surface. New parameters therefore cannot silently disappear.
pub(super) fn remaining(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    effect: &str, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let covered = covered_ids(effect);
    let missing: Vec<&ParamDesc> = params.iter().filter(|p| p.max > p.min && !covered.contains(&p.id.as_str()) && !is_banded(effect, &p.id) && dsp::modulation_parameter(&p.id).is_none()).collect();
    let th = ed.theme;
    if missing.is_empty() { return div().into_any_element(); }
    let mut groups: Vec<(&str, Vec<&ParamDesc>)> = Vec::new();
    for d in missing {
        let label = remaining_group(effect, &d.id);
        if let Some((_, controls)) = groups.iter_mut().find(|(name, _)| *name == label) { controls.push(d); }
        else { groups.push((label, vec![d])); }
    }
    div().flex().flex_col().gap(gpui::rems(12.0/16.)).children(groups.into_iter().map(|(label, descriptors)| {
        let mut controls = div().flex().flex_wrap().items_start().gap(gpui::rems(12./16.));
        for d in descriptors { controls = controls.child(control(ed, editor, owner, slot, key, effect, params, d, cx)); }
        div().flex().flex_col().gap(gpui::rems(10.0/16.)).pt(gpui::rems(10.0/16.)).border_t_1().border_color(th.shade)
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(th.text_secondary).child(label))
            .child(controls)
    })).into_any_element()
}

fn remaining_group(effect: &str, id: &str) -> &'static str {
    if matches!(effect, dsp::SYNTH_ID | dsp::ONE_SYNTH_ID) {
        if id.starts_with("osc3") { return "振荡器 3"; }
        if id.starts_with("osc4") { return "振荡器 4"; }
        if id.starts_with("lfo") { return "低频振荡调制"; }
        if id.starts_with("filter") || id.starts_with("cutoff") || id.starts_with("resonance") { return "滤波器与路由"; }
        if id.starts_with("fx_drive") { return "效果 · 驱动"; }
        if id.starts_with("fx_chorus") { return "效果 · 合唱"; }
        if id.starts_with("fx_phaser") || id == "fx_flanger" { return "效果 · 相位与镶边"; }
        if id.starts_with("fx_delay") { return "效果 · 延迟"; }
        if id.starts_with("fx_reverb") { return "效果 · 混响"; }
        if id.starts_with("fx_comp") { return "效果 · 压缩"; }
        if id.starts_with("arp_") { return "琶音器"; }
        if matches!(id, "mpe" | "mpe_range" | "bend_range") { return "MIDI 与 MPE"; }
        if matches!(id, "noise" | "pulse_width" | "drive" | "unison" | "unison_detune" | "osc_model" | "high_quality") { return "音色与齐奏"; }
        return "演奏";
    }
    if effect == dsp::REVERB_ID && id.starts_with("decay_") { return "衰减塑形"; }
    if effect == dsp::SATURATION_ID {
        if id.starts_with("split") || id == "band_count" { return "分频"; }
        return "调制与反馈";
    }
    if effect == dsp::DELAY_ID { return "反馈与调制"; }
    if matches!(id, "character" | "character_drive" | "character_pre") { return "音色"; }
    "处理与检测"
}

fn covered_ids(effect: &str) -> &'static [&'static str] {
    match effect {
        dsp::EQ_ID => &["output", "solo"],
        dsp::COMPRESSOR_ID => &["style", "threshold", "ratio", "knee", "range", "makeup", "attack", "release", "lookahead", "mix", "sc_hpf", "sc_lpf", "sc_listen"],
        dsp::GATE_ID => &["threshold", "range", "ratio", "mix", "attack", "hold", "release", "lookahead", "sc_hpf", "sc_lpf", "sc_listen"],
        dsp::DEESSER_ID => &["threshold", "range", "freq", "release", "mode", "sc_listen"],
        dsp::LIMITER_ID => &["gain", "ceiling", "release", "lookahead", "link"],
        dsp::REVERB_ID => &["predelay", "decay", "size", "damping", "diffusion", "width", "low_cut", "high_cut", "modulation", "mix", "post_low_gain", "post_high_gain", "post_mid_freq", "post_mid_gain", "post_mid_q"],
        dsp::DELAY_ID => &["time", "offset", "feedback", "drive", "low_cut", "high_cut", "width", "mix", "pingpong", "main_on", "main_level", "main_pan", "sync"],
        dsp::SATURATION_ID => &["style", "drive", "tone", "mix", "output"],
        dsp::MULTIBAND_ID => &["band_count", "xover1", "xover2", "xover3", "xover4", "xover5", "mix", "output", "sc_listen"],
        dsp::FILTER_ID | dsp::SIMPLON_ID => &["routing", "mix", "output", "lfo_rate", "lfo_depth", "env_depth", "env_attack", "env_release"],
        dsp::MICRO_ID => &["freq", "resonance", "shape", "drive", "env_depth", "env_attack", "env_release", "mix", "output"],
        dsp::SYNTH_ID | dsp::ONE_SYNTH_ID => &["osc1", "osc2", "osc2_semi", "detune", "osc2_mix", "width", "gain", "attack", "decay", "sustain", "release", "cutoff", "resonance", "env_amount"],
        _ => &[],
    }
}

fn is_banded(effect: &str, id: &str) -> bool {
    match effect {
        dsp::COMPRESSOR_ID => id.strip_prefix("sc").and_then(|s| s.split_once('_')).is_some_and(|(n,_)| n.parse::<usize>().is_ok()),
        dsp::REVERB_ID => ["decay", "post"].iter().any(|prefix| id.strip_prefix(prefix).and_then(|s| s.split_once('_')).is_some_and(|(n,_)| n.parse::<usize>().is_ok())),
        dsp::EQ_ID => id.strip_prefix('b').and_then(|s| s.split_once('_')).is_some_and(|(n, _)| n.parse::<usize>().is_ok()),
        dsp::MULTIBAND_ID => id.strip_prefix('b').and_then(|s| s.split_once('.')).is_some_and(|(n, _)| n.parse::<usize>().is_ok()),
        dsp::SATURATION_ID => id.strip_prefix('b').and_then(|s| s.split_once('_')).is_some_and(|(n, _)| n.parse::<usize>().is_ok()),
        dsp::FILTER_ID | dsp::SIMPLON_ID => id.strip_prefix('f').and_then(|s| s.split_once('.')).is_some_and(|(n, _)| n.parse::<usize>().is_ok()),
        dsp::DELAY_ID => id.strip_prefix("tap").and_then(|s| s.split_once('_')).is_some_and(|(n, _)| n.parse::<usize>().is_ok()),
        _ => false,
    }
}

fn enum_labels(effect: &str, id: &str) -> Option<&'static [&'static str]> {
    Some(match id {
        "filter_shape" | "filter2_shape" | "filter3_shape" | "filter4_shape" if matches!(effect, dsp::SYNTH_ID | dsp::ONE_SYNTH_ID) =>
            &["低通", "高通", "带通", "陷波"],
        "filter_routing" if matches!(effect, dsp::SYNTH_ID | dsp::ONE_SYNTH_ID) => &["串联", "并联"],
        "filter_character" if matches!(effect, dsp::SYNTH_ID | dsp::ONE_SYNTH_ID) => &["纯净", "柔和饱和", "谐波", "温和压缩", "硬削波", "不对称"],
        "osc_model" => &["经典", "柔和", "三次谐波", "倍频", "振幅塑形"],
        "high_quality" => &["关闭", "2×", "4×"],
        "arp_order" => &["上行", "下行", "往返", "按键顺序", "随机"],
        "arp_latch" => &["关闭", "保持", "叠加保持"],
        "arp_sync" | "fx_delay_sync" => &["自由", "1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 小节", "2 小节"],
        "fx_drive_style" => &["柔和饱和", "柔和削波", "正弦折叠", "硬削波"],
        "fx_flanger" => &["相位", "镶边"],
        "sc_channel" => &["立体声", "Mid", "Side", "左", "右"],
        id if effect == dsp::SATURATION_ID && id.starts_with('b') && id.ends_with("_channel") => &["双声道", "左 / Mid", "右 / Side"],
        "channel_mode" if effect == dsp::SATURATION_ID => &["L / R", "Mid / Side"],
        "channel_mode" => &dsp::DYNAMICS_CHANNEL_MODES,
        "detection" if effect == dsp::DEESSER_ID => &dsp::DEESSER_DETECTION,
        "detection" => &dsp::DETECTOR_MODES,
        "character" if !matches!(effect, dsp::FILTER_ID | dsp::SIMPLON_ID | dsp::MICRO_ID) => &dsp::COMP_CHARACTERS,
        "style" if effect == dsp::GATE_ID => &dsp::GATE_STYLES,
        "style" if effect == dsp::LIMITER_ID => &dsp::LIMITER_STYLES,
        "style" if effect == dsp::REVERB_ID => &dsp::REVERB_STYLES,
        "sync" if effect == dsp::DELAY_ID => &dsp::DELAY_SYNC,
        "oversampling" | "feedback_quality" => &dsp::quality::QUALITY_NAMES,
        "time_mode" => &["平滑", "磁带", "伸缩", "反向"],
        "routing" if effect == dsp::DELAY_ID => &["立体声", "M/S", "交叉", "串联"],
        "sync_feel" => &["直拍", "附点", "三连音"],
        "phase" if effect == dsp::EQ_ID => &["零延迟", "自然相位", "线性相位"],
        "phase" if effect == dsp::MULTIBAND_ID => &["动态相位", "最小相位", "线性相位"],
        "phase" => &["最小相位", "线性相位"],
        "resolution" => &["低", "中", "高"],
        "layout" if effect == dsp::MULTIBAND_ID => &["相邻频段", "悬空频段"],
        "predelay_sync" | "gate_sync" => &dsp::DELAY_SYNC,
        "trigger_mode" => &["音频", "MIDI", "节拍"],
        "sync" if effect == dsp::COMPRESSOR_ID => &dsp::DELAY_SYNC,
        "dither" => &["关闭", "TPDF", "柔和整形", "强整形"],
        "cabinet" => &["关闭", "开背音箱", "封闭 4×12", "贝斯箱体", "收音机"],
        id if id.ends_with(".character") || (id == "character" && matches!(effect, dsp::FILTER_ID | dsp::SIMPLON_ID | dsp::MICRO_ID)) => &dsp::FILTER_CHARACTERS,
        id if effect == dsp::MULTIBAND_ID && id.ends_with(".sc_source") => &["自动", "内部", "外部"],
        id if effect == dsp::MULTIBAND_ID && id.ends_with(".place") => &["立体声", "Mid", "Side", "左", "右"],
        id if effect == dsp::COMPRESSOR_ID && id.starts_with("sc") && id.ends_with("_shape") => &["峰值", "低架", "高架", "低切", "高切"],
        "osc3" | "osc4" | "lfo_wave" => &dsp::WAVES,
        "filter_shape" | "filter2_shape" => &dsp::FILTER_SHAPES,
        "filter_routing" => &dsp::FILTER_ROUTINGS,
        "filter_type" => &["低通", "高通", "带通"],
        "noise_type" => &["白噪声", "粉红噪声"],
        "osc1" | "osc2" => &dsp::WAVES,
        _ => return None,
    })
}

fn control(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    effect: &str, params: &[ParamDesc], d: &ParamDesc, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    if let Some(labels) = enum_labels(effect, &d.id).filter(|labels| labels.len() == d.step_count as usize + 1 && d.min == 0.0) {
        return div().flex().flex_col().gap(gpui::rems(5.0/16.)).child(div().text_xs().text_color(th.text_secondary).child(d.name.clone()))
            .child(choice(ed, owner, slot, params, &d.id, labels, cx)).into_any_element();
    }
    if d.step_count == 1 && d.min == 0.0 && d.max == 1.0 {
        return div().py(gpui::rems(6.0/16.)).child(toggle(ed, owner, slot, params, &d.id, &d.name, cx)).into_any_element();
    }
    knob(editor.clone(), Target { track: owner.clone(), slot, key }, d.clone(), &d.name, th, 42.0).into_any_element()
}

fn band_tabs(ed: &Editor, key: InsertKey, slot: usize, count: usize, selected: usize, prefix: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    div().flex().flex_wrap().gap(gpui::rems(4.0/16.)).children((0..count).map(|index| {
        let label = format!("{prefix} {}", index + 1);
        crate::ui_controls::selectable_button(format!("band-tab-{slot}-{index}"), label, index == selected, th)
            .role(Role::Tab).tab_index(0).border_color(with_alpha(band_color(index), 0.8))
            .on_click(cx.listener(move |ed, _, _, cx| { select_band(ed, key, index); cx.notify(); }))
            .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    select_band(ed, key, index); window.prevent_default(); cx.stop_propagation(); cx.notify();
                }
            }))
    })).into_any_element()
}

pub(super) fn compressor_sidechain(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme; let selected = selected_band(ed, key, 6); let axis = Axis::lin(-36., 24.);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let values = values(dsp::COMP_SPECS, params);
        let grid = [-24., -12., 0., 12.].map(|v| (axis.norm(v), format!("{v:+.0} dB")));
        paint_grid(window, plot, th, &freq_grid(), &grid, Some(axis.norm(0.)), texts);
        polyline(window, plot.o, (0..=240).map(|i| {
            let n = i as f64 / 240.; Some((plot.x(n), plot.y(axis.norm(dsp::comp_sidechain_response_db(&values, FREQ_AXIS.value(n), verdant_plugins::SAMPLE_RATE)))))
        }), 2., th.accent);
        texts.push(("侧链 EQ · 检测路径".into(), p(10.,5.,plot.o), 11., th.text_secondary));
    });
    let mut handles = Vec::new();
    for i in 0..6 {
        let prefix = format!("sc{}", i + 1); let on = param(params, &format!("{prefix}_on")).is_some_and(|p|p.value>=0.5);
        if !on && i != selected { continue; }
        let freq = param(params, &format!("{prefix}_freq")).map_or(1000.,|p|p.value);
        let gain = param(params, &format!("{prefix}_gain")).map_or(0.,|p|p.value);
        let has_gain = param(params, &format!("{prefix}_shape")).is_none_or(|p|p.value<3.);
        handles.push(Handle { control: Control::Band(i), at:(FREQ_AXIS.norm(freq),axis.norm(if has_gain {gain}else{-6.})),
            x:Some((format!("{prefix}_freq"),FREQ_AXIS)), y:has_gain.then(||(format!("{prefix}_gain"),axis)),color:band_color(i),label:(i+1).to_string(),
            hollow:!on,selected:i==selected,also:vec![(format!("{prefix}_on"),1.)],defaults:vec![(format!("{prefix}_gain"),0.)] });
    }
    let prefix = format!("sc{}", selected+1);
    let ids:Vec<_>=["freq","gain","q"].map(|s|format!("{prefix}_{s}")).into_iter().collect();let refs:Vec<_>=ids.iter().map(String::as_str).collect();
    div().flex().flex_col().gap(gpui::rems(8./16.))
        .child(graph(editor.clone(),Target{track:owner.clone(),slot,key},params.to_vec(),"comp-sidechain-eq","压缩器六段侧链均衡",th,painter,handles,None,None).w_full().h(gpui::rems(160./16.)))
        .child(band_tabs(ed,key,slot,6,selected,"侧链频段",cx))
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.))
            .child(toggle(ed,owner,slot,params,&format!("{prefix}_on"),"启用",cx))
            .child(choice(ed,owner,slot,params,&format!("{prefix}_shape"),&["峰值","低架","高架","低切","高切"],cx))
            .child(knob_row(editor,||Target{track:owner.clone(),slot,key},params,&refs,th,38.))).into_any_element()
}

pub(super) fn eq_dynamic(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let selected = selected_band(ed, key, dsp::EQ_BANDS);
    let prefix = format!("b{}_", selected + 1);
    let extras: Vec<&ParamDesc> = params.iter().filter(|p| p.id.starts_with(&prefix) && !["on", "freq", "gain", "q", "shape", "slope", "place", "slope_continuous", "slope_db"].contains(&&p.id[prefix.len()..])).collect();
    let solo = param(params, "solo").is_some_and(|p| p.value.round() as usize == selected + 1);
    let target = Target { track: owner.clone(), slot, key };
    let mut rows = div().flex().flex_wrap().items_center().gap(gpui::rems(8.0/16.))
        .child(crate::ui_controls::selectable_button(format!("eq-solo-{slot}"), "独听频段", solo, th)
            .on_click(cx.listener(move |ed, _, _, cx| write_if_current(ed, &target, vec![("solo".into(), if solo { 0. } else { selected as f64 + 1. })], cx))));
    for d in extras { rows = rows.child(control(ed, editor, owner, slot, key, dsp::EQ_ID, params, d, cx)); }
    div().flex().flex_col().gap(gpui::rems(8.0/16.))
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.))
            .child(div().text_xs().text_color(th.text_secondary).child(format!("频段 {} · 动态处理", selected + 1)))
            .child(sidechain_picker(ed, owner, slot, cx)))
        .child(rows).into_any_element()
}

pub(super) fn multiband(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let count = param(params, "band_count").map_or(6, |p| p.value.round().clamp(1., 6.) as usize);
    let selected = selected_band(ed, key, count);
    let target = || Target { track: owner.clone(), slot, key };
    let axis = Axis::lin(-48., 12.);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let values = values(dsp::MULTIBAND_SPECS, params);
        let grid = [-36., -24., -12., 0.].map(|db| (axis.norm(db), format!("{db:.0} dB")));
        paint_grid(window, plot, th, &freq_grid(), &grid, Some(axis.norm(0.)), texts);
        for band in 0..count {
            let pts: Vec<_> = (0..=240).map(|i| {
                let n = i as f64 / 240.;
                let db = dsp::multiband_band_response_db(&values, band, FREQ_AXIS.value(n), verdant_plugins::SAMPLE_RATE);
                (plot.x(n), plot.y(axis.norm(db)))
            }).collect();
            if band == selected { fill_under(window, plot, &pts, plot.y(0.), with_alpha(band_color(band), 0.15)); }
            polyline(window, plot.o, pts.into_iter().map(Some), if band == selected { 2.5 } else { 1.5 }, with_alpha(band_color(band), if band == selected { 1. } else { 0.65 }));
        }
        texts.push(("分频器频率响应".into(), p(10., 6., plot.o), 11., th.text_secondary));
    });
    let mut handles: Vec<Handle> = (1..count).filter_map(|index| {
        let id = format!("xover{index}");
        let d = param(params, &id)?;
        Some(Handle { control: Control::ParameterHandle(id.clone()), at: (FREQ_AXIS.norm(d.value), axis.norm(-6.)), x: Some((id.clone(), FREQ_AXIS)), y: None,
            color: band_color(index - 1), label: format!("{index}"), hollow: false, selected: false, also: vec![], defaults: vec![(id, d.default_value.unwrap_or(d.value))] })
    }).collect();
    if param(params, "layout").is_some_and(|p| p.value >= 0.5) {
        handles.clear();
        for field in ["low", "high"] {
            let id = format!("b{}.{field}", selected + 1);
            if let Some(d) = param(params, &id) {
                handles.push(Handle { control: Control::ParameterHandle(id.clone()), at: (FREQ_AXIS.norm(d.value), axis.norm(-6.)), x: Some((id.clone(), FREQ_AXIS)), y: None,
                    color: band_color(selected), label: if field == "low" { "低".into() } else { "高".into() }, hollow: false, selected: true,
                    also: vec![], defaults: vec![(id, d.default_value.unwrap_or(d.value))] });
            }
        }
    }
    let prefix = format!("b{}.", selected + 1);
    let mut controls = div().flex().flex_wrap().items_center().gap(gpui::rems(8.0/16.));
    for d in params.iter().filter(|p| p.id.starts_with(&prefix)) {
        let item = if d.id.ends_with(".mode") { choice(ed, owner, slot, params, &d.id, &dsp::MULTIBAND_MODES, cx) }
            else { control(ed, editor, owner, slot, key, dsp::MULTIBAND_ID, params, d, cx) };
        controls = controls.child(item);
    }
    let cross_ids: Vec<String> = (1..count).map(|i| format!("xover{i}")).collect();
    let cross_refs: Vec<&str> = cross_ids.iter().map(String::as_str).collect();
    div().flex().flex_col().gap(gpui::rems(12.0/16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "multiband-graph", "多段分频响应，拖动节点调整分频点", th, painter, handles, None, None).w_full().h(gpui::rems(225./16.)))
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.)).child(band_tabs(ed, key, slot, count, selected, "频段", cx)).child(sidechain_picker(ed, owner, slot, cx)))
        .child(controls)
        .child(div().pt(gpui::rems(8./16.)).border_t_1().border_color(th.shade).flex().flex_col().gap(gpui::rems(8./16.))
            .child(knob_row(editor, target, params, &cross_refs, th, 40.))
            .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.)).child(knob_row(editor, target, params, &["band_count", "mix", "output"], th, 42.))
                .child(toggle(ed, owner, slot, params, "sc_listen", "监听侧链", cx))))
        .into_any_element()
}

pub(super) fn filters(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    effect: &str, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let micro = effect == dsp::MICRO_ID;
    let count = if micro { 1 } else if effect == dsp::SIMPLON_ID { 2 } else { 4 };
    let specs = if micro { dsp::MICRO_SPECS } else if count == 2 { dsp::SIMPLON_SPECS } else { dsp::FILTER_SPECS };
    let selected = selected_band(ed, key, count);
    let target = || Target { track: owner.clone(), slot, key };
    let axis = Axis::lin(-48., 24.);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let values = values(specs, params);
        let grid = [-36., -24., -12., 0., 12.].map(|db| (axis.norm(db), format!("{db:.0} dB")));
        paint_grid(window, plot, th, &freq_grid(), &grid, Some(axis.norm(0.)), texts);
        let points: Vec<_> = (0..=240).map(|i| {
            let n = i as f64 / 240.; let f = FREQ_AXIS.value(n);
            let db = if micro { dsp::micro_response_db(&values, f, verdant_plugins::SAMPLE_RATE) }
                else { dsp::filter_response_db(&values, count, f, verdant_plugins::SAMPLE_RATE) };
            (plot.x(n), plot.y(axis.norm(db)))
        }).collect();
        fill_under(window, plot, &points, plot.y(0.), with_alpha(th.accent, 0.12));
        polyline(window, plot.o, points.into_iter().map(Some), 2.5, th.accent);
        texts.push(("静态频率响应 · 调制围绕截止频率运动".into(), p(10., 6., plot.o), 11., th.text_secondary));
    });
    let handles = (0..count).filter_map(|index| {
        let id = if micro { "freq".to_string() } else { format!("f{}.freq", index + 1) };
        let d = param(params, &id)?;
        let on = micro || param(params, &format!("f{}.enabled", index + 1)).is_some_and(|p| p.value >= 0.5);
        Some(Handle { control: Control::Band(index), at: (FREQ_AXIS.norm(d.value), axis.norm(0.)), x: Some((id.clone(), FREQ_AXIS)), y: None,
            color: band_color(index), label: format!("{}", index + 1), hollow: !on, selected: index == selected,
            also: if !on { vec![(format!("f{}.enabled", index + 1), 1.)] } else { vec![] }, defaults: vec![(id, d.default_value.unwrap_or(d.value))] })
    }).collect();
    let mut main = div().flex().flex_col().gap(gpui::rems(12./16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "filter-graph", "滤波器频响，拖动节点调整截止频率", th, painter, handles, None, None).w_full().h(gpui::rems(225./16.)));
    if micro {
        main = main.child(choice(ed, owner, slot, params, "shape", &dsp::FILTER_SHAPES[..2], cx))
            .child(knob_row(editor, target, params, &["freq", "resonance", "drive", "mix", "output"], th, 46.));
    } else {
        let prefix = format!("f{}.", selected + 1);
        let ids: Vec<String> = ["freq", "resonance", "drive"].map(|field| format!("{prefix}{field}")).into_iter().collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        main = main.child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.))
                .child(band_tabs(ed, key, slot, count, selected, "滤波器", cx))
                .child(choice(ed, owner, slot, params, "routing", &dsp::FILTER_ROUTINGS, cx)))
            .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.))
                .child(toggle(ed, owner, slot, params, &format!("{prefix}enabled"), "启用", cx))
                .child(choice(ed, owner, slot, params, &format!("{prefix}shape"), &dsp::FILTER_SHAPES, cx))
                .child(choice(ed, owner, slot, params, &format!("{prefix}slope"), &dsp::FILTER_SLOPES, cx)))
            .child(knob_row(editor, target, params, &refs, th, 46.))
            .child(knob_row(editor, target, params, &["lfo_rate", "lfo_depth", "mix", "output"], th, 42.));
    }
    if !micro {
        let prefix = format!("f{}.", selected + 1);
        let mut extras = div().flex().flex_wrap().gap(gpui::rems(8./16.));
        for d in params.iter().filter(|p| p.id.starts_with(&prefix) && !["enabled", "freq", "resonance", "drive", "shape", "slope"].contains(&&p.id[prefix.len()..])) {
            extras = extras.child(control(ed, editor, owner, slot, key, effect, params, d, cx));
        }
        main = main.child(extras);
    }
    main.child(div().pt(gpui::rems(8./16.)).border_t_1().border_color(th.shade).flex().flex_col().gap(gpui::rems(8./16.))
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.))
            .child(div().text_xs().text_color(th.text_secondary).child("输入包络调制"))
            .child(sidechain_picker(ed, owner, slot, cx)))
        .child(knob_row(editor, target, params, &["env_depth", "env_attack", "env_release"], th, 42.)))
        .into_any_element()
}

pub(super) fn saturation_bands(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let count = param(params, "band_count").map_or(1, |p| p.value.round().clamp(1., 6.) as usize);
    let selected = selected_band(ed, key, count);
    let target = || Target { track: owner.clone(), slot, key };
    let axis = Axis::lin(-24., 36.);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let mut cross: [f64; 5] = std::array::from_fn(|index| param(params, &format!("split{}", index+1)).map_or(dsp::SATURATION_SPECS[6+index].default, |p|p.value));
        cross.sort_by(f64::total_cmp);
        let grid = [-12., 0., 12., 24.].map(|db| (axis.norm(db), format!("{db:+.0} dB")));
        paint_grid(window, plot, th, &freq_grid(), &grid, Some(axis.norm(0.)), texts);
        for band in 0..count {
            let low = if band == 0 { dsp::FREQ_MIN } else { cross[band - 1] };
            let high = if band + 1 == count { dsp::FREQ_MAX } else { cross[band] };
            let drive = param(params, &format!("b{}_drive", band + 1)).map_or(0., |p| p.value);
            let (left, right) = (plot.x(FREQ_AXIS.norm(low)), plot.x(FREQ_AXIS.norm(high)));
            let y = plot.y(axis.norm(drive));
            let zero = plot.y(axis.norm(0.));
            window.paint_quad(fill(b(left, y.min(zero), (right - left).max(1.), (zero - y).abs().max(1.), plot.o), with_alpha(band_color(band), if band == selected { 0.23 } else { 0.10 })));
            polyline(window, plot.o, [Some((left, y)), Some((right, y))], if band == selected { 2.5 } else { 1.5 }, band_color(band));
            texts.push((format!("{}", band + 1), p((left + right) * 0.5, 8., plot.o), 11., th.text_secondary));
        }
    });
    let mut sorted: Vec<(String, f64)> = (1..=5).map(|i| {
        let id = format!("split{i}"); let value = param(params, &id).map_or(1000., |d| d.value); (id, value)
    }).collect();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut handles: Vec<Handle> = sorted.iter().take(count.saturating_sub(1)).map(|(id, value)| {
        let default = param(params, id).and_then(|p| p.default_value).unwrap_or(*value);
        Handle { control: Control::ParameterHandle(id.clone()), at: (FREQ_AXIS.norm(*value), axis.norm(0.)), x: Some((id.clone(), FREQ_AXIS)), y: None,
            color: th.text_secondary, label: String::new(), hollow: true, selected: false, also: vec![], defaults: vec![(id.clone(), default)] }
    }).collect();
    let low = if selected == 0 { dsp::FREQ_MIN } else { sorted[selected - 1].1 };
    let high = if selected + 1 == count { dsp::FREQ_MAX } else { sorted[selected].1 };
    let drive_id = format!("b{}_drive", selected + 1);
    let drive = param(params, &drive_id).map_or(0., |p| p.value);
    handles.push(Handle { control: Control::Band(selected), at: (FREQ_AXIS.norm((low * high).sqrt()), axis.norm(drive)), x: None,
        y: Some((drive_id.clone(), axis)), color: band_color(selected), label: (selected + 1).to_string(), hollow: false, selected: true, also: vec![], defaults: vec![(drive_id, 0.)] });
    let prefix = format!("b{}_", selected + 1);
    let mut band_controls = div().flex().flex_wrap().items_center().gap(gpui::rems(10./16.));
    for d in params.iter().filter(|p| p.id.starts_with(&prefix)) {
        let item = if d.id.ends_with("_style") { choice(ed, owner, slot, params, &d.id, &dsp::SATURATION_BAND_STYLES, cx) }
            else { control(ed, editor, owner, slot, key, dsp::SATURATION_ID, params, d, cx) };
        band_controls = band_controls.child(item);
    }
    div().flex().flex_col().gap(gpui::rems(10./16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "saturation-bands", "频段驱动：左右拖动分频点，上下拖动当前频段的驱动", th, painter, handles, None, None).w_full().h(gpui::rems(160./16.)))
        .child(band_tabs(ed, key, slot, count, selected, "频段", cx))
        .child(band_controls).into_any_element()
}

pub(super) fn reverb_nodes(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let selected = selected_band(ed, key, 8); let th = ed.theme;
    let mut result = div().flex().flex_col().gap(gpui::rems(10./16.))
        .child(band_tabs(ed, key, slot, 8, selected, "EQ 节点", cx));
    for (family, label, gain, shapes) in [("decay", "衰减塑形", "rate", &["峰值", "低架", "高架"][..]), ("post", "输出均衡", "gain", &["峰值", "低架", "高架", "低切", "高切"][..])] {
        let prefix = format!("{family}{}", selected + 1);
        let ids: Vec<_> = ["freq", gain, "q"].map(|s| format!("{prefix}_{s}")).into_iter().collect();
        let refs: Vec<_> = ids.iter().map(String::as_str).collect();
        result = result.child(div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.))
            .child(div().text_sm().child(label))
            .child(toggle(ed, owner, slot, params, &format!("{prefix}_on"), "启用", cx))
            .child(choice(ed, owner, slot, params, &format!("{prefix}_shape"), shapes, cx))
            .child(knob_row(editor, || Target { track: owner.clone(), slot, key }, params, &refs, th, 40.)));
    }
    result.into_any_element()
}

pub(super) fn delay(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let target = || Target { track: owner.clone(), slot, key };
    let selected = selected_band(ed, key, dsp::DELAY_TAPS + 1);
    let time_axis = Axis::lin(0., 150.);
    let level_axis = Axis::lin(-60., 12.);
    let tempo = ed.tempo();
    let bpm = tempo.bpm_at(tempo.tick_at(ed.playhead_seconds()).round() as i64);
    let painter: Painter = Rc::new(move |window, plot, params, texts| {
        let values = values(dsp::DELAY_SPECS, params);
        let base = dsp::delay_effective_time_ms(&values, Some(bpm), verdant_plugins::SAMPLE_RATE);
        let channel_times = dsp::delay_target_times_ms(&values, Some(bpm), verdant_plugins::SAMPLE_RATE);
        let grid = [25., 50., 75., 100., 125., 150.].map(|percent| (time_axis.norm(percent), format!("{:.0} ms", base * percent / 100.)));
        let levels = [-48., -24., 0.].map(|db| (level_axis.norm(db), format!("{db:.0} dB")));
        paint_grid(window, plot, th, &grid, &levels, Some(level_axis.norm(0.)), texts);
        let v = |id: &str, default: f64| param(params, id).map_or(default, |p| p.value);
        for tap in 0..=dsp::DELAY_TAPS {
            let prefix = if tap == 0 { "main".to_string() } else { format!("tap{tap}") };
            let on = v(&format!("{prefix}_on"), if tap == 0 { 1. } else { 0. }) >= 0.5;
            if !on && tap != selected { continue; }
            let percent = if tap == 0 { 100. } else { v(&format!("{prefix}_time"), 50.) };
            let level = v(&format!("{prefix}_level"), 0.);
            let pan = v(&format!("{prefix}_pan"), 0.) / 100.;
            // Channel positions and pan gains match the processor's static taps.
            // Modulation/feedback are intentionally not presented as measured audio.
            for (channel, time_scale, gain) in [(0, channel_times[0] / base, 1. - pan.max(0.)), (1, channel_times[1] / base, 1. + pan.min(0.))] {
                let color = with_alpha(band_color(channel), if on { 0.75 } else { 0.3 });
                let x = plot.x(time_axis.norm(percent * time_scale)) + if channel == 0 { -1.5 } else { 1.5 };
                let y = plot.y(level_axis.norm(level + 20. * gain.max(1e-6).log10()));
                window.paint_quad(fill(b(x - 1.5, y, 3., (plot.y(0.) - y).max(1.), plot.o), color));
            }
        }
        let mode_label = if v("time_mode", 0.) >= 2.5 { "反向窗口跨度" } else { "静态抽头目标" };
        texts.push((format!("{mode_label} · L {:.1} / R {:.1} ms · {bpm:.1} BPM", channel_times[0], channel_times[1]), p(10., 6., plot.o), 11., th.text_secondary));
    });
    let v = |id: &str, default: f64| param(params, id).map_or(default, |p| p.value);
    let handles = (0..=dsp::DELAY_TAPS).map(|tap| {
        let prefix = if tap == 0 { "main".to_string() } else { format!("tap{tap}") };
        let time_id = format!("{prefix}_time");
        let level_id = format!("{prefix}_level");
        let on_id = format!("{prefix}_on");
        let on = v(&on_id, if tap == 0 { 1. } else { 0. }) >= 0.5;
        let defaults = [time_id.as_str(), level_id.as_str()].iter().filter_map(|id| param(params, id)
            .and_then(|p| p.default_value.map(|value| ((*id).to_string(), value)))).collect();
        Handle { control: Control::Band(tap), at: (time_axis.norm(if tap == 0 { 100. } else { v(&time_id, 50.) }), level_axis.norm(v(&level_id, 0.))),
            x: (tap > 0).then_some((time_id, time_axis)), y: Some((level_id, level_axis)), color: th.accent,
            label: if tap == 0 { "主".into() } else { tap.to_string() }, hollow: !on, selected: tap == selected,
            also: if on { vec![] } else { vec![(on_id, 1.)] }, defaults }
    }).collect();
    let tabs = div().flex().flex_wrap().gap(gpui::rems(4./16.)).children((0..=dsp::DELAY_TAPS).map(|tap| {
        crate::ui_controls::selectable_button(format!("delay-tap-{slot}-{tap}"), if tap == 0 { "主".to_string() } else { tap.to_string() }, selected == tap, th)
            .role(Role::Tab).tab_index(0).aria_label(if tap == 0 { "主抽头".to_string() } else { format!("抽头 {tap}") })
            .on_click(cx.listener(move |ed, _, _, cx| { select_band(ed, key, tap); cx.notify(); }))
            .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") { select_band(ed, key, tap); window.prevent_default(); cx.stop_propagation(); cx.notify(); }
            }))
    }));
    let prefix = if selected == 0 { "main".to_string() } else { format!("tap{selected}") };
    let ids: Vec<String> = ["time", "level", "pan"].map(|field| format!("{prefix}_{field}")).into_iter().collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    div().flex().flex_col().gap(gpui::rems(10./16.))
        .child(graph(editor.clone(), target(), params.to_vec(), "delay-taps", "延迟抽头：左右拖动时间，上下拖动电平", th, painter, handles, None, None).w_full().h(gpui::rems(225./16.)))
        .child(tabs)
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(12./16.))
            .child(toggle(ed, owner, slot, params, &format!("{prefix}_on"), "启用抽头", cx))
            .child(knob_row(editor, target, params, &refs, th, 44.)))
        .child(choice(ed, owner, slot, params, "sync", &dsp::DELAY_SYNC, cx))
        .child(div().flex().flex_wrap().items_center().gap(gpui::rems(12./16.))
            .child(toggle(ed, owner, slot, params, "pingpong", "乒乓", cx))
            .child(sidechain_picker(ed, owner, slot, cx)))
        .child(knob_row(editor, target, params, &["time", "offset", "feedback", "drive", "low_cut", "high_cut", "width", "mix"], th, 42.))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_presets_only_write_real_parameters_at_valid_values() {
        for &id in &dsp::EFFECT_IDS {
            let params = if id == dsp::EQ_ID { EqParams::default().descs() }
                else {
                    let specs = match id {
                        dsp::COMPRESSOR_ID => dsp::COMP_SPECS, dsp::GATE_ID => dsp::GATE_SPECS,
                        dsp::DEESSER_ID => dsp::DEESSER_SPECS, dsp::LIMITER_ID => dsp::LIMITER_SPECS,
                        dsp::REVERB_ID => dsp::REVERB_SPECS, dsp::DELAY_ID => dsp::DELAY_SPECS,
                        dsp::SATURATION_ID => dsp::SATURATION_SPECS, dsp::MULTIBAND_ID => dsp::MULTIBAND_SPECS,
                        dsp::FILTER_ID => dsp::FILTER_SPECS, dsp::SIMPLON_ID => dsp::SIMPLON_SPECS,
                        dsp::MICRO_ID => dsp::MICRO_SPECS, _ => panic!("missing factory preset contract: {id}"),
                    };
                    dsp::table_descs(specs, &dsp::table_defaults(specs))
                };
            for (name, values) in presets(id) {
                for &(key, value) in values {
                    let descriptor = param(&params, key).unwrap_or_else(|| panic!("{id} / {name} refers to missing parameter {key}"));
                    assert_eq!(verdant_plugins::insert::parameter_editor_value(descriptor, value).unwrap(), value, "{id} / {name} / {key}");
                }
            }
        }
    }
}
