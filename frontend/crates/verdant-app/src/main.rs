//! Verdant 翠绿 — a GPUI singing-voice editor for the OpenGreendam backend.
//!
//! ```text
//! verdant [PROJECT.verp | --import FILE] [--worker real] [--worker-binary FILE]
//!         [--workspace DIR] [--worker-arg ARG]… [--silent-audio]
//!         [--voice-package DIR] [--voice-registry FILE]
//! ```
//!
//! `--worker real` launches the owned Rust `verdant-worker` executable. Voice
//! packages are user-provided. The desktop launches the Rust worker directly.

mod actions;
mod alert;
mod arrange;
mod auditions;
mod assets;
mod automation_lane;
mod clock;
mod commands;
mod curve_span;
mod devices;
mod dialogs;
mod disclosures;
mod demos;
mod desktop_engine;
mod editor;
mod export;
mod files;
mod generated {
    pub mod controls;
}
mod index;
mod instrument;
mod lanes;
mod lyric_view;
mod menu;
mod mix;
mod mixer;
mod paint;
mod phoneme_lane;
mod proto_roll;
mod roll;
mod ruler_navigation;
mod shortcuts;
mod table;
mod theme;
mod ui_controls;
mod view;
mod window_chrome;
mod voice_presets;

use std::path::PathBuf;

use gpui::{App, AppContext, Context, IntoElement, Render, Window, WindowOptions};
use gpui_platform::application;
use verdant_backend::WorkerConfig;

use crate::editor::{Editor, EditorOpts};

struct Args {
    project: Option<PathBuf>,
    import: Option<PathBuf>,
    audition_dir: Option<PathBuf>,
    worker_binary: Option<PathBuf>,
    voice_registry: Option<PathBuf>,
    workspace: Option<PathBuf>,
    worker_args: Vec<String>,
    silent_audio: bool,
    repo: Option<PathBuf>,
    appearance: Option<bool>,
    voice_package: Option<String>,
    /// `VERDANT_PLUGIN_HOST` / `--plughost`: plugin host binary override.
    plughost: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut a = Args {
        project: None,
        import: None,
        audition_dir: std::env::var_os("VERDANT_AUDITION_DIR").map(PathBuf::from),
        worker_binary: std::env::var_os("VERDANT_WORKER_BINARY").map(PathBuf::from),
        voice_registry: Some(std::env::var_os("VERDANT_VOICE_REGISTRY").map(PathBuf::from)
            .unwrap_or_else(|| files::state_dir().join("voices.json"))),
        workspace: None,
        worker_args: Vec::new(),
        silent_audio: false,
        appearance: None,
        repo: std::env::var_os("VERDANT_REPO").map(PathBuf::from),
        voice_package: std::env::var("VERDANT_VOICE_PACKAGE_DIR").ok(),
        plughost: std::env::var("VERDANT_PLUGIN_HOST").ok().map(PathBuf::from),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--runtime-features" => {
                println!("{}",serde_json::json!({"schema":"verdant-runtime-features/1","role":"gpui",
                    "version":env!("CARGO_PKG_VERSION"),"model_extensions":["verdant-voice"],
                    "voice_package":{"name":"verdant-voice","version":env!("CARGO_PKG_VERSION")},
                    "registry_schema_version":1,"managed_default":true,"immutable_voice_pins":true,
                    "resource_directory":verdant_backend::platform::resource_dir()}));
                std::process::exit(0);
            }
            "--worker" => {
                if it.next().as_deref() != Some("real") {
                    eprintln!("verdant: --worker accepts only real");
                    std::process::exit(2);
                }
            }
            "--audition-dir" => a.audition_dir = it.next().map(PathBuf::from),
            "--import" => {
                a.import = Some(it.next().map(PathBuf::from).unwrap_or_else(|| {
                    eprintln!("verdant: --import requires a source file");
                    std::process::exit(2);
                }));
            }
            "--worker-binary" => a.worker_binary = it.next().map(PathBuf::from),
            "--voice-registry" => a.voice_registry = it.next().map(PathBuf::from),
            "--workspace" => a.workspace = it.next().map(PathBuf::from),
            "--worker-arg" => a.worker_args.extend(it.next()),
            "--silent-audio" => a.silent_audio = true,
            "--appearance" => {
                a.appearance = match it.next().as_deref() {
                    Some("dark") => Some(true),
                    Some("light") => Some(false),
                    _ => None,
                }
            }
            "--repo" => a.repo = it.next().map(PathBuf::from).or(a.repo),
            "--voice-package" => a.voice_package = it.next(),
            "--plughost" => a.plughost = it.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!(
                    "verdant [PROJECT.verp | --import FILE] [--worker real] [--worker-binary FILE] [--workspace DIR] [--worker-arg ARG]... \
                     [--silent-audio] [--voice-package DIR] [--voice-registry FILE] [--plughost PATH] [--audition-dir DIR]\n\
                     Open, Save and Save As use .verp.\n\
                     Import: .ufdata, .ust, .ustx, .mid/.midi, .svp, .vsqx, .vpr."
                );
                std::process::exit(0);
            }
            p if !p.starts_with('-') => a.project = Some(PathBuf::from(p)),
            p => {
                eprintln!("verdant: unknown option {p}");
                std::process::exit(2);
            }
        }
    }
    if a.project.is_some() && a.import.is_some() {
        eprintln!("verdant: choose either a .verp project or --import FILE");
        std::process::exit(2);
    }
    if let Some(repo) = &mut a.repo { *repo = repo.canonicalize().unwrap_or_else(|_| repo.clone()); }
    a
}

