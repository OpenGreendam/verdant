import { definePrototype } from '@proto.ui/core';

type NoteGestureGuards = {
  hit_background: boolean;
  hit_selected_note: boolean;
  hit_unselected_note: boolean;
  hit_note_body: boolean;
  hit_resize_handle: boolean;
  toggle_modifier: boolean;
  extend_modifier: boolean;
  duplicate_modifier: boolean;
  pointer_threshold_crossed: boolean;
  base_revision_matches: boolean;
  base_revision_matches_and_delta_nonzero: boolean;
};

type NoteGestureDecision = {
  nextPhase: string;
  consumed: boolean;
  effects: string[];
};

// The session binding supplies immutable guard facts and applies the ordered effects.
function planNoteGesture(
  machine: string,
  phase: string,
  event: string,
  guards: NoteGestureGuards,
): NoteGestureDecision {
  if (machine !== 'marquee' && machine !== 'move' && machine !== 'resize' && machine !== 'duplicate') {
    return { nextPhase: phase, consumed: false, effects: [] };
  }

  if (phase === 'idle' && event === 'PointerDown') {
    if (machine === 'marquee' && guards.hit_background) {
      return handled('armed', ['Session.BeginMarqueeGesture', 'Host.CapturePointer']);
    }
    if (machine === 'resize' && guards.hit_resize_handle) {
      return handled('armed', ['Session.SelectIfUnselectedHit', 'Session.BeginResizeGesture', 'Host.CapturePointer']);
    }
    if (machine === 'move' && !guards.duplicate_modifier) {
      if (guards.hit_selected_note && guards.toggle_modifier) {
        return handled('idle', ['Session.ToggleHitNote']);
      }
      if (guards.hit_note_body && guards.toggle_modifier) {
        return handled('armed', ['Session.ToggleHitNote', 'Session.BeginMoveGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_note_body && guards.extend_modifier) {
        return handled('armed', ['Session.ExtendSelectionToHit', 'Session.BeginMoveGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_selected_note) {
        return handled('armed', ['Session.BeginMoveGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_unselected_note) {
        return handled('armed', ['Session.SelectOnlyHitNote', 'Session.BeginMoveGesture', 'Host.CapturePointer']);
      }
    }
    if (machine === 'duplicate' && guards.duplicate_modifier) {
      // Duplicate prioritizes unselected hits. Move prioritizes selected hits.
      if (guards.hit_unselected_note && guards.toggle_modifier) {
        return handled('armed', ['Session.ToggleHitNote', 'Session.BeginDuplicateGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_selected_note && guards.toggle_modifier) {
        return handled('idle', ['Session.ToggleHitNote']);
      }
      if (guards.hit_note_body && guards.extend_modifier) {
        return handled('armed', ['Session.ExtendSelectionToHit', 'Session.BeginDuplicateGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_unselected_note) {
        return handled('armed', ['Session.SelectOnlyHitNote', 'Session.BeginDuplicateGesture', 'Host.CapturePointer']);
      }
      if (guards.hit_selected_note) {
        return handled('armed', ['Session.BeginDuplicateGesture', 'Host.CapturePointer']);
      }
    }
  }

  const active = phase === 'armed' || phase === 'dragging';
  if (event === 'PointerMove' && (phase === 'dragging' || (phase === 'armed' && guards.pointer_threshold_crossed))) {
    if (machine === 'marquee') return handled('dragging', ['Session.UpdateMarqueePreview']);
    if (machine === 'resize') return handled('dragging', ['Session.UpdateResizePreview']);
    return handled('dragging', ['Session.UpdateMovePreview']);
  }
  if (event === 'PointerUp' && phase === 'dragging') {
    if (machine === 'marquee' && guards.base_revision_matches) {
      return finishWith('Session.CommitMarqueeSelection');
    }
    if (machine !== 'marquee' && guards.base_revision_matches_and_delta_nonzero) {
      if (machine === 'move') return finishWith('Domain.CommitMoveNotes');
      if (machine === 'resize') return finishWith('Domain.CommitResizeNotes');
      return finishWith('Domain.CommitDuplicateNotes');
    }
    return finishWith('Session.ReportConflictIfStale');
  }
  if (event === 'PointerUp' && phase === 'armed' && machine !== 'resize') {
    return finishWith('Session.ClickReleaseSelection');
  }
  if ((event === 'Cancel' && active) || (event === 'PointerUp' && phase === 'armed')) {
    return handled('idle', ['Session.ClearGesturePreview', 'Host.ReleasePointer']);
  }
  return { nextPhase: phase, consumed: active, effects: [] };
}

function handled(nextPhase: string, effects: string[]): NoteGestureDecision {
  return { nextPhase, consumed: true, effects };
}

function finishWith(effect: string): NoteGestureDecision {
  return handled('idle', [effect, 'Session.ClearGesturePreview', 'Host.ReleasePointer']);
}

export default definePrototype({
  name: 'verdant-note-gesture-planner',
  setup(def) {
    def.expose.method('planNoteGesture', planNoteGesture);
    return (renderer) => renderer.slot();
  },
});
