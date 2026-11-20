//! Native file dialogs and durable window preferences. Documents and recovery
//! logs belong to ProjectPersistence; preferences never live in the install prefix.

use std::path::{Path, PathBuf};
use gpui::{App, AppContext, Bounds, Context, FocusHandle, IntoElement, KeyDownEvent, Point, Render, ScrollHandle, Size, Window, WindowBounds, WindowOptions, div, prelude::*, px};
use crate::editor::Editor;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub window: Option<[f32; 4]>,
    pub maximized: bool,
    pub last_project_dir: Option<String>,
    pub last_export_dir: Option<String>,
    pub device: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub fn state_dir() -> PathBuf {
    verdant_backend::platform::state_dir()
}

pub fn load_prefs() -> Prefs {
    let path = state_dir().join("prefs.json");
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(e) => { eprintln!("verdant: invalid {}: {e}", path.display()); Prefs::default() }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Prefs::default(),
        Err(e) => { eprintln!("verdant: cannot read {}: {e}", path.display()); Prefs::default() }
    }
}

pub fn save_prefs(p: &Prefs) -> Result<(), String> {
    use std::io::Write;
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("prefs.json");
    let tmp = dir.join(format!("prefs.{}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(tmp, path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    std::fs::File::open(&dir).and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn render_cache_dir() -> PathBuf {
    if cfg!(any(windows, target_os = "macos")) { verdant_backend::platform::cache_dir().join("render-cache") }
    else { state_dir().join("render-cache") }
}

/// The desktop opens and saves native `.verp` documents.
pub const NATIVE_EXTENSION: &str = verdant_domain::io::PROJECT_EXTENSION;

/// The application version recorded as the creator of a newly created native
/// document. It describes the producing application, not the document schema.
pub const CREATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Extension key holding the creator version of a native document.
pub const CREATOR_EXTENSION: &str = "verdant.created_with";

/// True for the native project suffix.
pub fn is_native_project_path(path: &Path) -> bool {
    verdant_domain::io::has_project_extension(path)
}

/// An extensionless save destination gains the native project suffix.
pub fn native_save_path(path: &Path) -> Result<PathBuf, String> {
    verdant_domain::io::project_output_path(path).map_err(|error| error.to_string())
}

/// Record the creator version of a newly created native document. Ordinary
/// saves keep the recorded creator of the opened document.
pub fn stamp_creator(project: &mut verdant_domain::Project) {
    project.extensions.insert(CREATOR_EXTENSION.into(), serde_json::json!(CREATOR_VERSION));
}

/// The creator version recorded in a native document. Documents written
/// before the marker existed carry none and stay openable.
pub fn creator_version(project: &verdant_domain::Project) -> Option<String> {
    project.extensions.get(CREATOR_EXTENSION).and_then(|value| value.as_str()).map(str::to_string)
}

/// A document created by a newer application may contain content this build
/// cannot represent. Versions compare as real SemVer; an unreadable marker
/// counts as newer, so the user decides instead of the file being rewritten.
fn creator_is_newer(creator: &str, allowed: &str) -> bool {
    match (semver::Version::parse(creator), semver::Version::parse(allowed)) {
        (Ok(creator), Ok(allowed)) => creator.cmp_precedence(&allowed).is_gt(),
        _ => true,
    }
}

pub(crate) fn creator_requiring_approval(project: &verdant_domain::Project, allowed_creator: Option<&str>) -> Option<String> {
    creator_version(project).filter(|creator| {
        allowed_creator != Some(creator.as_str()) && creator_is_newer(creator, CREATOR_VERSION)
    })
}

/// The outcome of opening a native document: the document with its recovery
/// report, or the creator version that needs a user decision first.
pub enum NativeOpen {
    Opened(verdant_domain::ProjectPersistence, verdant_domain::RecoveryReport),
    Newer(String),
}

/// Open a native document under the creator-version gate. The gate is
/// evaluated before the recovery log is opened, so a refused document leaves
/// both the document and its journal untouched. `allowed_creator` approves
/// exactly the creator string shown in a prior confirmation.
pub fn open_native_project(path: &Path, allowed_creator: Option<&str>) -> Result<NativeOpen, String> {
    verdant_domain::io::require_project_path(path).map_err(|error| error.to_string())?;
    let mut newer: Option<String> = None;
    let mut gate = |project: &verdant_domain::Project| -> bool {
        newer = creator_requiring_approval(project, allowed_creator);
        newer.is_none()
    };
    match verdant_domain::ProjectPersistence::open_if(path, &mut gate) {
        Ok(Some((persistence, report))) => Ok(NativeOpen::Opened(persistence, report)),
        Ok(None) => Ok(NativeOpen::Newer(newer.unwrap_or_else(|| CREATOR_VERSION.to_string()))),
        Err(error) => Err(error.to_string()),
    }
}

pub fn newer_project_warning(path: &Path, creator: &str) -> String {
    format!("{}\n创建版本：{creator}\n当前版本：{CREATOR_VERSION}\n此工程来自更新版本或无法识别的版本。部分内容可能无法正确显示或保存；是否仍然打开？", path.display())
}

/// Save As suggests the native project suffix.
pub(crate) fn project_suggested_name(path: Option<&Path>) -> String {
    let stem = path.and_then(Path::file_stem).and_then(|name| name.to_str()).unwrap_or("project");
    format!("{stem}.{NATIVE_EXTENSION}")
}

pub(crate) fn untitled_working_path(state: &Path) -> PathBuf {
    state.join("untitled/project.verp")
}

/// Shared by the menu and command-line opening paths. A foreign score becomes
/// a separate recoverable native document; its input bytes are never saved over.
pub(crate) fn import_score_document(path: &Path, format: verdant_exchange::Format, documents: &Path)
    -> Result<(verdant_domain::ProjectPersistence, serde_json::Value, PathBuf), String>
{
    let bytes = read_bounded(path, verdant_exchange::Limits::default().max_file_bytes)?;
    let source_name = path.file_name().map(|name| name.to_string_lossy().to_string());
    let (mut project, report) = crate::menu::import_score_bytes(format, &bytes, source_name)?;
    if project.tracks.len() > verdant_audio::MAX_TRACKS {
        return Err(format!("此乐谱含 {} 轨，当前混音器上限为 {}；未截断轨道", project.tracks.len(), verdant_audio::MAX_TRACKS));
    }
    verdant_domain::TempoMap::new(&project.tempo_map, project.ppq).map_err(|error| error.to_string())?;
    verdant_domain::MeterMap::new(&project.time_signatures, project.ppq)?;
    stamp_creator(&mut project);
    let working = documents.join(format!("imported-{}.verp", verdant_domain::NoteId::generate()));
    let store = verdant_domain::ProjectStore::new(project);
    verdant_service::media::copy_project_assets(&store, path, &working).map_err(|error| error.to_string())?;
    let persistence = verdant_domain::ProjectPersistence::create_from_store(&working, store).map_err(|error| error.to_string())?;
    Ok((persistence, report, working))
}

pub fn import_project_document(path: &Path, documents: &Path)
    -> Result<(verdant_domain::ProjectPersistence, serde_json::Value, PathBuf), String>
{
    let format = import_project_format(path)?;
    import_score_document(path, format, documents)
}

fn import_project_format(path: &Path) -> Result<verdant_exchange::Format, String> {
    crate::menu::file_format(path)?.ok_or_else(|| ".verp 工程请使用「打开」".to_owned())
}

pub(crate) struct StartupDocument {
    pub persistence: verdant_domain::ProjectPersistence,
    pub recovery: verdant_domain::RecoveryReport,
    pub project_path: Option<PathBuf>,
    pub new_demo: bool,
    pub import_report: Option<String>,
}

pub(crate) enum StartupOpen {
    Opened(StartupDocument),
    Newer { path: PathBuf, creator: String },
}

pub(crate) fn open_startup_document(explicit: Option<&Path>, state: &Path, auditions: &[crate::auditions::AuditionProject], allowed_creator: Option<&str>)
    -> Result<StartupOpen, String>
{
    if let Some(path) = explicit {
        return match open_native_project(path, allowed_creator)? {
            NativeOpen::Opened(persistence, recovery) => {
                let project_path = crate::demos::save_destination(persistence.project(), Some(path), &state.join("documents"));
                Ok(StartupOpen::Opened(StartupDocument { persistence, recovery, project_path, new_demo:false, import_report:None }))
            }
            NativeOpen::Newer(creator) => Ok(StartupOpen::Newer { path:path.to_path_buf(), creator }),
        };
    }
    let working = untitled_working_path(state);
    let source = crate::demos::startup_source(working.exists(), false, !auditions.is_empty());
    let (persistence, recovery) = if source == crate::demos::StartupSource::Existing {
        match open_native_project(&working, allowed_creator)? {
            NativeOpen::Opened(persistence, recovery) => (persistence, recovery),
            NativeOpen::Newer(creator) => return Ok(StartupOpen::Newer { path: working, creator }),
        }
    } else {
        let mut project = if source == crate::demos::StartupSource::Audition {
            verdant_domain::ProjectPersistence::read_saved(&auditions[0].path).map_err(|error| error.to_string())?
        } else { crate::demos::default_project() };
        stamp_creator(&mut project);
        let store = verdant_domain::ProjectStore::new(project);
        if source == crate::demos::StartupSource::Audition {
            verdant_service::media::copy_project_assets(&store, &auditions[0].path, &working).map_err(|error| error.to_string())?;
        }
        (verdant_domain::ProjectPersistence::create_from_store(&working, store).map_err(|error| error.to_string())?, Default::default())
    };
    let project_path = crate::demos::save_destination(persistence.project(), explicit, &state.join("documents"));
    Ok(StartupOpen::Opened(StartupDocument { persistence, recovery, project_path, new_demo: source == crate::demos::StartupSource::Builtin, import_report: None }))
}

pub fn remembered_bounds(prefs: &Prefs, cx: &App) -> gpui::WindowBounds {
    match prefs.window {
        Some([x, y, w, h]) if [x, y, w, h].iter().all(|v| v.is_finite()) && w >= 640.0 && h >= 400.0 => {
            let b = Bounds::new(Point::new(px(x), px(y)), Size::new(px(w), px(h)));
            if prefs.maximized { gpui::WindowBounds::Maximized(b) } else { gpui::WindowBounds::Windowed(b) }
        }
        _ => gpui::WindowBounds::Windowed(Bounds::centered(None, gpui::size(px(1480.), px(920.)), cx)),
    }
}

pub fn capture_bounds(prefs: &mut Prefs, window: &Window) {
    match window.window_bounds() {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => {
            prefs.window = Some([b.origin.x.as_f32(), b.origin.y.as_f32(), b.size.width.as_f32(), b.size.height.as_f32()]);
            prefs.maximized = matches!(window.window_bounds(), gpui::WindowBounds::Maximized(_));
        }
        gpui::WindowBounds::Fullscreen(_) => {}
    }
}

/// Call after constructing an editor or replacing its project. Periodic
/// autosave already checkpoints the sidecar every 45 seconds in the slow pump;
/// opening that sidecar must also offer a deliberate accept/discard choice.
pub fn prompt_recovery(ed: &mut Editor, window: &mut Window, cx: &mut Context<Editor>) {
    let Some(report) = ed.persistence.pending_recovery().cloned() else { return };
    let path = ed.persistence.project_path().to_path_buf();
    let project_id = ed.project().project_id.clone();
    let revision = ed.revision();
    let detail = format!(
        "{}\n手动保存版本 r{}，可恢复版本 r{}。恢复时间：{} ms（Unix）。\n恢复将保留自动保存的工程和撤销历史；放弃恢复将打开最后手动保存的版本。两种选择都不会覆盖已保存的工程文件。",
        path.display(), report.saved_revision, report.recovered_revision, report.recovered_at_ms,
    );
    let answer = window.prompt(
        gpui::PromptLevel::Warning,
        "发现较新的自动保存，是否恢复？",
        Some(&detail),
        &["恢复自动保存", "放弃恢复，打开已保存版本"],
        cx,
    );
    cx.spawn_in(window, async move |this, cx| {
        let answer = answer.await;
        let _ = cx.update(|window, cx| this.update(cx, |ed, cx| {
            if ed.persistence.project_path() != path.as_path() || ed.project().project_id != project_id || ed.revision() != revision {
                ed.notice("工程已改变，未应用恢复选择；恢复日志仍保留");
                cx.notify();
                return;
            }
            match answer {
                Ok(0) => {
                    ed.persistence.accept_recovery();
                    ed.recovered = None;
                    ed.notice("已恢复自动保存和撤销历史；手动保存的工程文件未被覆盖");
                }
                Ok(1) => {
                    let result = ed.persistence.discard_recovery().and_then(|_| {
                        verdant_domain::ProjectPersistence::open(&path)
                    });
                    match result {
                        Ok((persistence, report)) => {
                            let project_path = ed.project_path.clone();
                            ed.replace_project_opened(persistence, report, window, cx);
                            // Untitled sessions use an internal working path;
                            // choosing the saved version must not turn that
                            // internal recovery file into a user Save target.
                            ed.project_path = project_path;
                            ed.notice("已放弃恢复，打开最后手动保存的版本；工程文件未被覆盖");
                        }
                        Err(error) => ed.notice(format!("无法放弃恢复：{error}")),
                    }
                }
                _ => ed.notice("尚未选择恢复方式；自动保存和手动保存的工程文件均已保留"),
            }
            cx.notify();
        }));
    }).detach();
}

pub fn open_project_dialog(ed: &mut Editor, window: &mut Window, cx: &mut Context<Editor>) {
    ed.end_lyric(true, None, window, cx);
    let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
        files: true, directories: false, multiple: false,
        prompt: Some("打开 Verdant 工程（.verp）".into()),
    });
    cx.spawn_in(window, async move |this, cx| {
        let picked = rx.await.map_err(|e| e.to_string()).and_then(|result| result.map_err(|e| e.to_string()));
        let _ = cx.update(|window, cx| this.update(cx, |ed, cx| match picked {
            Ok(Some(mut paths)) => if let Some(path) = paths.pop() { ed.open_project_path(path, window, cx); },
            Ok(None) => {},
            Err(e) => { ed.notice(format!("打开对话框失败：{e}")); cx.notify(); }
        }));
    }).detach();
}

