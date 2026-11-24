import { definePrototype, tw, type DefHandle, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asFocusScope, asOverlay } from '@proto.ui/hooks';

type Props = {
  label?: string; description?: string; role?: string; revision?: string;
  open?: boolean; disabled?: boolean; defaultChoice?: number; cancelChoice?: number; choices?: number;
  responseKinds?: readonly number[] | null;
};

// The application supplies response meanings and native content. Modal scope,
// entry/restoration, accepted visibility and keyboard response policy belong
// to this retained compiled root, including prompts outside the editor entity.
export default definePrototype<Props>({
  name: 'verdant-overlay-control',
  setup(def: DefHandle<Props>) {
    def.props.define({
      label: { type: 'string', default: '' }, description: { type: 'string', default: '' },
      role: { type: 'string', default: 'dialog' }, revision: { type: 'string', default: '' },
      open: { type: 'boolean', default: true }, disabled: { type: 'boolean', default: false },
      defaultChoice: { type: 'number', default: 1 }, cancelChoice: { type: 'number', default: 0 },
      choices: { type: 'number', default: 2 },
      responseKinds: { type: 'object', default: null },
    });
    const label = def.state.string('label', ''); const description = def.state.string('description', '');
    const role = def.state.string('role', 'dialog'); const open = def.state.bool('open', false);
    const disabled = def.state.bool('disabled', false); const modal = def.state.bool('modal', true);
    const defaultChoice = def.state.numberDiscrete('defaultChoice', -1);
    const cancelChoice = def.state.numberDiscrete('cancelChoice', -1);
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
    def.expose.state('defaultChoice', defaultChoice); def.expose.state('cancelChoice', cancelChoice);
    def.expose.state('scopeActive', scope.active); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('response', { payload: 'json' });
    def.expose.event('selection', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false; let disposed = false; let epoch = 0;
    const ready = (): boolean => run !== null && attached && !disposed && open.get() && !disabled.get();
    // Optional response facts: 0 unavailable, 1 ordinary, 2 cancel, 3 suggested.
    // Alert hosts supply facts; the source resolves the default/cancel policy
    // and rejects unavailable actions for every activation route.
    const validKinds = (values: readonly number[], start: number, end: number): boolean => {
      if (end <= start) return true;
      if (end - start === 1) return values[start] === 0 || values[start] === 1 || values[start] === 2 || values[start] === 3;
      const middle = (start + end - (start + end) % 2) / 2;
      return validKinds(values, start, middle) && validKinds(values, middle, end);
    };
    const firstKind = (values: readonly number[], start: number, end: number, kind: number): number => {
      if (end <= start) return -1;
      if (end - start === 1) return values[start] === kind ? start : -1;
      const middle = (start + end - (start + end) % 2) / 2;
      const first = firstKind(values, start, middle, kind);
      return first >= 0 ? first : firstKind(values, middle, end, kind);
    };
    const validChoices = (props: Readonly<Props>): boolean => {
      const count = props.choices ?? 2;
      if (!Number.isFinite(count) || count % 1 !== 0 || count < 0 || count > 1000000) return false;
      const kinds = props.responseKinds;
      return kinds === null || kinds === undefined || (Array.isArray(kinds) && kinds.length === count && validKinds(kinds, 0, kinds.length));
    };
    const responseIndex = (props: Readonly<Props>, cancel: boolean): number => {
      if (!validChoices(props)) return -1;
      const kinds = props.responseKinds;
      if (kinds !== null && kinds !== undefined) {
        const index = firstKind(kinds, 0, kinds.length, cancel ? 2 : 3);
        return index >= 0 || cancel ? index : firstKind(kinds, 0, kinds.length, 1);
      }
      const index = cancel ? props.cancelChoice ?? 0 : props.defaultChoice ?? 1;
      return Number.isFinite(index) && index % 1 === 0 && index >= 0 && index < (props.choices ?? 2) ? index : -1;
    };
    const admits = (index: number, basis: string): boolean => {
      if (!run || !ready()) return false;
      const props = run.props.get();
      if (basis !== (props.revision ?? '') || !validChoices(props) || !Number.isFinite(index) || index % 1 !== 0 || index < 0 || index >= (props.choices ?? 2)) return false;
      const kinds = props.responseKinds;
      if (kinds !== null && kinds !== undefined && kinds[index] === 0) return false;
      return true;
    };
    const choose = (index: number, basis: string): boolean => {
      if (!admits(index, basis) || !run) return false;
      run.expose.emit('response', { index, revision: basis });
      return true;
    };
    const key = (name: string): boolean => {
      if (!run || !ready()) return false;
      const props = run.props.get();
      if (name === 'Escape') return choose(responseIndex(props, true), props.revision ?? '');
      if (name === 'Enter') return choose(responseIndex(props, false), props.revision ?? '');
      return false;
    };
    def.expose.method('choose', choose); def.expose.method('key', key);
    // A controlled child selection retains its proposed value through the
    // modal's response authority; native hosts never reconstruct a toggle.
    def.expose.method('chooseSelection', (index: number, selected: boolean, basis: string): boolean => {
      if (typeof selected !== 'boolean' || !admits(index, basis) || !run) return false;
      run.expose.emit('selection', { index, selected, revision: basis });
      return true;
    });
    // Child fields/actions forward semantic requests with the authority they
    // observed, rather than looking up a newly opened dialog's default action.
    def.expose.method('requestResponse', (name: string, basis: string): boolean => {
      if (!run || basis !== (run.props.get().revision ?? '')) return false;
      return key(name);
    });
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
      defaultChoice.set(responseIndex(props, false), 'reason: overlay default response');
      if (ticket !== epoch || disposed) return;
      cancelChoice.set(responseIndex(props, true), 'reason: overlay cancel response');
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
    def.props.watch(['label', 'description', 'role', 'open', 'disabled', 'defaultChoice', 'cancelChoice', 'choices', 'responseKinds', 'revision'], sync);
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; if (scope.isActive()) scope.deactivate(); run = null; });
    def.lifecycle.onBeforeDispose(() => { ++epoch; disposed = true; attached = false; run = null; });
    def.feedback.style.use(tw('w-full h-full'));
    return renderer => renderer.slot();
  },
});
