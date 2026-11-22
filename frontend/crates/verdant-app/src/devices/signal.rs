//! Cached, measured insert audio. Refresh/FFT work belongs to the host polling
//! task; rendering only takes a nonblocking snapshot of its bounded cache.

use super::*;

fn dbfs(value: f32) -> f64 { 20.0 * f64::from(value.abs()).max(1e-6).log10() }

fn waveform_frames(total: u64, points: usize) -> usize {
    let stride = verdant_plugins::telemetry::WAVEFORM_STRIDE;
    let partial = (total % stride as u64) as usize;
    if partial == 0 { points * stride } else { points.saturating_sub(1) * stride + partial }
}

pub(super) fn panel(ed: &Editor, editor: &Entity<Editor>, owner: &DeviceTarget, slot: usize, key: InsertKey,
    cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let cached = if slot == crate::instrument::INSTRUMENT_SLOT {
        match owner { DeviceTarget::Track(t) => ed.instrument_telemetry(t), DeviceTarget::Master => None }
    } else { ed.live_device_insert(owner, slot).filter(|handle| handle.key() == key).and_then(|handle| handle.cached_telemetry()) };
    let Some(snapshot) = cached.filter(|snapshot| snapshot.frames_total > 0 && snapshot.window_frames > 0) else {
        return div().text_xs().text_color(th.text_secondary).child("信号分析 · 等待音频").into_any_element();
    };
    let waveform = ed.device_anim.get(&key).is_some_and(|state| state.signal_waveform);
    let stale = snapshot.captured_at.elapsed() > Duration::from_secs(1);
    let window_ms = snapshot.window_frames as f64 / snapshot.sample_rate.max(1.) * 1000.;
    let wave_frames = waveform_frames(snapshot.frames_total, snapshot.waveform_input.len());
    let wave_ms = wave_frames as f64 / snapshot.sample_rate.max(1.) * 1000.;
    let mut header = div().flex().flex_wrap().items_center().gap(gpui::rems(6./16.))
        .child(div().flex_1().text_xs().text_color(th.text_secondary).child(format!("{} · 电平 {:.0} ms · {} {:.0} ms · 音频累计 {:.2} s", if stale { "上次音频" } else { "实时信号" }, window_ms,
            if waveform { "波形" } else { "频谱" }, if waveform { wave_ms } else { window_ms }, snapshot.frames_total as f64 / snapshot.sample_rate.max(1.))));
    for (show_waveform, label) in [(false, "频谱"), (true, "波形")] {
        header = header.child(crate::ui_controls::selectable_button(format!("signal-mode-{slot}-{show_waveform}"), label, waveform == show_waveform, th)
            .tab_index(0)
            .on_click(cx.listener(move |ed, _, _, cx| {
                ed.device_anim.entry(key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now())).signal_waveform = show_waveform;
                cx.notify();
            }))
            .on_key_down(cx.listener(move |ed, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    ed.device_anim.entry(key).or_insert_with(|| DeviceAnim::settled(Vec::new(), Instant::now())).signal_waveform = show_waveform;
                    window.prevent_default(); cx.stop_propagation(); cx.notify();
                }
            })));
    }
    let reset_target = owner.clone();
    header = header.child(crate::ui_controls::selectable_button(format!("meters-reset-{slot}"), "重置计量", false, th)
        .tab_index(0).on_click(cx.listener(move |ed, _, _, cx| {
            if !ed.device_target_matches(&reset_target, slot, key) { return; }
            if slot == crate::instrument::INSTRUMENT_SLOT {
                if let DeviceTarget::Track(track) = &reset_target { ed.reset_instrument_meters(track, cx); }
                return;
            }
            let Some(handle) = ed.live_device_insert(&reset_target, slot).filter(|h| h.key() == key) else { return; };
            cx.spawn(async move |this, cx| {
                let result = cx.background_spawn(async move { handle.reset_meters() }).await;
                let _ = this.update(cx, |ed, cx| { if let Err(e) = result { ed.notice(format!("无法重置计量：{e}")); } cx.notify(); });
            }).detach();
        })));
    let reference = workflow::reference_handle(ed, owner, slot, key).and_then(|h| h.cached_telemetry());
    let reference_name = workflow::read_workspace(ed, &Target { track: owner.clone(), slot, key }).reference.map(|r| r.name);
    let has_reference = reference.is_some();
    let measured = snapshot.clone();
    let painter: Painter = Rc::new(move |window, plot, _, texts| {
        let input_color = with_alpha(th.text_secondary, if stale { 0.35 } else { 0.65 });
        let output_color = with_alpha(th.accent, if stale { 0.5 } else { 1. });
        if waveform {
            let axis = Axis::lin(-1., 1.);
            let xs = [0., 0.5, 1.].map(|n| (n, format!("{:.0} ms", (n - 1.) * wave_ms)));
            paint_grid(window, plot, th, &xs, &[(0.25, "−0.5".into()), (0.75, "+0.5".into())], Some(0.5), texts);
            for (points, color) in [(&measured.waveform_input, input_color), (&measured.waveform_output, output_color)] {
                for (index, point) in points.iter().enumerate() {
                    let stride = verdant_plugins::telemetry::WAVEFORM_STRIDE;
                    let start = index * stride;
                    let frames = (wave_frames - start).min(stride);
                    let x = plot.x(start as f64 / wave_frames.max(1) as f64);
                    let upper = plot.y(axis.norm(point.max as f64));
                    let lower = plot.y(axis.norm(point.min as f64));
                    window.paint_quad(fill(b(x, upper, (plot.inner_w() * frames as f64 / wave_frames.max(1) as f64).max(1.), (lower - upper).max(1.), plot.o), color));
                }
            }
        } else {
            let axis = Axis::lin(-96., 12.);
            let ys = [-72., -48., -24., 0.].map(|db| (axis.norm(db), format!("{db:.0} dBFS")));
            paint_grid(window, plot, th, &freq_grid(), &ys, None, texts);
            for (bins, color) in [(&measured.spectrum_input_db, input_color), (&measured.spectrum_output_db, output_color)] {
                let points = measured.spectrum_frequencies.iter().zip(bins).filter_map(|(&freq, &level)| {
                    let freq = freq as f64;
                    (freq >= FREQ_AXIS.min && freq <= FREQ_AXIS.max && level.is_finite())
                        .then(|| Some((plot.x(FREQ_AXIS.norm(freq)), plot.y(axis.norm(level as f64)))))
                });
                polyline(window, plot.o, points, 1.5, color);
            }
            if let Some(reference) = &reference {
                let points = reference.spectrum_frequencies.iter().zip(&reference.spectrum_output_db).filter_map(|(&freq, &db)| {
                    (freq >= FREQ_AXIS.min as f32 && freq <= FREQ_AXIS.max as f32).then(|| Some((plot.x(FREQ_AXIS.norm(freq as f64)), plot.y(axis.norm(db as f64)))))
                });
                polyline(window, plot.o, points, 1.5, with_alpha(band_color(2), if reference.captured_at.elapsed() > Duration::from_secs(1) { 0.4 } else { 0.9 }));
            }
        }
    });
    let gain_points = snapshot.gain_reduction_db.clone();
    let gain_painter: Painter = Rc::new(move |window, plot, _, texts| {
        let axis = Axis::lin(-60., 24.);
        let grid = [-48., -24., 0., 12.].map(|v| (axis.norm(v), format!("{v:+.0} dB")));
        paint_grid(window, plot, th, &[], &grid, Some(axis.norm(0.)), texts);
        let points = gain_points.iter().enumerate().map(|(i, value)| Some((plot.x(i as f64 / gain_points.len().saturating_sub(1).max(1) as f64), plot.y(axis.norm(*value as f64)))));
        polyline(window, plot.o, points, 2., th.accent);
        texts.push(("内部增益 · 最近处理块".into(), p(10., 5., plot.o), 10., th.text_secondary));
    });
    div().flex().flex_col().gap(gpui::rems(8./16.)).pb(gpui::rems(10./16.)).border_b_1().border_color(th.shade)
        .child(header)
        .when(has_reference, |d| d.child(div().text_xs().text_color(band_color(2)).child(format!("参考输出 · {}", reference_name.unwrap_or_default()))))
        .child(graph(editor.clone(), Target { track: owner.clone(), slot, key }, Vec::new(), "signal-analyzer",
            if waveform { "实际输入和输出波形" } else { "实际输入和输出频谱" }, th, painter, vec![], None, None).w_full().h(gpui::rems(285./16.)))
        .child(div().flex().flex_wrap().gap(gpui::rems(12./16.))
            .child(meters("输入", snapshot.input_peak, snapshot.input_rms, th.text_secondary, th))
            .child(meters("输出", snapshot.output_peak, snapshot.output_rms, th.accent, th)))
        .children(snapshot.loudness.as_ref().map(|reading| loudness(reading, th)))
        .when(!snapshot.gain_reduction_db.is_empty(), |d| d.child(graph(editor.clone(), Target { track: owner.clone(), slot, key }, Vec::new(),
            "gain-history", "内部检测器增益历史", th, gain_painter, vec![], None, None).w_full().h(gpui::rems(130./16.))))
        .into_any_element()
}