pub fn save_as_dialog(ed: &mut Editor, window: &mut Window, cx: &mut Context<Editor>) {
    ed.end_lyric(true, None, window, cx);
    let dir = ed.project_path.as_deref().and_then(Path::parent).map(Path::to_path_buf)
        .or_else(|| ed.prefs.last_project_dir.as_ref().map(PathBuf::from)).unwrap_or_else(|| PathBuf::from("."));
    let suggested = project_suggested_name(ed.project_path.as_deref());
    let rx = cx.prompt_for_new_path(&dir, Some(&suggested));
    cx.spawn(async move |this, cx| {
        let picked = rx.await.map_err(|e| e.to_string()).and_then(|result| result.map_err(|e| e.to_string()));
        let _ = this.update(cx, |ed, cx| match picked {
            Ok(Some(path)) => ed.save_project_as(path, cx),
            Ok(None) => {},
            Err(e) => { ed.notice(format!("保存对话框失败：{e}")); cx.notify(); }
        });
    }).detach();
}

/// Native project-copy export and offline WAV export are separate actions.
pub fn export_dialog(ed: &mut Editor, window: &mut Window, cx: &mut Context<Editor>) {
    export_picker(ed, false, window, cx);
}

pub fn export_audio_dialog(ed: &mut Editor, window: &mut Window, cx: &mut Context<Editor>) {
    export_picker(ed, true, window, cx);
}

