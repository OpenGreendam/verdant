import { definePrototype, tw, type A11ySetValueEventPayload, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';

type Props = {
  value?: number; min?: number; max?: number; step?: number; fineStep?: number;
  defaultValue?: number; label?: string; disabled?: boolean; revision?: string;
  role?: string; dragMode?: string; doubleAction?: string; tapAction?: string;
  previewEcho?: boolean; keyboardMode?: string;
};

// Geometry and musical/plugin unit conversion stay at the native boundary.
// This instance owns the edit transaction, keyboard/assistive proposals, focus,
// provisional display, cancellation and lifecycle. Every emitted write carries
// the authority token on which it was based; the host decides whether to accept.
export default definePrototype<Props>({
  name: 'verdant-continuous-control',
  setup(def) {
    def.props.define({
      value: { type: 'number', default: 0 }, min: { type: 'number', default: 0 },
      max: { type: 'number', default: 1 }, step: { type: 'number', default: 0.01 },
      fineStep: { type: 'number', default: 0.001 }, defaultValue: { type: 'number', default: 0 },
      label: { type: 'string', default: '' }, disabled: { type: 'boolean', default: false },
      revision: { type: 'string', default: '' }, role: { type: 'string', default: 'slider' },
      dragMode: { type: 'string', default: 'absolute' }, doubleAction: { type: 'string', default: 'reset' },
      tapAction: { type: 'string', default: 'none' }, previewEcho: { type: 'boolean', default: false },
      keyboardMode: { type: 'string', default: 'full' },
    });
    const phase = def.state.string('phase', 'idle');
    const value = def.state.numberRange('value', 0, { min: -1e15, max: 1e15 });
    const initialValue = def.state.numberRange('initialValue', 0, { min: -1e15, max: 1e15 });
    const minimum = def.state.numberRange('minimum', 0, { min: -1e15, max: 1e15 });
    const maximum = def.state.numberRange('maximum', 1, { min: -1e15, max: 1e15 });
    const disabled = def.state.bool('disabled', false);
    const label = def.state.string('label', '');
    const role = def.state.string('role', 'slider');
    const focus = asFocusable();
    const accessible = asAccessible();
    focus.configure({ disabled: false });
    accessible.role(role);
    accessible.name(label);
    accessible.state('valueMin', minimum);
    accessible.state('valueMax', maximum);
    accessible.state('valueNow', value);
    accessible.state('disabled', disabled);
    accessible.action('increment', { event: 'host:verdant-scalar-increment' });
    accessible.action('decrement', { event: 'host:verdant-scalar-decrement' });
    accessible.action('setValue', { event: 'host:verdant-scalar-set-value' });
    // Native visual slots have no intrinsic size. The authored root is the
    // interactive surface and occupies the allocation provided by its host.
    def.feedback.style.use(tw('w-full h-full'));
    def.expose.state('value', value);
    def.expose.state('initialValue', initialValue);
    def.expose.state('phase', phase);
    def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('interactionStart', { payload: 'json' });
    def.expose.event('valuePreview', { payload: 'json' });
    def.expose.event('valueCommit', { payload: 'json' });
    def.expose.event('interactionCancel', { payload: 'json' });
    def.expose.event('activate', { payload: 'json' });
    def.expose.event('editRequest', { payload: 'json' });

    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let syncing = false;
    let epoch = 0;
    let authorityEpoch = 0;
    let accepted = 0;
    let revision = '';
    let stepPolicy = 0.01; let finePolicy = 0.001;
    let dragPolicy = 'absolute'; let doublePolicy = 'reset'; let tapPolicy = 'none'; let keyboardPolicy = 'full';
    const current = (ticket: number): boolean => !disposed && run !== null && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => !disposed && run !== null && authorityEpoch === ticket;
    const enabled = (): boolean => run !== null && attached && !disposed && !syncing && !disabled.get();
    const clamp = (next: number): number => {
      if (!Number.isFinite(next)) return minimum.get();
      if (next < minimum.get()) return minimum.get();
      if (next > maximum.get()) return maximum.get();
      return next;
    };
    const publish = (next: number, ticket: number): boolean => {
      if (!current(ticket)) return false;
      value.set(clamp(next), 'reason: continuous control display');
      if (!current(ticket)) return false;
      if (run && attached) run.update();
      return current(ticket);
    };
    const cancel = (): boolean => {
      const ticket = ++epoch;
      if (!run || phase.get() === 'idle') return false;
      const basis = revision;
      phase.set('idle', 'reason: continuous control canceled');
      if (!current(ticket) || !publish(accepted, ticket)) return false;
      if (run) run.expose.emit('interactionCancel', { value: accepted, revision: basis });
      return true;
    };
    const propose = (next: number, ticket: number): boolean => {
      if (!current(ticket) || !enabled() || !Number.isFinite(next)) return false;
      const proposal = clamp(next);
      const basis = revision;
      phase.set('idle', 'reason: continuous control proposal');
      if (!current(ticket) || !publish(accepted, ticket)) return false;
      if (run) run.expose.emit('valueCommit', { value: proposal, revision: basis });
      return true;
    };
    const preview = (next: number, ticket: number): boolean => {
      if (!current(ticket) || !enabled() || !Number.isFinite(next)) return false;
      if (!publish(next, ticket)) return false;
      if (run) run.expose.emit('valuePreview', { value: value.get(), revision });
      return current(ticket);
    };
    const edit = (): boolean => {
      if (!enabled()) return false;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || !enabled()) return false;
      if (run) run.expose.emit('editRequest', { value: accepted, revision });
      return true;
    };
    const reset = (): boolean => {
      if (!enabled()) return false;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || !enabled()) return false;
      return propose(run?.props.get().defaultValue ?? accepted, ticket);
    };
    def.expose.method('begin', (candidate: number, jump: boolean, resetModifier: boolean, double: boolean) => {
      if (!enabled() || phase.get() !== 'idle' || !Number.isFinite(candidate)) return false;
      if (double && run?.props.get().doubleAction === 'edit') return edit();
      if (resetModifier || (double && run?.props.get().doubleAction === 'reset')) return reset();
      const ticket = ++epoch;
      focus.focusSelf();
      if (!current(ticket) || !enabled()) return false;
      initialValue.set(accepted, 'reason: continuous control starting authority');
      if (!current(ticket)) return false;
      phase.set(jump ? 'dragging' : 'armed', 'reason: continuous control press');
      if (!current(ticket)) return false;
      if (run) run.expose.emit('interactionStart', { value: accepted, revision });
      if (!current(ticket)) return false;
      if (jump) return preview(candidate, ticket);
      if (run) run.update();
      return current(ticket);
    });
    const sample = (candidate: number, fineCandidate: number, crossed: boolean, fine: boolean, ticket: number): boolean => {
      if (!current(ticket) || !enabled() || phase.get() === 'idle') return false;
      if (!Number.isFinite(candidate) || !Number.isFinite(fineCandidate)) { cancel(); return false; }
      if (phase.get() === 'armed' && !crossed) return true;
      phase.set('dragging', 'reason: continuous control dragging');
      if (!current(ticket)) return false;
      let next = fine ? fineCandidate : candidate;
      const handle = run;
      if (handle && handle.props.get().dragMode === 'steps') {
        const amount = fine ? (handle.props.get().fineStep ?? 0.001) : (handle.props.get().step ?? 0.01);
        next = initialValue.get() + Math.round(candidate) * amount;
      }
      return preview(next, ticket);
    };
    def.expose.method('sample', (candidate: number, fineCandidate: number, crossed: boolean, fine: boolean) => {
      const ticket = ++epoch;
      return sample(candidate, fineCandidate, crossed, fine, ticket);
    });
    def.expose.method('finish', (candidate: number, fineCandidate: number, crossed: boolean, fine: boolean) => {
      const ticket = ++epoch;
      if (!sample(candidate, fineCandidate, crossed, fine, ticket) || !current(ticket)) return false;
      if (phase.get() === 'dragging') return propose(value.get(), ticket);
      const action = run?.props.get().tapAction ?? 'none';
      phase.set('idle', 'reason: continuous control released');
      if (!current(ticket) || !publish(accepted, ticket)) return false;
      if (run && action === 'activate') run.expose.emit('activate', { value: accepted, revision });
      if (action === 'commit') return propose(accepted, ticket);
      if (run && action !== 'activate' && action !== 'commit') run.expose.emit('interactionCancel', { value: accepted, revision });
      return true;
    });
    def.expose.method('cancel', cancel);
    def.expose.method('edit', edit);
    def.expose.method('reset', reset);
    const adjust = (direction: number, fine: boolean): boolean => {
      if (!enabled() || !Number.isFinite(direction)) return false;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || !enabled()) return false;
      const props = run?.props.get();
      const amount = fine ? (props?.fineStep ?? 0.001) : (props?.step ?? 0.01);
      return propose(accepted + direction * amount, ticket);
    };
    def.expose.method('adjust', adjust);
    def.event.on('host:verdant-scalar-increment', () => { adjust(1, false); });
    def.event.on('host:verdant-scalar-decrement', () => { adjust(-1, false); });
    def.event.on('host:verdant-scalar-set-value', (_current, opaque) => {
      const event = opaque as A11ySetValueEventPayload;
      const next = event.value;
      if (!enabled() || typeof next !== 'number' || !Number.isFinite(next)) return;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || !enabled()) return;
      propose(next, ticket);
    });
    def.event.on('key.down', (_current, event) => {
      if (event.key === 'Escape') {
        if (phase.get() !== 'idle') event.control.requestDefaultActionPrevention();
        cancel(); return;
      }
      if (!enabled() || event.altKey || event.ctrlKey || event.metaKey) return;
      const key = event.key ?? '';
      // A scalar inside a traversable point collection owns value steps while
      // the collection owns row navigation and returning to its parent. Keep
      // that division in the source, before consuming the physical key.
      if (run?.props.get().keyboardMode === 'vertical-step'
        && key !== 'ArrowUp' && key !== 'ArrowDown' && key !== '+' && key !== '=' && key !== '-') return;
      if (key !== 'ArrowLeft' && key !== 'ArrowDown' && key !== 'ArrowRight' && key !== 'ArrowUp'
        && key !== 'PageUp' && key !== 'PageDown' && key !== 'Home' && key !== 'End'
        && key !== 'Enter' && key !== 'Backspace' && key !== '+' && key !== '=' && key !== '-') return;
      event.control.requestDefaultActionPrevention();
      if (key === 'Enter') { edit(); return; }
      if (key === 'Backspace') { reset(); return; }
      if (key === 'Home' || key === 'End') {
        const ticket = epoch + 1; cancel();
        if (current(ticket) && enabled()) propose(key === 'Home' ? minimum.get() : maximum.get(), ticket);
        return;
      }
      const direction = key === 'PageUp' ? 10 : key === 'PageDown' ? -10
        : (key === 'ArrowRight' || key === 'ArrowUp' || key === '+' || key === '=') ? 1 : -1;
      adjust(direction, event.shiftKey === true);
    });
    const sync = (handle: RunHandle<Props>) => {
      if (disposed) return;
      const props = handle.props.get();
      const basis = revision;
      const previous = accepted;
      const policyChanged = stepPolicy !== (props.step ?? 0.01) || finePolicy !== (props.fineStep ?? 0.001)
        || dragPolicy !== (props.dragMode ?? 'absolute')
        || doublePolicy !== (props.doubleAction ?? 'reset') || tapPolicy !== (props.tapAction ?? 'none')
        || keyboardPolicy !== (props.keyboardMode ?? 'full');
      const echoed = props.previewEcho === true && phase.get() !== 'idle'
        && (props.revision ?? '') === revision && props.value === value.get()
        && (props.min ?? 0) === minimum.get() && (props.max ?? 1) === maximum.get()
        && props.disabled !== true && !policyChanged;
      // Echoing the currently emitted provisional value is not a competing
      // input. In particular, a synchronous host echo cannot abort finish()
      // between its terminal preview and its one commit proposal.
      if (!echoed) ++epoch;
      const ticket = ++authorityEpoch;
      syncing = true;
      run = handle;
      stepPolicy = props.step ?? 0.01; finePolicy = props.fineStep ?? 0.001;
      dragPolicy = props.dragMode ?? 'absolute'; doublePolicy = props.doubleAction ?? 'reset'; tapPolicy = props.tapAction ?? 'none';
      keyboardPolicy = props.keyboardMode ?? 'full';
      const low = props.min ?? 0;
      const high = props.max ?? 1;
      const valid = Number.isFinite(low) && Number.isFinite(high) && low < high
        && Number.isFinite(props.value ?? 0) && low >= -1e15 && high <= 1e15;
      const rangeChanged = low !== minimum.get() || high !== maximum.get();
      minimum.set(valid ? low : 0, 'reason: continuous control minimum');
      if (!authorityCurrent(ticket)) return;
      maximum.set(valid ? high : 1, 'reason: continuous control maximum');
      if (!authorityCurrent(ticket)) return;
      const next = clamp(props.value ?? 0);
      const nextRevision = props.revision ?? '';
      const echo = props.previewEcho === true && phase.get() !== 'idle'
        && nextRevision === revision && next === value.get();
      const shouldCancel = phase.get() !== 'idle' && (!valid || rangeChanged || policyChanged || props.disabled === true
        || nextRevision !== revision || (!echo && next !== accepted));
      if (!echo) accepted = next;
      revision = nextRevision;
      if (shouldCancel) {
        phase.set('idle', 'reason: continuous control authority changed');
        if (!authorityCurrent(ticket)) return;
      }
      label.set(props.label ?? '', 'reason: continuous control label');
      if (!authorityCurrent(ticket)) return;
      role.set(props.role ?? 'slider', 'reason: continuous control role');
      if (!authorityCurrent(ticket)) return;
      disabled.set(!valid || props.disabled === true, 'reason: continuous control policy');
      if (!authorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!authorityCurrent(ticket)) return;
      if (phase.get() === 'idle') {
        value.set(accepted, 'reason: continuous control authoritative display');
        if (!authorityCurrent(ticket)) return;
      }
      if (shouldCancel) handle.expose.emit('interactionCancel', { value: previous, revision: basis });
      if (!authorityCurrent(ticket)) return;
      syncing = false;
      if (attached) handle.update();
    };
    const retire = (permanent: boolean) => {
      const handle = run;
      const wasActive = phase.get() !== 'idle';
      const basis = revision;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = false; attached = false; disposed = disposed || permanent; run = null;
      if (permanent) return;
      phase.set('idle', 'reason: continuous control detached');
      if (authorityEpoch !== ticket) return;
      value.set(accepted, 'reason: continuous control detached display');
      if (authorityEpoch !== ticket) return;
      if (wasActive && handle) handle.expose.emit('interactionCancel', { value: accepted, revision: basis });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { if (!disposed) { attached = true; sync(handle); } });
    def.props.watch(['value', 'min', 'max', 'step', 'fineStep', 'defaultValue', 'label', 'disabled',
      'revision', 'role', 'dragMode', 'doubleAction', 'tapAction', 'previewEcho', 'keyboardMode'], sync);
    def.event.on('host:blur', cancel);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
