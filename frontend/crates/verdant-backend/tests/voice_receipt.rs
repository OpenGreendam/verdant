//! Native voice rendering requires an exact source-import receipt.
use std::collections::BTreeMap;
use verdant_backend::voices::SourceReceipt;

fn originals() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("sn16k.tkn".into(), "821b7cd9e7b489077dd55bf581ba90e1dffd3816c6fedcedc320744b498deead".into()),
        ("sn16k.bin".into(), "213aa67a5b92fc311d524da6d9868f87df584b00e2eb9a0b9fb46822602353ff".into()),
    ])
}

fn receipt() -> SourceReceipt {
    SourceReceipt {
        schema: "verdant-voice-source-admission/1".into(),
        original_source_sha256: originals(),
        source_manifest_sha256: "a".repeat(64),
        rights_attested: true,
        attestation_is_legal_determination: false,
    }
}

#[test]
fn voice_receipt_requires_the_exact_source_manifest_and_originals() {
    let receipt = receipt();
    receipt.validate_for(&"a".repeat(64), &originals()).unwrap();
    assert!(receipt.validate_for(&"b".repeat(64), &originals()).is_err());
    let mut wrong_originals = originals();
    wrong_originals.insert("sn16k.bin".into(), "b".repeat(64));
    assert!(receipt.validate_for(&"a".repeat(64), &wrong_originals).is_err());
    assert!(receipt.validate_for(&"a".repeat(64), &BTreeMap::new()).is_err());
}

#[test]
fn graph_metadata_cannot_create_a_source_use_attestation() {
    for field in ["rights", "legal", "schema", "source"] {
        let mut receipt = receipt();
        match field {
            "rights" => receipt.rights_attested = false,
            "legal" => receipt.attestation_is_legal_determination = true,
            "schema" => receipt.schema = "native48-export/1".into(),
            "source" => { receipt.original_source_sha256.remove("sn16k.bin"); },
            _ => unreachable!(),
        }
        assert!(receipt.validate_for(&"a".repeat(64), &originals()).is_err());
    }
}