fn export_picker(ed: &mut Editor, audio: bool, window: &mut Window, cx: &mut Context<Editor>) {
    ed.end_lyric(true, None, window, cx);
    let dir = ed.prefs.last_export_dir.as_ref().map(PathBuf::from)
        .or_else(|| ed.project_path.as_deref().and_then(Path::parent).map(Path::to_path_buf)).unwrap_or_else(|| PathBuf::from("."));
    let extension = if audio { "wav" } else { verdant_domain::io::PROJECT_EXTENSION };
    let native_name = project_suggested_name(ed.project_path.as_deref());
    let suggested = if audio { Path::new(&native_name).with_extension("wav").to_string_lossy().to_string() } else { native_name };
    let rx = cx.prompt_for_new_path(&dir, Some(&suggested));
    cx.spawn(async move |this, cx| {
        let picked = rx.await.map_err(|e| e.to_string()).and_then(|result| result.map_err(|e| e.to_string()));
        let _ = this.update(cx, |ed, cx| match picked {
            Ok(Some(mut path)) => {
                if path.extension().is_none() { path.set_extension(extension); }
                ed.prefs.last_export_dir = path.parent().map(|p| p.display().to_string());
                if audio { ed.export_audio(path, cx); } else { ed.export_project(&path, cx); }
            }
            Ok(None) => {},
            Err(e) => { ed.notice(format!("导出对话框失败：{e}")); cx.notify(); }
        });
    }).detach();
}


