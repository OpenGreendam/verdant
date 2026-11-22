//! Shared typed modulation routing. Native parameter indices remain the saved ABI.
use super::*;
use gpui::Render;

#[derive(Clone)]
pub(super) struct SourceDrag { key: InsertKey, generation: u64, source: usize }
struct DragLabel { label: String, theme: Theme }
impl Render for DragLabel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().px(gpui::rems(12./16.)).py(gpui::rems(8./16.)).rounded(gpui::rems(8./16.)).bg(self.theme.control_pressed).text_color(self.theme.text).child(self.label.clone())
    }
}

pub(super) fn docked_sources(ed: &Editor, target: &Target) -> gpui::AnyElement {
    if !workflow::read_workspace(ed, target).matrix_open { return div().into_any_element(); }
    let th = ed.theme; let key = target.key; let generation = ed.document_generation;
    div().id("native-modulation-sources").role(Role::Group).aria_label("拖动调制来源到参数")
        .w_full().min_w_0().h(px(42.)).flex_none().overflow_x_scroll()
        .bg(th.headerbar).border_b_1().border_color(th.shade)
        .child(div().h_full().flex().items_center().gap(px(6.)).px(px(16.)).children(
            dsp::MOD_SOURCE_LABELS.iter().enumerate().skip(1).map(move |(source, label)| {
                div().id(SharedString::from(format!("docked-source-{source}"))).role(Role::Button)
                    .aria_label(format!("拖动 {label} 到参数旋钮")).flex_none().cursor_pointer()
                    .px(px(10.)).py(px(6.)).rounded(px(6.)).bg(th.control).text_size(px(12.))
                    .child((*label).to_string())
                    .on_drag(SourceDrag { key, generation, source }, move |_, _, _, cx| {
                        cx.new(|_| DragLabel { label: (*label).into(), theme: th })
                    })
            }))).into_any_element()
}

fn available_targets<'a>(effect: &str, params: &'a [ParamDesc]) -> Vec<(usize, &'a ParamDesc)> {
    let instrument = matches!(effect, dsp::SYNTH_ID | dsp::ONE_SYNTH_ID).then(|| dsp::instrument_specs(effect));
    params.iter().enumerate().filter(|(_, d)| {
        dsp::modulation_parameter(&d.id).is_none() && d.max > d.min &&
        instrument.as_ref().and_then(|specs| specs.iter().find(|s| s.id == d.id)).copied()
            .or_else(|| dsp::spec_for(effect, &d.id)).is_some_and(|s| s.automatable)
    }).collect()
}
fn effect_id<'a>(ed: &'a Editor, target: &Target) -> Option<String> {
    if target.slot == crate::instrument::INSTRUMENT_SLOT {
        let DeviceTarget::Track(t) = &target.track else { return None };
        ed.project().track(t)?.instrument()?.get("id")?.as_str().map(str::to_string)
    } else { ed.device_insert_specs(&target.track).get(target.slot).map(|s| s.id.clone()) }
}
pub(super) fn drop_source(ed: &mut Editor, target: &Target, id: &str, source: &SourceDrag, cx: &mut Context<Editor>) {
    if source.key != target.key || source.generation != ed.document_generation { return; }
    let Some(params) = current_params(ed, target) else { return };
    let Some(effect) = effect_id(ed, target) else { return };
    let Some(index) = available_targets(&effect, &params).into_iter().find(|(_, d)| d.id == id).map(|(i, _)| i) else { return };
    let slot = (1..=8).find(|slot| param(&params, &format!("mod{slot}_source")).is_some_and(|d| d.value == 0.));
    let Some(slot) = slot else { ed.notice("8 个调制连线已用完，请先关闭一条连线"); cx.notify(); return };
    write_if_current(ed, target, vec![(format!("mod{slot}_source"), source.source as f64), (format!("mod{slot}_target"), index as f64),
        (format!("mod{slot}_amount"), 25.)], cx);
}

fn target_picker(ed: &Editor, target: &Target, effect: &str, params: &[ParamDesc], id: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let choices = available_targets(effect, params);
    let selected = param(params, id).map_or(0, |d| d.value.round() as usize);
    let label = choices.iter().find(|(i, _)| *i == selected).map_or("选择目标".into(), |(_, d)| d.name.clone());
    let choices: Vec<_> = choices.into_iter().map(|(i, d)| (i, d.name.clone())).collect();
    let target = target.clone(); let id = id.to_string();
    dropdown_button(format!("mod-target-{}-{id}", target.slot).into(), label, ed.theme, move |_| {
        use crate::editor::MenuAction as M; use crate::menu::MenuItem as I;
        choices.iter().map(|(i, name)| I::action(name.clone(), M::SetDeviceParams { target: target.track.clone(), slot: target.slot,
            expected_key: Some(target.key), values: vec![(id.clone(), *i as f64)] }).checked(*i == selected)).collect()
    }, cx).into_any_element()
}

