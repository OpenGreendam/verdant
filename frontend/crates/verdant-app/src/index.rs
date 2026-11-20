//! Bounded note lookups for the editor surfaces (TODO §5: 100k notes must
//! not force per-frame whole-track scans).
//!
//! [`Track::notes_in_range`] owns the shared interval tree, including subtree
//! end and pitch bounds. This cache adds the complementary pieces the UI
//! needs every frame:
//!
//! * a membership set for preview/ready marking (no `Vec::contains` sweeps),
//! * a note-id → position map (no `track.note` linear probes), and
//! * the domain range iterator so roll, lane and arrange painting never
//!   build a second interval index or re-sort the full note vector.
//!
//! The position map is rebuilt lazily on `revision` or `track` change;
//! range queries borrow the domain bounds already maintained by commits.

use std::collections::{BTreeSet, HashMap, HashSet};

use verdant_domain::{Edge, Note, NoteId, Project, Revision, Track, TrackId};
use verdant_session::{Hit, Viewport};

pub struct LanguageSegment {
    pub key: String,
    pub start_tick: i64,
    pub end_tick: i64,
    pub selected: String,
}

#[derive(Default)]
pub struct LanguageSegments {
    revision: Option<Revision>,
    tracks: HashMap<TrackId, Vec<LanguageSegment>>,
}

impl LanguageSegments {
    pub fn sync(&mut self, project: &Project, revision: Revision) {
        if self.revision == Some(revision) { return; }
        self.revision = Some(revision);
        self.tracks.clear();
        if !project.extensions.contains_key(verdant_backend::phonemizer::EXTENSION_KEY) { return; }
        let Ok(settings) = verdant_backend::phonemizer::settings_from_project(project) else { return };
        for (key, segment) in settings.segments {
            self.tracks.entry(segment.track).or_default().push(LanguageSegment {
                key, start_tick: segment.start_tick, end_tick: segment.end_tick, selected: segment.selected,
            });
        }
        for segments in self.tracks.values_mut() {
            segments.sort_unstable_by_key(|segment| segment.start_tick);
        }
    }

    pub fn visible(&self, track: &TrackId, start: i64, end: i64) -> &[LanguageSegment] {
        let Some(segments) = self.tracks.get(track) else { return &[] };
        if end <= start { return &[]; }
        // Admitted segments are disjoint on each track. Both ends are ordered.
        let first = segments.partition_point(|segment| segment.end_tick <= start);
        let count = segments[first..].partition_point(|segment| segment.start_tick < end);
        &segments[first..first + count]
    }

    pub fn at(&self, track: &TrackId, tick: i64) -> Option<&LanguageSegment> {
        self.visible(track, tick, tick.saturating_add(1)).first()
    }
}

/// Cached per-track lookups, valid while the project revision is unchanged.
#[derive(Clone, Debug, Default)]
pub struct NoteIndex {
    track: Option<TrackId>,
    revision: u64,
    /// `id → position` in the track's `notes` vector.
    positions: HashMap<NoteId, usize>,
}

impl NoteIndex {
    /// Rebuild when the track or revision changed; otherwise reuse.
    pub fn sync(&mut self, project: &Project, track: &TrackId, revision: Revision) {
        if self.track.as_ref() == Some(track) && self.revision == revision.get() {
            return;
        }
        self.positions.clear();
        if let Some(t) = project.track(track) {
            self.positions.reserve(t.notes.len());
            for (i, n) in t.notes.iter().enumerate() {
                self.positions.insert(n.id.clone(), i);
            }
        }
        self.track = Some(track.clone());
        self.revision = revision.get();
    }

