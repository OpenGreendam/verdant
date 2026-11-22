//! Spectrum capture, cross-instance matching and editable EQ sketches.
use super::*;
use gpui::{App, AppContext, Bounds, Render, WeakEntity, WindowBounds, WindowOptions, size};

pub(super) fn tools(ed: &Editor, editor: &Entity<Editor>, target: Target, effect: &str, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let th = ed.theme;
    let mut row = div().flex().flex_wrap().items_center().gap(gpui::rems(8./16.));
    if effect == dsp::EQ_ID {
        let sketch_editor = editor.clone(); let sketch_target = target.clone();
        row = row.child(crate::ui_controls::selectable_button(format!("eq-sketch-{}", target.slot), "绘制 EQ", false, th)
            .on_click(move |_, _, cx| open_sketch(sketch_editor.clone(), sketch_target.clone(), cx)));
        let match_editor = editor.clone(); let match_target = target.clone();
        row = row.child(crate::ui_controls::selectable_button(format!("eq-match-{}", target.slot), "参考频谱与匹配", false, th)
            .on_click(move |_, _, cx| open_references(match_editor.clone(), match_target.clone(), cx)));
        let grab_target = target.clone();
        row = row.child(crate::ui_controls::selectable_button(format!("eq-grab-{}", target.slot), "抓取频谱峰值", false, th)
            .on_click(cx.listener(move |ed, _, _, cx| grab_peak(ed, &grab_target, cx))));
    } else if effect == dsp::REVERB_ID {
        row = row.child(crate::ui_controls::selectable_button(format!("reverb-ir-{}", target.slot), "从 IR 拟合", false, th)
            .on_click(cx.listener(move |ed, _, _, cx| import_ir(ed, &target, cx))));
    }
    row.into_any_element()
}

fn grab_peak(ed: &mut Editor, target: &Target, cx: &mut Context<Editor>) {
    let Some(params) = current_params(ed, target) else { return };
    let Some(s) = ed.live_device_insert(&target.track, target.slot).and_then(|h| h.cached_telemetry()) else { ed.notice("请先播放音频以捕获频谱"); cx.notify(); return };
    let Some((freq, level)) = s.spectrum_frequencies.iter().zip(&s.spectrum_input_db)
        .filter(|(f, db)| **f >= 30. && **f <= 18000. && **db > -90.)
        .max_by(|a,b| a.1.total_cmp(b.1)) else { ed.notice("当前测量窗口没有足够的输入信号"); cx.notify(); return };
    let eq = EqParams::from_descs(&params);
    let Some(band) = eq.bands.iter().position(|b| !b.on) else { ed.notice("24 个频段已全部启用"); cx.notify(); return };
    let values = vec![(dsp::eq_param_id(band, "on"), 1.), (dsp::eq_param_id(band, "shape"), BandKind::Bell.index() as f64),
        (dsp::eq_param_id(band, "freq"), *freq as f64), (dsp::eq_param_id(band, "gain"), 0.), (dsp::eq_param_id(band, "q"), 2.)];
    let level = *level;
    write_if_current(ed, target, values, cx); select_band(ed, target.key, band);
    ed.notice(format!("已在频谱峰值处建立频段（输入 {level:.1} dBFS）")); cx.notify();
}
fn import_ir(ed: &mut Editor, target: &Target, cx: &mut Context<Editor>) {
    let target = target.clone(); let generation = ed.document_generation;
    let rx = cx.prompt_for_paths(gpui::PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("拟合 IR".into()) });
    cx.spawn(async move |this, cx| {
        let paths = match rx.await {
            Ok(Ok(Some(paths))) => paths,
            Ok(Ok(None)) => return,
            result => {
                let message = match result {
                    Ok(Err(error)) => format!("无法打开 IR 文件选择器：{error}"),
                    Err(error) => format!("IR 文件选择器已关闭：{error}"),
                    _ => unreachable!(),
                };
                let _ = this.update(cx, |ed, cx| { ed.notice(message); cx.notify(); }); return;
            }
        };
        {
            let Some(path) = paths.into_iter().next() else { return };
            let result = cx.background_spawn(async move {
                if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 { return Err("IR 文件超过 64 MiB".into()); }
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let limits = verdant_media::ImportLimits { max_source_bytes: 64 * 1024 * 1024, max_decoded_frames: 384_000 * 60, max_pcm_bytes: 128 * 1024 * 1024, ..Default::default() };
                let audio = verdant_media::decode_bytes(&bytes, path.extension().and_then(|e| e.to_str()), limits).map_err(|e| e.to_string())?;
                // Average energies with a preserved sign to avoid cancelling
                // anti-phase stereo IRs during decay estimation.
                let mono: Vec<f32> = audio.samples.chunks(audio.channels as usize).map(|frame| {
                    if frame.len() == 1 { frame[0] } else { ((frame[0]*frame[0]+frame[1]*frame[1])*0.5).sqrt().copysign(if frame[0].abs() >= frame[1].abs() { frame[0] } else { frame[1] }) }
                }).collect();
                dsp::fit_reverb_ir(&mono, f64::from(audio.sample_rate))
            }).await;
            let _ = this.update(cx, |ed, cx| {
                if ed.document_generation != generation || !ed.device_target_matches(&target.track, target.slot, target.key) { return; }
                match result {
                    Ok(values) => {
                        if ed.set_device_params(target.track.clone(), target.slot, values, cx) { ed.notice("已从 IR 拟合衰减与预延迟，可在历史中撤销"); }
                        else { ed.notice("请等待插件载入或当前修改完成后重试 IR 拟合"); }
                    },
                    Err(error) => ed.notice(format!("IR 拟合失败：{error}")),
                }
                cx.notify();
            });
        }
    }).detach();
}

