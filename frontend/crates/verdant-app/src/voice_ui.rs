//! Voice package setup keeps source selection and the user's declaration explicit.
//! File classification is only a UI preflight; the worker verifies the bytes,
//! complete bank, ONNX ABI and atomic registry installation.
use super::*;
use gpui::{Div, Role, div, prelude::*, px};
use verdant_backend::voices::{SourceEvidence, VoiceAssets};

#[derive(Clone, Debug)]
pub struct VoiceImportDraft {
    pub assets: VoiceAssets,
    pub evidence: SourceEvidence,
}

pub fn selected_voice_assets(paths: &[PathBuf]) -> Result<VoiceImportDraft, String> {
    let mut assets = VoiceAssets::default();
    let (mut token, mut model) = (None, None);
    for path in paths {
        let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("请选择实际文件与声库目录，不能使用符号链接".into());
        }
        let slot = if metadata.is_dir() {
            let files=verdant_backend::voices::package_files(path).map_err(|error|error.to_string())?;
            if !path.join(files[0].1).is_file() { return Err("所选目录没有 verdant-voice 声库清单".into()); }
            &mut assets.voice_package
        } else if metadata.is_file() {
            match path.extension().and_then(|v| v.to_str()).map(str::to_ascii_lowercase).as_deref() {
                Some("tkn") => &mut token,
                Some("bin") => &mut model,
                _ => return Err("请选择 verdant-voice 声库目录及来源 .tkn 和 .bin".into()),
            }
        } else {
            return Err("资产必须是普通文件或声库目录".into());
        };
        if slot.replace(path.clone()).is_some() {
            return Err("每种资产只能选择一份；尚未提交安装".into());
        }
    }
    if assets.voice_package.is_none() { return Err("请选择一个 verdant-voice 声库目录".into()); }
    Ok(VoiceImportDraft {
        assets,
        evidence: SourceEvidence {
            token: token.ok_or("还需要原始 .tkn 文件")?,
            model: model.ok_or("还需要原始 .bin 文件")?,
            rights_attested: false,
        },
    })
}

/// New editor documents use the shared current render defaults.
pub fn new_project() -> Project {
    Project::new_empty()
}

