import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asFocusScope, asScrollSurface } from '@proto.ui/hooks';

type Props = { label?: string; description?: string; role?: string; autoFocus?: boolean; trapFocus?: boolean; axes?: string; tabStop?: boolean };

// The native slot supplies document layout. Scroll extents, native wheel
// projection and focus targets are host facts; keyboard page/edge requests,
// scope traversal and the ScrollSurface lifetime are portable author policy.
export default definePrototype<Props>({
  name: 'verdant-scroll-region',
  setup(def) {
    def.props.define({ label: { type: 'string', default: '' }, description: { type: 'string', default: '' },
      role: { type: 'string', default: 'document' }, autoFocus: { type: 'boolean', default: false },
      trapFocus: { type: 'boolean', default: false }, axes: { type: 'string', default: 'vertical' },
      tabStop: { type: 'boolean', default: true } });
    const label = def.state.string('label', ''); const description = def.state.string('description', '');
    const role = def.state.string('role', 'document');
    const focus = asFocusable(); focus.configure({ disabled: false });
    const scope = asFocusScope(); scope.configure({ trap: true, loop: true, navigation: 'tab', entry: 'manual', restore: 'none' });
    const scroll = asScrollSurface(); scroll.configure({ axes: 'vertical', projection: 'system' });
    const a11y = asAccessible(); a11y.role(role); a11y.name(label); a11y.description(description);
    def.expose.state('position', scroll.vertical.position); def.expose.state('atEnd', scroll.vertical.atEnd);
    def.expose.state('focused', focus.focused); def.expose.state('scopeActive', scope.active);
    let attached = false; let disposed = false; let epoch = 0; let axes = 'vertical';
    const key = (name: string, shift: boolean): boolean => {
      // A descendant input owns its arrows, Home/End and Space. Only the
      // focused document itself handles navigation here. FocusScope supplies
      // Tab when explicitly enabled; ordinary scrollers never trap it.
      if (!attached || disposed || !focus.focused.get()) return false;
      if (axes === 'horizontal') {
        if (name === 'ArrowLeft') scroll.request({ kind: 'by', axis: 'horizontal', delta: -40 });
        else if (name === 'ArrowRight') scroll.request({ kind: 'by', axis: 'horizontal', delta: 40 });
        else if (name === 'PageUp') scroll.request({ kind: 'page', axis: 'horizontal', direction: 'before' });
        else if (name === 'PageDown') scroll.request({ kind: 'page', axis: 'horizontal', direction: 'after' });
        else if (name === 'Home') scroll.request({ kind: 'to', axis: 'horizontal', position: 0 });
        else if (name === 'End') scroll.request({ kind: 'to-end', axis: 'horizontal' });
        else return false;
        return true;
      }
      if (axes === 'both' && (name === 'ArrowLeft' || name === 'ArrowRight')) {
        scroll.request({ kind: 'by', axis: 'horizontal', delta: name === 'ArrowLeft' ? -40 : 40 });
        return true;
      }
      if (name === 'ArrowUp') scroll.request({ kind: 'by', axis: 'vertical', delta: -40 });
      else if (name === 'ArrowDown') scroll.request({ kind: 'by', axis: 'vertical', delta: 40 });
      else if (name === 'PageUp' || (name === ' ' && shift)) scroll.request({ kind: 'page', axis: 'vertical', direction: 'before' });
      else if (name === 'PageDown' || name === ' ') scroll.request({ kind: 'page', axis: 'vertical', direction: 'after' });
      else if (name === 'Home') scroll.request({ kind: 'to', axis: 'vertical', position: 0 });
      else if (name === 'End') scroll.request({ kind: 'to-end', axis: 'vertical' });
      else return false;
      return true;
    };
    def.expose.method('key', key);
    def.expose.method('focusSelf', () => { if (attached && !disposed) focus.focusSelf(); });
    def.event.on('key.down', (_run, event) => {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      if (key(event.key ?? '', event.shiftKey === true)) event.control.requestDefaultActionPrevention();
    });
    const sync = (run: RunHandle<Props>) => {
      if (disposed) return; const ticket = ++epoch; const props = run.props.get();
      label.set(props.label ?? '', 'reason: scroll region name'); if (epoch !== ticket || disposed) return;
      description.set(props.description ?? '', 'reason: scroll region description'); if (epoch !== ticket || disposed) return;
      role.set(props.role ?? 'document', 'reason: scroll region role'); if (epoch !== ticket || disposed) return;
      focus.setNavParticipation(props.tabStop === false ? 'none' : 'auto');
      if (epoch !== ticket || disposed) return;
      axes = props.axes === 'horizontal' ? 'horizontal' : props.axes === 'both' ? 'both' : 'vertical';
      if (axes === 'horizontal') scroll.sync({ axes: 'horizontal' });
      else if (axes === 'both') scroll.sync({ axes: 'both' });
      else scroll.sync({ axes: 'vertical' });
      if (epoch !== ticket || disposed) return;
      if (attached && props.trapFocus === true && !scope.isActive()) scope.activate();
      else if (props.trapFocus !== true && scope.isActive()) scope.deactivate();
      if (epoch !== ticket || disposed) return;
      if (attached) run.update();
    };
    def.lifecycle.onCreated(sync);
    def.props.watch(['label', 'description', 'role', 'autoFocus', 'trapFocus', 'axes', 'tabStop'], sync);
    def.lifecycle.onMounted((run) => {
      if (disposed) return; attached = true; sync(run); const ticket = epoch;
      if (ticket !== epoch || !attached || disposed) return;
      if (run.props.get().autoFocus) focus.focusSelf();
    });
    // Also clears the observed active fact when the root itself had focus;
    // an inactive scope need not be present in the host's scope stack.
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; scope.deactivate(); });
    def.lifecycle.onBeforeDispose(() => { ++epoch; attached = false; disposed = true; scope.deactivate(); });
    def.feedback.style.use(tw('w-full h-full min-h-0'));
    return renderer => renderer.slot();
  },
});