    /// O(1) note lookup through the position map.
    pub fn note<'a>(&self, track: &'a Track, id: &NoteId) -> Option<&'a Note> {
        self.positions.get(id).map(|&i| &track.notes[i])
    }

    /// Membership without a map probe (hot paint paths).
    pub fn contains(&self, id: &NoteId) -> bool {
        self.positions.contains_key(id)
    }

    pub fn position(&self, id: &NoteId) -> Option<usize> { self.positions.get(id).copied() }

    /// The domain interval query in document order, with identical boundaries.
    pub fn notes_in_range<'a>(&self, track: &'a Track, from: i64, to: i64) -> impl DoubleEndedIterator<Item = &'a Note> + Clone + use<'a> {
        track.notes_in_range(from, to)
    }

    /// Same priority rules as `verdant_session::hit_test_with` (selected
    /// handle → selected body → topmost body) but without rescanning the
    /// track: only viewport candidates are considered.
    #[allow(clippy::too_many_arguments)]
    pub fn hit_test(
        &self,
        track: &Track,
        vp: &Viewport,
        selection: &BTreeSet<NoteId>,
        x: f64,
        y: f64,
        handle_px: f64,
        outside_px: f64,
    ) -> Hit {
        let tick = vp.x_to_tick(x);
        let slack = (outside_px + handle_px).max(verdant_session::viewport::MIN_NOTE_BODY_PX) / vp.px_per_tick;
        let pitch = vp.y_to_pitch(y);
        // Keep adjacent rows at floating-point boundaries; rectangle tests
        // below retain the exact top-inclusive/bottom-exclusive hit rules.
        let candidates = track.notes_in_pitch_range(
            (tick - slack).floor() as i64,
            ((tick + slack).ceil() as i64).saturating_add(1),
            (pitch.floor() as i32).saturating_sub(1),
            pitch.ceil() as i32,
        );

        for n in candidates.clone().rev().filter(|n| selection.contains(&n.id)) {
            let r = vp.note_rect(n);
            if y < r.y || y >= r.bottom() {
                continue;
            }
            let zone = handle_px.min(r.w / 3.0);
            if x >= r.right() - zone && x <= r.right() + outside_px {
                return Hit::Handle(n.id.clone(), Edge::End);
            }
            if x >= r.x - outside_px && x < r.x + zone {
                return Hit::Handle(n.id.clone(), Edge::Start);
            }
        }
        let mut body: Option<&NoteId> = None;
        for n in candidates.rev() {
            if vp.note_body_rect(n).contains(x, y) {
                if selection.contains(&n.id) {
                    return Hit::Body(n.id.clone());
                }
                body.get_or_insert(&n.id);
            }
        }
        body.map(|id| Hit::Body(id.clone())).unwrap_or(Hit::Background)
    }
}

/// Read-only snapshot used by callers that want a cheap membership set for
/// previews (e.g. the rubber-band's current hits).
pub type NoteSet = HashSet<NoteId>;

#[cfg(test)]
mod tests {
    use super::*;
    use verdant_domain::{Command, CommandBatch, ProjectStore};

    fn note(id: &str, start: i64, duration: i64, pitch: i32) -> Note {
        let mut note = Note::new(start, duration, pitch);
        note.id = id.into();
        note
    }

    fn project(notes: Vec<Note>) -> Project {
        let mut project = Project::new_empty();
        project.tracks[0].monophonic = false;
        project.tracks[0].notes = notes;
        project.tracks[0].refresh_index();
        project
    }