pub(crate) fn read_bounded(path:&Path,maximum:usize)->Result<Vec<u8>,String>{
    use std::io::Read;
    let metadata=std::fs::metadata(path).map_err(|e|e.to_string())?;
    if !metadata.is_file()||metadata.len()>maximum as u64{return Err(format!("文件须为不超过 {} MiB 的普通文件",maximum/1024/1024));}
    let file=std::fs::File::open(path).map_err(|e|e.to_string())?;let mut bytes=Vec::with_capacity(metadata.len()as usize);
    file.take(maximum as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>maximum{return Err("文件超出导入大小限制".into());}Ok(bytes)
}
pub(crate) fn atomic_bytes(path:&Path,bytes:&[u8])->Result<(),String>{
    use std::io::Write;
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name=path.file_name().ok_or("路径缺少文件名")?.to_string_lossy();let temporary=parent.join(format!(".{name}.{}.tmp",verdant_domain::NoteId::generate()));
    let result=(||{let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e|e.to_string())?;
        file.write_all(bytes).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;drop(file);std::fs::rename(&temporary,path).map_err(|e|e.to_string())?;Ok(())})();
    if result.is_err(){let _=std::fs::remove_file(&temporary);}result
}

pub fn export_score_dialog(ed:&mut Editor,format:verdant_exchange::Format,window:&mut Window,cx:&mut Context<Editor>){
    if !ed.commit_pending_lyric(cx)||!ed.flush_insert_states(cx){return;}
    let project=ed.store().snapshot();
    if format==verdant_exchange::Format::Ust&&project.tracks.len()>1{
        // UST carries one vocal lane, so the exported track is an explicit choice.
        let items=project.tracks.iter().enumerate().map(|(index,track)|crate::menu::MenuItem::action(format!("{} · 第 {} 轨",track.name,index+1),crate::editor::MenuAction::Workflow(crate::editor::WorkflowAction::Integration(crate::editor::integrations::IntegrationAction::ExportScoreTrack(format,index))))).collect::<Vec<_>>();
        ed.show_menu(window.mouse_position(),items,window,cx);return;
    }
    export_score_with(ed,format,None,window,cx);
}