fn source_picker(ed: &Editor, target: &Target, params: &[ParamDesc], id: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let selected = param(params, id).map_or(0, |p| p.value.round() as usize);
    let label = dsp::MOD_SOURCE_LABELS.get(selected).unwrap_or(&"关闭").to_string();
    let target = target.clone(); let id = id.to_string();
    dropdown_button(format!("mod-source-choice-{}-{id}", target.slot).into(), label, ed.theme, move |_| {
        use crate::editor::MenuAction as M; use crate::menu::MenuItem as I;
        dsp::MOD_SOURCE_LABELS.iter().enumerate().map(|(i, label)| I::action(*label, M::SetDeviceParams {
            target: target.track.clone(), slot: target.slot, expected_key: Some(target.key), values: vec![(id.clone(), i as f64)] }).checked(i == selected)).collect()
    }, cx).into_any_element()
}

fn xy_pad(editor: &Entity<Editor>, target: &Target, params: &[ParamDesc], th: Theme) -> gpui::AnyElement {
    let axis = Axis::lin(-100., 100.);
    let x = param(params, "mxy_x").map_or(0., |d| d.value);
    let y = param(params, "mxy_y").map_or(0., |d| d.value);
    let handle = Handle { control: Control::Handle("modulation-xy"),
        at: (axis.norm(x), axis.norm(y)), x: Some(("mxy_x".into(), axis)),
        y: Some(("mxy_y".into(), axis)), color: th.accent, label: "XY".into(),
        hollow: false, selected: true, also: Vec::new(),
        defaults: vec![("mxy_x".into(), 0.), ("mxy_y".into(), 0.)] };
    let painter: Painter = Rc::new(move |window, plot, _, texts| {
        let ticks = [-100., -50., 0., 50., 100.].map(|v| (axis.norm(v), format!("{v:+.0}%")));
        paint_grid(window, plot, th, &ticks, &ticks, Some(0.5), texts);
        window.paint_quad(fill(b(plot.x(axis.norm(x)), 0., 1., plot.h, plot.o), with_alpha(th.accent, 0.4)));
        window.paint_quad(fill(b(0., plot.y(axis.norm(y)), plot.w, 1., plot.o), with_alpha(th.accent, 0.4)));
    });
    let set: DoubleClick = Rc::new(move |(x, y)| Some((vec![("mxy_x".into(), axis.value(x)),
        ("mxy_y".into(), axis.value(y))], None)));
    div().w_full().max_w(gpui::rems(420./16.)).flex().flex_col().gap(gpui::rems(6./16.))
        .child(graph(editor.clone(), target.clone(), params.to_vec(), "modulation-xy",
            "XY 调制控制：拖动圆点同时调整 X 与 Y，双击位置设定坐标", th, painter, vec![handle], None, Some(set))
            .w_full().h(gpui::rems(200./16.)))
        .child(div().text_xs().text_color(th.text_secondary).child("拖动 XY 圆点 · Shift 微调 · 双击设定位置 · Alt 点击复位"))
        .into_any_element()
}