impl Editor {
    pub(super) fn install_voice_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Result<(), String> {
        if self.worker.is_none() { return Err("等待本地后端就绪，再安装声库".into()); }
        // Native pickers often cannot select directories and regular files in
        // one operation. Keep the two explicit steps portable across backends.
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false, directories: true, multiple: false,
            prompt: Some("选择 verdant-voice 声库目录".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await {
                let Ok(sources) = cx.update(|_, cx| cx.prompt_for_paths(gpui::PathPromptOptions {
                    files: true, directories: false, multiple: true,
                    prompt: Some("选择您持有的来源 .tkn 与 .bin 文件".into()),
                })) else { return; };
                let Ok(Ok(Some(originals))) = sources.await else { return; };
                paths.extend(originals);
                let result = cx.background_spawn(async move { selected_voice_assets(&paths) }).await;
                let _ = cx.update(|window, cx| this.update(cx, |ed, cx| match result {
                    Ok(draft) => ed.prompt_text("导入 verdant-voice 声库：稳定引用 ID | 显示名称",
                        PromptPurpose::Integration(integrations::IntegrationPrompt::InstallVoice(draft)),
                        "", window, cx),
                    Err(error) => { ed.notice(error); cx.notify(); }
                }));
            }
        }).detach();
        Ok(())
    }

    pub(super) fn submit_voice_import(&mut self, draft: &VoiceImportDraft, text: &str) -> Result<(), String> {
        let fields = text.split('|').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 2 || fields.iter().any(|s| s.is_empty()) {
            return Err("填写 稳定引用 ID | 显示名称".into());
        }
        if !draft.evidence.rights_attested {
            return Err("请先确认您有权使用这些原始数据及其迁移声库".into());
        }
        let worker = self.worker.as_ref().ok_or("后端尚未就绪，未提交安装")?;
        let request = verdant_backend::Request::admit_voice_package(
            format!("admit-voice-{}", NoteId::generate()), fields[0].into(), fields[1].into(),
            draft.assets.clone(), draft.evidence.clone(),
        );
        worker.send(&request).map_err(|error| format!("安装请求发送失败：{error}"))?;
        self.notice("正在验证 verdant-voice 声库和来源文件");
        Ok(())
    }

    pub fn voice_import_details(&self, cx: &mut Context<Self>) -> Option<Div> {
        let prompt = self.text_prompt.as_ref()?;
        let PromptPurpose::Integration(integrations::IntegrationPrompt::InstallVoice(draft)) =
            &prompt.purpose else { return None; };
        let th = self.theme;
        let accepted = draft.evidence.rights_attested;
        let instance = prompt.instance.clone();
        let basis = crate::proto_controls::editor_authority(self);
        let authority = format!("{basis}:{instance}:{:?}:{:?}:{:?}", draft.assets.voice_package, draft.evidence.token, draft.evidence.model);
        let label = "我有权使用这些原始数据及其迁移声库";
        Some(div().flex().flex_col().gap(px(6.)).text_xs()
            .child(format!("声库：{}", draft.assets.voice_package.as_ref()?.display()))
            .child(format!("原始 TKN：{}", draft.evidence.token.display()))
            .child(format!("原始 BIN：{}", draft.evidence.model.display()))
            .child(div().text_color(th.text_secondary).child("安装会核对文件内容并保存来源记录。原始文件匹配与您的使用权声明分别记录。"))
            .child(crate::proto_controls::action("voice-rights-attestation", label).role(Role::CheckBox)
                .authority(authority)
                .aria_selected(accepted).tab_index(0).p(px(8.)).rounded(px(5.)).bg(th.control)
                .child(format!("{} {label}", if accepted { "☑" } else { "☐" }))
                .on_click(cx.listener(move |ed, event: &crate::proto_controls::Activation, _, cx| {
                    if crate::proto_controls::editor_authority(ed) != basis { return; }
                    let Some(prompt) = &mut ed.text_prompt else { return; };
                    if prompt.instance != instance { return; }
                    if let PromptPurpose::Integration(integrations::IntegrationPrompt::InstallVoice(draft)) = &mut prompt.purpose {
                        if draft.evidence.rights_attested != accepted { return; }
                        draft.evidence.rights_attested = event.selected();
                        cx.notify();
                    }
                }))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_missing_route_documents_use_the_current_default() {
        let project = new_project();
        assert_eq!(project.render_preferences.route, "verdant-voice");
        let mut old = serde_json::to_value(Project::new_empty()).unwrap();
        old.as_object_mut().unwrap().remove("render_preferences");
        let restored: Project = serde_json::from_value(old).unwrap();
        assert_eq!(restored.render_preferences.route, "verdant-voice");
    }

    #[test]
    fn canonical_import_recognizes_the_root_manifest_and_rejects_legacy_layouts() {
        let root=std::env::temp_dir().join(format!("verdant-voice-ui-{}",NoteId::generate()));
        let bank=root.join("bank");std::fs::create_dir_all(&bank).unwrap();
        let token=root.join("original.tkn");let model=root.join("original.bin");
        std::fs::write(&token,b"source token").unwrap();std::fs::write(&model,b"source model").unwrap();
        std::fs::write(bank.join("voice-source.json"),b"{}").unwrap();
        let paths=[bank.clone(),token,model];
        assert!(selected_voice_assets(&paths).is_err());
        let extension=bank.join("verdant-voice");std::fs::create_dir(&extension).unwrap();
        std::fs::write(extension.join("manifest.json"),b"{}\n").unwrap();
        assert!(selected_voice_assets(&paths).is_err());
        let artifact = |file: &str| {
            std::fs::write(bank.join(file), file.as_bytes()).unwrap();
            serde_json::json!({"file":file,"bytes":file.len(),"sha256":verdant_domain::digest::sha256_hex(file.as_bytes())})
        };
        let artifacts = verdant_backend::voices::PACKAGE_FILES[1..].iter().map(|(role, file)| ((*role).to_string(), artifact(file)))
            .collect::<serde_json::Map<_, _>>();
        let manifest=serde_json::json!({"schema":"verdant-voice-package/2","name":"verdant-voice",
            "version":env!("CARGO_PKG_VERSION"),"strategy":verdant_backend::voices::VOICE_STRATEGY,
            "artifacts":artifacts,"records":{"notice":artifact("NOTICE.txt"),"authorization":artifact("AUTHORIZATION.json")}});
        std::fs::write(bank.join("manifest.json"),serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(!selected_voice_assets(&paths).unwrap().evidence.rights_attested);
        assert!(selected_voice_assets(&paths[..2]).is_err());
        std::fs::remove_file(bank.join("manifest.json")).unwrap();
        assert!(selected_voice_assets(&paths).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
