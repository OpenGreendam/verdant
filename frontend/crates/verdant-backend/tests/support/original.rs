use verdant_backend::{Capabilities, voices::DESKTOP_ORIGINAL_REFERENCE};
use verdant_domain::Project;

const TOKEN:&str="821b7cd9e7b489077dd55bf581ba90e1dffd3816c6fedcedc320744b498deead";
const MODEL:&str="213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff";

pub fn project() -> Project {
    let mut project = Project::new_empty();
    project.tracks[0].render_route = Some("C0".into());
    project.tracks[0].voice_ref = Some(DESKTOP_ORIGINAL_REFERENCE.into());
    project
}

pub fn capabilities(mut value: serde_json::Value) -> Capabilities {
    value["voices"] = serde_json::json!({"version":1,"voices":{
        (DESKTOP_ORIGINAL_REFERENCE): {"available":true,"cache_identity":"original-fixture","files":{
            "token":{"available":true,"sha256":TOKEN},"model":{"available":true,"sha256":MODEL}
        }}
    }});
    serde_json::from_value(value).unwrap()
}