fn advanced_sources(ed: &Editor, editor: &Entity<Editor>, target: &Target, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme; let owner = &target.track; let slot = target.slot;
    let state = workflow::read_workspace(ed, target); let toggle_target = target.clone();
    let mut section = div().flex().flex_col().gap(gpui::rems(10./16.))
        .child(crate::ui_controls::selectable_button(format!("mod-advanced-{slot}"), "高级来源与 XLFO 步进编辑", state.modulation_advanced_open, th)
            .tab_index(0).on_click(cx.listener(move |ed, _, _, cx| {
                let mut state = workflow::read_workspace(ed, &toggle_target);
                state.modulation_advanced_open = !state.modulation_advanced_open;
                workflow::write_workspace(ed, &toggle_target, &state); cx.notify();
            })));
    if !state.modulation_advanced_open { return section.into_any_element(); }
    let retriggers = ["自由运行", "每次按键", "连奏起点"];
    for index in 1..=2 {
        let trigger = format!("mlfo{index}_retrigger");
        let knobs: Vec<_> = ["phase", "balance", "offset", "level"].map(|suffix| format!("mlfo{index}_{suffix}")).into();
        section = section.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("LFO {index} · 相位与触发")))
            .child(choice(ed, owner, slot, params, &trigger, &retriggers, cx))
            .child(knob_row(editor, || target.clone(), params, &knobs.iter().map(String::as_str).collect::<Vec<_>>(), th, 32.));
    }
    section = section.child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("触发包络 · 来源与范围"))
        .child(choice(ed, owner, slot, params, "menv_trigger", &["MIDI", "音频", "MIDI 与音频"], cx))
        .child(choice(ed, owner, slot, params, "menv_range", &["单极", "延音为零"], cx))
        .child(knob_row(editor, || target.clone(), params, &["menv_threshold", "menv_hysteresis", "menv_level"], th, 32.))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("包络跟随 · 检测"))
        .child(choice(ed, owner, slot, params, "mfollower_mode", &["电平", "瞬态"], cx))
        .child(knob_row(editor, || target.clone(), params, &["mfollower_gain", "mfollower_level"], th, 32.))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("MIDI CC 来源 · 默认调制轮（CC 1）"))
        .child(knob_row(editor, || target.clone(), params, &["mmidi_cc", "mmidi_channel"], th, 32.))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("XLFO · 序列长度与时间"))
        .child(choice(ed, owner, slot, params, "mseq_sync",
            &["自由", "1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 小节", "2 小节", "4 小节", "8 小节", "16 小节", "1/8 三连音", "1/8 附点"], cx))
        .child(choice(ed, owner, slot, params, "mseq_retrigger", &retriggers, cx))
        .child(toggle(ed, owner, slot, params, "mseq_snap", "半音吸附", cx))
        .child(knob_row(editor, || target.clone(), params,
            &["mseq_length", "mseq_rate", "mseq_offset", "mseq_balance", "mseq_glide", "mseq_phase", "mseq_level"], th, 32.));
    let length = param(params, "mseq_length").map_or(16, |d| d.value.round() as usize).clamp(1, 16);
    let selected = state.sequencer_step.min(length-1);
    let tabs = div().flex().flex_wrap().gap(gpui::rems(4./16.)).children((0..length).map(|step| {
        let target = target.clone();
        crate::ui_controls::selectable_button(format!("seq-detail-{slot}-{step}"), (step+1).to_string(), step == selected, th)
            .tab_index(0).on_click(cx.listener(move |ed, _, _, cx| {
                let mut state = workflow::read_workspace(ed, &target); state.sequencer_step = step;
                workflow::write_workspace(ed, &target, &state); cx.notify();
            }))
    }));
    let step = format!("mseq_step{}", selected+1); let glide = format!("mseq_glide{}", selected+1);
    let curve = format!("mseq_curve{}", selected+1); let random = format!("mseq_random{}", selected+1);
    section.child(div().text_sm().child("选中一步，调整该步的数值、滑移、曲线与随机保持"))
        .child(tabs)
        .child(choice(ed, owner, slot, params, &curve, &["线性", "平方", "平方根", "正弦"], cx))
        .child(toggle(ed, owner, slot, params, &random, "该步使用随机值", cx))
        .child(knob_row(editor, || target.clone(), params, &[&step, &glide], th, 36.))
        .into_any_element()
}