fn project_root(args: &Args) -> std::io::Result<PathBuf> {
    if let Some(root) = &args.repo { return root.canonicalize(); }
    if cfg!(any(windows, target_os = "macos")) {
        if let Some(project) = args.project.as_ref().or(args.import.as_ref()) {
            return project.canonicalize()?.parent().map(PathBuf::from)
                .ok_or_else(|| std::io::Error::other("project has no parent directory"));
        }
        let documents = files::state_dir().join("documents");
        std::fs::create_dir_all(&documents)?;
        return documents.canonicalize();
    }
    std::env::current_dir()
}


struct StartupWarning;

impl Render for StartupWarning {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { gpui::div() }
}

struct EditorStartup {
    prefs: files::Prefs,
    worker: WorkerConfig,
    auditions: Vec<auditions::AuditionProject>,
    plughost: Option<PathBuf>,
    appearance: Option<bool>,
    silent_audio: bool,
}

impl EditorStartup {
    fn editor(self, document: files::StartupDocument, window: &mut Window, cx: &mut Context<Editor>) -> Editor {
        let files::StartupDocument { persistence, recovery, project_path, new_demo: _, import_report } = document;
        let selection = if self.silent_audio {
            verdant_audio::DeviceSelection::Silent
        } else {
            self.prefs.device.clone().map(verdant_audio::DeviceSelection::Device).unwrap_or(verdant_audio::DeviceSelection::Default)
        };
        let audio = verdant_audio::device::OutputEngine::open(selection);
        let opts = EditorOpts { project_path, audition_projects: self.auditions, prefs: self.prefs, plughost_path: self.plughost };
        let mut editor = Editor::new(persistence, recovery, opts, self.worker, audio, window, cx);
        if let Some(report) = import_report {
            editor.integrations.report = report;
            editor.notice("工程已导入，可在文件菜单查看导入 / 导出报告。请选择 .verp 保存工作副本。");
        }
        editor.appearance_override = self.appearance;
        editor
    }
}

