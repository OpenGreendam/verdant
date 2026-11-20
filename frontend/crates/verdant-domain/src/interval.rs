//! Derived interval bounds for notes in document order.
//!
//! Leaves refer directly to the sorted note slice. Each branch stores its
//! latest end and pitch bounds, so a song-spanning note only keeps its own
//! ancestor branches open. Queries walk surviving branches in either document
//! order without allocating, copying notes or building a traversal stack.

use crate::Note;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bounds {
    end: i64,
    low: i32,
    high: i32,
}

impl Bounds {
    const EMPTY: Self = Self { end: i64::MIN, low: i32::MAX, high: i32::MIN };

    fn merge(self, other: Self) -> Self {
        Self { end: self.end.max(other.end), low: self.low.min(other.low), high: self.high.max(other.high) }
    }

    fn intersects(self, from: i64, low: i32, high: i32) -> bool {
        self.end > from && self.low <= high && self.high >= low
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IntervalIndex {
    bounds: Vec<Bounds>,
    leaves: usize,
    notes: usize,
}

impl IntervalIndex {
    pub(crate) fn rebuild(&mut self, notes: &[Note]) {
        self.notes = notes.len();
        self.leaves = if notes.is_empty() { 0 } else { notes.len().next_power_of_two() };
        self.bounds.resize(self.leaves * 2, Bounds::EMPTY);
        if notes.is_empty() {
            return;
        }
        let leaves = &mut self.bounds[self.leaves..];
        for (bound, note) in leaves.iter_mut().zip(notes) {
            *bound = Bounds { end: note.end_tick(), low: note.midi_note, high: note.midi_note };
        }
        leaves[notes.len()..].fill(Bounds::EMPTY);
        for node in (1..self.leaves).rev() {
            self.bounds[node] = self.bounds[node * 2].merge(self.bounds[node * 2 + 1]);
        }
    }

    pub(crate) fn query<'a>(&'a self, notes: &'a [Note], from: i64, to: i64, low: i32, high: i32) -> NoteRange<'a> {
        assert_eq!(self.notes, notes.len(), "call Track::refresh_index after directly mutating notes");
        let back = if low <= high { notes.partition_point(|note| note.start_tick < to) } else { 0 };
        NoteRange {
            index: self,
            notes,
            from,
            low,
            high,
            front: 0,
            back,
            forward: Cursor::root(self.leaves),
            backward: Cursor::root(self.leaves),
            #[cfg(test)]
            visited: 0,
        }
    }
}

/// A subtree in the complete binary tree. Moving to the next sibling ascends
/// past exhausted ancestors; no query-local stack or parent table is needed.
#[derive(Clone, Copy, Debug)]
struct Cursor {
    node: usize,
    start: usize,
    len: usize,
}

impl Cursor {
    fn root(len: usize) -> Self {
        Self { node: usize::from(len != 0), start: 0, len }
    }

    fn descend(&mut self, reverse: bool) {
        self.len /= 2;
        self.node *= 2;
        if reverse {
            self.node += 1;
            self.start += self.len;
        }
    }

    fn skip(&mut self, reverse: bool) {
        let end = self.start;
        if !reverse {
            self.start += self.len;
        }
        let exhausted_side = usize::from(!reverse);
        while self.node > 1 && self.node % 2 == exhausted_side {
            self.node /= 2;
            self.len *= 2;
        }
        if self.node == 1 {
            self.node = 0;
        } else if reverse {
            self.node -= 1;
            self.start = end - self.len;
        } else {
            self.node += 1;
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NoteRange<'a> {
    index: &'a IntervalIndex,
    notes: &'a [Note],
    from: i64,
    low: i32,
    high: i32,
    /// Unconsumed note positions, shared by the two traversal directions.
    front: usize,
    back: usize,
    forward: Cursor,
    backward: Cursor,
    #[cfg(test)]
    visited: usize,
}

impl<'a> NoteRange<'a> {
    fn next_note(&mut self, reverse: bool) -> Option<&'a Note> {
        if self.front >= self.back {
            return None;
        }
        let cursor = if reverse { &mut self.backward } else { &mut self.forward };
        while cursor.node != 0 {
            if cursor.start >= self.back || cursor.start + cursor.len <= self.front {
                cursor.skip(reverse);
                continue;
            }
            #[cfg(test)]
            { self.visited += 1; }
            if !self.index.bounds[cursor.node].intersects(self.from, self.low, self.high) {
                cursor.skip(reverse);
            } else if cursor.len > 1 {
                cursor.descend(reverse);
            } else {
                let position = cursor.start;
                cursor.skip(reverse);
                if reverse {
                    self.back = position;
                } else {
                    self.front = position + 1;
                }
                return Some(&self.notes[position]);
            }
        }
        self.front = self.back;
        None
    }
}

impl<'a> Iterator for NoteRange<'a> {
    type Item = &'a Note;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_note(false)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.back - self.front))
    }
}

impl DoubleEndedIterator for NoteRange<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next_note(true)
    }
}