fn meter_number(value: f64) -> String {
    if value <= -119.9 { "−∞".into() } else { format!("{value:.1}") }
}
fn loudness(reading: &dsp::quality::LoudnessReading, th: Theme) -> gpui::AnyElement {
    div().flex().flex_wrap().gap(gpui::rems(12./16.)).children([
        ("M · 400 ms", format!("{} LUFS", meter_number(reading.momentary_lufs))),
        ("S · 3 s", format!("{} LUFS", meter_number(reading.short_term_lufs))),
        ("综合 I", format!("{} LUFS", meter_number(reading.integrated_lufs))),
        ("响度范围 LRA", format!("{:.1} LU", reading.loudness_range_lu)),
        ("最大 M / S", format!("{} / {} LUFS", meter_number(reading.maximum_momentary_lufs), meter_number(reading.maximum_short_term_lufs))),
        ("True Peak · L / R", format!("{} / {} dBTP", meter_number(reading.true_peak_dbtp[0]), meter_number(reading.true_peak_dbtp[1]))),
        ("最大 TP · L / R", format!("{} / {} dBTP", meter_number(reading.maximum_true_peak_dbtp[0]), meter_number(reading.maximum_true_peak_dbtp[1]))),
    ].into_iter().map(|(name, value)| div().flex().flex_col().gap(gpui::rems(2./16.))
        .child(div().text_xs().text_color(th.text_secondary).child(name))
        .child(div().text_sm().child(value)))).into_any_element()
}