/// True when an imported VSQX carried a real voice table, so a VSQX export can
/// reuse that licensed metadata instead of inventing a compID.
fn has_vsqx_voice_table(project:&verdant_domain::Project)->bool{
    project.extensions.get("exchange_vsqx").and_then(|value|value.get("voices")).is_some()
}

pub fn export_score_with(ed:&mut Editor,format:verdant_exchange::Format,track:Option<usize>,window:&mut Window,cx:&mut Context<Editor>){
    if !ed.commit_pending_lyric(cx)||!ed.flush_insert_states(cx){return;}
    let project=ed.store().snapshot();let mut options=verdant_exchange::ExportOptions::default();
    if let Some(index)=track{options.track=Some(index);}
    if format==verdant_exchange::Format::Ust&&options.track.is_none(){options.track=ed.track_index(&ed.session.track);}
    if let Some(value)=project.extensions.get("vocaloid_voice"){match serde_json::from_value(value.clone()){Ok(voice)=>options.vocaloid_voice=Some(voice),Err(error)=>{ed.notice(format!("Vocaloid 元数据无效：{error}"));cx.notify();return;}}}
    if format==verdant_exchange::Format::Vsqx&&options.vocaloid_voice.is_none()&&!has_vsqx_voice_table(&project){
        ed.notice("VSQX 需要真实歌手元数据：请先填写 Vocaloid 歌手元数据后再导出");
        ed.prompt_text("Vocaloid 歌手元数据：真实 16 字符 compID | 歌手名称 | 语言 0..127 | 程序 0..127",crate::editor::PromptPurpose::Integration(crate::editor::integrations::IntegrationPrompt::Vocaloid),"",window,cx);cx.notify();return;
    }
    // Keep the selected UST lane and admitted voice metadata across the options
    // window. The snapshot also prevents a later track reorder changing its meaning.
    let tracks=project.tracks.iter().map(|t|t.name.clone()).collect();
    let (send,receive)=async_channel::bounded(1);
    let bounds=Bounds::centered(None,gpui::size(px(580.),px(620.)),cx);
    let opened=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(bounds)),app_id:Some("verdant".into()),..Default::default()},move|window,cx|{
        window.set_window_title(&format!("{} 导出选项 — Verdant",score_format_name(format)));
        cx.new(|cx|ScoreExportDialog::new(format,options,tracks,send,window,cx))
    });
    if let Err(error)=opened{ed.notice(format!("无法打开导出选项：{error}"));cx.notify();return;}
    cx.spawn_in(window,async move|this,cx|{
        if let Ok(options)=receive.recv().await{
            let _=cx.update(|window,cx|this.update(cx,|ed,cx|pick_score_export(ed,format,project,options,window,cx)));
        }
    }).detach();
}

