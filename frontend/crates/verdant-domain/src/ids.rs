//! Opaque persistent identifiers (SPEC §4.1). IDs are UUID strings for new
//! objects; contract fixtures use short aliases such as `n1`, which are equally
//! valid opaque strings. Ordering by ID never carries time meaning.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn generate() -> Self {
                $name(uuid::Uuid::new_v4().to_string())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_string())
            }
        }
    };
}

opaque_id!(NoteId);
opaque_id!(TrackId);
opaque_id!(PointId);

/// Monotonic project revision, JSON-safe (`0..=2^53-1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Revision(u64);

impl<'de> Deserialize<'de> for Revision {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("revision exceeds 2^53-1"))
    }
}

impl Revision {
    pub const MAX: u64 = (1u64 << 53) - 1;

    pub fn new(value: u64) -> Option<Self> {
        (value <= Self::MAX).then_some(Revision(value))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    /// The next revision, or `None` on overflow (commit must be refused).
    pub fn next(self) -> Option<Self> {
        Revision::new(self.0 + 1)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}