    fn ids<'a>(notes: impl Iterator<Item = &'a Note>) -> Vec<&'a str> {
        notes.map(|note| note.id.as_str()).collect()
    }

    #[test]
    fn range_queries_share_domain_order_and_positions_follow_revisions_and_tracks() {
        let mut project = project(vec![
            note("b", 9000, 10, 60), note("long", 0, 10_000, 60), note("a", 9000, 10, 60),
        ]);
        let first = project.tracks[0].id.clone();
        let mut other = Track::new("other", "blue");
        other.notes = vec![note("other", 10, 20, 72)];
        let second = other.id.clone();
        project.tracks.push(other);
        let mut store = ProjectStore::new(project);
        let mut index = NoteIndex::default();
        index.sync(store.project(), &first, store.revision());
        let track = store.project().track(&first).unwrap();
        assert_eq!(ids(index.notes_in_range(track, 9005, 9006)), ["long", "a", "b"]);
        assert_eq!(ids(index.notes_in_range(track, 9005, 9006).rev()), ids(track.notes_in_range(9005, 9006).rev()));
        assert_eq!(index.position(&"long".into()), Some(0));
        store.apply(store.revision(), CommandBatch::single("move long note", Command::SetTiming {
            track: first.clone(), note_id: "long".into(), start_tick: 9500, duration_tick: 10,
        })).unwrap();
        index.sync(store.project(), &first, store.revision());
        let track = store.project().track(&first).unwrap();
        assert_eq!(ids(index.notes_in_range(track, 9005, 9006)), ["a", "b"]);
        assert_eq!(index.position(&"long".into()), Some(2));
        assert_eq!(index.note(track, &"long".into()).unwrap().start_tick, 9500);
        store.undo().unwrap();
        index.sync(store.project(), &first, store.revision());
        assert_eq!(index.position(&"long".into()), Some(0));
        assert_eq!(ids(index.notes_in_range(store.project().track(&first).unwrap(), 9005, 9006)), ["long", "a", "b"]);
        index.sync(store.project(), &second, store.revision());
        assert!(!index.contains(&"long".into()));
        assert_eq!(index.position(&"other".into()), Some(0));
        assert_eq!(ids(index.notes_in_range(store.project().track(&second).unwrap(), 10, 11)), ["other"]);
    }

    #[test]
    fn narrow_note_selects_on_its_painted_extension() {
        let project = project(vec![note("short", 100, 1, 60), note("next", 112, 100, 60)]);
        let track = &project.tracks[0];
        let mut index = NoteIndex::default();
        index.sync(&project, &track.id, Revision::new(0).unwrap());
        let vp = Viewport { px_per_tick: 0.1, ..Viewport::default() };
        let y = vp.pitch_to_y(60.5);
        let selection = BTreeSet::new();
        assert_eq!(index.hit_test(track, &vp, &selection, 11.0, y, 7.0, 4.0), Hit::Body("short".into()));
        assert_eq!(index.hit_test(track, &vp, &selection, 11.5, y, 7.0, 4.0), Hit::Body("next".into()));
    }

    #[test]
    fn pitch_pruned_hits_keep_selected_handle_body_and_topmost_priority() {
        let project = project(vec![
            note("long", 0, 1000, 60), note("a", 10, 100, 60), note("z", 10, 100, 60),
            note("short", 40, 10, 60), note("above", 0, 1000, 61), note("below", 0, 1000, 59),
        ]);
        let track = &project.tracks[0];
        let mut index = NoteIndex::default();
        index.sync(&project, &track.id, Revision::new(0).unwrap());
        let vp = Viewport { start_tick: 0.0, px_per_tick: 1.0, top_pitch: 64.25, row_height: 18.0, ..Viewport::default() };
        let rect = vp.note_rect(track.note(&"a".into()).unwrap());
        for selection in [BTreeSet::new(), [NoteId::from("a")].into(), [NoteId::from("short")].into()] {
            for x in [-4.0, 7.0, 10.0, 13.0, 40.0, 45.0, 49.0, 50.0, 107.0, 110.0, 113.0, 500.0, 1001.0] {
                for y in [rect.y - 0.001, rect.y, rect.y + rect.h / 2.0, rect.bottom() - 0.001, rect.bottom(), rect.bottom() + 0.001] {
                    for (handle, outside) in [(6.0, 3.0), (14.0, 8.0)] {
                        assert_eq!(
                            index.hit_test(track, &vp, &selection, x, y, handle, outside),
                            verdant_session::hit_test_with(track, &vp, &selection, x, y, handle, outside),
                            "x={x}, y={y}, handle={handle}, selection={selection:?}",
                        );
                    }
                }
            }
        }
    }
}
