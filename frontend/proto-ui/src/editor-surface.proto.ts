import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';

type Props = { label?: string; revision?: string; disabled?: boolean; threshold?: number; commandMode?: string };

/**
 * Portable editing surface. A host supplies hit testing and domain transactions;
 * this component owns gesture intent, navigation, axis/snap policy, contact
 * arbitration, cancellation, focus and lifecycle. A sample is admitted only for the current contact and
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
      commandMode: { type: 'string', default: '' },
    });
    const phase = def.state.string('phase', 'idle');
    const pointer = def.state.string('pointer', '');
    const lease = def.state.numberDiscrete('lease', 0);
    const operation = def.state.string('operation', '');
    const axis = def.state.string('axis', '');
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
    def.expose.state('operation', operation);
    def.expose.state('axis', axis);
    def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('interactionCancel', { payload: 'json' });
    def.expose.event('command', { payload: 'json' });
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
    let freeTime = false;
    const current = (ticket: number): boolean => run !== null && !disposed && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => run !== null && !disposed && authorityEpoch === ticket;
    const finite = (x: number, y: number): boolean => Number.isFinite(x) && Number.isFinite(y);
    const cancel = (): boolean => {
      const owner = pointer.get();
      const retired = lease.get();
      const basis = revision;
      const active = phase.get() !== 'idle';
      const ticket = ++epoch;
      freeTime = false;
      phase.set('idle', 'reason: editing contact retired');
      if (!current(ticket)) return active;
      pointer.set('', 'reason: editing contact retired');
      if (!current(ticket)) return active;
      lease.set(0, 'reason: editing contact retired');
      if (!current(ticket)) return active;
      operation.set('', 'reason: editing operation retired');
      if (!current(ticket)) return active;
      axis.set('', 'reason: editing axis retired');
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
      freeTime = false;
      nextLease += 1;
      operation.set('', 'reason: editing operation acquired');
      if (!current(ticket)) return false;
      axis.set('', 'reason: editing axis acquired');
      if (!current(ticket)) return false;
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
      operation.set('', 'reason: editing operation completed');
      if (!current(ticket)) return false;
      axis.set('', 'reason: editing axis completed');
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
    const navigate = (source: string, dx: number, dy: number, scale: number, control: boolean, shift: boolean, alt: boolean, arrange: boolean) => {
      const none = { kind: 'none', x: 0, y: 0, scale: 1, exponent: 0 };
      if (!run || syncing || !attached || disposed || disabled.get() || !finite(dx, dy)) return none;
      // Buttons and remappable keyboard commands share the same authored zoom
      // step. The host only evaluates scale ** exponent and musical geometry.
      if (source === 'zoom-in') return { kind: 'zoom-x', x: 0, y: 0, scale: 1.25, exponent: 1 };
      if (source === 'zoom-out') return { kind: 'zoom-x', x: 0, y: 0, scale: 0.8, exponent: 1 };
      if (source === 'pinch') return Number.isFinite(scale) && scale > 0
        ? { kind: 'zoom-x', x: 0, y: 0, scale, exponent: 1 } : none;
      if (source !== 'wheel') return none;
      if (control) return { kind: 'zoom-x', x: 0, y: 0, scale: 1.0015, exponent: dy };
      if (!arrange && alt) return { kind: 'zoom-y', x: 0, y: 0, scale: 1.0015, exponent: dy };
      if (shift) return { kind: 'pan', x: -(dx !== 0 ? dx : dy), y: 0, scale: 1, exponent: 0 };
      if (arrange && dx === 0) return none;
      return { kind: 'pan', x: -dx, y: arrange ? 0 : -dy, scale: 1, exponent: 0 };
    };
    def.expose.method('navigate', navigate);
    def.expose.method('wheel', (dx: number, dy: number, control: boolean, shift: boolean, alt: boolean, arrange: boolean) => navigate('wheel', dx, dy, 1, control, shift, alt, arrange));
    def.expose.method('pinch', (scale: number) => navigate('pinch', 0, 0, scale, false, false, false, false));
    const intent = (kind: string, selection: string, snap: boolean) => {
      const none = { kind: 'none', selection: 'none', snap: false };
      if (!run || syncing || !attached || disposed || disabled.get() || operation.get() !== '' || kind === 'none') return none;
      const ticket = epoch;
      operation.set(kind, 'reason: editing operation chosen');
      return current(ticket) && !syncing && attached && !disabled.get() ? { kind, selection, snap } : none;
    };
    def.expose.method('regionGesture', (hit: string, range: boolean, secondary: boolean, extend: boolean,
      split: boolean, clicks: number, left: number, right: number, fadeIn: number, fadeOut: number, fadeBand: boolean, alt: boolean) => {
      if (!secondary && phase.get() !== 'pressed') return { kind: 'none', selection: 'none', snap: false };
      if (secondary) return intent(hit === 'empty' ? 'lane-menu' : 'region-menu', hit === 'audio' && extend ? 'extend' : 'replace', !alt);
      if (range) return intent('range', 'replace', !alt);
      if (hit === 'empty') return intent(clicks >= 2 ? 'create-part' : 'seek', 'clear', !alt);
      if (hit !== 'audio' && hit !== 'part') return { kind: 'none', selection: 'none', snap: false };
      if (hit === 'audio' && extend) return intent('select', 'extend', !alt);
      if (hit === 'audio' && split) return intent('split', 'replace', !alt);
      if (clicks >= 2) return intent(hit === 'audio' ? 'region-menu' : 'open-part', 'replace', !alt);
      if (hit === 'audio' && fadeBand && fadeIn >= 0 && fadeIn < 7) return intent('fade-in', 'replace', !alt);
      if (hit === 'audio' && fadeBand && fadeOut >= 0 && fadeOut < 7) return intent('fade-out', 'replace', !alt);
      if (left >= 0 && left < 7) return intent('resize-left', 'replace', !alt);
      if (right >= 0 && right < 7) return intent('resize-right', 'replace', !alt);
      return intent('move-region', 'replace', !alt);
    });
    def.expose.method('pointGesture', (family: string, hit: boolean, secondary: boolean, clicks: number, alt: boolean) => {
      if (!secondary && phase.get() !== 'pressed') return { kind: 'none', selection: 'none', snap: false };
      if (family !== 'tempo' && family !== 'automation') return { kind: 'none', selection: 'none', snap: false };
      if (secondary) return intent(family === 'tempo' || hit ? 'point-menu' : 'none', 'none', !alt);
      if (family === 'tempo' && clicks >= 2) return intent('edit-tempo', 'none', !alt);
      return intent(family === 'tempo' ? (hit ? 'move-tempo' : 'create-tempo') : (hit ? 'move-automation' : 'create-automation'), 'none', !alt);
    });
    def.expose.method('curveGesture', (tool: string, eraser: boolean, hit: boolean, selected: boolean,
      points: boolean, portamento: boolean, shift: boolean, alt: boolean) => {
      if (phase.get() !== 'pressed' || operation.get() !== '' || (tool !== 'select' && tool !== 'eraser'
        && tool !== 'line' && tool !== 'smooth' && tool !== 'pitch' && tool !== 'pencil')) return { kind: 'none', selection: 'none', snap: false };
      freeTime = portamento;
      if (eraser || tool === 'eraser') return intent('curve-erase', 'none', false);
      if (tool === 'select' && points && hit) return intent('curve-point', shift ? 'toggle' : selected ? 'keep' : 'replace', !alt && !portamento);
      if (tool === 'select') return intent('curve-select', 'none', !alt && !portamento);
      if (tool === 'line') return intent('curve-line', 'none', false);
      if (tool === 'smooth') return intent('curve-smooth', 'none', false);
      if (tool === 'pitch') return intent('pitch-draw', 'none', false);
      return intent('curve-pencil', 'none', false);
    });
    def.expose.method('noteGesture', (tool: string, hit: boolean, touch: boolean, longPress: boolean,
      modulation: boolean, alt: boolean) => {
      const none = { kind: 'none', selection: 'none', snap: false };
      if (phase.get() !== 'pressed' || (tool !== 'pointer' && tool !== 'pencil' && tool !== 'eraser' && tool !== 'pitch')) return none;
      if (longPress) return tool === 'pitch' ? none : intent(hit ? 'note-lyric' : 'note-marquee', 'none', false);
      if (tool === 'pitch') return intent('pitch-draw', hit ? 'replace' : 'none', false);
      if (!touch && modulation) return intent('note-modulation', 'none', false);
      if (touch && tool === 'pointer' && !hit) return none;
      if (tool === 'eraser') return hit ? intent('erase-note', 'none', false) : none;
      if (tool === 'pencil' && !hit) return intent('draw-note', 'none', !alt);
      return intent('note-edit', 'none', !alt);
    });
    def.expose.method('phonemeGesture', (boundary: boolean, note: boolean, editable: boolean) => {
      if (phase.get() !== 'pressed') return { kind: 'none', selection: 'none', snap: false };
      return intent(boundary && editable ? 'phoneme-drag' : note ? 'select-note' : 'none', note ? 'replace' : 'none', false);
    });
    def.expose.method('rulerGesture', (secondary: boolean, clicks: number, arrange: boolean, segment: boolean,
      loopStart: boolean, loopEnd: boolean, shift: boolean, alt: boolean) => {
      if (!secondary && phase.get() !== 'pressed') return { kind: 'none', selection: 'none', snap: false };
      if (secondary) return intent('ruler-menu', 'none', true);
      if (clicks >= 2 && !arrange && segment) return intent('edit-segment', 'none', true);
      if (loopStart) return intent('drag-loop-start', 'none', true);
      if (loopEnd) return intent('drag-loop-end', 'none', true);
      if (shift) return intent('set-loop-start', 'none', true);
      if (alt) return intent('set-loop-end', 'none', true);
      return intent('scrub', 'none', true);
    });
    def.expose.method('motion', (x: number, y: number, shift: boolean, alt: boolean) => {
      const none = { kind: 'none', snap: false, fine: 1, axis: '', moveX: false, moveY: false };
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() === 'idle' || operation.get() === '' || !finite(x, y)) return none;
      const kind = operation.get();
      const dx = x - originX; const dy = y - originY;
      const ax = dx < 0 ? -dx : dx; const ay = dy < 0 ? -dy : dy;
      const curve = kind === 'curve-point' || kind === 'curve-select' || kind === 'curve-line'
        || kind === 'curve-pencil' || kind === 'curve-smooth' || kind === 'curve-erase' || kind === 'pitch-draw';
      const ticket = epoch;
      if (!shift || !curve) axis.set('', 'reason: editing axis released');
      else if (axis.get() === '' && (ax >= 3 || ay >= 3)) axis.set(ax >= ay ? 'x' : 'y', 'reason: editing axis chosen');
      if (!current(ticket)) return none;
      const snap = !alt && !freeTime && kind !== 'curve-line' && kind !== 'curve-pencil' && kind !== 'curve-smooth' && kind !== 'curve-erase' && kind !== 'pitch-draw' && kind !== 'phoneme-drag';
      return { kind, snap, fine: shift ? 0.1 : 1, axis: axis.get(), moveX: ax >= 3, moveY: ay >= 3 };
    });
    def.expose.method('edgePan', (x: number, width: number) => {
      const none = { position: Number.isFinite(x) ? x : 0, delta: 0, active: false };
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() === 'idle'
        || (operation.get() !== 'note-edit' && operation.get() !== 'note-marquee') || !finite(x, width) || width <= 0) return none;
      if (x > width - 16) return { position: x < width + 40 ? x : width + 40, delta: x - (width - 16) < 40 ? x - (width - 16) : 40, active: true };
      if (x < 16) return { position: x, delta: x - 16 > -40 ? x - 16 : -40, active: true };
      return none;
    });
    def.expose.method('rulerSample', (position: number, outside: number, elapsed: number) => {
      const none = { position: 0, delta: 0, active: false, snap: false, accepted: false };
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() === 'idle'
        || operation.get() !== 'scrub' || !finite(position, outside) || !Number.isFinite(elapsed) || elapsed < 0) return none;
      const distance = outside < 0 ? -outside : outside;
      const speed = 100 + 9 * distance;
      const velocity = outside === 0 ? 0 : (outside < 0 ? -1 : 1) * (speed < 2000 ? speed : 2000);
      return { position, delta: velocity * (elapsed < 0.05 ? elapsed : 0.05), active: outside !== 0,
        snap: phase.get() === 'pressed', accepted: true };
    });
    def.event.on('key.down', (_run, event) => {
      const handle = run;
      if (!handle || syncing || !attached || disposed || disabled.get() || event.repeat) return;
      const audio = handle.props.get().commandMode === 'audio';
      const secondary = event.ctrlKey || event.metaKey;
      let command = '';
      if (audio) {
        if ((event.key === 'Delete' || event.key === 'Backspace') && !secondary) command = event.shiftKey ? 'ripple-delete' : 'delete';
        else if ((event.key === 'i' || event.key === 'I') && secondary) command = 'split';
        else if ((event.key === 'a' || event.key === 'A') && secondary) command = 'select-all';
        else if ((event.key === 'j' || event.key === 'J') && secondary) command = 'join';
        else if ((event.key === 'r' || event.key === 'R') && !secondary && !event.altKey) command = 'range-tool';
        else if ((event.key === 'v' || event.key === 'V') && !secondary && !event.altKey) command = 'select-tool';
        else if (event.key === 'Escape') command = 'clear-range';
      }
      if (command === '' && (event.key !== 'Escape' || phase.get() === 'idle')) return;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket) || syncing || !attached || disabled.get()) return;
      event.control.requestDefaultActionPrevention();
      if (command !== '') handle.expose.emit('command', { command, revision });
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
        operation.set('', 'reason: editing authority changed');
        if (!authorityCurrent(ticket)) return;
        axis.set('', 'reason: editing authority changed');
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
      operation.set('', 'reason: editing operation detached');
      if (authorityEpoch !== ticket) return;
      axis.set('', 'reason: editing axis detached');
      if (authorityEpoch !== ticket) return;
      if (active && handle) handle.expose.emit('interactionCancel', { pointer: owner, revision: basis, lease: retired });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { attached = true; sync(handle); });
    def.props.watch(['label', 'revision', 'disabled', 'threshold', 'commandMode'], sync);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
