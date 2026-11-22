import { definePrototype, tw, type A11ySetValueEventPayload, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';

type Props = {
  x?: number; y?: number; minX?: number; maxX?: number; minY?: number; maxY?: number;
  hasX?: boolean; hasY?: boolean; label?: string; xLabel?: string; yLabel?: string;
  disabled?: boolean; revision?: string;
};

/** A two-dimensional control with an atomic controlled proposal. Pointer,
 * keyboard and assistive routes share its source-owned phase and values. */
export default definePrototype<Props>({
  name: 'verdant-graph-point',
  setup(def) {
    def.props.define({
      x: { type: 'number', default: 0.5 }, y: { type: 'number', default: 0.5 },
      minX: { type: 'number', default: 0 }, maxX: { type: 'number', default: 1 },
      minY: { type: 'number', default: 0 }, maxY: { type: 'number', default: 1 },
      hasX: { type: 'boolean', default: true }, hasY: { type: 'boolean', default: true },
      label: { type: 'string', default: '' }, xLabel: { type: 'string', default: '横轴' }, yLabel: { type: 'string', default: '纵轴' },
      disabled: { type: 'boolean', default: false }, revision: { type: 'string', default: '' },
    });
    const phase = def.state.string('phase', 'idle');
    const x = def.state.numberRange('x', 0.5, { min: 0, max: 1 });
    const y = def.state.numberRange('y', 0.5, { min: 0, max: 1 });
    const axis = def.state.string('axis', 'y');
    const label = def.state.string('label', '');
    const disabled = def.state.bool('disabled', false);
    const valueNow = def.state.numberRange('valueNow', 0.5, { min: 0, max: 1 });
    const valueMin = def.state.numberRange('valueMin', 0, { min: 0, max: 1 });
    const valueMax = def.state.numberRange('valueMax', 1, { min: 0, max: 1 });
    const focus = asFocusable();
    const a11y = asAccessible();
    focus.configure({ disabled: false });
    a11y.role('slider');
    a11y.name(label);
    a11y.state('valueNow', valueNow);
    a11y.state('valueMin', valueMin);
    a11y.state('valueMax', valueMax);
    a11y.state('disabled', disabled);
    a11y.action('increment', { event: 'host:verdant-point-increment' });
    a11y.action('decrement', { event: 'host:verdant-point-decrement' });
    a11y.action('setValue', { event: 'host:verdant-point-set-value' });
    // A two-axis point exposes the active scalar axis. Space/Activate switches
    // that axis; arrows still address X and Y directly without a mode switch.
    a11y.action('activate', { event: 'host:verdant-point-axis' });
    def.feedback.style.use(tw('w-full h-full rounded-full select-none'));
    def.expose.state('phase', phase); def.expose.state('x', x); def.expose.state('y', y);
    def.expose.state('axis', axis); def.expose.state('disabled', disabled);
    def.expose.state('focused', focus.focused); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('valueCommit', { payload: 'json' });
    def.expose.event('interactionCancel', { payload: 'json' });
    def.expose.event('selected', { payload: 'json' });
    def.expose.event('context', { payload: 'json' });
    def.expose.event('exact', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false; let disposed = false; let syncing = false;
    let epoch = 0; let authorityEpoch = 0;
    let revision = ''; let acceptedX = 0.5; let acceptedY = 0.5;
    let lowX = 0; let highX = 1; let lowY = 0; let highY = 1;
    let hasX = true; let hasY = true; let baseLabel = ''; let xLabel = ''; let yLabel = '';
    const current = (ticket: number): boolean => run !== null && !disposed && epoch === ticket;
    const authorityCurrent = (ticket: number): boolean => run !== null && !disposed && authorityEpoch === ticket;
    const clamp = (v: number, low: number, high: number): number => {
      if (!Number.isFinite(v) || v < low) return low;
      if (v > high) return high;
      return v;
    };
    const projection = (ticket: number, authority: boolean): boolean => {
      const valid = (): boolean => authority ? authorityCurrent(ticket) : current(ticket);
      if (!valid()) return false;
      const isX = axis.get() === 'x';
      label.set(baseLabel + '，' + (isX ? xLabel : yLabel) + '归一化值；方向键调整，空格切换坐标轴', 'reason: graph axis label');
      if (!valid()) return false;
      valueNow.set(isX ? x.get() : y.get(), 'reason: graph axis value');
      if (!valid()) return false;
      valueMin.set(isX ? lowX : lowY, 'reason: graph axis minimum');
      if (!valid()) return false;
      valueMax.set(isX ? highX : highY, 'reason: graph axis maximum');
      return valid();
    };
    const publish = (nextX: number, nextY: number, ticket: number): boolean => {
      if (!current(ticket)) return false;
      x.set(clamp(nextX, lowX, highX), 'reason: graph horizontal preview');
      if (!current(ticket)) return false;
      y.set(clamp(nextY, lowY, highY), 'reason: graph vertical preview');
      if (!current(ticket)) return false;
      if (!projection(ticket, false)) return false;
      if (run && attached) run.update();
      return current(ticket);
    };
    const cancel = (): boolean => {
      const handle = run; const ticket = ++epoch; const active = phase.get() !== 'idle'; const basis = revision;
      if (!handle) return false;
      phase.set('idle', 'reason: graph interaction cancelled');
      if (!current(ticket) || !publish(acceptedX, acceptedY, ticket)) return false;
      if (active) handle.expose.emit('interactionCancel', { revision: basis });
      return active;
    };
    const propose = (nextX: number, nextY: number, reset: boolean, ticket: number): boolean => {
      const handle = run;
      if (!handle || !current(ticket) || !attached || disabled.get()) return false;
      const proposalX = clamp(nextX, lowX, highX); const proposalY = clamp(nextY, lowY, highY); const basis = revision;
      phase.set('idle', 'reason: graph proposal settled');
      if (!current(ticket) || !publish(acceptedX, acceptedY, ticket)) return false;
      handle.expose.emit('valueCommit', { x: proposalX, y: proposalY, reset, revision: basis });
      return true;
    };
    def.expose.method('begin', (reset: boolean, double: boolean) => {
      const handle = run;
      if (!handle || syncing || !attached || disposed || disabled.get() || phase.get() !== 'idle') return false;
      const ticket = ++epoch;
      focus.focusSelf();
      if (!current(ticket) || !attached || disabled.get()) return false;
      handle.expose.emit('selected', { revision });
      if (!current(ticket)) return false;
      if (reset || double) return propose(acceptedX, acceptedY, true, ticket);
      phase.set('pressed', 'reason: graph interaction acquired');
      return publish(acceptedX, acceptedY, ticket);
    });
    const sampleAt = (dx: number, dy: number, travelX: number, travelY: number, fine: boolean, ticket: number): boolean => {
      if (!run || !current(ticket) || syncing || !attached || disabled.get() || phase.get() === 'idle') return false;
      if (!Number.isFinite(dx) || !Number.isFinite(dy) || !Number.isFinite(travelX) || !Number.isFinite(travelY)
        || travelX <= 0 || travelY <= 0) { cancel(); return false; }
      if (phase.get() === 'pressed' && dx * dx + dy * dy < 9) return true;
      phase.set('dragging', 'reason: graph movement admitted');
      if (!current(ticket)) return false;
      const scale = fine ? 0.1 : 1;
      return publish(hasX ? acceptedX + dx / travelX * scale : acceptedX,
        hasY ? acceptedY - dy / travelY * scale : acceptedY, ticket);
    };
    def.expose.method('sample', (dx: number, dy: number, travelX: number, travelY: number, fine: boolean) => {
      return sampleAt(dx, dy, travelX, travelY, fine, ++epoch);
    });
    def.expose.method('finish', (dx: number, dy: number, travelX: number, travelY: number, fine: boolean) => {
      const ticket = ++epoch;
      if (!sampleAt(dx, dy, travelX, travelY, fine, ticket) || !current(ticket)) return false;
      if (phase.get() === 'dragging') return propose(x.get(), y.get(), false, ticket);
      phase.set('idle', 'reason: graph click settled');
      return publish(acceptedX, acceptedY, ticket);
    });
    def.expose.method('cancel', cancel);
    const adjust = (which: string, amount: number, endpoint: number): boolean => {
      if (!run || syncing || !attached || disposed || disabled.get() || (which === 'x' ? !hasX : !hasY)) return false;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket)) return false;
      axis.set(which, 'reason: graph keyboard axis');
      if (!current(ticket)) return false;
      let nextX = acceptedX; let nextY = acceptedY;
      if (which === 'x') nextX = endpoint < 0 ? lowX : endpoint > 0 ? highX : acceptedX + amount;
      else nextY = endpoint < 0 ? lowY : endpoint > 0 ? highY : acceptedY + amount;
      return propose(nextX, nextY, false, ticket);
    };
    const switchAxis = () => {
      if (!run || syncing || !attached || disposed || disabled.get() || !hasX || !hasY) return;
      const ticket = epoch + 1;
      cancel();
      if (!current(ticket)) return;
      axis.set(axis.get() === 'x' ? 'y' : 'x', 'reason: graph assistive axis');
      if (!projection(ticket, false)) return;
      if (run) run.update();
    };
    const context = () => {
      const handle = run;
      if (!handle || syncing || !attached || disposed || disabled.get()) return false;
      const ticket = epoch + 1; cancel();
      if (!current(ticket)) return false;
      handle.expose.emit('context', { revision });
      return true;
    };
    def.expose.method('openContext', context);
    def.event.on('host:verdant-point-axis', switchAxis);
    def.event.on('host:verdant-point-increment', () => adjust(axis.get(), 0.01, 0));
    def.event.on('host:verdant-point-decrement', () => adjust(axis.get(), -0.01, 0));
    def.event.on('host:verdant-point-set-value', (_handle, opaque) => {
      const event = opaque as A11ySetValueEventPayload; const next = event.value;
      if (typeof next !== 'number' || !Number.isFinite(next) || !run || syncing || !attached || disabled.get()) return;
      adjust(axis.get(), next - (axis.get() === 'x' ? acceptedX : acceptedY), 0);
    });
    def.event.on('key.down', (_handle, event) => {
      if (event.key === 'Escape') { cancel(); event.control.requestDefaultActionPrevention(); return; }
      if (!run || syncing || !attached || disposed || disabled.get() || event.ctrlKey || event.metaKey || event.altKey) return;
      const amount = event.shiftKey ? 0.001 : 0.01;
      let accepted = false;
      if (event.key === 'ArrowLeft') accepted = adjust(hasX ? 'x' : 'y', -amount, 0);
      else if (event.key === 'ArrowRight') accepted = adjust(hasX ? 'x' : 'y', amount, 0);
      else if (event.key === 'ArrowUp') accepted = adjust(hasY ? 'y' : 'x', amount, 0);
      else if (event.key === 'ArrowDown') accepted = adjust(hasY ? 'y' : 'x', -amount, 0);
      else if (event.key === 'Home') accepted = adjust(axis.get(), 0, -1);
      else if (event.key === 'End') accepted = adjust(axis.get(), 0, 1);
      else if (event.key === 'PageUp') accepted = adjust(axis.get(), amount * 10, 0);
      else if (event.key === 'PageDown') accepted = adjust(axis.get(), -amount * 10, 0);
      else if (event.key === ' ' || event.key === 'Space') { switchAxis(); accepted = true; }
      else if (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey)) accepted = context();
      else if (event.key === 'Backspace') { const ticket = epoch + 1; cancel(); accepted = propose(acceptedX, acceptedY, true, ticket); }
      else if (event.key === 'Enter') { const handle = run; const ticket = epoch + 1; cancel(); if (handle && current(ticket)) { handle.expose.emit('exact', { axis: axis.get(), revision }); accepted = true; } }
      if (accepted) event.control.requestDefaultActionPrevention();
    });
    def.event.on('host:blur', cancel);
    const sync = (handle: RunHandle<Props>) => {
      if (disposed) return;
      ++epoch; const ticket = ++authorityEpoch; syncing = true; run = handle;
      const props = handle.props.get(); const nextRevision = props.revision ?? '';
      const nextX = clamp(props.x ?? 0.5, props.minX ?? 0, props.maxX ?? 1);
      const nextY = clamp(props.y ?? 0.5, props.minY ?? 0, props.maxY ?? 1);
      const shouldCancel = phase.get() !== 'idle' && (nextRevision !== revision || nextX !== acceptedX || nextY !== acceptedY || props.disabled === true);
      revision = nextRevision; acceptedX = nextX; acceptedY = nextY;
      lowX = props.minX ?? 0; highX = props.maxX ?? 1; lowY = props.minY ?? 0; highY = props.maxY ?? 1;
      hasX = props.hasX ?? true; hasY = props.hasY ?? true;
      baseLabel = props.label ?? ''; xLabel = props.xLabel ?? '横轴'; yLabel = props.yLabel ?? '纵轴';
      disabled.set(props.disabled ?? false, 'reason: graph policy');
      if (!authorityCurrent(ticket)) return;
      focus.setDisabled(disabled.get());
      if (!authorityCurrent(ticket)) return;
      if (shouldCancel) phase.set('idle', 'reason: graph authority changed');
      if (!authorityCurrent(ticket)) return;
      if (!hasY) axis.set('x', 'reason: graph available axis');
      else if (!hasX) axis.set('y', 'reason: graph available axis');
      if (!authorityCurrent(ticket)) return;
      if (phase.get() === 'idle') {
        x.set(acceptedX, 'reason: graph horizontal authority');
        if (!authorityCurrent(ticket)) return;
        y.set(acceptedY, 'reason: graph vertical authority');
        if (!authorityCurrent(ticket)) return;
      }
      if (!projection(ticket, true)) return;
      if (shouldCancel) handle.expose.emit('interactionCancel', { revision });
      if (!authorityCurrent(ticket)) return;
      syncing = false; if (attached) handle.update();
    };
    const retire = (permanent: boolean) => {
      const handle = run; const active = phase.get() !== 'idle'; const basis = revision;
      ++epoch; const ticket = ++authorityEpoch; attached = false; disposed = disposed || permanent; run = null; syncing = false;
      if (permanent) return;
      phase.set('idle', 'reason: graph point detached');
      if (authorityEpoch !== ticket) return;
      x.set(acceptedX, 'reason: graph point detached horizontal');
      if (authorityEpoch !== ticket) return;
      y.set(acceptedY, 'reason: graph point detached vertical');
      if (authorityEpoch !== ticket) return;
      if (active && handle) handle.expose.emit('interactionCancel', { revision: basis });
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((handle) => { attached = true; sync(handle); });
    def.props.watch(['x','y','minX','maxX','minY','maxY','hasX','hasY','label','xLabel','yLabel','disabled','revision'], sync);
    def.lifecycle.onUnmounted(() => retire(false));
    def.lifecycle.onBeforeDispose(() => retire(true));
    return (renderer) => renderer.slot();
  },
});
