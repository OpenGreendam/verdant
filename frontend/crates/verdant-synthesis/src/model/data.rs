//! Original predicate tables, admitted once and shared by every loaded voice.
use std::{collections::BTreeMap, sync::OnceLock};
use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const CURRENT: &[u8] = include_bytes!("data/original_context_profile.json");
const EXTENSIONS: &[u8] = include_bytes!("data/original_context_extensions.json");
const HIERARCHY: &[u8] = include_bytes!("data/original_context_hierarchy.json");
const POS: &[u8] = include_bytes!("data/original_context_pos.json");
const INVENTORY: &[u8] = include_bytes!("../../../verdant-backend/src/data/phoneme_inventory.json");
const MANIFEST: &[u8] = include_bytes!("data_manifest.json");

pub(super) fn digest(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    PhoneId, PhoneGroup, Silence, MandarinVoiced, Language, Tone, Stress,
    OnsetCoda, PronunciationStyle, Hierarchy, PosLiteral, PosGroup, PosAmbiguous,
}
#[derive(Debug, Deserialize)]
pub(super) struct Spec {
    pub kind: Kind,
    pub argument1: i32,
    pub argument2: i32,
    pub comparison_mode: i32,
    pub rhs: i32,
    pub function_rva: u32,
    #[serde(default = "current_position")]
    pub position: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub feature_key: String,
}
fn current_position() -> String { "C".into() }
#[derive(Debug, Deserialize)]
pub(super) struct PhoneRow {
    pub name: String,
    pub first_class: i32,
    pub groups: Vec<i32>,
}
#[derive(Debug, Deserialize)]
pub(super) struct NativePhone {
    pub phone_id: i32,
    pub native_language: String,
    pub native_phone: String,
    pub role: String,
    pub voiced: bool,
}
#[derive(Debug)]
pub(super) struct Profiles {
    pub questions: BTreeMap<String, Spec>,
    pub phones: BTreeMap<i32, PhoneRow>,
    pub mandarin: BTreeMap<String, i32>,
    pub english: BTreeMap<String, i32>,
    pub pos_groups: BTreeMap<i32, Vec<String>>,
    pub native_phones: BTreeMap<i32, (String, NativePhone)>,
    pub manifest: Value,
}
impl Profiles {
    fn load() -> Result<Self> {
        let manifest: Value = serde_json::from_slice(MANIFEST)?;
        let sources = [
            ("original_context_profile.json", CURRENT, "ogd-original-current-context/1"),
            ("original_context_extensions.json", EXTENSIONS, "ogd-original-explicit-context-extensions/1"),
            ("original_context_hierarchy.json", HIERARCHY, "ogd-original-explicit-hierarchy-predicates/1"),
            ("original_context_pos.json", POS, "ogd-original-explicit-pos-predicates/1"),
        ];
        let mut values = Vec::with_capacity(sources.len());
        for (name, bytes, schema) in sources {
            ensure!(manifest["files"][name]["sha256"].as_str() == Some(digest(bytes).as_str()), "embedded source fingerprint mismatch: {name}");
            let value: Value = serde_json::from_slice(bytes).with_context(|| format!("invalid embedded profile {name}"))?;
            ensure!(value["schema_version"] == schema, "unsupported embedded profile {name}");
            values.push(value);
        }
        let (base, extension, hierarchy, pos) = (&values[0], &values[1], &values[2], &values[3]);
        ensure!(extension["base_current_profile_sha256"].as_str() == Some(digest(CURRENT).as_str()), "original context base profile mismatch");
        for value in [extension, hierarchy, pos] {
            ensure!(value["source_module_sha256"] == base["source_module_sha256"] && value["restored_memory_sha256"] == base["restored_memory_sha256"], "original predicate sources do not match");
        }
        for (name, bytes) in [("original_context_profile.json", CURRENT), ("original_context_extensions.json", EXTENSIONS)] {
            ensure!(hierarchy["base_profiles_sha256"][name].as_str() == Some(digest(bytes).as_str()), "original hierarchy base profile mismatch");
        }
        let mut questions = BTreeMap::new();
        for value in &values {
            for (name, spec) in value["questions"].as_object().context("profile has no questions")? {
                let spec: Spec = serde_json::from_value(spec.clone())?;
                ensure!(matches!(spec.comparison_mode, 0 | 4), "unverified original comparison mode");
                ensure!(matches!(spec.position.as_str(), "C" | "L" | "R"), "unsupported original context position");
                ensure!(questions.insert(name.clone(), spec).is_none(), "original context extension replaces {name}");
            }
        }
        let mut phones = BTreeMap::new();
        let mut mandarin = BTreeMap::new();
        let mut english = BTreeMap::new();
        for (id, row) in base["phones"].as_object().context("profile has no phones")? {
            let id: i32 = id.parse()?;
            let row: PhoneRow = serde_json::from_value(row.clone())?;
            if (3..=63).contains(&id) { ensure!(mandarin.insert(row.name.clone(), id).is_none(), "duplicate Mandarin phone name"); }
            if (174..=235).contains(&id) { ensure!(english.insert(row.name.clone(), id).is_none(), "duplicate English phone name"); }
            ensure!(phones.insert(id, row).is_none(), "duplicate original phone ID");
        }
        let mut pos_groups = BTreeMap::new();
        for (id, members) in pos["groups"].as_object().context("POS profile has no groups")? {
            pos_groups.insert(id.parse()?, serde_json::from_value(members.clone())?);
        }
        ensure!(manifest["files"]["phoneme_inventory.json"]["sha256"].as_str() == Some(digest(INVENTORY).as_str()), "phoneme inventory fingerprint mismatch");
        let inventory: Value = serde_json::from_slice(INVENTORY)?;
        ensure!(inventory["schema_version"] == "verdant-phonemes/2", "unsupported phoneme inventory");
        let mut native_phones = BTreeMap::new();
        for (key, row) in inventory["native_phones"].as_object().context("inventory has no native phones")? {
            let row: NativePhone = serde_json::from_value(row.clone())?;
            if native_phones.insert(row.phone_id, (key.clone(), row)).is_some() { bail!("duplicate native phone ID"); }
        }
        Ok(Self { questions, phones, mandarin, english, pos_groups, native_phones, manifest })
    }
}
pub(super) fn profiles() -> Result<&'static Profiles> {
    static PROFILES: OnceLock<std::result::Result<Profiles, String>> = OnceLock::new();
    match PROFILES.get_or_init(|| Profiles::load().map_err(|e| format!("{e:#}"))) {
        Ok(value) => Ok(value),
        Err(error) => bail!("{error}"),
    }
}
