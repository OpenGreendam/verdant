import { definePrototype, type A11ySetValueEventPayload, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';
import { dragNormalized, planControlGesture, type ControlDecision } from './control-gesture-planner.proto';

type Props = {
  value?: number;
  defaultValue?: number;
  step?: number;
  label?: string;
  disabled?: boolean;
  revision?: string;
};

// The instance owns portable interaction and provisional display. `value` is
// authoritative host truth; defaultValue is the reset target. A commit proposes
// {value, revision}; rejection leaves the authoritative value displayed.
export default definePrototype<Props>({
  name: 'verdant-normalized-knob',
  setup(def) {
    const clamp = (value: number): number => {
      if (value !== value) return 0;
      if (value < 0) return 0;
      if (value > 1) return 1;
      return value;
    };
    const hasEffect = (effects: string[], effect: string, index: number = 0): boolean => {
      if (index >= effects.length) return false;
      if (effects[index] === effect) return true;
      return hasEffect(effects, effect, index + 1);
    };
    const plan = planControlGesture;
    const normalizeDrag = dragNormalized;
    def.props.define({
      value: { type: 'number', default: 0 },
      defaultValue: { type: 'number', default: 0 },
      step: { type: 'number', default: 0.01 },
      label: { type: 'string', default: '' },
      disabled: { type: 'boolean', default: false },
      revision: { type: 'string', default: '' },
    });
    const phase = def.state.string('phase', 'idle');
    const value = def.state.numberRange('value', 0, { min: 0, max: 1 });
    const disabled = def.state.bool('disabled', false);
    const label = def.state.string('label', '');
    const minimum = def.state.numberDiscrete('minimum', 0);
    const maximum = def.state.numberDiscrete('maximum', 1);
    const focus = asFocusable();
    const accessible = asAccessible();
    focus.configure({ disabled: false });
    accessible.role('slider');
    accessible.name(label);
    accessible.state('valueMin', minimum);
    accessible.state('valueMax', maximum);
    accessible.state('valueNow', value);
    accessible.state('disabled', disabled);
    // Explicit host-extension names are admitted by both portable Runtime and
    // native action dispatch; the default a11y:* names are not core events.
    accessible.action('increment', { event: 'host:verdant-knob-increment' });
    accessible.action('decrement', { event: 'host:verdant-knob-decrement' });
    accessible.action('setValue', { event: 'host:verdant-knob-set-value' });
    def.expose.state('value', value);
    def.expose.state('phase', phase);
    def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('valueCommit', { payload: 'json' });
    def.expose.event('interactionCancel', { payload: 'json' });

    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let epoch = 0;
    let authorityEpoch = 0;
    let syncing = false;
    let initial = 0;
    let accepted = 0;
    let revision = '';
    const isCurrent = (ticket: number): boolean => !disposed && run !== null && epoch === ticket;
    const isAuthorityCurrent = (ticket: number): boolean => !disposed && run !== null && authorityEpoch === ticket;
    const publish = (next: number, ticket: number): boolean => {
      if (!isCurrent(ticket)) return false;
      value.set(clamp(next), 'reason: normalized knob display');
      if (!isCurrent(ticket)) return false;
      if (run && attached) run.update();
      return isCurrent(ticket);
    };
    const cancel = (): boolean => {
      // Even an idle cancellation retires a suspended reset/keyboard request.
      const ticket = ++epoch;
      if (!run || phase.get() === 'idle') return false;
      const basis = revision;
      phase.set('idle', 'reason: normalized knob canceled');
      if (!isCurrent(ticket) || !publish(accepted, ticket)) return false;
      if (run) run.expose.emit('interactionCancel', { revision: basis });
      return true;
    };
    const propose = (next: number, ticket: number): boolean => {
      if (!isCurrent(ticket) || !attached || disabled.get()) return false;
      const proposal = clamp(next);
      const basis = revision;
      // Settle before invoking the owner. A synchronous acceptance may publish
      // newer props; no stale state writes follow the externally visible event.
      if (!publish(accepted, ticket)) return false;
      if (run) run.expose.emit('valueCommit', { value: proposal, revision: basis });
      return true;
    };
    const apply = (decision: ControlDecision, preview: number, ticket: number): boolean => {
      if (!isCurrent(ticket) || !attached) return false;
      const reset = hasEffect(decision.effects, 'Control.Reset');
      const commit = hasEffect(decision.effects, 'Control.Commit');
      const proposal = reset ? clamp(run?.props.get().defaultValue ?? 0) : value.get();
      if (hasEffect(decision.effects, 'Control.Begin')) initial = accepted;
      phase.set(decision.nextPhase, 'reason: normalized knob input');
      if (!isCurrent(ticket)) return false;
      if (reset || commit) return propose(proposal, ticket);
      if (hasEffect(decision.effects, 'Control.Preview')) {
        if (!publish(preview, ticket)) return false;
      } else if (hasEffect(decision.effects, 'Control.Discard') || hasEffect(decision.effects, 'Control.Begin')) {
        if (!publish(accepted, ticket)) return false;
      } else {
        if (run) run.update();
        if (!isCurrent(ticket)) return false;
      }
      return decision.consumed;
    };
    const sampleAt = (delta: number, crossed: boolean, fine: boolean, ticket: number): boolean => {
      if (!isCurrent(ticket) || syncing || !attached || disabled.get() || phase.get() === 'idle') return false;
      return apply(plan(phase.get(), 'PointerMove', {
        hit_control: true, threshold_crossed: crossed, reset_modifier: false, double_click: false,
      }), normalizeDrag(initial, delta, 180, fine), ticket);
    };
    def.expose.method('begin', (reset: boolean, double: boolean) => {
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() !== 'idle') return false;
      const ticket = ++epoch;
      focus.focusSelf();
      if (!isCurrent(ticket) || !attached || disabled.get()) return false;
      return apply(plan(phase.get(), 'PointerDown', {
        hit_control: true, threshold_crossed: false, reset_modifier: reset, double_click: double,
      }), accepted, ticket);
    });
    def.expose.method('sample', (delta: number, crossed: boolean, fine: boolean) => {
      const ticket = ++epoch;
      return sampleAt(delta, crossed, fine, ticket);
    });
    def.expose.method('finish', (delta: number, crossed: boolean, fine: boolean) => {
      const ticket = ++epoch;
      if (!sampleAt(delta, crossed, fine, ticket) || !isCurrent(ticket)) return false;
      return apply(plan(phase.get(), 'PointerUp', {
        hit_control: true, threshold_crossed: crossed, reset_modifier: false, double_click: false,
      }), value.get(), ticket);
    });
    def.expose.method('cancel', cancel);
    const adjust = (key: string, fine: boolean): boolean => {
      const current = run;
      if (!current || syncing || !attached || disposed || disabled.get()) return false;
      const ticket = epoch + 1;
      cancel();
      // Keyboard and assistive actions start from accepted truth after cancel,
      // never from an unaccepted pointer preview. A reentrant owner supersedes
      // this request by advancing the epoch before any proposal can escape.
      if (!isCurrent(ticket) || !attached || disabled.get()) return false;
      let next = accepted;
      const rawStep = current.props.get().step ?? 0.01;
      const step = rawStep > 0 && rawStep <= 1 ? rawStep : 0.01;
      const amount = step * (fine ? 0.1 : 1);
      if (key === 'Home') next = 0;
      else if (key === 'End') next = 1;
      else if (key === 'ArrowUp' || key === 'ArrowRight') next += amount;
      else if (key === 'ArrowDown' || key === 'ArrowLeft') next -= amount;
      else if (key === 'PageUp') next += step * 10;
      else if (key === 'PageDown') next -= step * 10;
      else return false;
      return propose(next, ticket);
    };
    def.event.on('host:verdant-knob-increment', () => {
      adjust('ArrowUp', false);
    });
    def.event.on('host:verdant-knob-decrement', () => {
      adjust('ArrowDown', false);
    });
    def.event.on('host:verdant-knob-set-value', (_current, opaque) => {
      const event = opaque as A11ySetValueEventPayload;
      const next = event.value;
      if (typeof next !== 'number' || !Number.isFinite(next)) return;
      if (!run || syncing || !attached || disposed || disabled.get()) return;
      const ticket = epoch + 1;
      cancel();
      if (!isCurrent(ticket) || !attached || disabled.get()) return;
      // Assistive writes use the same revision-checked proposal as a drag or
      // key. They cannot adopt an unaccepted preview or bypass host policy.
      propose(next, ticket);
    });
    def.event.on('key.down', (_current, event) => {
      if (event.key === 'Escape') {
        if (phase.get() !== 'idle') event.control.requestDefaultActionPrevention();
        cancel();
        return;
      }
      if (!run || !attached || disposed || disabled.get() || event.altKey || event.ctrlKey || event.metaKey) return;
      if (event.key !== 'Home' && event.key !== 'End'
        && event.key !== 'ArrowUp' && event.key !== 'ArrowRight'
        && event.key !== 'ArrowDown' && event.key !== 'ArrowLeft'
        && event.key !== 'PageUp' && event.key !== 'PageDown') return;
      event.control.requestDefaultActionPrevention();
      adjust(event.key ?? '', event.shiftKey === true);
    });
    const sync = (current: RunHandle<Props>) => {
      if (disposed) return;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = true;
      run = current;
      const props = current.props.get();
      const next = clamp(props.value ?? 0);
      const nextRevision = props.revision ?? '';
      const shouldCancel = phase.get() !== 'idle'
        && (next !== accepted || nextRevision !== revision || props.disabled === true);
      // Publish the new authority before any observable state/focus write.
      accepted = next;
      revision = nextRevision;
      if (shouldCancel) {
        phase.set('idle', 'reason: normalized knob authority changed');
        if (!isAuthorityCurrent(ticket)) return;
      }
      label.set(props.label ?? '', 'reason: normalized knob label');
      if (!isAuthorityCurrent(ticket)) return;
      disabled.set(props.disabled ?? false, 'reason: normalized knob policy');
      if (!isAuthorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!isAuthorityCurrent(ticket)) return;
      if (phase.get() === 'idle') {
        value.set(accepted, 'reason: normalized knob authority display');
        if (!isAuthorityCurrent(ticket)) return;
      }
      // A public observer may cancel an idle interaction while these fields
      // are being published. That retires input, not the new props: finish
      // their value/focus projection. Only newer props or detach supersede it.
      // Admit fresh input after all authority fields and cancellation settle.
      if (shouldCancel) current.expose.emit('interactionCancel', { revision: nextRevision });
      if (!isAuthorityCurrent(ticket)) return;
      syncing = false;
      if (attached) current.update();
    };
    const retire = (permanent: boolean) => {
      const current = run;
      const wasActive = phase.get() !== 'idle';
      const basis = revision;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = false;
      attached = false;
      disposed = disposed || permanent;
      run = null;
      // Native disposal detaches first, so onUnmounted already publishes the
      // cancellation. before-dispose permits lexical retirement only: neither
      // State.set nor RunHandle callbacks are available in that terminal phase.
      if (permanent) return;
      phase.set('idle', 'reason: normalized knob detached');
      if (authorityEpoch !== ticket) return;
      value.set(accepted, 'reason: normalized knob detached display');
      if (authorityEpoch !== ticket) return;
      if (wasActive && current) current.expose.emit('interactionCancel', { revision: basis });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((current) => {
      if (disposed) return;
      attached = true;
      sync(current);
    });
    def.props.watch(['value', 'defaultValue', 'step', 'label', 'disabled', 'revision'], sync);
    def.event.on('host:blur', cancel);
    // A press may cancel when its pointer leaves the root hitbox. The captured
    // MoveGesture has a different lifetime and must keep sampling outside it;
    // its lease invokes exposed cancel for blur, detach, or lost ownership.
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