fn open_sketch(editor: Entity<Editor>, target: Target, cx: &mut App) {
    if current_params(editor.read(cx), &target).is_none() { return; }
    let th = editor.read(cx).theme; let generation = editor.read(cx).document_generation;
    let weak = editor.downgrade();
    let bounds = Bounds::centered(None, size(px(800.), px(440.)), cx);
    let _ = cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), app_id: Some("verdant".into()), ..Default::default() }, move |window, cx| {
        window.set_window_title("绘制 EQ — Verdant");
        cx.new(|_| Sketch { editor: weak, target, generation, points: [None; 128], dragging: false, last: None, th, message: String::new() })
    });
}
struct Sketch {
    editor: WeakEntity<Editor>, target: Target, generation: u64, points: [Option<f64>; 128],
    dragging: bool, last: Option<(usize, f64)>, th: Theme, message: String,
}
fn draw_segment(points: &mut [Option<f64>;128], from: Option<(usize,f64)>, index: usize, value: f64) {
    let (start, old) = from.unwrap_or((index, value));
    let lo = start.min(index); let hi = start.max(index);
    for (offset, point) in points.iter_mut().enumerate().take(hi + 1).skip(lo) {
        let ratio = if start == index { 1. } else { (offset as f64 - start as f64)/(index as f64 - start as f64) };
        *point = Some((old + (value-old)*ratio).clamp(-24.,24.));
    }
}
impl Render for Sketch {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let th = self.th; let points = self.points; let entity = cx.entity();
        div().size_full().bg(th.surface).text_color(th.text).p(px(18.)).flex().flex_col().gap(px(12.))
            .child(div().text_lg().font_weight(FontWeight::BOLD).child("绘制目标频响"))
            .child(div().text_sm().text_color(th.text_secondary).child("在图中按住并拖动。应用后拟合为最多 24 个可继续编辑的参数频段。"))
            .child(canvas(|b,w,_| w.insert_hitbox(b, HitboxBehavior::Normal), move |bounds, hitbox, window, cx| {
                let plot = Plot { o: bounds.origin, w: f64::from(bounds.size.width), h: f64::from(bounds.size.height) };
                let axis = Axis::lin(-24.,24.); let mut texts = Texts::new();
                let grid = [-24.,-12.,0.,12.,24.].map(|db| (axis.norm(db), format!("{db:+.0} dB")));
                paint_grid(window, plot, th, &freq_grid(), &grid, Some(0.5), &mut texts);
                polyline(window, plot.o, points.iter().enumerate().map(|(i,v)| v.map(|v| (plot.x(i as f64/127.),plot.y(axis.norm(v))))), 2.5, th.accent);
                flush_texts(window,cx,bounds,texts);
                let sample = move |p: gpui::Point<Pixels>| {
                    let n = ((f64::from(p.x-plot.o.x)-plot.x(0.))/plot.inner_w()).clamp(0.,1.);
                    let y = ((plot.y(0.)-f64::from(p.y-plot.o.y))/(plot.y(0.)-plot.y(1.))).clamp(0.,1.);
                    ((n*127.).round() as usize,axis.value(y))
                };
                let e=entity.clone(); let h=hitbox.clone();
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble || event.button != MouseButton::Left || !h.is_hovered(window) { return }
                    window.capture_pointer(h.id); let (index,value)=sample(event.position);
                    e.update(cx,|v,cx|{v.dragging=true;v.last=Some((index,value));v.points[index]=Some(value);cx.notify();});cx.stop_propagation();
                });
                let e=entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent,phase,_,cx|{
                    if phase!=DispatchPhase::Bubble {return}
                    e.update(cx,|v,cx|{if v.dragging {let(index,value)=sample(event.position);draw_segment(&mut v.points,v.last,index,value);v.last=Some((index,value));cx.notify();}});
                });
                let e=entity.clone();
                window.on_mouse_event(move |event:&MouseUpEvent,phase,window,cx|{
                    if phase!=DispatchPhase::Bubble||event.button!=MouseButton::Left{return}
                    e.update(cx,|v,cx|{if v.dragging{v.dragging=false;v.last=None;window.release_pointer();cx.notify();}});
                });
            }).w_full().flex_1())
            .child(div().text_xs().text_color(th.text_secondary).child(self.message.clone()))
            .child(div().flex().justify_end().gap(px(8.))
                .child(crate::ui_controls::selectable_button("sketch-clear","清除",false,th).on_click(cx.listener(|v,_,_,cx|{v.points=[None;128];cx.notify();})))
                .child(crate::ui_controls::selectable_button("sketch-cancel","取消",false,th).on_click(|_,w,_|w.remove_window()))
                .child(crate::ui_controls::selectable_button("sketch-apply","应用",true,th).on_click(cx.listener(|v,_,window,cx|{
                    let points:Vec<_>=v.points.iter().enumerate().filter_map(|(i,p)|p.map(|p|(FREQ_AXIS.value(i as f64/127.),p))).collect();
                    let writes=match dsp::eq_fit_curve(&points,24){Ok(w)=>w,Err(e)=>{v.message=e;cx.notify();return}};
                    let target=v.target.clone();let generation=v.generation;
                    let result=v.editor.update(cx,|ed,cx|ed.document_generation==generation&&ed.device_target_matches(&target.track,target.slot,target.key)&&ed.set_device_params(target.track,target.slot,writes,cx));
                    if matches!(result,Ok(true)){window.remove_window()}else{v.message="参数尚未提交，请等待插件就绪".into();cx.notify();}
                }))))
    }
}

