import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asTrigger } from '@proto.ui/hooks';

type Props = { note?: number; label?: string; revision?: string; disabled?: boolean; momentary?: boolean };

/** Sustained musical key. Contacts and keyboard repeat suppression live here. */
export default definePrototype<Props>({
  name: 'verdant-performance-key',
  setup(def) {
    def.props.define({
      note: { type: 'number', default: 60 }, label: { type: 'string', default: '' },
      revision: { type: 'string', default: '' }, disabled: { type: 'boolean', default: false },
      momentary: { type: 'boolean', default: false },
    });
    const pressed = def.state.bool('pressed', false);
    const disabled = def.state.bool('disabled', false);
    const label = def.state.string('label', '');
    const focus = asFocusable();
    const a11y = asAccessible();
    asTrigger();
    focus.configure({ disabled: false });
    a11y.role('button');
    a11y.name(label);
    a11y.state('pressed', pressed);
    a11y.state('disabled', disabled);
    a11y.action('activate', { event: 'host:verdant-note-toggle' });
    def.feedback.style.use(tw('w-full h-full select-none'));
    def.expose.state('pressed', pressed);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('pressedChange', { payload: 'json' });
    def.expose.event('trigger', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let syncing = false;
    let epoch = 0;
    let authorityEpoch = 0;
    let revision = '';
    let note = 60;
    let momentary = false;
    let direct = false;
    let typing = false;
    let latched = false;
    const current = (ticket: number): boolean => run !== null && !disposed && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => run !== null && !disposed && authorityEpoch === ticket;
    const publish = (ticket: number): boolean => {
      const handle = run;
      if (!handle || !current(ticket)) return false;
      const next = attached && !disabled.get() && (direct || typing || latched);
      if (next === pressed.get()) return false;
      const basis = revision;
      pressed.set(next, 'reason: performance key contacts');
      if (!current(ticket)) return false;
      handle.expose.emit('pressedChange', { pressed: next, revision: basis });
      if (current(ticket) && attached) handle.update();
      return current(ticket);
    };
    const cancel = (): boolean => {
      const ticket = ++epoch;
      direct = false;
      typing = false;
      latched = false;
      return publish(ticket);
    };
    const keyNote = (key: string): number => {
      if (key === 'a') return 60;
      if (key === 'w') return 61;
      if (key === 's') return 62;
      if (key === 'e') return 63;
      if (key === 'd') return 64;
      if (key === 'f') return 65;
      if (key === 't') return 66;
      if (key === 'g') return 67;
      if (key === 'y') return 68;
      if (key === 'h') return 69;
      if (key === 'u') return 70;
      if (key === 'j') return 71;
      if (key === 'k') return 72;
      return -1;
    };
    def.expose.method('keyDown', (key: string) => {
      if (!run || syncing || !attached || disposed || disabled.get() || keyNote(key) !== note) return false;
      const repeated = typing;
      typing = true;
      const ticket = ++epoch;
      publish(ticket);
      if (momentary && !repeated && current(ticket) && run) run.expose.emit('trigger', { revision });
      return true;
    });
    def.expose.method('keyUp', (key: string) => {
      if (!run || syncing || !attached || disposed || keyNote(key) !== note) return false;
      typing = false;
      publish(++epoch);
      return true;
    });
    def.expose.method('cancel', cancel);
    def.event.on('press.start', () => {
      if (!run || syncing || !attached || disposed || disabled.get()) return;
      direct = true;
      const ticket = ++epoch;
      publish(ticket);
      if (momentary && current(ticket) && run) run.expose.emit('trigger', { revision });
    });
    const releaseDirect = () => { direct = false; publish(++epoch); };
    def.event.on('press.end', releaseDirect);
    def.event.on('press.cancel', releaseDirect);
    def.event.on('pointer.cancel', releaseDirect);
    def.event.on('host:blur', cancel);
    def.event.on('host:verdant-note-toggle', () => {
      const handle = run;
      if (!handle || syncing || !attached || disposed || disabled.get()) return;
      if (momentary) { handle.expose.emit('trigger', { revision }); return; }
      latched = !latched;
      publish(++epoch);
    });
    def.event.on('key.down', (_run, event) => {
      if (event.key === 'Escape') {
        event.control.requestDefaultActionPrevention();
        cancel();
      }
    });
    const sync = (handle: RunHandle<Props>) => {
      if (disposed) return;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = true;
      run = handle;
      const props = handle.props.get();
      const nextRevision = props.revision ?? '';
      const nextNote = props.note ?? 60;
      const release = nextRevision !== revision || nextNote !== note || props.disabled === true || (props.momentary ?? false) !== momentary;
      const wasPressed = pressed.get();
      const basis = revision;
      revision = nextRevision;
      note = nextNote;
      momentary = props.momentary ?? false;
      if (release) { direct = false; typing = false; latched = false; }
      label.set(props.label ?? '', 'reason: performance key name');
      if (!authorityCurrent(ticket)) return;
      disabled.set(props.disabled ?? false, 'reason: performance key policy');
      if (!authorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!authorityCurrent(ticket)) return;
      const next = attached && !disabled.get() && (direct || typing || latched);
      pressed.set(next, 'reason: performance key authority');
      if (!authorityCurrent(ticket)) return;
      if (wasPressed !== next) handle.expose.emit('pressedChange', { pressed: next, revision: basis });
      if (!authorityCurrent(ticket)) return;
      syncing = false;
      if (attached) handle.update();
    };
    const retire = (permanent: boolean) => {
      const handle = run;
      const wasPressed = pressed.get();
      const basis = revision;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = false;
      attached = false;
      disposed = disposed || permanent;
      run = null;
      direct = false;
      typing = false;
      latched = false;
      if (permanent) return;
      pressed.set(false, 'reason: performance key detached');
      if (authorityEpoch !== ticket) return;
      if (wasPressed && handle) handle.expose.emit('pressedChange', { pressed: false, revision: basis });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { attached = true; sync(handle); });
    def.props.watch(['note', 'label', 'revision', 'disabled', 'momentary'], sync);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