impl std::iter::FusedIterator for NoteRange<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Track;

    fn note(id: &str, start: i64, duration: i64, pitch: i32) -> Note {
        let mut note = Note::new(start, duration, pitch);
        note.id = id.into();
        note
    }

    fn track(notes: Vec<Note>) -> Track {
        let mut track = Track::new("intervals", "teal");
        track.notes = notes;
        track.refresh_index();
        track
    }

    fn ids<'a>(notes: impl Iterator<Item = &'a Note>) -> Vec<&'a str> {
        notes.map(|note| note.id.as_str()).collect()
    }

    #[test]
    fn long_overlaps_equal_starts_and_half_open_boundaries_keep_document_order() {
        let track = track(vec![
            note("at-end", 1000, 20, 62), note("b", 10, 30, 61), note("long", 0, 1000, 60),
            note("late", 999, 2, 60), note("a", 10, 10, 60), note("before", -10, 10, 59),
        ]);
        assert_eq!(ids(track.notes_in_range(10, 20)), ["long", "a", "b"]);
        assert_eq!(ids(track.notes_in_range(20, 30)), ["long", "b"]);
        assert_eq!(ids(track.notes_in_range(1000, 1001)), ["late", "at-end"]);
        assert_eq!(ids(track.notes_in_range(999, 1000).rev()), ["late", "long"]);
        assert_eq!(ids(track.notes_in_pitch_range(10, 20, 60, 60)), ["long", "a"]);
        assert_eq!(ids(track.notes_in_pitch_range(10, 20, 60, 61)), ["long", "a", "b"]);
        assert_eq!(track.notes_in_pitch_range(10, 20, 61, 60).count(), 0);
    }

    #[test]
    fn range_directions_and_pitch_filters_match_the_note_predicate() {
        let mut notes: Vec<_> = (0..257).rev().map(|i| {
            note(&format!("n{i:03}"), (i / 3) * 10, if i % 17 == 0 { 2000 } else { i % 31 + 1 }, 48 + (i % 24) as i32)
        }).collect();
        notes.push(note("minimum", i64::MIN, 8, i32::MIN));
        notes.push(note("maximum", i64::MAX - 8, 16, i32::MAX));
        assert_eq!(notes.last().unwrap().end_tick(), i64::MAX);
        let track = track(notes);
        for from in [i64::MIN, -1, 0, 10, 39, 400, 850, 2000, i64::MAX - 1, i64::MAX] {
            for to in [i64::MIN, 0, 10, 40, 401, 851, 3000, i64::MAX] {
                for (low, high) in [(i32::MIN, i32::MAX), (48, 48), (50, 58), (72, 127), (60, 59)] {
                    let expected: Vec<_> = track.notes.iter().filter(|n| {
                        n.start_tick < to && n.end_tick() > from && n.midi_note >= low && n.midi_note <= high
                    }).collect();
                    let range = track.notes_in_pitch_range(from, to, low, high);
                    assert_eq!(ids(range.clone()), ids(expected.iter().copied()));
                    assert_eq!(ids(range.clone().rev()), ids(expected.iter().rev().copied()));
                    let mut range = range;
                    let (mut front, mut back) = (0, expected.len());
                    while front < back {
                        assert_eq!(range.next(), Some(expected[front]));
                        front += 1;
                        if front < back {
                            back -= 1;
                            assert_eq!(range.next_back(), Some(expected[back]));
                        }
                    }
                    assert!(range.next().is_none());
                    assert!(range.next_back().is_none());
                    assert!(range.next().is_none());
                }
            }
        }
    }

    #[test]
    fn song_spanning_note_does_not_open_unrelated_time_or_pitch_branches() {
        let mut notes: Vec<_> = (0..100_000).map(|i| note(&format!("n{i:06}"), i * 10, 2, 64)).collect();
        notes[0].duration_tick = 2_000_000;
        notes[0].midi_note = 60;
        let mut index = IntervalIndex::default();
        index.rebuild(&notes);
        let height = index.leaves.ilog2() as usize;
        for reverse in [false, true] {
            let mut range = index.query(&notes, 900_000, 900_001, i32::MIN, i32::MAX);
            let found = if reverse { range.by_ref().rev().count() } else { range.by_ref().count() };
            assert_eq!(found, 2);
            assert!(range.visited <= 8 * height, "visited {} branches at height {height}", range.visited);
            let mut range = index.query(&notes, 900_000, 900_001, 60, 60);
            let found = if reverse { range.by_ref().rev().count() } else { range.by_ref().count() };
            assert_eq!(found, 1);
            assert!(range.visited <= 4 * height, "visited {} pitch branches at height {height}", range.visited);
        }
    }

    #[test]
    fn rebuild_discards_removed_notes_and_padding_bounds() {
        let mut track = track(vec![note("a", 0, 1000, 60), note("b", 1, 1000, 61), note("c", 2, 1000, 62)]);
        assert_eq!(track.notes_in_range(900, 901).count(), 3);
        track.notes.truncate(1);
        track.notes[0].duration_tick = 2;
        track.refresh_index();
        assert_eq!(track.notes_in_range(900, 901).count(), 0);
        assert_eq!(ids(track.notes_in_range(0, 1).rev()), ["a"]);
        track.notes.clear();
        track.refresh_index();
        assert_eq!(track.notes_in_range(i64::MIN, i64::MAX).count(), 0);
    }

    #[test]
    fn document_load_rebuilds_note_order_and_interval_pitch_bounds() {
        let mut project = crate::Project::new_empty();
        project.tracks[0].notes = vec![note("late", 900, 20, 64), note("long", 0, 1000, 60), note("early", 10, 10, 64)];
        let encoded = serde_json::to_string(&project).unwrap();
        let (loaded, _) = crate::io::parse_project(&encoded).unwrap();
        let track = &loaded.tracks[0];
        assert_eq!(ids(track.notes_in_range(900, 901)), ["long", "late"]);
        assert_eq!(ids(track.notes_in_pitch_range(900, 901, 60, 60)), ["long"]);
        assert_eq!(ids(track.notes_in_pitch_range(900, 901, 64, 64).rev()), ["late"]);
    }
}
