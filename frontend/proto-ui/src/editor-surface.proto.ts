import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';

type Props = { label?: string; revision?: string; disabled?: boolean; threshold?: number; commands?: string };

/**
 * Portable editing surface. A host supplies hit testing and domain transactions;
 * this component owns contact arbitration, movement admission, cancellation,
 * focus and lifecycle. A sample is admitted only for the current contact and
 * authority. The last sample is applied before a successful terminal result.
 */
export default definePrototype<Props>({
  name: 'verdant-editor-surface',
  setup(def) {
    def.props.define({
      label: { type: 'string', default: '' },
      revision: { type: 'string', default: '' },
      disabled: { type: 'boolean', default: false },
      threshold: { type: 'number', default: 0 },
      commands: { type: 'string', default: 'none' },
    });
    const phase = def.state.string('phase', 'idle');
    const pointer = def.state.string('pointer', '');
    const lease = def.state.numberDiscrete('lease', 0);
    const disabled = def.state.bool('disabled', false);
    const label = def.state.string('label', '');
    const focus = asFocusable();
    const accessible = asAccessible();
    focus.configure({ disabled: false });
    accessible.role('group');
    accessible.name(label);
    accessible.state('disabled', disabled);
    def.feedback.style.use(tw('w-full h-full overflow-hidden'));
    def.expose.state('phase', phase);
    def.expose.state('pointer', pointer);
    def.expose.state('lease', lease);
    def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('interactionCancel', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let epoch = 0;
    let authorityEpoch = 0;
    let syncing = false;
    let revision = '';
    let originX = 0;
    let originY = 0;
    let nextLease = 0;
    const current = (ticket: number): boolean => run !== null && !disposed && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => run !== null && !disposed && authorityEpoch === ticket;
    const finite = (x: number, y: number): boolean => Number.isFinite(x) && Number.isFinite(y);
    const cancel = (): boolean => {
      const owner = pointer.get();
      const retired = lease.get();
      const basis = revision;
      const active = phase.get() !== 'idle';
      const ticket = ++epoch;
      phase.set('idle', 'reason: editing contact retired');
      if (!current(ticket)) return active;
      pointer.set('', 'reason: editing contact retired');
      if (!current(ticket)) return active;
      lease.set(0, 'reason: editing contact retired');
      if (!current(ticket)) return active;
      if (active && run) run.expose.emit('interactionCancel', { pointer: owner, revision: basis, lease: retired });
      if (current(ticket) && run) run.update();
      return active;
    };
    def.expose.method('begin', (owner: string, x: number, y: number) => {
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() !== 'idle'
        || owner === '' || !finite(x, y)) return false;
      const ticket = ++epoch;
      originX = x;
      originY = y;
      nextLease += 1;
      lease.set(nextLease, 'reason: editing contact acquired');
      if (!current(ticket)) return false;
      pointer.set(owner, 'reason: editing contact acquired');
      if (!current(ticket)) return false;
      phase.set('pressed', 'reason: editing contact acquired');
      if (!current(ticket)) return false;
      focus.focusSelf();
      if (!current(ticket)) return false;
      if (run) run.update();
      return current(ticket);
    });
    const sample = (owner: string, x: number, y: number): boolean => {
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() === 'idle' || pointer.get() !== owner) return false;
      if (!finite(x, y)) { cancel(); return false; }
      const ticket = epoch;
      const raw = run.props.get().threshold ?? 0;
      const threshold = raw > 0 && Number.isFinite(raw) ? raw : 0;
      const dx = x - originX;
      const dy = y - originY;
      if (phase.get() === 'pressed' && dx * dx + dy * dy < threshold * threshold) return false;
      phase.set('dragging', 'reason: editing movement accepted');
      if (!current(ticket)) return false;
      if (run) run.update();
      return current(ticket);
    };
    def.expose.method('sample', sample);
    def.expose.method('finish', (owner: string, x: number, y: number) => {
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() === 'idle' || pointer.get() !== owner) return false;
      if (!finite(x, y)) { cancel(); return false; }
      const ticket = ++epoch;
      phase.set('idle', 'reason: editing contact completed');
      if (!current(ticket)) return false;
      pointer.set('', 'reason: editing contact completed');
      if (!current(ticket)) return false;
      lease.set(0, 'reason: editing contact completed');
      if (!current(ticket)) return false;
      if (run) run.update();
      return current(ticket);
    });
    def.expose.method('cancel', cancel);
    def.expose.method('cancelPointer', (owner: string) => {
      if (phase.get() === 'idle' || pointer.get() !== owner) return false;
      return cancel();
    });
    const activate = () => {
      if (!run || syncing || !attached || disposed || disabled.get()) return false;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || syncing || !attached || disabled.get()) return false;
      focus.focusSelf();
      return current(ticket) && !syncing && attached && !disabled.get();
    };
    def.expose.method('activate', activate);
    def.expose.method('doubleActivate', (clicks: number) => Number.isFinite(clicks) && clicks >= 2 && activate());
    // Device units (pixels, wheel lines and pinch scale) are normalized by the
    // host. Modifier priority and navigation intent are authored semantics.
    def.expose.method('wheel', (dx: number, dy: number, control: boolean, shift: boolean, alt: boolean, arrange: boolean) => {
      if (!run || syncing || !attached || disposed || disabled.get() || !finite(dx, dy)) return { kind: 'none', x: 0, y: 0, amount: 0 };
      if (control) return { kind: 'zoom-x', x: 0, y: 0, amount: dy };
      if (!arrange && alt) return { kind: 'zoom-y', x: 0, y: 0, amount: dy };
      if (shift) return { kind: 'pan', x: -(dx !== 0 ? dx : dy), y: 0, amount: 0 };
      if (arrange && dx === 0) return { kind: 'none', x: 0, y: 0, amount: 0 };
      return { kind: 'pan', x: -dx, y: arrange ? 0 : -dy, amount: 0 };
    });
    def.expose.method('pinch', (scale: number) => {
      return run !== null && !syncing && attached && !disposed && !disabled.get() && Number.isFinite(scale) && scale > 0;
    });
    // Application command vocabulary stays authored alongside the contact
    // state. Native code maps only portable keys and the returned domain ID.
    def.expose.method('audioKey', (key: string, shift: boolean, secondary: boolean, alt: boolean, repeat: boolean, basis: string) => {
      if (!run || syncing || !attached || disposed || disabled.get() || repeat || basis !== revision
        || run.props.get().commands !== 'audio') return '';
      let command = '';
      if ((key === 'Delete' || key === 'Backspace') && !secondary) command = shift ? 'ripple-delete' : 'delete';
      else if ((key === 'i' || key === 'I') && secondary) command = 'split';
      else if ((key === 'a' || key === 'A') && secondary) command = 'select-all';
      else if ((key === 'j' || key === 'J') && secondary) command = 'join';
      else if ((key === 'r' || key === 'R') && !secondary && !alt) command = 'range-tool';
      else if ((key === 'v' || key === 'V') && !secondary && !alt) command = 'select-tool';
      else if (key === 'Escape') command = 'escape';
      if (command === '') return '';
      const ticket = epoch + 1;
      cancel();
      return current(ticket) && attached && !syncing && !disabled.get() ? command : '';
    });
    def.event.on('key.down', (_run, event) => {
      if (event.key !== 'Escape' || phase.get() === 'idle') return;
      event.control.requestDefaultActionPrevention();
      cancel();
    });
    def.event.on('host:blur', cancel);
    const sync = (handle: RunHandle<Props>) => {
      if (disposed) return;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = true;
      run = handle;
      const props = handle.props.get();
      const nextRevision = props.revision ?? '';
      const owner = pointer.get();
      const retired = lease.get();
      const basis = revision;
      const shouldCancel = phase.get() !== 'idle' && (nextRevision !== revision || props.disabled === true);
      revision = nextRevision;
      if (shouldCancel) {
        phase.set('idle', 'reason: editing authority changed');
        if (!authorityCurrent(ticket)) return;
        pointer.set('', 'reason: editing authority changed');
        if (!authorityCurrent(ticket)) return;
        lease.set(0, 'reason: editing authority changed');
        if (!authorityCurrent(ticket)) return;
      }
      disabled.set(props.disabled ?? false, 'reason: editing policy');
      if (!authorityCurrent(ticket)) return;
      label.set(props.label ?? '', 'reason: editing surface label');
      if (!authorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!authorityCurrent(ticket)) return;
      if (shouldCancel) handle.expose.emit('interactionCancel', { pointer: owner, revision: basis, lease: retired });
      if (!authorityCurrent(ticket)) return;
      syncing = false;
      if (attached) handle.update();
    };
    const retire = (permanent: boolean) => {
      const handle = run;
      const active = phase.get() !== 'idle';
      const owner = pointer.get();
      const retired = lease.get();
      const basis = revision;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = false;
      attached = false;
      disposed = disposed || permanent;
      run = null;
      if (permanent) return;
      phase.set('idle', 'reason: editing surface detached');
      if (authorityEpoch !== ticket) return;
      pointer.set('', 'reason: editing surface detached');
      if (authorityEpoch !== ticket) return;
      lease.set(0, 'reason: editing surface detached');
      if (authorityEpoch !== ticket) return;
      if (active && handle) handle.expose.emit('interactionCancel', { pointer: owner, revision: basis, lease: retired });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { attached = true; sync(handle); });
    def.props.watch(['label', 'revision', 'disabled', 'threshold', 'commands'], sync);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