fn startup_document(args: &Args, state: &std::path::Path, auditions: &[auditions::AuditionProject]) -> Result<files::StartupOpen, String> {
    if let Some(source) = &args.import {
        let (persistence, report, _) = files::import_project_document(source, &state.join("documents"))?;
        return Ok(files::StartupOpen::Opened(files::StartupDocument {
            persistence, recovery: Default::default(), project_path: None, new_demo: false,
            import_report: Some(serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?),
        }));
    }
    files::open_startup_document(args.project.as_deref(), state, auditions, None)
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args = parse_args();
    let audition_dir = args.audition_dir.clone().unwrap_or_else(|| files::state_dir().join("auditions"));
    let audition_projects = if audition_dir.exists() || args.audition_dir.is_some() {
        match auditions::load_catalog(&audition_dir) {
            Ok(projects) => projects,
            Err(error) => { eprintln!("verdant: listening catalogue unavailable: {error}"); Vec::new() }
        }
    } else { Vec::new() };
    let state = files::state_dir();
    let document = match startup_document(&args, &state, &audition_projects) {
        Ok(opened) => opened,
        Err(error) => {
            eprintln!("verdant: cannot open project: {error}");
            std::process::exit(2);
        }
    };
    let workspace = args.workspace.clone().unwrap_or_else(|| {
        if cfg!(any(windows, target_os = "macos")) { verdant_backend::platform::cache_dir().join("worker") }
        else { files::state_dir().join("worker") }
    });
    let project_root = project_root(&args).unwrap_or_else(|error| {
        eprintln!("verdant: cannot resolve project root: {error}");
        std::process::exit(2);
    });
    let mut extra: Vec<String> = args.worker_args.clone();
    let worker_cfg = {
        let binary = args.worker_binary.clone().or_else(|| {
            let name = if cfg!(windows) { "verdant-worker.exe" } else { "verdant-worker" };
            std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        }).unwrap_or_else(|| {
            eprintln!("verdant: cannot resolve verdant-worker; set --worker-binary or VERDANT_WORKER_BINARY");
            std::process::exit(2);
        });
        if !binary.is_file() {
            eprintln!("verdant: Rust worker missing: {}; build verdant-worker or set --worker-binary", binary.display());
            std::process::exit(2);
        }
        for (flag, value) in [("--voice-package", args.voice_package.as_deref())] {
            if let Some(value) = value { extra.push(flag.into()); extra.push(value.into()); }
        }
        if let Some(path) = &args.voice_registry { extra.push("--voice-registry".into()); extra.push(path.display().to_string()); }
        extra.push("--project-root".into()); extra.push(project_root.display().to_string());
        let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
        WorkerConfig::native_worker(binary, workspace, &extra)
    };
    let startup = EditorStartup {
        prefs: files::load_prefs(), worker: worker_cfg, auditions: audition_projects,
        plughost: args.plughost, appearance: args.appearance, silent_audio: args.silent_audio,
    };
    let explicit = args.project;

    application().with_assets(assets::Assets).run(move |cx: &mut App| {
        // macOS and Windows keep their native alerts.
        #[cfg(not(any(target_os = "macos", windows)))]
        alert::install(cx);
        actions::bind(cx);
        cx.set_menus(actions::menus());
        crate::disclosures::i18n::initialize(&startup.prefs, cx);
        let bounds = files::remembered_bounds(&startup.prefs, cx);
        let mut options = WindowOptions { window_bounds: Some(bounds), app_id: Some("verdant".into()), ..Default::default() };
        // The toolbar is the title bar (see window_chrome).
        window_chrome::apply(&mut options, ui_controls::bar_height(false));
        match document {
            files::StartupOpen::Opened(document) => {
                cx.open_window(options, |window, cx| {
                    let editor = cx.new(|cx| startup.editor(document, window, cx));
                    focus_editor(&editor, window, cx);
                    editor
                }).expect("open window");
            }
            files::StartupOpen::Newer { mut path, mut creator } => {
                let handle = cx.open_window(options, |window, cx| {
                    window.set_window_title("Verdant 翠绿");
                    cx.new(|_| StartupWarning)
                }).expect("open project version warning window");
                cx.spawn(async move |cx| {
                    let document = loop {
                        let detail = files::newer_project_warning(&path, &creator);
                        let Ok(answer) = handle.update(cx, |_, window, cx| window.prompt(
                            gpui::PromptLevel::Warning, "此工程由较新版本的 Verdant 创建", Some(&detail),
                            &["取消", "继续打开"], cx,
                        )) else { return; };
                        if !matches!(answer.await, Ok(1)) { cx.update(|cx| cx.quit()); return; }
                        match files::open_startup_document(explicit.as_deref(), &state, &startup.auditions, Some(&creator)) {
                            Ok(files::StartupOpen::Opened(document)) => break document,
                            Ok(files::StartupOpen::Newer { path: changed_path, creator: changed_creator }) => {
                                path = changed_path;
                                creator = changed_creator;
                            }
                            Err(error) => {
                                eprintln!("verdant: cannot open project: {error}");
                                if let Ok(answer) = handle.update(cx, |_, window, cx| window.prompt(
                                    gpui::PromptLevel::Critical, "无法打开工程", Some(&error), &["关闭"], cx,
                                )) { let _ = answer.await; }
                                cx.update(|cx| cx.quit());
                                return;
                            }
                        }
                    };
                    let _ = handle.update(cx, |_, window, cx| {
                        let editor = window.replace_root(cx, |window, cx| startup.editor(document, window, cx));
                        focus_editor(&editor, window, cx);
                    });
                }).detach();
            }
        }
        cx.activate(true);
    });
}

fn focus_editor(editor: &gpui::Entity<Editor>, window: &mut Window, cx: &mut App) {
    window.set_window_title("Verdant 翠绿");
    let focus = editor.read(cx).focus_roll.clone();
    window.focus(&focus, cx);
}

