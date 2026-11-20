//! Verdant editor session: everything interactive that is *not* project truth.

mod bindings;
pub mod hit;
pub mod lyric;
pub mod session;
pub mod viewport;

pub use hit::{Hit, hit_test, hit_test_touch, hit_test_with};
pub use lyric::{LyricOutcome, LyricSession, TextEditState, parse_lyric_input};
pub use session::{CancelReason, EditorSession, Effect, Gesture, GestureKind, InputEvent, Modifiers, Phase, PointerId, Preview};
pub use viewport::{MusicPoint, Rect, Snap, Viewport};