fn matching_spectrum(frequencies: &[f32], levels: &[f32]) -> Result<Vec<(f64, f64)>, String> {
    let points: Vec<_> = frequencies.iter().zip(levels)
        .filter(|(f, _)| (FREQ_AXIS.min..=FREQ_AXIS.max).contains(&(**f as f64)))
        .map(|(frequency, level)| (*frequency as f64, *level as f64)).collect();
    if points.len() < 2 || points.iter().any(|(f, level)| !f.is_finite() || !level.is_finite()) {
        return Err("频谱数据不足或无效，请重新捕获音频".into());
    }
    if points.iter().all(|(_, level)| *level <= -100.) {
        return Err("当前频谱接近静音，请在两个实例有声音时进行匹配".into());
    }
    Ok(points)
}

fn open_references(editor: Entity<Editor>, target: Target, cx: &mut App) {
    let th=editor.read(cx).theme; let generation=editor.read(cx).document_generation;let weak=editor.downgrade();let observed=editor.clone();
    let bounds=Bounds::centered(None,size(px(740.),px(560.)),cx);
    let _=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(bounds)),app_id:Some("verdant".into()),..Default::default()},move|window,cx|{
        window.set_window_title("参考频谱与匹配 — Verdant");
        cx.new(|cx|{cx.observe(&observed,|_,_,cx|cx.notify()).detach();ReferenceWindow{editor:weak,target,generation,th,message:String::new()}})
    });
}
struct ReferenceWindow {editor:WeakEntity<Editor>,target:Target,generation:u64,th:Theme,message:String}
impl Render for ReferenceWindow {
 fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
  let Some(editor)=self.editor.upgrade()else{window.remove_window();return div().into_any_element()};
  let ed=editor.read(cx);if ed.document_generation!=self.generation{window.remove_window();return div().into_any_element()}
  let th=self.th;let mut rows=div().id("references").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(px(8.));
  let mut owners:Vec<_>=ed.project().tracks.iter().map(|t|(DeviceTarget::Track(t.id.clone()),t.name.clone())).collect();owners.push((DeviceTarget::Master,"主总线".into()));
  for(owner,bus)in owners{
   for(slot,spec)in ed.device_insert_specs(&owner).iter().enumerate(){
    let Some(handle)=ed.live_device_insert(&owner,slot)else{continue};
    if handle.key()==self.target.key{continue}
    let reference=Target{track:owner.clone(),slot,key:handle.key()};let reference_id=workflow::workspace_id(ed,&reference);
    let name=format!("{bus} · {}",spec.name);let overlay_name=name.clone();let overlay_target=self.target.clone();let weak=self.editor.clone();
    let reference_target=reference.clone();let destination=self.target.clone();let generation=self.generation;
    rows=rows.child(div().p(px(10.)).rounded(px(8.)).bg(th.card).flex().items_center().gap(px(8.)).child(div().flex_1().text_sm().child(name))
      .child(crate::ui_controls::selectable_button(format!("overlay-{}",handle.key().0),"叠加频谱",false,th).on_click(move|_,_,cx|{
        let _=weak.update(cx,|ed,cx|{if ed.document_generation!=generation{return}let mut state=workflow::read_workspace(ed,&overlay_target);
          state.reference=reference_id.clone().map(|identity|workflow::Reference{identity,name:overlay_name.clone()});workflow::write_workspace(ed,&overlay_target,&state);cx.notify();});
      }))
      .child(crate::ui_controls::selectable_button(format!("match-{}",handle.key().0),"匹配",false,th).on_click(cx.listener(move|view,_,_,cx|{
        let destination=destination.clone();let reference=reference_target.clone();
        let Some(ed)=view.editor.upgrade()else{return};let ed=ed.read(cx);
        let Some(source)=ed.live_device_insert(&destination.track,destination.slot).filter(|h|h.key()==destination.key)else{return};
        let Some(target)=ed.live_device_insert(&reference.track,reference.slot).filter(|h|h.key()==reference.key)else{return};
        let generation=view.generation;view.message="正在捕获两个实例的实际频谱…".into();cx.notify();
        cx.spawn(async move|this,cx|{
          let result=cx.background_spawn(async move{
            source.refresh_telemetry().map_err(|e|e.to_string())?;target.refresh_telemetry().map_err(|e|e.to_string())?;
            let a=source.cached_telemetry().ok_or("当前 EQ 尚无音频")?;let b=target.cached_telemetry().ok_or("参考实例尚无音频")?;
            if a.frames_total==0||b.frames_total==0{return Err("请先播放两个实例的音频".into())}
            let source = matching_spectrum(&a.spectrum_frequencies, &a.spectrum_input_db)?;
            let reference = matching_spectrum(&b.spectrum_frequencies, &b.spectrum_output_db)?;
            dsp::eq_match_spectra(&source, &reference, 24)
          }).await;
          let _=this.update(cx,|view,cx|{
            match result{
              Ok(writes)=>{let target=destination.clone();let result=view.editor.update(cx,|ed,cx|ed.document_generation==generation&&ed.device_target_matches(&target.track,target.slot,target.key)&&ed.set_device_params(target.track,target.slot,writes,cx));
                view.message=if matches!(result,Ok(true)){"已提交匹配曲线，可继续编辑频段并撤销".into()}else{"插件已改变或正在提交其他参数".into()};},
              Err(e)=>view.message=e
            }cx.notify();
          });
        }).detach();
      }))));
   }
  }
  div().size_full().bg(th.surface).text_color(th.text).p(px(18.)).flex().flex_col().gap(px(12.))
   .child(div().text_lg().font_weight(FontWeight::BOLD).child("参考频谱与 EQ 匹配"))
   .child(div().text_sm().text_color(th.text_secondary).child("匹配当前 EQ 的输入与参考实例的输出。先播放代表性的音频；匹配是一组可撤销的静态频段。"))
   .child(rows).child(div().id("reference-status").role(Role::Status).text_xs().child(self.message.clone()))
   .child(div().flex().justify_end().gap(px(8.))
     .child(crate::ui_controls::selectable_button("reference-clear","清除叠加",false,th).on_click(cx.listener(|view,_,_,cx|{
       let target=view.target.clone();let generation=view.generation;
       let _=view.editor.update(cx,|ed,cx|{
         if ed.document_generation!=generation||!ed.device_target_matches(&target.track,target.slot,target.key){return}
         let mut state=workflow::read_workspace(ed,&target);state.reference=None;
         workflow::write_workspace(ed,&target,&state);cx.notify();
       });view.message="已清除参考频谱叠加".into();cx.notify();
     })))
     .child(crate::ui_controls::selectable_button("reference-close","关闭",false,th).on_click(|_,w,_|w.remove_window()))).into_any_element()
 }
}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn matching_filters_fft_dc_and_nyquist_and_rejects_silence() {
  let frequencies = [0., 23.4375, 1000., 19992.1875, 24000.];
  let points = matching_spectrum(&frequencies, &[-10., -70., -20., -80., -100.]).unwrap();
  assert_eq!(points.len(), 3); assert_eq!(points[0].0, 23.4375);
  assert!(dsp::eq_match_spectra(&points, &points, 8).is_ok());
  assert!(matching_spectrum(&frequencies, &[-120.;5]).is_err());
  assert!(matching_spectrum(&frequencies, &[-10., -70., f32::NAN, -80., -100.]).is_err());
 }
 #[test] fn sketch_interpolates_both_directions_without_breaks(){
  let mut points=[None;128];draw_segment(&mut points,Some((10,-12.)),20,12.);
  assert_eq!(points[10],Some(-12.));assert_eq!(points[15],Some(0.));assert_eq!(points[20],Some(12.));
  draw_segment(&mut points,Some((20,12.)),10,-12.);assert_eq!(points[15],Some(0.));assert!(points[..10].iter().all(Option::is_none));
 }
}
