import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asFocusScope, asScrollSurface } from '@proto.ui/hooks';

type Props = { label?: string; description?: string; role?: string; autoFocus?: boolean; trapFocus?: boolean; tabStop?: boolean; axes?: string; revealIndex?: number; revealRevision?: string };

// The native slot supplies document layout. Scroll extents, native wheel
// projection and focus targets are host facts; keyboard page/edge requests,
// scope traversal and the ScrollSurface lifetime are portable author policy.
export default definePrototype<Props>({
  name: 'verdant-scroll-region',
  setup(def) {
    def.props.define({ label: { type: 'string', default: '' }, description: { type: 'string', default: '' },
      role: { type: 'string', default: 'document' }, autoFocus: { type: 'boolean', default: false }, trapFocus: { type: 'boolean', default: false }, tabStop: { type: 'boolean', default: true }, axes: { type: 'string', default: 'vertical' },
      revealIndex: { type: 'number', default: -1 }, revealRevision: { type: 'string', default: '' } });
    const label = def.state.string('label', ''); const description = def.state.string('description', '');
    const role = def.state.string('role', 'document');
    const focus = asFocusable(); focus.configure({ disabled: false });
    const scope = asFocusScope(); scope.configure({ trap: true, loop: true, navigation: 'tab', entry: 'manual', restore: 'none' });
    const scroll = asScrollSurface(); scroll.configure({ axes: 'both', projection: 'system' });
    const a11y = asAccessible(); a11y.role(role); a11y.name(label); a11y.description(description);
    def.expose.state('horizontalPosition', scroll.horizontal.position);
    def.expose.state('position', scroll.vertical.position); def.expose.state('atEnd', scroll.vertical.atEnd);
    def.expose.state('focused', focus.focused); def.expose.state('scopeActive', scope.active);
    let attached = false; let disposed = false; let epoch = 0; let axes = 'vertical';
    let revealIndex = -1; let revealRevision = '';
    const key = (name: string, shift: boolean): boolean => {
      if (!attached || disposed) return false;
      if (name === 'Tab') { if (!scope.isActive()) return false; if (shift) scope.focusPrev(); else scope.focusNext(); return true; }
      // A horizontal document leaves vertical arrows to its surrounding content.
      if (axes === 'horizontal' && (name === 'ArrowUp' || name === 'ArrowDown')) return false;
      const horizontal = axes === 'horizontal' || (axes === 'both' && shift);
      if ((name === 'ArrowLeft' || name === 'ArrowRight') && axes !== 'vertical') {
        scroll.request({ kind: 'by', axis: 'horizontal', delta: name === 'ArrowLeft' ? -40 : 40 }); return true;
      }
      if (name === 'ArrowUp') { if (horizontal) scroll.request({ kind: 'by', axis: 'horizontal', delta: -40 }); else scroll.request({ kind: 'by', axis: 'vertical', delta: -40 }); }
      else if (name === 'ArrowDown') { if (horizontal) scroll.request({ kind: 'by', axis: 'horizontal', delta: 40 }); else scroll.request({ kind: 'by', axis: 'vertical', delta: 40 }); }
      else if (name === 'PageUp' || (name === ' ' && shift)) { if (horizontal) scroll.request({ kind: 'page', axis: 'horizontal', direction: 'before' }); else scroll.request({ kind: 'page', axis: 'vertical', direction: 'before' }); }
      else if (name === 'PageDown' || name === ' ') { if (horizontal) scroll.request({ kind: 'page', axis: 'horizontal', direction: 'after' }); else scroll.request({ kind: 'page', axis: 'vertical', direction: 'after' }); }
      else if (name === 'Home') { if (horizontal) scroll.request({ kind: 'to', axis: 'horizontal', position: 0 }); else scroll.request({ kind: 'to', axis: 'vertical', position: 0 }); }
      else if (name === 'End') { if (horizontal) scroll.request({ kind: 'to-end', axis: 'horizontal' }); else scroll.request({ kind: 'to-end', axis: 'vertical' }); }
      else return false;
      return true;
    };
    const keyInput = (name: string, shift: boolean, command: boolean, alt: boolean): boolean => {
      if (command || alt) return false;
      return key(name, shift);
    };
    def.expose.method('key', key);
    def.expose.method('keyInput', keyInput);
    // Layout contributes measured item/viewport edges. The source validates
    // the accepted target and owns nearest-visible reveal policy on that axis.
    def.expose.method('revealGeometry', (index: number, start: number, end: number, viewportStart: number, viewportEnd: number, basis: string): boolean => {
      if (!attached || disposed || basis !== revealRevision || index !== revealIndex
        || !Number.isFinite(index) || index < 0 || index % 1 !== 0
        || !Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(viewportStart) || !Number.isFinite(viewportEnd)
        || end < start || viewportEnd <= viewportStart) return false;
      // An oversized item already spanning the viewport is visible; moving it
      // would make nearest-reveal alternate between its opposite edges.
      if (start <= viewportStart && end >= viewportEnd) return true;
      const delta = start < viewportStart ? start - viewportStart : end > viewportEnd ? end - viewportEnd : 0;
      if (delta !== 0) {
        if (axes === 'horizontal') scroll.request({ kind: 'by', axis: 'horizontal', delta });
        else scroll.request({ kind: 'by', axis: 'vertical', delta });
      }
      return true;
    });
    def.expose.method('focusSelf', () => { if (attached && !disposed) focus.focusSelf(); });
    def.event.on('key.down', (_run, event) => {
      // Native descendants own their editing keys. The explicit keyInput
      // method remains the host's fallback after a child leaves a key free.
      if (!focus.focused.get()) return;
      if (keyInput(event.key ?? '', event.shiftKey === true, event.ctrlKey === true || event.metaKey === true, event.altKey === true)) event.control.requestDefaultActionPrevention();
    });
    const sync = (run: RunHandle<Props>) => {
      if (disposed) return; const ticket = ++epoch; const props = run.props.get();
      axes = props.axes === 'horizontal' || props.axes === 'both' ? props.axes : 'vertical';
      if (axes === 'horizontal') scroll.sync({ axes: 'horizontal' });
      else if (axes === 'both') scroll.sync({ axes: 'both' });
      else scroll.sync({ axes: 'vertical' });
      if (epoch !== ticket || disposed) return;
      revealIndex = props.revealIndex ?? -1; revealRevision = props.revealRevision ?? '';
      label.set(props.label ?? '', 'reason: scroll region name'); if (epoch !== ticket || disposed) return;
      description.set(props.description ?? '', 'reason: scroll region description'); if (epoch !== ticket || disposed) return;
      role.set(props.role ?? 'document', 'reason: scroll region role'); if (epoch !== ticket || disposed) return;
      focus.setNavParticipation(props.tabStop === false ? 'none' : 'auto'); if (epoch !== ticket || disposed) return;
      if (attached) {
        if (props.trapFocus === true && !scope.isActive()) scope.activate();
        else if (props.trapFocus !== true && scope.isActive()) scope.deactivate();
        if (epoch !== ticket || disposed) return;
        run.update();
      }
    };
    def.lifecycle.onCreated(sync);
    def.props.watch(['label', 'description', 'role', 'autoFocus', 'trapFocus', 'tabStop', 'axes', 'revealIndex', 'revealRevision'], sync);
    def.lifecycle.onMounted((run) => {
      if (disposed) return; attached = true; sync(run); const ticket = epoch;
      if (ticket !== epoch || !attached || disposed) return;
      if (run.props.get().autoFocus) focus.focusSelf();
    });
    // Also clear the observed active fact when the scope root itself had focus.
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; scope.deactivate(); });
    def.lifecycle.onBeforeDispose(() => { ++epoch; attached = false; disposed = true; scope.deactivate(); });
    def.feedback.style.use(tw('w-full h-full min-h-0'));
    return renderer => renderer.slot();
  },
});