fn pick_score_export(ed:&mut Editor,format:verdant_exchange::Format,project:std::sync::Arc<verdant_domain::Project>,options:verdant_exchange::ExportOptions,window:&mut Window,cx:&mut Context<Editor>){
    let directory=ed.prefs.last_export_dir.as_ref().map(PathBuf::from).unwrap_or_else(||PathBuf::from("."));let name=format!("score.{}",format.extension());let rx=cx.prompt_for_new_path(&directory,Some(&name));
    let executor=cx.background_executor().clone();
    cx.spawn_in(window,async move|this,cx|{
        let picked=rx.await.map_err(|e|e.to_string()).and_then(|r|r.map_err(|e|e.to_string()));
        if let Ok(Some(mut path))=picked{if path.extension().is_none(){path.set_extension(format.extension());}let output=path.clone();let result=executor.spawn(async move{let exported=verdant_exchange::export(format,&project,&options).map_err(|e|e.to_string())?;atomic_bytes(&output,&exported.bytes)?;Ok::<_,String>(exported.report)}).await;
            let _=cx.update(|_window,cx|this.update(cx,|ed,cx|match result{
                Ok(report)=>{ed.integrations.report=remember_export_report(&mut ed.prefs,&report);ed.prefs.last_export_dir=path.parent().map(|p|p.display().to_string());ed.prefs_dirty=true;ed.notice(format!("已导出 {} · {} 项格式报告（右侧只读面板）",path.display(),report.issues.len()));ed.show_integration_report(cx);}
                Err(error)=>{ed.notice(format!("乐谱导出失败：{error}"));cx.notify();}
            }));
        }else if let Err(error)=picked{let _=this.update(cx,|ed,cx|{ed.notice(error);cx.notify();});}
    }).detach();
}

fn export_score_file(format:verdant_exchange::Format,project:&verdant_domain::Project,options:&verdant_exchange::ExportOptions,path:&Path)->Result<verdant_exchange::ExchangeReport,String>{
    let exported=verdant_exchange::export(format,project,options).map_err(|e|e.to_string())?;
    atomic_bytes(path,&exported.bytes)?;
    Ok(exported.report)
}

/// The complete per-field JSON is both the report dialog's content and the
/// durable preferences value used by “交换报告”; no loss is reduced to a count.
fn remember_export_report(prefs:&mut Prefs,report:&verdant_exchange::ExchangeReport)->String{
    let value=serde_json::to_value(report).expect("exchange report");
    let text=serde_json::to_string_pretty(&value).expect("exchange report text");
    prefs.extra.insert("last_exchange_report".into(),value);
    text
}

fn score_format_name(format:verdant_exchange::Format)->&'static str{
    use verdant_exchange::Format::*;
    match format{Midi=>"MIDI",Ust=>"UST",Ustx=>"USTX",Svp=>"SVP",Vsqx=>"VSQX",Vpr=>"VPR",Ufdata=>"UtaFormatix Data"}
}

