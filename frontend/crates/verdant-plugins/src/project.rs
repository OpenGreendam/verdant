//! Project-format bridge: `track.unknown["inserts"]`.
//!
//! PLUGIN-HOST.md writes `track.extensions`; `verdant_domain::Track` carries
//! serde-flattened `unknown: Map<String, Value>` instead, so the `inserts` key
//! lands there and round-trips byte-identically with everything else.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Key inside `track.unknown` holding the array of insert specs.
pub const INSERTS_KEY: &str = "inserts";

/// One persisted insert slot. Unknown keys are preserved verbatim in `extra`
/// so future host data written by newer Verdant versions survives a
/// save→load cycle untouched.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InsertSpec {
    /// Persistent slot identity, distinct from the format's plugin `id` and
    /// the live process's `InsertKey`. New inserts receive a UUID. Legacy
    /// documents retain an empty id until the domain/app journals migration;
    /// reading a document must never invent an unrecorded automation target.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub instance_id: String,
    /// `clap` | `vst3` | `au` | `builtin`.
    pub format: String,
    /// Format-specific plugin identifier (CLAP id, VST3 class id, AU fourcc
    /// triple `type:sub:manu`).
    pub id: String,
    /// Display name at the time the insert was saved.
    pub name: String,
    /// Plugin version string at save time (informational).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    /// Last known install location. Only a relocation hint: restoring finds
    /// the plugin by (`format`, `id`) in the catalog if `path` moved.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Opaque plugin state blob, base64. Kept verbatim even when the plugin is
    /// missing — a missing insert must never lose its state.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub state_b64: String,
    /// User bypass flag; persisted, not tied to crash/missing status.
    #[serde(default)]
    pub bypass: bool,
    /// Anything newer code wrote that this build doesn't know.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl InsertSpec {
    pub fn new(
        format: impl Into<String>,
        id: impl Into<String>,
        name: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            format: format.into(),
            id: id.into(),
            name: name.into(),
            version: String::new(),
            path: path.into(),
            state_b64: String::new(),
            bypass: false,
            extra: Map::new(),
        }
    }
}

/// Read all insert specs from a track's `unknown` map. Malformed entries are
/// dropped rather than failing the whole track (a corrupt insert must not
/// block project open).
pub fn read_inserts(unknown: &Map<String, Value>) -> Vec<InsertSpec> {
    unknown
        .get(INSERTS_KEY)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value::<InsertSpec>(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Write insert specs back. Removes the key entirely when empty so files stay
/// clean. All other keys in `unknown` are left untouched.
pub fn write_inserts(unknown: &mut Map<String, Value>, specs: &[InsertSpec]) {
    if specs.is_empty() {
        unknown.remove(INSERTS_KEY);
        return;
    }
    let arr = specs
        .iter()
        .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
        .collect();
    unknown.insert(INSERTS_KEY.to_string(), Value::Array(arr));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_unknown_keys() {
        let mut spec = InsertSpec::new("clap", "org.example.reverb", "Reverb", "/x/y.clap");
        spec.version = "1.2.0".into();
        spec.state_b64 = "QUJD".into();
        spec.extra
            .insert("future_field".into(), Value::from(serde_json::json!({"a": [1, 2]})));

        let mut unknown: Map<String, Value> = Map::new();
        unknown.insert("unrelated".into(), Value::from("keep me"));
        write_inserts(&mut unknown, &[spec.clone()]);

        let back = read_inserts(&unknown);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0], spec);
        assert_eq!(
            back[0].extra.get("future_field"),
            Some(&serde_json::json!({"a": [1, 2]}))
        );
        assert_eq!(unknown.get("unrelated"), Some(&Value::from("keep me")));
    }

    #[test]
    fn empty_write_removes_key() {
        let mut unknown: Map<String, Value> = Map::new();
        write_inserts(&mut unknown, &[InsertSpec::new("clap", "a", "b", "c")]);
        assert!(unknown.contains_key(INSERTS_KEY));
        write_inserts(&mut unknown, &[]);
        assert!(!unknown.contains_key(INSERTS_KEY));
    }

    #[test]
    fn malformed_entries_are_skipped() {
        let mut unknown: Map<String, Value> = Map::new();
        unknown.insert(
            INSERTS_KEY.into(),
            Value::Array(vec![
                serde_json::json!(42),
                serde_json::json!({"format": "clap", "id": "x", "name": "n"}),
            ]),
        );
        let specs = read_inserts(&unknown);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].id, "x");
        assert!(specs[0].instance_id.is_empty());
        // Legacy identity migration belongs to a journalled domain command.
        let value = serde_json::to_value(&specs[0]).unwrap();
        assert!(value.get("instance_id").is_none());
    }
}
