import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asTrigger } from '@proto.ui/hooks';

type Props = {
  label?: string; description?: string; role?: string; disabled?: boolean;
  selected?: boolean; selectable?: boolean; tabStop?: boolean; contextOnly?: boolean; revision?: string; position?: number; count?: number; expanded?: boolean; expandable?: boolean; submitOnEnter?: boolean; contextMenu?: boolean;
};

/** Portable action semantics shared by the Adwaita host's buttons, toggles and tabs. */
export default definePrototype<Props>({
  name: 'verdant-action-control',
  setup(def) {
    def.props.define({
      label: { type: 'string', default: '' }, description: { type: 'string', default: '' },
      role: { type: 'string', default: 'button' }, disabled: { type: 'boolean', default: false },
      selected: { type: 'boolean', default: false }, selectable: { type: 'boolean', default: false },
      tabStop: { type: 'boolean', default: true }, contextOnly: { type: 'boolean', default: false },
      revision: { type: 'string', default: '' }, position: { type: 'number', default: 0 }, count: { type: 'number', default: 0 },
      expanded: { type: 'boolean', default: false }, expandable: { type: 'boolean', default: false }, submitOnEnter: { type: 'boolean', default: false }, contextMenu: { type: 'boolean', default: false },
    });
    const label = def.state.string('label', '');
    const description = def.state.string('description', '');
    const role = def.state.string('role', 'button');
    const disabled = def.state.bool('disabled', false);
    const selected = def.state.bool('selected', false);
    const toggle = def.state.bool('toggle', false);
    const checked = def.state.bool('checkedRole', false);
    const selectable = def.state.bool('selectableRole', false);
    const pressed = def.state.bool('pressed', false);
    const expanded = def.state.bool('expanded', false); const expandable = def.state.bool('expandable', false);
    const hovered = def.state.bool('hovered', false);
    const focus = asFocusable();
    focus.configure({ disabled: false });
    asTrigger();
    const a11y = asAccessible();
    a11y.role(role); a11y.name(label); a11y.description(description);
    a11y.state('disabled', disabled);
    a11y.state('expanded', expanded, { when: expandable });
    a11y.state('pressed', selected, { when: toggle });
    a11y.state('checked', selected, { when: checked });
    a11y.state('selected', selected, { when: selectable });
    a11y.action('activate', { event: 'host:verdant-action-activate' });
    def.expose.state('disabled', disabled); def.expose.state('selected', selected);
    def.expose.state('pressed', pressed); def.expose.state('hovered', hovered);
    def.expose.state('focused', focus.focused); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('activate', { payload: 'json' });
    def.expose.event('navigate', { payload: 'json' });
    def.expose.event('submit', { payload: 'json' });
    def.expose.event('context', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let generation = 0;
    let textGeneration = 0;
    // A release belongs to the exact authority that admitted its press. The
    // host may replace a same-labelled slot while the pointer/key is held.
    let pressGeneration = -1;
    // Keep one physical Enter consumed even if its submit callback replaces Props.
    let enterSubmitted = false;
    const retirePress = () => {
      pressGeneration = -1;
      if (disposed) return;
      pressed.set(false, 'reason: action retired');
    };
    const clear = () => {
      if (disposed) return;
      const ticket = generation;
      retirePress();
      if (disposed || generation !== ticket) return;
      hovered.set(false, 'reason: action retired');
    };
    const activate = (name: 'activate' | 'context', origin: string, key: string, shiftKey: boolean, ctrlKey: boolean, metaKey: boolean, altKey: boolean) => {
      if (!run || !attached || disposed || disabled.get()) return false;
      const ticket = generation;
      const revision = run.props.get().revision ?? '';
      if (name === 'context' && !run.props.get().contextMenu) return false;
      const next = name === 'context' ? run.props.get().selected ?? false : !(run.props.get().selected ?? false);
      retirePress();
      if (!run || !attached || disposed || disabled.get() || ticket !== generation) return false;
      // Selection remains authoritative until the owner accepts the proposal.
      run.expose.emit(name, { selected: next, revision, origin, key, shiftKey, ctrlKey, metaKey, altKey });
      return true;
    };
    const context = (x: number, y: number, keyboard: boolean, basis: string): boolean => {
      const owner = run;
      if (!owner || !attached || disposed || disabled.get() || !owner.props.get().contextMenu
        || basis !== (owner.props.get().revision ?? '') || !Number.isFinite(x) || !Number.isFinite(y)) return false;
      const ticket = generation; focus.focusSelf();
      if (ticket !== generation || !attached || disposed || disabled.get()) return false;
      owner.expose.emit('context', { x, y, keyboard, selected: owner.props.get().selected ?? false, revision: basis, origin: keyboard ? 'keyboard' : 'press', key: keyboard ? 'ContextMenu' : '', shiftKey: false, ctrlKey: false, metaKey: false, altKey: false });
      return true;
    };
    // Accessible readouts may change every frame (for example playback time).
    // Updating text does not replace the command that admitted a held press.
    const syncText = (current: RunHandle<Props>) => {
      if (disposed) return;
      const ticket = ++textGeneration;
      const policy = generation;
      const props = current.props.get();
      label.set(props.label ?? '', 'reason: action name');
      if (ticket !== textGeneration || policy !== generation || disposed) return;
      description.set(props.description ?? '', 'reason: action description');
      if (ticket !== textGeneration || policy !== generation || disposed) return;
      if (attached) current.update();
    };
    const sync = (current: RunHandle<Props>) => {
      if (disposed) return;
      const ticket = ++generation;
      run = current;
      const props = current.props.get();
      pressGeneration = -1;
      pressed.set(false, 'reason: action authority changed');
      if (ticket !== generation || disposed) return;
      disabled.set(props.disabled ?? false, 'reason: action policy');
      if (ticket !== generation) return;
      focus.setDisabled(disabled.get());
      if (ticket !== generation) return;
      const previous = selected.get();
      selected.set(props.selected ?? false, 'reason: accepted selection');
      if (ticket !== generation || disposed) return;
      focus.setNavParticipation(props.tabStop === false || ((props.count ?? 0) > 0 && !selected.get()) ? 'none' : 'auto');
      if (ticket !== generation || disposed) return;
      if (attached && !previous && selected.get() && (props.count ?? 0) > 0) focus.focusSelf();
      if (ticket !== generation || disposed) return;
      role.set(props.role ?? 'button', 'reason: action role');
      if (ticket !== generation || disposed) return;
      expanded.set(props.expanded ?? false, 'reason: accepted expanded state');
      if (ticket !== generation || disposed) return;
      expandable.set(props.expandable ?? false, 'reason: expansion semantics');
      if (ticket !== generation || disposed) return;
      syncText(current);
      if (ticket !== generation || disposed) return;
      checked.set(props.role === 'switch' || props.role === 'checkbox' || props.role === 'menuitemcheckbox' || props.role === 'menuitemradio' || props.role === 'radio', 'reason: checked semantics');
      if (ticket !== generation || disposed) return;
      selectable.set(props.role === 'tab' || props.role === 'option' || props.role === 'row', 'reason: selected semantics');
      if (ticket !== generation || disposed) return;
      toggle.set((props.selectable ?? false) && !checked.get() && !selectable.get(), 'reason: toggle semantics');
      if (ticket !== generation || disposed) return;
      if (disabled.get()) clear();
      if (attached) current.update();
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((current) => { if (!disposed) { attached = true; sync(current); } });
    def.props.watch(['role', 'disabled', 'selected', 'selectable', 'revision', 'position', 'count', 'expanded', 'expandable', 'submitOnEnter', 'contextMenu', 'contextOnly', 'tabStop'], sync);
    def.props.watch(['label', 'description'], syncText);
    def.event.on('press.start', (_current, event) => {
      if (!run || !attached || disposed || disabled.get()) return;
      if (event.key === 'Enter' && (enterSubmitted || run.props.get().submitOnEnter)) { retirePress(); return; }
      pressGeneration = generation;
      pressed.set(true, 'reason: action press');
    });
    def.event.on('pointer.enter', () => { if (attached && !disabled.get()) hovered.set(true, 'reason: action enter'); });
    def.event.on('pointer.leave', clear);
    def.event.on('press.cancel', retirePress);
    def.event.on('pointer.cancel', clear);
    // Losing focus cancels a press, but does not move the pointer. Only the
    // host's actual pointer-leave/modality fact withdraws hover.
    def.event.on('host:blur', retirePress);
    def.event.on('press.commit', (_current, event) => {
      if (!run || !attached || disposed || disabled.get()) return;
      if (event.key === 'Enter' && (enterSubmitted || run.props.get().submitOnEnter)) {
        retirePress(); event.control.requestDefaultActionPrevention(); return;
      }
      if (pressGeneration !== generation) return;
      const ticket = generation;
      const name: 'activate' | 'context' = event.key && run.props.get().contextOnly ? 'context' : 'activate';
      event.control.requestDefaultActionPrevention();
      if (ticket === generation && pressGeneration === ticket && attached && !disposed && !disabled.get()) {
        activate(name, event.key ? 'keyboard' : 'press', event.key ?? '', event.shiftKey === true,
          event.ctrlKey === true, event.metaKey === true, event.altKey === true);
      }
    });
    def.event.on('key.up', (_current, event) => {
      if (event.key === 'Enter') enterSubmitted = false;
      if (event.key === 'Enter' || event.key === ' ') {
        pressGeneration = -1;
        if (attached && !disposed) pressed.set(false, 'reason: action key released');
      }
    });
    def.event.on('host:verdant-action-activate', () => activate(run?.props.get().contextOnly ? 'context' : 'activate', 'assistive', '', false, false, false, false));
    def.event.on('context.menu', (_current, event) => {
      if (activate('context', 'press', '', event.shiftKey === true, event.ctrlKey === true,
        event.metaKey === true, event.altKey === true)) event.control.requestDefaultActionPrevention();
    });
    def.event.on('key.down', (_current, event) => {
      if (event.key === 'Enter' && !event.repeat) enterSubmitted = false;
      if (run && attached && !disabled.get() && run.props.get().contextMenu
        && (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey && !event.ctrlKey && !event.metaKey && !event.altKey))) {
        const ticket = generation;
        event.control.requestDefaultActionPrevention();
        if (ticket === generation && attached && !disposed && !disabled.get()) {
          activate('context', 'keyboard', event.key ?? '', event.shiftKey === true, event.ctrlKey === true,
            event.metaKey === true, event.altKey === true);
        }
        return;
      }
      if (run && attached && !disabled.get() && event.key === 'Enter' && run.props.get().submitOnEnter) {
        const owner = run; const ticket = generation; const revision = owner.props.get().revision ?? '';
        enterSubmitted = true;
        event.control.requestDefaultActionPrevention();
        if (ticket !== generation || !attached || disposed || disabled.get()) return;
        retirePress();
        if (ticket === generation && attached && !disposed && !disabled.get()) owner.expose.emit('submit', { revision });
        return;
      }
      if (run && attached && !disabled.get() && !event.ctrlKey && !event.metaKey && !event.altKey && (run.props.get().count ?? 0) > 0) {
        const count = run.props.get().count ?? 0;
        const position = run.props.get().position ?? 0;
        let next = position;
        if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (position + count - 1) % count;
        else if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (position + 1) % count;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = count - 1;
        if (next !== position) {
          const owner = run;
          const ticket = generation;
          const revision = owner.props.get().revision ?? '';
          event.control.requestDefaultActionPrevention();
          if (generation !== ticket || !attached || disposed || disabled.get()) return;
          owner.expose.emit('navigate', { index: next, revision });
          return;
        }
      }
      if (role.get() === 'combobox' && event.key === 'ArrowDown' && !disabled.get() && !event.ctrlKey && !event.metaKey && !event.altKey) {
        const ticket = generation;
        event.control.requestDefaultActionPrevention();
        if (ticket === generation && attached && !disposed && !disabled.get()) activate('activate', 'keyboard', event.key ?? '', event.shiftKey === true, event.ctrlKey === true, event.metaKey === true, event.altKey === true);
      }
    });
    def.expose.method('requestActivate', () => activate('activate', 'programmatic', '', false, false, false, false));
    def.expose.method('requestContext', context);
    def.expose.method('focusSelf', () => { if (attached && !disabled.get()) focus.focusSelf(); });
    def.lifecycle.onUnmounted(() => { ++generation; enterSubmitted = false; attached = false; clear(); run = null; });
    def.lifecycle.onBeforeDispose(() => { ++generation; enterSubmitted = false; disposed = true; attached = false; run = null; });
    // The host paints the Adwaita content; this semantic root covers that exact
    // rectangle, owns its focus target, and supplies every activation route.
    def.feedback.style.use(tw('absolute inset-0 w-full h-full select-none'));
    return (renderer) => renderer.slot();
  },
});