/// Blank MIDI PPQ keeps the exporter's source-resolution policy. An explicit
/// value must fit SMF's metrical division; zero/SMPTE values are never clamped.
fn resolve_score_options(format:verdant_exchange::Format,mut options:verdant_exchange::ExportOptions,ppq:&str)->Result<verdant_exchange::ExportOptions,String>{
    if format==verdant_exchange::Format::Vsqx&&!options.preserve_extensions&&options.vocaloid_voice.is_none(){
        return Err("VSQX 需要真实歌手元数据：请保留来源扩展中的歌手表，或取消后填写 Vocaloid 歌手元数据".into());
    }
    if format==verdant_exchange::Format::Midi{
        options.midi_ppq=if ppq.trim().is_empty(){None}else{
            Some(ppq.trim().parse::<u16>().ok().filter(|v|(1..=32767).contains(v)).ok_or("MIDI PPQ 须为 1–32767 的整数；留空使用来源分辨率")?)
        };
    }
    Ok(options)
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum ScoreOptionControl{Utf8,Cp932,Ppq,Preserve,Track(Option<usize>),Cancel,Export}

struct ScoreExportDialog{
    format:verdant_exchange::Format,
    options:verdant_exchange::ExportOptions,
    tracks:Vec<String>,
    ppq:String,
    replace_ppq:bool,
    controls:Vec<(ScoreOptionControl,FocusHandle)>,
    focus:FocusHandle,
    scroll:ScrollHandle,
    send:async_channel::Sender<verdant_exchange::ExportOptions>,
    error:std::option::Option<std::string::String>,
}

impl ScoreExportDialog{
    fn new(format:verdant_exchange::Format,options:verdant_exchange::ExportOptions,tracks:Vec<String>,send:async_channel::Sender<verdant_exchange::ExportOptions>,window:&mut Window,cx:&mut Context<Self>)->Self{
        use ScoreOptionControl::*;
        let mut kinds=Vec::new();
        if format==verdant_exchange::Format::Ust{kinds.extend([Utf8,Cp932]);}
        if format==verdant_exchange::Format::Midi{kinds.push(Ppq);}
        kinds.push(Preserve);
        if format!=verdant_exchange::Format::Ust{kinds.push(Track(None));}
        kinds.extend((0..tracks.len()).map(|i|Track(Some(i))));
        kinds.extend([Cancel,Export]);
        let controls=kinds.into_iter().map(|k|(k,cx.focus_handle())).collect::<Vec<_>>();
        let focus=cx.focus_handle();
        window.focus(&controls[0].1,cx);
        let ppq=options.midi_ppq.map(|p|p.to_string()).unwrap_or_default();
        Self{format,options,tracks,ppq,replace_ppq:true,controls,focus,scroll:ScrollHandle::default(),send,error:None}
    }

    fn activate(&mut self,control:ScoreOptionControl,window:&mut Window,cx:&mut Context<Self>){
        use ScoreOptionControl::*;
        match control{
            Utf8=>self.options.ust_encoding=verdant_exchange::UstEncoding::Utf8,
            Cp932=>self.options.ust_encoding=verdant_exchange::UstEncoding::Cp932,
            Ppq=>self.replace_ppq=true,
            Preserve=>self.options.preserve_extensions=!self.options.preserve_extensions,
            Track(track)=>self.options.track=track,
            Cancel=>{window.remove_window();return;}
            Export=>match resolve_score_options(self.format,self.options.clone(),&self.ppq){
                Ok(options)=>{if self.send.try_send(options).is_ok(){window.remove_window();return;}self.error=Some("原工程窗口已关闭，请重新导出".into());}
                Err(error)=>self.error=Some(error),
            },
        }
        cx.notify();
    }

    fn key_down(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>){
        let key=event.keystroke.key.as_str();
        let index=self.controls.iter().position(|(_,focus)|focus.is_focused(window)).unwrap_or(0);
        let control=self.controls[index].0;
        if key=="tab"{
            let len=self.controls.len();let next=if event.keystroke.modifiers.shift{(index+len-1)%len}else{(index+1)%len};
            window.focus(&self.controls[next].1,cx);self.replace_ppq=true;
        }else if key=="escape"{window.remove_window();}
        else if key=="enter"{self.activate(if control==ScoreOptionControl::Cancel{control}else{ScoreOptionControl::Export},window,cx);}
        else if key=="space"&&control!=ScoreOptionControl::Ppq{self.activate(control,window,cx);}
        else if control==ScoreOptionControl::Ppq{
            match key{
                "backspace"|"delete"=>{if self.replace_ppq||key=="delete"{self.ppq.clear();}else{self.ppq.pop();}self.replace_ppq=false;}
                "up"|"down"=>{let value=self.ppq.parse::<i32>().unwrap_or(480);self.ppq=(value+if key=="up"{1}else{-1}).clamp(1,32767).to_string();self.replace_ppq=true;}
                digits if digits.len()==1&&digits.as_bytes()[0].is_ascii_digit()=>{if self.replace_ppq{self.ppq.clear();self.replace_ppq=false;}if self.ppq.len()<5{self.ppq.push_str(digits);}}
                _=>return,
            }
            self.error=None;
        }else{return;}
        window.prevent_default();cx.stop_propagation();cx.notify();
    }

    fn control(&self,kind:ScoreOptionControl,label:String,selected:bool,window:&Window,cx:&mut Context<Self>)->gpui::Stateful<gpui::Div>{
        let focus=self.controls.iter().find(|(k,_)|*k==kind).expect("dialog control").1.clone();
        let th=crate::theme::Theme::for_appearance(window.appearance());
        let focused=focus.is_focused(window);
        div().id(gpui::SharedString::from(format!("score-option-{}",self.controls.iter().position(|(k,_)|*k==kind).unwrap())))
            .role(if kind==ScoreOptionControl::Ppq{gpui::Role::SpinButton}else{gpui::Role::Button})
            .aria_label(if kind==ScoreOptionControl::Ppq{format!("MIDI PPQ：{label}")}else{label.clone()})
            .aria_selected(selected).track_focus(&focus)
            .when(kind==ScoreOptionControl::Ppq,|d|d.aria_value(label.clone()).aria_min_numeric_value(1.).aria_max_numeric_value(32767.).aria_numeric_value_step(1.))
            .p(px(8.)).rounded(px(4.)).border_1().border_color(if focused{th.accent}else{th.separator})
            .bg(if selected{th.selection}else{th.control}).text_sm().text_color(th.text)
            .child(label).on_click(cx.listener(move|view,_,window,cx|{window.focus(&focus,cx);view.activate(kind,window,cx);} ))
    }
}

impl Render for ScoreExportDialog{
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
        use ScoreOptionControl::*;
        let th=crate::theme::Theme::for_appearance(window.appearance());
        let mut form=div().flex().flex_col().gap(px(10.))
            .child(div().text_lg().child(format!("{} 导出选项",score_format_name(self.format))))
            .child(div().text_sm().text_color(th.text_secondary).child("导出后显示逐字段格式报告，并保存到“最近交换报告”。"));
        if self.format==verdant_exchange::Format::Ust{
            form=form.child("歌词编码").child(div().flex().gap(px(8.))
                .child(self.control(Utf8,"UTF-8".into(),matches!(self.options.ust_encoding,verdant_exchange::UstEncoding::Utf8),window,cx))
                .child(self.control(Cp932,"CP932（日文 Windows）".into(),matches!(self.options.ust_encoding,verdant_exchange::UstEncoding::Cp932),window,cx)));
        }
        if self.format==verdant_exchange::Format::Midi{
            form=form.child("MIDI PPQ（每四分音符 tick 数，1–32767）")
                .child(self.control(Ppq,if self.ppq.is_empty(){"自动".into()}else{self.ppq.clone()},false,window,cx))
                .child(div().text_xs().text_color(th.text_secondary).child("点击后直接输入数字；↑↓ 微调，Delete 清空为自动。自动保留来源分辨率；来源超过 32767 时使用 960。"));
        }
        form=form.child(self.control(Preserve,format!("{} 保留来源扩展",if self.options.preserve_extensions{"☑"}else{"☐"}),self.options.preserve_extensions,window,cx))
            .child(div().text_xs().text_color(th.text_secondary).child("仅兼容的来源字段可随文件保留；未支持、保留和量化字段均列入报告。"))
            .child(if self.format==verdant_exchange::Format::Ust{"导出轨道（UST 仅支持单轨）"}else{"导出轨道"});
        if self.format!=verdant_exchange::Format::Ust{form=form.child(self.control(Track(None),"全部轨道".into(),self.options.track.is_none(),window,cx));}
        for(index,name)in self.tracks.iter().enumerate(){form=form.child(self.control(Track(Some(index)),format!("第 {} 轨 · {}",index+1,name),self.options.track==Some(index),window,cx));}
        if let Some(error)=&self.error{form=form.child(div().text_sm().text_color(th.error).child(error.clone()));}
        form=form.child(div().text_xs().text_color(th.text_secondary).child("Tab 切换，空格选择，Enter 继续，Esc 取消。"))
            .child(div().flex().justify_end().gap(px(8.))
            .child(self.control(Cancel,"取消 · Esc".into(),false,window,cx))
            .child(self.control(Export,"选择保存位置… · Enter".into(),false,window,cx)));
        div().id("score-export-options").role(gpui::Role::Dialog).aria_label("乐谱导出选项").track_focus(&self.focus)
            .size_full().overflow_y_scroll().track_scroll(&self.scroll).p(px(20.)).bg(th.surface).text_color(th.text)
            .on_key_down(cx.listener(Self::key_down)).child(form)
    }
}

