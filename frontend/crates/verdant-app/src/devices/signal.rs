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
    let Some(snapshot) = ed.live_device_insert(owner, slot).filter(|handle| handle.key() == key).and_then(|handle| handle.cached_telemetry())
        .filter(|snapshot| snapshot.frames_total > 0 && snapshot.window_frames > 0) else {
        return div().text_xs().text_color(th.text_secondary).child("信号分析 · 等待音频").into_any_element();
    };
    let waveform = ed.device_anim.get(&key).is_some_and(|state| state.signal_waveform);
    let stale = snapshot.captured_at.elapsed() > Duration::from_secs(1);
    let window_ms = snapshot.window_frames as f64 / snapshot.sample_rate.max(1.) * 1000.;
    let wave_frames = waveform_frames(snapshot.frames_total, snapshot.waveform_input.len());
    let wave_ms = wave_frames as f64 / snapshot.sample_rate.max(1.) * 1000.;
    let mut header = div().flex().flex_wrap().items_center().gap(px(6.))
        .child(div().flex_1().text_xs().text_color(th.text_secondary).child(format!("{} · 电平 {:.0} ms · {} {:.0} ms", if stale { "上次音频" } else { "实时信号" }, window_ms,
            if waveform { "波形" } else { "频谱" }, if waveform { wave_ms } else { window_ms })));
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
        }
    });
    div().flex().flex_col().gap(px(8.)).pb(px(10.)).border_b_1().border_color(th.shade)
        .child(header)
        .child(graph(editor.clone(), Target { track: owner.clone(), slot, key }, Vec::new(), "signal-analyzer",
            if waveform { "实际输入和输出波形" } else { "实际输入和输出频谱" }, th, painter, vec![], None, None).w_full().h(px(130.)))
        .child(div().flex().flex_wrap().gap(px(12.))
            .child(meters("输入", snapshot.input_peak, snapshot.input_rms, th.text_secondary, th))
            .child(meters("输出", snapshot.output_peak, snapshot.output_rms, th.accent, th)))
        .into_any_element()
}

fn meters(label: &str, peaks: [f32; 2], rms: [f32; 2], color: gpui::Rgba, th: Theme) -> gpui::AnyElement {
    div().flex_1().min_w(px(220.)).flex().flex_col().gap(px(3.))
        .child(div().text_xs().text_color(color).child(format!("{label} · 峰值 / RMS")))
        .children((0..2).map(|channel| {
            let peak = dbfs(peaks[channel]);
            let average = dbfs(rms[channel]);
            let clipping = peaks[channel] >= 1.;
            div().flex().items_center().gap(px(6.))
                .child(div().text_xs().text_color(th.text_secondary).child(if channel == 0 { "L" } else { "R" }))
                .child(canvas(|_, _, _| (), move |bounds, _, window, _| {
                    let width = f64::from(bounds.size.width);
                    let height = f64::from(bounds.size.height);
                    window.paint_quad(fill(bounds, th.control));
                    window.paint_quad(fill(b(0., 0., width * ((average + 60.) / 60.).clamp(0., 1.), height, bounds.origin), with_alpha(color, 0.5)));
                    let marker = width * ((peak + 60.) / 60.).clamp(0., 1.);
                    window.paint_quad(fill(b((marker - 2.).max(0.), 0., 2., height, bounds.origin), if clipping { th.error } else { color }));
                }).flex_1().h(px(6.)))
                .child(div().w(px(118.)).text_xs().text_color(if clipping { th.error } else { th.text_secondary })
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
