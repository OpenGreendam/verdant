use verdant_domain::Project;
use verdant_exchange::{Limits, parse_project_json};

#[test]
fn complete_native_shaped_foreign_identity_is_rejected_by_both_readers() {
    let ordinary = serde_json::to_value(Project::new_empty()).unwrap();
    for (field, value) in [
        ("format", serde_json::json!("unrelated-project")),
        ("version", serde_json::json!("1.0")),
        ("version", serde_json::json!("")),
        ("project_id", serde_json::json!("")),
        ("revision", serde_json::json!(9_007_199_254_740_992u64)),
    ] {
        let mut invalid = ordinary.clone();
        invalid[field] = value;
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(parse_project_json(&bytes, &Limits::default()).is_err(), "accepted invalid {field}");
        assert!(verdant_domain::io::parse_project(std::str::from_utf8(&bytes).unwrap()).is_err());
    }
}

#[test]
fn native_schema_gate_keeps_unknown_fields_duplicate_rejection_and_resource_limits() {
    let mut project = Project::new_empty();
    project.version = "0.2".into();
    project.unknown.insert("future_note".into(), serde_json::json!({"keep":true}));
    let bytes = serde_json::to_vec(&project).unwrap();
    let accepted = parse_project_json(&bytes, &Limits::default()).unwrap();
    assert_eq!(accepted.version, "0.2");
    assert_eq!(accepted.unknown["future_note"]["keep"], true);
    assert_eq!(accepted, verdant_domain::io::parse_project(std::str::from_utf8(&bytes).unwrap()).unwrap().0);
    let duplicate = format!("{{\"format\":\"greendam-project\",{}", &std::str::from_utf8(&bytes).unwrap()[1..]);
    assert!(parse_project_json(duplicate.as_bytes(), &Limits::default()).unwrap_err().to_string().contains("duplicate JSON key"));
    let limits = Limits { max_file_bytes: bytes.len() - 1, ..Default::default() };
    assert!(parse_project_json(&bytes, &limits).unwrap_err().to_string().contains("byte limit"));
}
