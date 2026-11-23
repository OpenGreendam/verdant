import { definePrototype } from '@proto.ui/core';

// Portable decisions for knobs, curve handles and threshold handles. The host
// supplies immutable hit/threshold facts and applies the ordered effects.
export type ControlGuards = {
  hit_control: boolean;
  threshold_crossed: boolean;
  reset_modifier: boolean;
  double_click: boolean;
};

export type ControlDecision = {
  nextPhase: string;
  consumed: boolean;
  effects: string[];
};

export function planControlGesture(phase: string, event: string, guards: ControlGuards): ControlDecision {
  if (phase === 'idle' && event === 'PointerDown') {
    if (!guards.hit_control) return { nextPhase: 'idle', consumed: false, effects: [] };
    // Double click, or Alt / Option click, restores the default; it never drags.
    if (guards.double_click || guards.reset_modifier) {
      return { nextPhase: 'idle', consumed: true, effects: ['Control.Select', 'Control.Reset'] };
    }
    return { nextPhase: 'armed', consumed: true, effects: ['Control.Select', 'Control.Begin', 'Host.CapturePointer'] };
  }
  if (phase === 'armed' && event === 'PointerMove') {
    // A click selects; only movement past the threshold edits the value.
    if (!guards.threshold_crossed) return { nextPhase: 'armed', consumed: true, effects: [] };
    return { nextPhase: 'dragging', consumed: true, effects: ['Control.Preview'] };
  }
  if (phase === 'dragging' && event === 'PointerMove') {
    return { nextPhase: 'dragging', consumed: true, effects: ['Control.Preview'] };
  }
  if (phase === 'dragging' && event === 'PointerUp') {
    return { nextPhase: 'idle', consumed: true, effects: ['Control.Commit', 'Host.ReleasePointer'] };
  }
  if (phase === 'armed' && event === 'PointerUp') {
    return { nextPhase: 'idle', consumed: true, effects: ['Control.Discard', 'Host.ReleasePointer'] };
  }
  if ((phase === 'armed' || phase === 'dragging') && event === 'Cancel') {
    return { nextPhase: 'idle', consumed: true, effects: ['Control.Discard', 'Host.ReleasePointer'] };
  }
  return { nextPhase: phase, consumed: false, effects: [] };
}

// A drag of `travel` pixels sweeps the whole range; the fine modifier
// (Shift) divides the rate by ten. Values are normalized to 0…1.
export function dragNormalized(start: number, delta: number, travel: number, fine: boolean): number {
  const span = travel > 1 ? travel : 1;
  const rate = fine ? 0.1 : 1;
  const next = start + (delta / span) * rate;
  if (next < 0) return 0;
  if (next > 1) return 1;
  return next;
}

export default definePrototype({
  name: 'verdant-control-gesture-planner',
  setup(def) {
    def.expose.method('planControlGesture', planControlGesture);
    def.expose.method('dragNormalized', dragNormalized);
    return (renderer) => renderer.slot();
  },
});
