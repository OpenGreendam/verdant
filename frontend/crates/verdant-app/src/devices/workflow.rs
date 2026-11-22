//! Local editing workspace and portable presets for built-in devices.
//! Neither preference metadata nor file I/O enters the audio callback.
use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub(super) mod library;
pub(crate) mod instances;
pub(in crate::devices) mod text_field;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Workspace {
    pub comparison: Comparison,
    pub wet_lock: bool,
    pub matrix_open: bool,
    pub modulation_advanced_open: bool,
    pub sequencer_step: usize,
    /// Local presentation only; never changes processing or parameter history.
    pub editor_page: u8,
    pub expanded_sections: std::collections::BTreeSet<String>,
    pub reference: Option<Reference>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Reference { pub identity: String, pub name: String }

pub(crate) fn reference_handle(ed: &Editor, owner: &DeviceTarget, slot: usize, key: InsertKey) -> Option<std::sync::Arc<verdant_plugins::InsertHandle>> {
    let reference = read_workspace(ed, &Target { track: owner.clone(), slot, key }).reference?;
    let mut owners: Vec<_> = ed.project().tracks.iter().map(|t| DeviceTarget::Track(t.id.clone())).collect();
    owners.push(DeviceTarget::Master);
    for owner in owners {
        for slot in 0..ed.device_insert_specs(&owner).len() {
            let Some(handle) = ed.live_device_insert(&owner, slot) else { continue };
            if handle.key() == key { continue; }
            let target = Target { track: owner.clone(), slot, key: handle.key() };
            if workspace_id(ed, &target).as_deref() == Some(&reference.identity) { return Some(handle); }
        }
    }
    None
}

/// A unique UUID follows an instance across slot moves and host rebuilds.
/// Ambiguous historical slots deliberately remain tied to their live process.
pub(super) fn workspace_id(ed: &Editor, target: &Target) -> Option<String> {
    if !ed.device_target_matches(&target.track, target.slot, target.key) { return None; }
    let owner = match &target.track { DeviceTarget::Track(t) => format!("track/{t}"), DeviceTarget::Master => "master".into() };
    let identity = if target.slot == crate::instrument::INSTRUMENT_SLOT {
        let DeviceTarget::Track(t) = &target.track else { return None };
        let id = ed.project().track(t)?.instrument()?.get("id")?.as_str()?;
        format!("instrument/{id}")
    } else {
        let specs = ed.device_insert_specs(&target.track);
        let spec = specs.get(target.slot)?;
        if !spec.instance_id.is_empty() && specs.iter().filter(|s| s.instance_id == spec.instance_id).count() == 1 {
            format!("{}/{}/{}", spec.format, spec.id, spec.instance_id)
        } else { format!("live/{}/{}/{}", ed.document_generation, target.key.0, spec.id) }
    };
    Some(format!("{}/{owner}/{identity}", ed.project().project_id))
}

pub(super) fn target_for_key(ed: &Editor, key: InsertKey) -> Option<Target> {
    for track in &ed.project().tracks {
        let owner = DeviceTarget::Track(track.id.clone());
        if let Some(id) = track.instrument().and_then(|s| s.get("id")).and_then(serde_json::Value::as_str) {
            if crate::instrument::instrument_key_for_generation(&track.id, id, ed.document_generation) == key {
                return Some(Target { track: owner, slot: crate::instrument::INSTRUMENT_SLOT, key });
            }
        }
        for slot in 0..ed.device_insert_specs(&owner).len() {
            if ed.live_device_insert(&owner, slot).is_some_and(|h| h.key() == key) {
                return Some(Target { track: owner, slot, key });
            }
        }
    }
    for slot in 0..ed.device_insert_specs(&DeviceTarget::Master).len() {
        if ed.live_device_insert(&DeviceTarget::Master, slot).is_some_and(|h| h.key() == key) {
            return Some(Target { track: DeviceTarget::Master, slot, key });
        }
    }
    None
}

pub(super) fn read_workspace(ed: &Editor, target: &Target) -> Workspace {
    workspace_id(ed, target).and_then(|id| ed.prefs.extra.get("device_workspaces")?.get(id).cloned())
        .and_then(|v| serde_json::from_value::<Workspace>(v).ok()).map(|mut state| {
            // Transactions never survive a process restart. Only confirmed A/B
            // selection is durable, while the two snapshots are retained.
            state.comparison.pending = None;
            state.comparison.active = state.comparison.active.min(1);
            state
        }).unwrap_or_default()
}

pub(super) fn write_workspace(ed: &mut Editor, target: &Target, state: &Workspace) {
    let Some(id) = workspace_id(ed, target) else { return };
    let Ok(value) = serde_json::to_value(state) else { return };
    let map = ed.prefs.extra.entry("device_workspaces").or_insert_with(|| serde_json::json!({}));
    if !map.is_object() { *map = serde_json::json!({}); }
    if map.get(&id) != Some(&value) { map[id] = value; ed.prefs_dirty = true; }
}

/// Move only an in-flight comparison for the same logical instance. This
/// state stays in memory: a process/document restart never restores a pending
/// request, and a different project, owner or instance cannot inherit it.
fn take_pending_comparison(states: &mut std::collections::HashMap<InsertKey, DeviceAnim>, identity: &str, key: InsertKey) -> Option<Comparison> {
    let previous = states.iter().find_map(|(old_key, state)| {
        (*old_key != key && state.workspace_identity.as_deref() == Some(identity)
            && state.comparison.pending.is_some()).then_some(*old_key)
    })?;
    let state = states.get_mut(&previous)?;
    let comparison = state.comparison.clone();
    state.comparison.pending = None;
    Some(comparison)
}

pub(super) fn restore_and_reconcile(ed: &mut Editor, key: InsertKey, params: &[ParamDesc], snap: bool) {
    let Some(target) = target_for_key(ed, key) else { return };
    let Some(identity) = workspace_id(ed, &target) else { return };
    if !ed.device_anim.contains_key(&key) {
        let mut state = DeviceAnim::settled(params.iter().map(|d| d.value).collect(), Instant::now());
        state.comparison = take_pending_comparison(&mut ed.device_anim, &identity, key)
            .unwrap_or_else(|| read_workspace(ed, &target).comparison);
        state.workspace_identity = Some(identity.clone());
        ed.device_anim.insert(key, state);
    }
    if let Some(state) = ed.device_anim.get_mut(&key) { state.workspace_identity = Some(identity); }
    if snap { return; }
    let busy = ed.plugin_busy.contains(&key)
        || (target.slot != crate::instrument::INSTRUMENT_SLOT && ed.inserts_loading);
    let state = ed.device_anim.get_mut(&key).unwrap();
    let pending = state.comparison.pending.is_some();
    let current: Vec<_> = params.iter().map(|p| (p.id.clone(), p.value)).collect();
    state.comparison.reconcile(&current, busy);
    if pending && state.comparison.pending.is_none() { save_comparison(ed, &target); }
}

/// Older workspaces may predate appended native controls. A bank always
/// becomes a complete snapshot in the current descriptor order, with new
/// parameters set to their defaults rather than leaking from the other bank.
pub(super) fn comparison_values(params: &[ParamDesc], saved: &[(String, f64)]) -> Result<Vec<(String, f64)>, String> {
    preset_values(params, &saved.iter().cloned().collect(), false)
}

pub(super) fn save_comparison(ed: &mut Editor, target: &Target) {
    let Some(comparison) = ed.device_anim.get(&target.key).map(|a| a.comparison.clone()) else { return };
    let mut state = read_workspace(ed, target);
    state.comparison = comparison;
    // A failed/in-flight write must not become a selected bank after restart.
    state.comparison.pending = None;
    write_workspace(ed, target, &state);
}

pub(super) fn preset_values(params: &[ParamDesc], values: &BTreeMap<String, f64>, wet_lock: bool) -> Result<Vec<(String, f64)>, String> {
    for (id, value) in values {
        let Some(desc) = param(params, id) else { return Err(format!("预设含未知参数：{id}")) };
        let validated = verdant_plugins::insert::parameter_editor_value(desc, *value).map_err(|e| e.to_string())?;
        if validated != *value { return Err(format!("预设参数不是有效离散值：{id}")); }
    }
    Ok(params.iter().filter_map(|d| {
        if wet_lock && d.id == "mix" { Some((d.id.clone(), d.value)) }
        else { values.get(&d.id).copied().or(d.default_value).map(|v| (d.id.clone(), v)) }
    }).collect())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PresetFile {
    pub format: String,
    pub version: u32,
    pub device: String,
    pub name: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub origin_factory_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    pub parameters: BTreeMap<String, f64>,
}

impl PresetFile {
    pub fn new(device: &str, name: &str, tags: &str, params: &[ParamDesc]) -> Result<Self, String> {
        let mut result = Self { format: "verdant-device-preset".into(), version: 1, device: device.into(),
            name: name.trim().into(), tags: tags.split([',', '，', ';', '；']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect(),
            category: String::new(), author: String::new(), description: String::new(), origin_factory_id: None,
            favorite: false, parameters: params.iter().map(|d| (d.id.clone(), d.value)).collect() };
        result.tags.sort(); result.tags.dedup();
        result.validate(device, params)?;
        Ok(result)
    }
    pub fn validate(&self, device: &str, params: &[ParamDesc]) -> Result<(), String> {
        if self.format != "verdant-device-preset" || self.version != 1 { return Err("不是支持的 Verdant 预设文件版本".into()); }
        if self.device != device { return Err(format!("预设属于 {}，当前设备为 {device}", self.device)); }
        if self.name.trim().is_empty() || self.name.chars().count() > 120 { return Err("预设名称须为 1–120 个字符".into()); }
        if self.tags.len() > 16 || self.tags.iter().any(|t| t.chars().count() > 40 || t.trim().is_empty()) { return Err("最多 16 个标签，每个最多 40 个字符".into()); }
        if self.category.chars().count() > 80 || self.author.chars().count() > 120 || self.description.chars().count() > 2000 || self.origin_factory_id.as_ref().is_some_and(|id| id.chars().count() > 160) { return Err("预设类别、作者或说明过长".into()); }
        if self.parameters.is_empty() { return Err("预设没有参数".into()); }
        preset_values(params, &self.parameters, false).map(|_| ())
    }
    pub fn matches(&self, query: &str) -> bool {
        let text = format!("{} {} {} {} {}", self.name, self.category, self.author, self.description, self.tags.join(" ")).to_lowercase();
        query.to_lowercase().split_whitespace().all(|word| text.contains(word))
    }
}

pub(super) fn preset_dir(device: &str) -> PathBuf {
    // Device IDs only name an allowlisted built-in family; never a path from
    // an imported preset file.
    let safe: String = device.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    crate::files::state_dir().join("plugin-presets").join(safe)
}

pub(super) fn read_preset(path: &Path, device: &str, params: &[ParamDesc]) -> Result<PresetFile, String> {
    let bytes = read_preset_bytes(path)?;
    let preset: PresetFile = serde_json::from_slice(&bytes).map_err(|e| format!("预设 JSON 无效：{e}"))?;
    preset.validate(device, params)?;
    Ok(preset)
}

pub(super) fn read_preset_bytes(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|e| e.to_string())?
        .take(1_048_577).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 { return Err("预设文件超过 1 MiB".into()); }
    Ok(bytes)
}

pub(super) fn save_preset(path: &Path, preset: &PresetFile) -> Result<(), String> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let bytes = serde_json::to_vec_pretty(preset).map_err(|e| e.to_string())?;
    crate::files::atomic_bytes(path, &bytes)
}

pub(super) fn fresh_path(device: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    preset_dir(device).join(format!("{stamp}-{}.vpreset", NEXT.fetch_add(1, Ordering::Relaxed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descs() -> Vec<ParamDesc> {
        vec![ParamDesc { id: "mix".into(), name: "Mix".into(), value: 23., min: 0., max: 100., unit: "%".into(), step_count: 0, default_value: Some(100.) },
            ParamDesc { id: "shape".into(), name: "Shape".into(), value: 1., min: 0., max: 3., unit: "".into(), step_count: 3, default_value: Some(0.) }]
    }
    #[test] fn preset_roundtrip_is_strict_and_mix_lock_applies_only_when_loading() {
        let params = descs();
        let preset = PresetFile::new("verdant.builtin.filter", "温暖 人声", "voice, warm, voice", &params).unwrap();
        let decoded: PresetFile = serde_json::from_str(&serde_json::to_string(&preset).unwrap()).unwrap();
        decoded.validate("verdant.builtin.filter", &params).unwrap();
        let mut legacy = serde_json::to_value(&decoded).unwrap();
        legacy.as_object_mut().unwrap().remove("favorite");
        assert!(!serde_json::from_value::<PresetFile>(legacy).unwrap().favorite);
        let mut favorite = decoded.clone(); favorite.favorite = true;
        assert!(serde_json::from_str::<PresetFile>(&serde_json::to_string(&favorite).unwrap()).unwrap().favorite);
        assert_eq!(decoded.tags, ["voice", "warm"]);
        assert!(decoded.matches("warm 人声")); assert!(!decoded.matches("bright"));
        assert!(decoded.validate("verdant.builtin.eq", &params).is_err());
        let values = BTreeMap::from([("mix".into(), 70.)]);
        assert_eq!(preset_values(&params, &values, true).unwrap(), vec![("mix".into(), 23.), ("shape".into(), 0.)]);
        assert_eq!(preset_values(&params, &values, false).unwrap()[0].1, 70.);
        assert!(preset_values(&params, &BTreeMap::from([("shape".into(), 1.5)]), false).is_err());
        assert!(preset_values(&params, &BTreeMap::from([("missing".into(), 1.)]), false).is_err());
        assert!(preset_values(&params, &BTreeMap::from([("mix".into(), f64::NAN)]), false).is_err());
    }
    #[test] fn preset_metadata_remains_compatible_with_legacy_v1_and_is_searchable() {
        let params = descs();
        let preset = PresetFile::new("verdant.builtin.filter", "Warm", "voice", &params).unwrap();
        let mut old = serde_json::to_value(&preset).unwrap();
        for key in ["category", "author", "description", "origin_factory_id", "favorite"] { old.as_object_mut().unwrap().remove(key); }
        let mut restored: PresetFile = serde_json::from_value(old).unwrap();
        restored.validate("verdant.builtin.filter", &params).unwrap();
        assert!(restored.category.is_empty()); assert!(restored.origin_factory_id.is_none());
        restored.category = "Vocal".into(); restored.author = "Verdant".into(); restored.description = "Tame harsh upper mids".into();
        restored.origin_factory_id = Some("filter-vocal-warm".into());
        assert!(restored.matches("vocal verdant harsh"));
        let roundtrip: PresetFile = serde_json::from_value(serde_json::to_value(restored.clone()).unwrap()).unwrap();
        assert_eq!(roundtrip.origin_factory_id, restored.origin_factory_id);
        restored.description = "x".repeat(2001); assert!(restored.validate("verdant.builtin.filter", &params).is_err());
    }
    #[test] fn comparison_migrates_old_banks_to_current_descriptors_before_commit() {
        let params = descs();
        let old = vec![("mix".into(), 70.)];
        let next = comparison_values(&params, &old).unwrap();
        assert_eq!(next, vec![("mix".into(), 70.), ("shape".into(), 0.)]);
        let mut comparison = Comparison { active: 0, slots: [Some(vec![("mix".into(), 23.), ("shape".into(), 1.)]), Some(next.clone())], pending: Some(1) };
        comparison.reconcile(&next, false);
        assert_eq!(comparison.active, 1);
        assert!(comparison.pending.is_none());
        assert!(comparison_values(&params, &[("shape".into(), 1.5)]).is_err());
        assert!(comparison_values(&params, &[("removed".into(), 1.)]).is_err());
    }
    #[test] fn comparison_pending_follows_only_the_same_live_logical_instance() {
        let a = vec![("mix".into(), 10.)]; let b = vec![("mix".into(), 80.)];
        let mut previous = DeviceAnim::settled(vec![10.], Instant::now());
        previous.workspace_identity = Some("project/track/instance".into());
        previous.comparison = Comparison { active: 0, slots: [Some(a), Some(b.clone())], pending: Some(1) };
        let mut states = std::collections::HashMap::from([(InsertKey(10), previous)]);
        assert!(take_pending_comparison(&mut states, "project/track/different", InsertKey(11)).is_none());
        assert_eq!(states[&InsertKey(10)].comparison.pending, Some(1));
        let mut moved = take_pending_comparison(&mut states, "project/track/instance", InsertKey(11)).unwrap();
        assert!(states[&InsertKey(10)].comparison.pending.is_none());
        moved.reconcile(&b, true); assert_eq!(moved.active, 0);
        moved.reconcile(&b, false); assert_eq!(moved.active, 1);
        assert!(take_pending_comparison(&mut states, "project/track/instance", InsertKey(12)).is_none());
    }
    #[test] fn comparison_workspace_serializes_both_confirmed_banks() {
        let state = Workspace { comparison: Comparison { active: 1, slots: [Some(vec![("mix".into(), 10.)]), Some(vec![("mix".into(), 80.)])], pending: None }, wet_lock: true, matrix_open: false, modulation_advanced_open: false, sequencer_step: 0, reference: None, ..Default::default() };
        let restored: Workspace = serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap();
        assert_eq!(restored.comparison.active, 1);
        assert_eq!(restored.comparison.slots[0].as_ref().unwrap()[0].1, 10.);
        assert!(restored.wet_lock);
    }
}