fn meters(label: &str, peaks: [f32; 2], rms: [f32; 2], color: gpui::Rgba, th: Theme) -> gpui::AnyElement {
    div().flex_1().min_w(gpui::rems(220./16.)).flex().flex_col().gap(gpui::rems(3./16.))
        .child(div().text_xs().text_color(color).child(format!("{label} · 峰值 / RMS")))
        .children((0..2).map(|channel| {
            let peak = dbfs(peaks[channel]);
            let average = dbfs(rms[channel]);
            let clipping = peaks[channel] >= 1.;
            div().flex().items_center().gap(gpui::rems(6./16.))
                .child(div().text_xs().text_color(th.text_secondary).child(if channel == 0 { "L" } else { "R" }))
                .child(canvas(|_, _, _| (), move |bounds, _, window, _| {
                    let width = f64::from(bounds.size.width);
                    let height = f64::from(bounds.size.height);
                    window.paint_quad(fill(bounds, th.control));
                    window.paint_quad(fill(b(0., 0., width * ((average + 60.) / 60.).clamp(0., 1.), height, bounds.origin), with_alpha(color, 0.5)));
                    let marker = width * ((peak + 60.) / 60.).clamp(0., 1.);
                    window.paint_quad(fill(b((marker - 2.).max(0.), 0., 2., height, bounds.origin), if clipping { th.error } else { color }));
                }).flex_1().h(gpui::rems(6./16.)))
                .child(div().w(gpui::rems(118./16.)).text_xs().text_color(if clipping { th.error } else { th.text_secondary })
                    .child(format!("{peak:.1} / {average:.1} dBFS")))
        })).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meter_levels_keep_silence_finite_and_do_not_hide_clipping() {
        assert_eq!(dbfs(0.), -120.);
        assert!((dbfs(0.5) + 6.0206).abs() < 0.0001);
        assert!((dbfs(2.) - 6.0206).abs() < 0.0001);
    }
    #[test]
    fn waveform_time_scale_includes_the_partial_bucket_and_older_history() {
        assert_eq!(waveform_frames(0, 0), 0);
        assert_eq!(waveform_frames(17, 1), 17);
        assert_eq!(waveform_frames(2048, 32), 2048);
        assert_eq!(waveform_frames(8192, 128), 8192);
        assert_eq!(waveform_frames(8193, 128), 8129);
    }
}

