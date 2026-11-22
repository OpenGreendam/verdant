import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asFocusScope, asOverlay } from '@proto.ui/hooks';

type Props = {
  label?: string; description?: string; role?: string; revision?: string;
  open?: boolean; disabled?: boolean; defaultChoice?: number; cancelChoice?: number; choices?: number;
};

// The application supplies response meanings and native content. Modal scope,
// entry/restoration, accepted visibility and keyboard response policy belong
// to this retained compiled root, including prompts outside the editor entity.
export default definePrototype<Props>({
  name: 'verdant-overlay-control',
  setup(def) {
    def.props.define({
      label: { type: 'string', default: '' }, description: { type: 'string', default: '' },
      role: { type: 'string', default: 'dialog' }, revision: { type: 'string', default: '' },
      open: { type: 'boolean', default: true }, disabled: { type: 'boolean', default: false },
      defaultChoice: { type: 'number', default: 1 }, cancelChoice: { type: 'number', default: 0 },
      choices: { type: 'number', default: 2 },
    });
    const label = def.state.string('label', ''); const description = def.state.string('description', '');
    const role = def.state.string('role', 'dialog'); const open = def.state.bool('open', false);
    const disabled = def.state.bool('disabled', false); const modal = def.state.bool('modal', true);
    const focus = asFocusable(); focus.configure({ disabled: false });
    const scope = asFocusScope();
    scope.configure({ trap: true, loop: true, navigation: 'tab', entry: 'first', emptyPolicy: 'container', restore: 'previous' });
    const overlay = asOverlay();
    overlay.configure({ defaultOpen: true, modal: true, portal: false, anchored: false,
      closeOnEscape: false, closeOnOutsidePress: false, closeOnFocusOutside: false,
      entry: 'manual', restore: 'none', layerRole: 'dialog-content' });
    overlay.keepMounted();
    const a11y = asAccessible(); a11y.role(role); a11y.name(label); a11y.description(description);
    a11y.state('modal', modal); a11y.state('disabled', disabled);
    def.expose.state('open', open); def.expose.state('disabled', disabled);
    def.expose.state('scopeActive', scope.active); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('response', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false; let disposed = false; let epoch = 0;
    const ready = (): boolean => run !== null && attached && !disposed && open.get() && !disabled.get();
    const choose = (index: number, basis: string): boolean => {
      if (!run || !ready()) return false;
      const owner = run;
      const props = owner.props.get();
      if (basis !== (props.revision ?? '') || !Number.isFinite(index) || index % 1 !== 0 || index < 0 || index >= (props.choices ?? 2)) return false;
      owner.expose.emit('response', { index, revision: basis });
      return true;
    };
    const key = (name: string): boolean => {
      if (!run || !ready()) return false;
      const props = run.props.get();
      if (name === 'Escape') return choose(props.cancelChoice ?? 0, props.revision ?? '');
      if (name === 'Enter') return choose(props.defaultChoice ?? 1, props.revision ?? '');
      return false;
    };
    def.expose.method('choose', choose); def.expose.method('key', key);
    def.expose.method('focusSelf', () => { if (ready()) focus.focusSelf(); });
    def.event.on('key.down', (_current, event) => {
      if (key(event.key ?? '')) event.control.requestDefaultActionPrevention();
    });
    const sync = (owner: RunHandle<Props>) => {
      if (disposed) return;
      run = owner; const ticket = ++epoch; const props = owner.props.get();
      label.set(props.label ?? '', 'reason: overlay name');
      if (ticket !== epoch || disposed) return;
      description.set(props.description ?? '', 'reason: overlay description');
      if (ticket !== epoch || disposed) return;
      role.set(props.role ?? 'dialog', 'reason: overlay role');
      if (ticket !== epoch || disposed) return;
      disabled.set(props.disabled ?? false, 'reason: overlay policy');
      if (ticket !== epoch || disposed) return;
      open.set(props.open ?? true, 'reason: accepted overlay visibility');
      if (ticket !== epoch || disposed) return;
      focus.setDisabled(!open.get() || disabled.get());
      if (ticket !== epoch || disposed) return;
      if (open.get()) overlay.openOverlay('controlled.sync'); else overlay.close('controlled.sync');
      if (ticket !== epoch || disposed || !attached) return;
      if (open.get() && !scope.isActive()) scope.activate();
      else if (!open.get() && scope.isActive()) scope.deactivate();
      if (ticket !== epoch || disposed) return;
      owner.update();
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((owner) => { if (!disposed) { attached = true; sync(owner); } });
    def.props.watch(['label', 'description', 'role', 'open', 'disabled', 'defaultChoice', 'cancelChoice', 'choices', 'revision'], sync);
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; if (scope.isActive()) scope.deactivate(); run = null; });
    def.lifecycle.onBeforeDispose(() => { ++epoch; disposed = true; attached = false; run = null; });
    def.feedback.style.use(tw('w-full h-full'));
    return renderer => renderer.slot();
  },
});