#[cfg(test)]
#[path="files_export_options_tests.rs"]
mod export_options_tests;

#[cfg(test)]
#[path="files_project_tests.rs"]
mod project_tests;

impl Editor{
    pub fn import_project_path(&mut self,path:PathBuf,window:&mut Window,cx:&mut Context<Self>){
        let format = match import_project_format(&path) {
            Ok(format) => format,
            Err(error) => { self.notice(format!("导入失败：{error}")); cx.notify(); return; }
        };
        if !self.commit_pending_lyric(cx)||!self.flush_insert_states(cx){return;}if let Err(error)=self.persistence.autosave(){self.notice(error.to_string());cx.notify();return;}
        let current_id=self.project().project_id.clone();let base=self.revision();
        let executor=cx.background_executor().clone();
        cx.spawn_in(window,async move|this,cx|{let source=path.clone();let imported=executor.spawn(async move{
            import_score_document(&source,format,&state_dir().join("documents"))
        }).await;
        let _=cx.update(|window,cx|this.update(cx,|ed,cx|{let result=(||->Result<(),String>{let(persistence,report,working)=imported?;if ed.project().project_id!=current_id||ed.revision()!=base{return Err("导入期间当前工程已改变；导入结果保留在恢复工程中，请重新选择打开".into());}
            ed.replace_project_opened(persistence,Default::default(),window,cx);ed.project_path=None;ed.remember_project(&working);ed.integrations.report=serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?;ed.notice(format!("已导入 {}；源文件保持原样，保存时选择 Verdant 工程路径",path.display()));ed.show_integration_report(cx);Ok(())})();if let Err(error)=result{ed.notice(format!("导入失败：{error}"));cx.notify();}}));
        }).detach();
    }
}