/// The GUI only reads immutable host snapshots; no analysis work is scheduled
/// by the drawing code and empty streams never acquire a fabricated trace.
pub(super) fn snapshot(ed: &Editor, owner: &DeviceTarget, slot: usize, key: InsertKey) -> Option<std::sync::Arc<verdant_plugins::telemetry::TelemetrySnapshot>> {
    let cached = if slot == crate::instrument::INSTRUMENT_SLOT {
        match owner { DeviceTarget::Track(t) => ed.instrument_telemetry(t), DeviceTarget::Master => None }
    } else { ed.live_device_insert(owner, slot).filter(|h| h.key() == key).and_then(|h| h.cached_telemetry()) };
    cached.filter(|s| s.frames_total > 0 && s.window_frames > 0)
}

pub(super) fn paint_spectrum(window: &mut Window, plot: Plot, measured: Option<&verdant_plugins::telemetry::TelemetrySnapshot>, th: Theme, texts: &mut Texts) {
    let Some(measured) = measured else {
        texts.push(("频谱 · 等待音频".into(), p(12., 9., plot.o), 11., th.text_secondary));
        return;
    };
    let stale = measured.captured_at.elapsed() > Duration::from_secs(1);
    let axis = Axis::lin(-96., 0.);
    for (bins, color, filled) in [
        (&measured.spectrum_input_db, with_alpha(th.text_secondary, if stale { 0.22 } else { 0.38 }), false),
        (&measured.spectrum_output_db, with_alpha(th.accent, if stale { 0.22 } else { 0.4 }), true),
    ] {
        let points: Vec<_> = measured.spectrum_frequencies.iter().zip(bins).filter_map(|(&freq, &level)| {
            (freq >= FREQ_AXIS.min as f32 && freq <= FREQ_AXIS.max as f32 && level.is_finite())
                .then(|| (plot.x(FREQ_AXIS.norm(freq as f64)), plot.y(axis.norm(level as f64))))
        }).collect();
        if filled { fill_under(window, plot, &points, plot.y(0.), with_alpha(th.accent, if stale { 0.035 } else { 0.07 })); }
        polyline(window, plot.o, points.into_iter().map(Some), 1., color);
    }
    texts.push((format!("{} · 频谱 −96…0 dBFS", if stale { "上次音频" } else { "输入 / 输出" }), p(12., 9., plot.o), 11., th.text_secondary));
}

