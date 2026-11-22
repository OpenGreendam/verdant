import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asTrigger } from '@proto.ui/hooks';

type Props = { label?: string; revision?: string; disabled?: boolean };

/** A transferable source. The host transports opaque leases; only this source
 * can accept a drop, and each lease is consumed once or retired on cancellation. */
export default definePrototype<Props>({
  name: 'verdant-drag-source',
  setup(def) {
    def.props.define({ label: { type: 'string', default: '' }, revision: { type: 'string', default: '' },
      disabled: { type: 'boolean', default: false } });
    const phase = def.state.string('phase', 'idle');
    const label = def.state.string('label', '');
    const disabled = def.state.bool('disabled', false);
    const focus = asFocusable();
    const a11y = asAccessible();
    asTrigger();
    focus.configure({ disabled: false });
    a11y.role('button');
    a11y.name(label);
    a11y.state('disabled', disabled);
    a11y.action('activate', { event: 'host:verdant-drag-choose' });
    def.feedback.style.use(tw('w-full h-full select-none'));
    def.expose.state('phase', phase);
    def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused);
    def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('choose', { payload: 'json' });
    def.expose.event('cancelled', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let syncing = false;
    let epoch = 0;
    let authorityEpoch = 0;
    let revision = '';
    let nextLease = 0;
    let lease = 0;
    const current = (ticket: number): boolean => run !== null && !disposed && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => run !== null && !disposed && authorityEpoch === ticket;
    const cancel = (expected: number = 0): boolean => {
      if (expected !== 0 && expected !== lease) return false;
      const handle = run;
      const retired = lease;
      const basis = revision;
      const ticket = ++epoch;
      lease = 0;
      phase.set('idle', 'reason: transfer cancelled');
      if (!current(ticket)) return retired !== 0;
      if (handle && retired !== 0) handle.expose.emit('cancelled', { lease: retired, revision: basis });
      if (handle && attached && current(ticket)) handle.update();
      return retired !== 0;
    };
    def.expose.method('begin', () => {
      if (!run || syncing || !attached || disposed || disabled.get() || lease !== 0) return 0;
      const ticket = ++epoch;
      focus.focusSelf();
      if (!current(ticket) || !attached || disabled.get()) return 0;
      nextLease += 1;
      lease = nextLease;
      const token = lease;
      phase.set('dragging', 'reason: transfer acquired');
      if (!current(ticket)) return 0;
      if (run) run.update();
      return current(ticket) ? token : 0;
    });
    def.expose.method('accept', (expected: number, basis: string) => {
      if (!run || syncing || !attached || disposed || disabled.get() || phase.get() !== 'dragging'
        || expected === 0 || expected !== lease || basis !== revision) return false;
      const ticket = ++epoch;
      lease = 0;
      phase.set('idle', 'reason: transfer consumed');
      if (!current(ticket)) return false;
      if (run) run.update();
      return current(ticket);
    });
    def.expose.method('cancel', cancel);
    const choose = () => {
      const handle = run;
      if (!handle || syncing || !attached || disposed || disabled.get() || lease !== 0) return;
      const ticket = ++epoch;
      phase.set('idle', 'reason: transfer chooser');
      if (!current(ticket)) return;
      handle.expose.emit('choose', { revision });
    };
    def.event.on('press.start', () => {
      if (!run || syncing || !attached || disposed || disabled.get() || lease !== 0) return;
      phase.set('pressed', 'reason: transfer pressed');
    });
    def.event.on('press.commit', choose);
    def.event.on('host:verdant-drag-choose', choose);
    def.event.on('press.cancel', () => { if (lease === 0) cancel(); });
    def.event.on('key.down', (_handle, event) => {
      if (event.key === 'Escape' && (lease !== 0 || phase.get() !== 'idle')) {
        event.control.requestDefaultActionPrevention();
        cancel();
      }
    });
    def.event.on('host:blur', () => cancel());
    const sync = (handle: RunHandle<Props>) => {
      if (disposed) return;
      ++epoch;
      const ticket = ++authorityEpoch;
      syncing = true;
      run = handle;
      const props = handle.props.get();
      const nextRevision = props.revision ?? '';
      const retired = nextRevision !== revision || props.disabled === true ? lease : 0;
      const basis = revision;
      revision = nextRevision;
      if (retired !== 0) lease = 0;
      label.set(props.label ?? '', 'reason: transfer label');
      if (!authorityCurrent(ticket)) return;
      disabled.set(props.disabled ?? false, 'reason: transfer policy');
      if (!authorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!authorityCurrent(ticket)) return;
      if (lease === 0) phase.set('idle', 'reason: transfer authority');
      if (!authorityCurrent(ticket)) return;
      if (retired !== 0) handle.expose.emit('cancelled', { lease: retired, revision: basis });
      if (!authorityCurrent(ticket)) return;
      syncing = false;
      if (attached) handle.update();
    };
    const retire = (permanent: boolean) => {
      const handle = run;
      const retired = lease;
      const basis = revision;
      ++epoch;
      const ticket = ++authorityEpoch;
      attached = false;
      disposed = disposed || permanent;
      syncing = false;
      run = null;
      lease = 0;
      if (permanent) return;
      phase.set('idle', 'reason: transfer detached');
      if (authorityEpoch !== ticket) return;
      if (handle && retired !== 0) handle.expose.emit('cancelled', { lease: retired, revision: basis });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { attached = true; sync(handle); });
    def.props.watch(['label', 'revision', 'disabled'], sync);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