pub(super) fn panel(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    effect: &str, params: &[ParamDesc], cx: &mut Context<Editor>) -> gpui::AnyElement {
    if param(params, "mod1_source").is_none() { return div().into_any_element(); }
    let th = ed.theme; let target = Target { track: owner.clone(), slot, key };
    let state = workflow::read_workspace(ed, &target);
    let toggle_target = target.clone();
    let used = (1..=8).filter(|n| param(params, &format!("mod{n}_source")).is_some_and(|d| d.value > 0.)).count();
    let mut panel = div().flex().flex_col().gap(gpui::rems(12./16.)).pt(gpui::rems(12./16.)).border_t_1().border_color(th.shade)
        .child(crate::ui_controls::selectable_button(format!("mod-open-{slot}"), format!("调制与 MIDI · {used} / 8 连线"), state.matrix_open, th)
            .tab_index(0).on_click(cx.listener(move |ed, _, _, cx| {
                let mut state = workflow::read_workspace(ed, &toggle_target); state.matrix_open = !state.matrix_open;
                workflow::write_workspace(ed, &toggle_target, &state); cx.notify();
            })));
    if !state.matrix_open { return panel.into_any_element(); }
    let sources = div().flex().flex_wrap().gap(gpui::rems(6./16.)).children(dsp::MOD_SOURCE_LABELS.iter().enumerate().skip(1).map(|(source, label)| {
        div().id(SharedString::from(format!("mod-source-{slot}-{source}"))).role(Role::Button).aria_label(format!("拖动 {label} 到参数旋钮"))
            .cursor_pointer().px(gpui::rems(10./16.)).py(gpui::rems(7./16.)).rounded(gpui::rems(6./16.)).bg(th.control).text_sm().child((*label).to_string())
            .on_drag(SourceDrag { key, generation: ed.document_generation, source }, move |_, _, _, cx| {
                let label = (*label).to_string(); cx.new(|_| DragLabel { label, theme: th })
            })
    }));
    panel = panel.child(div().text_xs().text_color(th.text_secondary).child("拖动来源到参数旋钮，或使用下方来源与目标选择器。深度按参数全范围的百分比缩放。"))
        .child(sources);
    for index in 1..=8 {
        let source = format!("mod{index}_source");
        let target_id = format!("mod{index}_target");
        let amount = format!("mod{index}_amount"); let curve = format!("mod{index}_curve");
        panel = panel.child(div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.)).p(gpui::rems(8./16.)).rounded(gpui::rems(8./16.)).bg(th.card)
            .child(div().text_xs().child(format!("{index}")))
            .child(source_picker(ed, &target, params, &source, cx))
            .child(target_picker(ed, &target, effect, params, &target_id, cx))
            .child(knob_row(editor, || target.clone(), params, &[&amount, &curve], th, 32.)));
    }
    for lfo in 1..=2 {
        let rate = format!("mlfo{lfo}_rate"); let shape = format!("mlfo{lfo}_shape"); let sync = format!("mlfo{lfo}_sync");
        panel = panel.child(div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.)).child(div().text_sm().child(format!("LFO {lfo}")))
            .child(choice(ed, owner, slot, params, &shape, &["正弦", "三角", "上锯齿", "下锯齿", "方波", "随机保持"], cx))
            .child(choice(ed, owner, slot, params, &sync, &["自由", "1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 小节", "2 小节"], cx))
            .child(knob_row(editor, || target.clone(), params, &[&rate], th, 36.)));
    }
    panel = panel.child(div().text_sm().child("触发包络与包络跟随"))
        .child(knob_row(editor, || target.clone(), params, &["menv_attack", "menv_decay", "menv_sustain", "menv_release", "mfollower_attack", "mfollower_release"], th, 36.))
        .child(div().text_sm().child("XY 与 16 步进"))
        .child(xy_pad(editor, &target, params, th))
        .child(knob_row(editor, || target.clone(), params, &["mxy_x", "mxy_y", "mseq_rate"], th, 36.));
    let steps: Vec<String> = (1..=16).map(|i| format!("mseq_step{i}")).collect();
    let refs: Vec<&str> = steps.iter().map(String::as_str).collect();
    panel = panel.child(knob_row(editor, || target.clone(), params, &refs, th, 28.))
        .child(advanced_sources(ed, editor, &target, params, cx))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("MIDI Learn"))
        .child(div().text_xs().text_color(th.text_secondary).child("先选择目标，再点击“学习”并转动外接控制器。CC = −1 为关闭，通道 0 为全部通道。"));
    for index in 1..=8 {
        let cc_id = format!("midi{index}_cc"); let channel_id = format!("midi{index}_channel");
        let target_id = format!("midi{index}_target"); let learn_target = target.clone();
        panel = panel.child(div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.))
            .child(div().text_xs().child(format!("{index}")))
            .child(target_picker(ed, &target, effect, params, &target_id, cx))
            .child(knob_row(editor, || target.clone(), params, &[&cc_id, &channel_id], th, 30.))
            .child(crate::ui_controls::selectable_button(format!("learn-{slot}-{index}"), "学习", false, th).tab_index(0)
                .on_click(cx.listener(move |ed, _, _, cx| {
                    ed.begin_device_midi_learn(learn_target.track.clone(), learn_target.slot, learn_target.key, index - 1, cx);
                }))));
    }
    panel.into_any_element()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn modulation_targets_exclude_metadata_and_latency_controls() {
        let mut params = dsp::table_descs(dsp::COMP_SPECS, &dsp::table_defaults(dsp::COMP_SPECS));
        params.extend(dsp::modulation::ModParams::default().descs(params.len()));
        let targets = available_targets(dsp::COMPRESSOR_ID, &params);
        assert!(targets.iter().any(|(_, d)| d.id == "threshold"));
        assert!(!targets.iter().any(|(_, d)| d.id == "lookahead" || d.id.starts_with("mod") || d.id.starts_with("midi")));
    }
}
