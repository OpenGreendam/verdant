//! Content digests. A digest identifies musical content, never UI state
//! (zoom, hover, selection) and never the revision counter (SPEC §11).

use std::io::{self, Write};

use sha2::{Digest, Sha256};

use crate::model::Project;

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Digest of the project's document content with `revision`/`status` zeroed.
pub fn project_content_digest(p: &Project) -> String {
    let mut hash = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut hash, &p.document(0, None)).expect("project serializes");
    hex::encode(hash.0.finalize())
}

struct DigestWriter(Sha256);

impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
