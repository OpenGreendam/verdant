//! Lyric text editing model behind the GPUI `EntityInputHandler` bridge
//! (SPEC §7).
//!
//! * Rust stores UTF-8 byte ranges; the platform speaks UTF-16 code units. All
//!   conversions land on char boundaries.
//! * Cursor movement and deletion step by grapheme cluster, so a ZWJ emoji, a
//!   combining sequence or a non-BMP character is never split.
//! * Marked (IME composition) text never reaches the project. The session
//!   reports [`LyricOutcome::Commit`] only when composition ends or the user
//!   confirms (Enter / Tab / blur).

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use verdant_domain::{NoteId, TrackId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextEditState {
    pub text: String,
    /// UTF-8 byte range. `start == end` is a caret.
    pub selection: Range<usize>,
    pub reversed: bool,
    /// UTF-8 byte range of IME marked text, if composing.
    pub marked: Option<Range<usize>>,
}

impl TextEditState {
    pub fn new(text: &str) -> Self {
        let end = text.len();
        TextEditState { text: text.to_string(), selection: 0..end, reversed: false, marked: None }
    }

    pub fn cursor(&self) -> usize {
        if self.reversed { self.selection.start } else { self.selection.end }
    }

    pub fn is_composing(&self) -> bool {
        self.marked.is_some()
    }

    // ---- UTF-16 <-> UTF-8 -------------------------------------------------

    pub fn offset_to_utf16(&self, offset: usize) -> usize {
        self.text[..offset.min(self.text.len())].chars().map(char::len_utf16).sum()
    }

    /// Maps a UTF-16 offset to the UTF-8 offset of the char boundary at or
    /// before it (a position inside a surrogate pair snaps back).
    pub fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut u16_count = 0;
        for (i, ch) in self.text.char_indices() {
            if u16_count + ch.len_utf16() > offset {
                return i;
            }
            u16_count += ch.len_utf16();
        }
        self.text.len()
    }

    pub fn range_to_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(r.start)..self.offset_to_utf16(r.end)
    }

    pub fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(r.start)..self.offset_from_utf16(r.end)
    }

    // ---- platform input handler operations ---------------------------------

    /// `replace_text_in_range`: inserts committed text, ending composition.
    pub fn replace_text_in_range(&mut self, range_utf16: Option<Range<usize>>, new_text: &str) {
        let range = range_utf16.map(|r| self.range_from_utf16(&r)).or(self.marked.clone()).unwrap_or(self.selection.clone());
        self.text.replace_range(range.clone(), new_text);
        let caret = range.start + new_text.len();
        self.selection = caret..caret;
        self.reversed = false;
        self.marked = None;
    }

    /// `replace_and_mark_text_in_range`: composition update.
    pub fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
    ) {
        let range = range_utf16.map(|r| self.range_from_utf16(&r)).or(self.marked.clone()).unwrap_or(self.selection.clone());
        self.text.replace_range(range.clone(), new_text);
        self.marked = if new_text.is_empty() { None } else { Some(range.start..range.start + new_text.len()) };
        self.selection = match new_selected_range_utf16 {
            // Relative to the inserted text, per the platform contract.
            Some(sel) => {
                let base = self.offset_to_utf16(range.start);
                let len = new_text.encode_utf16().count();
                self.range_from_utf16(&(base + sel.start.min(len)..base + sel.end.min(len)))
            }
            None => {
                let c = range.start + new_text.len();
                c..c
            }
        };
        self.reversed = false;
    }

    pub fn unmark_text(&mut self) {
        self.marked = None;
    }

    pub fn text_for_range_utf16(&self, r: Range<usize>) -> (String, Range<usize>) {
        let range = self.range_from_utf16(&r);
        let adjusted = self.range_to_utf16(&range);
        (self.text[range].to_string(), adjusted)
    }

    // ---- editing commands ----------------------------------------------------

    fn prev_boundary(&self, offset: usize) -> usize {
        self.text.grapheme_indices(true).rev().find_map(|(i, _)| (i < offset).then_some(i)).unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text.grapheme_indices(true).find_map(|(i, _)| (i > offset).then_some(i)).unwrap_or(self.text.len())
    }

    pub fn move_to(&mut self, offset: usize) {
        self.selection = offset..offset;
        self.reversed = false;
    }

    pub fn left(&mut self) {
        if self.selection.is_empty() {
            self.move_to(self.prev_boundary(self.cursor()));
        } else {
            self.move_to(self.selection.start);
        }
    }

    pub fn right(&mut self) {
        if self.selection.is_empty() {
            self.move_to(self.next_boundary(self.cursor()));
        } else {
            self.move_to(self.selection.end);
        }
    }

    pub fn select_to(&mut self, offset: usize) {
        if self.reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.reversed = !self.reversed;
            self.selection = self.selection.end..self.selection.start;
        }
    }

    pub fn select_left(&mut self) {
        self.select_to(self.prev_boundary(self.cursor()));
    }

    pub fn select_right(&mut self) {
        self.select_to(self.next_boundary(self.cursor()));
    }

    pub fn select_all(&mut self) {
        self.selection = 0..self.text.len();
        self.reversed = false;
    }

    pub fn home(&mut self) {
        self.move_to(0);
    }

    pub fn end(&mut self) {
        self.move_to(self.text.len());
    }

    pub fn backspace(&mut self) {
        if self.selection.is_empty() {
            self.select_to(self.prev_boundary(self.cursor()));
        }
        self.replace_text_in_range(None, "");
    }

    pub fn delete(&mut self) {
        if self.selection.is_empty() {
            self.select_to(self.next_boundary(self.cursor()));
        }
        self.replace_text_in_range(None, "");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LyricOutcome {
    None,
    /// Commit this text as one `SetLyric` transaction.
    Commit(String),
}

/// One inline lyric editing session bound to a note.
#[derive(Clone, Debug)]
pub struct LyricSession {
    pub track: TrackId,
    pub note_id: NoteId,
    pub state: TextEditState,
    /// Text last committed to the project (or the original lyric).
    committed: String,
}

impl LyricSession {
    pub fn new(track: TrackId, note_id: NoteId, current: &str) -> Self {
        LyricSession { track, note_id, state: TextEditState::new(current), committed: current.to_string() }
    }

    /// Composition finished (platform committed text). One transaction.
    pub fn on_ime_commit(&mut self, range_utf16: Option<Range<usize>>, text: &str) -> LyricOutcome {
        let was_composing = self.state.is_composing();
        self.state.replace_text_in_range(range_utf16, text);
        if was_composing { self.take_commit() } else { LyricOutcome::None }
    }

    /// Enter / Tab / focus loss. While composing, the IME owns these keys, so
    /// the caller must not route them here (SPEC §7 priority).
    pub fn confirm(&mut self) -> LyricOutcome {
        debug_assert!(!self.state.is_composing(), "IME owns confirm while composing");
        self.take_commit()
    }

    fn take_commit(&mut self) -> LyricOutcome {
        if self.state.text == self.committed {
            return LyricOutcome::None;
        }
        self.committed = self.state.text.clone();
        LyricOutcome::Commit(self.committed.clone())
    }
}

/// Parse inline lyric syntax. `啊/a` sets lyric `啊` with explicit pinyin `a`;
/// a bare ASCII syllable like `fa1` is taken as its own explicit pinyin.
/// Returns `(lyric, Some(pinyin))` only when the user typed the pinyin.
pub fn parse_lyric_input(input: &str) -> (String, Option<String>) {
    let input = input.trim();
    if let Some((lyric, py)) = input.rsplit_once('/') {
        let py = py.trim().to_lowercase();
        if is_pinyin_like(&py) {
            return (lyric.trim().to_string(), Some(py));
        }
    }
    let lower = input.to_lowercase();
    if is_pinyin_like(&lower) {
        return (input.to_string(), Some(lower));
    }
    (input.to_string(), None)
}

fn is_pinyin_like(s: &str) -> bool {
    let body = s.trim_end_matches(|c: char| ('0'..='5').contains(&c));
    if s.len() - body.len() > 1 {
        return false;
    }
    let mut chars = body.chars().peekable();
    let mut letters = 0;
    while let Some(c) = chars.next() {
        if "\u{304}\u{301}\u{30c}\u{300}\u{308}".contains(c) { continue; }
        if !c.is_ascii_lowercase() && !"üāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňǹḿ".contains(c) {
            return false;
        }
        letters += 1;
        if c == 'u' && chars.peek() == Some(&':') {
            chars.next();
        }
    }
    (1..=6).contains(&letters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_round_trip_non_bmp_and_zwj() {
        // 𝄞 is non-BMP (2 UTF-16 units), 👩‍👧 is a ZWJ sequence, é is e + U+0301.
        let s = TextEditState::new("绿𝄞e\u{301}👩\u{200d}👧");
        assert_eq!(s.offset_to_utf16(s.text.len()), 1 + 2 + 2 + 2 + 1 + 2);
        // Inside the surrogate pair snaps back to the char boundary.
        assert_eq!(s.offset_from_utf16(2), "绿".len());
        for (i, _) in s.text.char_indices() {
            assert_eq!(s.offset_from_utf16(s.offset_to_utf16(i)), i);
        }
    }

    #[test]
    fn grapheme_backspace_never_splits() {
        let mut s = TextEditState::new("ae\u{301}👩\u{200d}👧");
        s.end();
        s.backspace();
        assert_eq!(s.text, "ae\u{301}");
        s.backspace();
        assert_eq!(s.text, "a");
        let mut s = TextEditState::new("x𝄞y");
        s.move_to(1);
        s.delete();
        assert_eq!(s.text, "xy");
    }

    #[test]
    fn ime_composition_then_commit() {
        let mut l = LyricSession::new("t".into(), "n1".into(), "");
        l.state.replace_and_mark_text_in_range(None, "lü", Some(2..2));
        assert!(l.state.is_composing());
        l.state.replace_and_mark_text_in_range(None, "绿", Some(1..1));
        l.state.replace_and_mark_text_in_range(None, "绿坝", Some(2..2));
        assert_eq!(l.state.marked, Some(0.."绿坝".len()));
        assert_eq!(l.on_ime_commit(None, "绿坝"), LyricOutcome::Commit("绿坝".into()));
        assert_eq!(l.state.text, "绿坝");
        // Enter afterwards does not create a second transaction.
        assert_eq!(l.confirm(), LyricOutcome::None);
    }

    #[test]
    fn lyric_syntax() {
        assert_eq!(parse_lyric_input("啊/a"), ("啊".into(), Some("a".into())));
        assert_eq!(parse_lyric_input("fa1"), ("fa1".into(), Some("fa1".into())));
        assert_eq!(parse_lyric_input("绿坝"), ("绿坝".into(), None));
        assert_eq!(parse_lyric_input("海"), ("海".into(), None));
        assert_eq!(parse_lyric_input(" 好/HAO4 "), ("好".into(), Some("hao4".into())));
        assert_eq!(parse_lyric_input("女/nv3"), ("女".into(), Some("nv3".into())));
        assert_eq!(parse_lyric_input("女/nu:3"), ("女".into(), Some("nu:3".into())));
        assert_eq!(parse_lyric_input("女/NǙ"), ("女".into(), Some("nǚ".into())));
        assert_eq!(parse_lyric_input("妈/ma\u{304}"), ("妈".into(), Some("ma\u{304}".into())));
        assert_eq!(parse_lyric_input("好/hao0"), ("好".into(), Some("hao0".into())));
        assert_eq!(parse_lyric_input("ju:an2"), ("ju:an2".into(), Some("ju:an2".into())));
        for invalid in ["女/n:u3", "女/nu::3", "好/hao00", "好/hao6"] {
            assert_eq!(parse_lyric_input(invalid), (invalid.into(), None));
        }
        assert_eq!(parse_lyric_input("a/b/c"), ("a/b".into(), Some("c".into())));
    }
}