pub(super) fn compact(ed: &Editor, owner: &DeviceTarget, slot: usize, key: InsertKey) -> gpui::AnyElement {
    let th = ed.theme;
    let Some(s) = snapshot(ed, owner, slot, key) else {
        return div().min_h(gpui::rems(26./16.)).flex().items_center().text_xs().text_color(th.text_secondary)
            .child("等待音频 · 输入与输出计量在播放后显示").into_any_element();
    };
    let stale = s.captured_at.elapsed() > Duration::from_secs(1);
    let label = |name: &str, peaks: [f32; 2], color| div().flex().items_center().gap(gpui::rems(6./16.))
        .child(div().text_xs().text_color(th.text_secondary).child(name.to_string()))
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(color)
            .child(format!("{} / {} dBFS", meter_number(dbfs(peaks[0])), meter_number(dbfs(peaks[1])))));
    let mut row = div().flex().flex_wrap().items_center().gap(gpui::rems(16./16.)).min_h(gpui::rems(26./16.))
        .child(label("IN L/R", s.input_peak, th.text_secondary))
        .child(label("OUT L/R", s.output_peak, if s.output_peak.iter().any(|p| *p >= 1.) { th.error } else { th.accent }));
    if let Some(gain) = s.gain_reduction_db.last() {
        row = row.child(div().text_xs().text_color(th.text_secondary).child(format!("检测器增益 {gain:+.1} dB")));
    }
    row.child(div().flex_1()).child(div().text_xs().text_color(th.text_tertiary)
        .child(if stale { "上次音频" } else { "实时" })).into_any_element()
}

/// Actual recent waveform peaks on a dBFS time axis, separated from the static
/// transfer graph and from the detector-block history on the meters page.
pub(super) fn history(ed: &Editor, editor: &Entity<Editor>, target: Target, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let _ = cx;
    let measured = snapshot(ed, &target.track, target.slot, target.key);
    let th = ed.theme;
    let painter: Painter = Rc::new(move |window, plot, _, texts| {
        let axis = Axis::lin(-72., 6.);
        let ms = measured.as_ref().map_or(0., |s| waveform_frames(s.frames_total, s.waveform_input.len()) as f64 / s.sample_rate.max(1.) * 1000.);
        let xs = [0., 0.5, 1.].map(|x| (x, format!("{:.0} ms", (x - 1.) * ms)));
        let ys = [-60., -36., -12., 0.].map(|x| (axis.norm(x), if x == 0. { String::new() } else { format!("{x:.0} dBFS") }));
        paint_grid(window, plot, th, &xs, &ys, None, texts);
        let Some(s) = measured.as_ref() else {
            texts.push(("实际电平历史 · 等待音频".into(), p(12., 9., plot.o), 11., th.text_secondary)); return;
        };
        let stale = s.captured_at.elapsed() > Duration::from_secs(1);
        for (samples, color, fill) in [
            (&s.waveform_input, with_alpha(th.text_secondary, if stale {0.25} else {0.45}), false),
            (&s.waveform_output, with_alpha(th.accent, if stale {0.5} else {0.9}), true),
        ] {
            let points: Vec<_> = samples.iter().enumerate().map(|(index, point)| {
                (plot.x(index as f64 / samples.len().saturating_sub(1).max(1) as f64), plot.y(axis.norm(dbfs(point.min.abs().max(point.max.abs())))))
            }).collect();
            if fill { fill_under(window, plot, &points, plot.y(0.), with_alpha(th.accent, 0.1)); }
            polyline(window, plot.o, points.into_iter().map(Some), if fill {1.75} else {1.}, color);
        }
        texts.push((format!("{} · 输入 / 输出波形峰值", if stale { "上次音频" } else { "实时" }), p(12., 9., plot.o), 11., th.text_secondary));
    });
    graph(editor.clone(), target, Vec::new(), "dynamics-history", "实际输入与输出电平历史", th, painter, vec![], None, None)
        .w_full().min_w(gpui::rems(240./16.)).h(gpui::rems(260./16.)).into_any_element()
}
