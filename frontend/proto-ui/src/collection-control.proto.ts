import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable } from '@proto.ui/hooks';

type Props = {
  mode?: string; label?: string; cursor?: number;
  expanded?: number; revision?: string; disabled?: boolean; autoFocus?: boolean; columns?: number;
};

// Item kinds: 0 unavailable/header, 1 action, 2 submenu. Host supplies content
// facts; this instance owns traversal, expansion, activation and dismissal.
export default definePrototype<Props>({
  name: 'verdant-collection-control',
  setup(def) {
    def.props.define({
      mode: { type: 'string', default: 'listbox' }, label: { type: 'string', default: '' },
      cursor: { type: 'number', default: 0 },
      expanded: { type: 'number', default: -1 }, revision: { type: 'string', default: '' },
      disabled: { type: 'boolean', default: false }, autoFocus: { type: 'boolean', default: false }, columns: { type: 'number', default: 0 },
    });
    const cursor = def.state.numberDiscrete('cursor', 0);
    const expanded = def.state.numberDiscrete('expanded', -1);
    const disabled = def.state.bool('disabled', false);
    const role = def.state.string('role', 'listbox');
    const label = def.state.string('label', '');
    const table = def.state.bool('table', false);
    const rows = def.state.numberDiscrete('rows', 0);
    const columns = def.state.numberDiscrete('columns', 0);
    const focus = asFocusable(); focus.configure({ disabled: false });
    const a11y = asAccessible(); a11y.role(role); a11y.name(label); a11y.state('disabled', disabled);
    a11y.state('rowCount', rows, { when: table }); a11y.state('columnCount', columns, { when: table });
    def.expose.state('mode', role);
    def.expose.state('cursor', cursor); def.expose.state('expanded', expanded);
    def.expose.state('focused', focus.focused); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('select', { payload: 'json' });
    def.expose.event('activate', { payload: 'json' });
    def.expose.event('expand', { payload: 'json' });
    def.expose.event('close', { payload: 'json' });
    def.expose.event('command', { payload: 'json' });
    let current: RunHandle<Props> | null = null;
    let attached = false;
    let disposed = false;
    let epoch = 0;
    const ready = (): boolean => current !== null && attached && !disposed && !disabled.get();
    let items: number[] = [];
    const kind = (index: number): number => Number.isFinite(index) && index % 1 === 0 && index >= 0 && index < items.length ? items[index] ?? 0 : 0;
    const count = (): number => items.length;
    const min = (a: number, b: number): number => a < b ? a : b;
    const max = (a: number, b: number): number => a > b ? a : b;
    const find = (start: number, end: number, forward: boolean): number => {
      if (end <= start) return -1;
      if (end - start === 1) return kind(start) > 0 ? start : -1;
      const sum = start + end;
      const middle = (sum - sum % 2) / 2;
      const candidate = forward ? find(start, middle, true) : find(middle, end, false);
      if (candidate >= 0) return candidate;
      return forward ? find(middle, end, true) : find(start, middle, false);
    };
    const seek = (requested: number, direction: number, _remaining: number, wrap: boolean): number => {
      const index = min(count(), max(-1, requested));
      const candidate = direction > 0 ? find(max(0, index), count(), true) : find(0, min(count(), index + 1), false);
      if (candidate >= 0 || !wrap) return candidate;
      return direction > 0 ? find(0, max(0, index), true) : find(min(count(), index + 1), count(), false);
    };
    const validItems = (values: number[], start: number, end: number): boolean => {
      if (end <= start) return true;
      if (end - start === 1) return values[start] === 0 || values[start] === 1 || values[start] === 2;
      const sum = start + end;
      const middle = (sum - sum % 2) / 2;
      return validItems(values, start, middle) && validItems(values, middle, end);
    };
    const emit = (name: 'select' | 'activate' | 'expand' | 'close' | 'command', index: number, extend: boolean, key: string) => {
      if (current) current.expose.emit(name, { index, extend, key, revision: current.props.get().revision ?? '' });
    };
    const changeExpansion = (next: number, name: string): boolean => {
      if (!ready()) return false;
      const ticket = ++epoch;
      if (next < 0 && expanded.get() >= 0) {
        cursor.set(expanded.get(), 'reason: closing submenu restores parent');
        if (ticket !== epoch || !ready()) return false;
      }
      expanded.set(next, 'reason: submenu expansion');
      if (ticket !== epoch || !ready()) return false;
      emit('expand', next, false, name);
      if (current && ticket === epoch) current.update();
      return true;
    };
    const select = (index: number, extend: boolean, explicit: boolean): boolean => {
      if (!ready() || kind(index) === 0) return false;
      // Hovering the cursor again is redundant; an explicit click or key may
      // select the initially unselected row or collapse an existing range.
      if (!explicit && index === cursor.get() && !extend) return true;
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection selection');
      if (ticket !== epoch || !ready()) return false;
      emit('select', index, extend, '');
      if (current && ticket === epoch) current.update();
      return true;
    };
    const choose = (index: number, extend: boolean): boolean => {
      if (!ready() || kind(index) === 0) return false;
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection activation cursor');
      if (!ready() || epoch !== ticket) return false;
      if (kind(index) === 2) {
        const next = expanded.get() === index ? -1 : index;
        return changeExpansion(next, '');
      } else emit('activate', index, extend, '');
      return true;
    };
    const key = (name: string, shift: boolean, command: boolean): boolean => {
      if (!ready() || !current) return false;
      const mode = current.props.get().mode ?? 'listbox';
      if (mode === 'searchlist' && command && (name === 'f' || name === 'F')) {
        emit('command', cursor.get(), false, 'Find'); return true;
      }
      if (mode === 'points' && (name === 'Escape' || name === 'Enter')) { emit('close', cursor.get(), shift, name); return true; }
      if (mode === 'points' && name === 'Tab') {
        const next = cursor.get() + (shift ? -1 : 1);
        if (next < 0 || next >= count()) emit('close', cursor.get(), shift, name);
        else select(next, false, true);
        return true;
      }
      if (mode === 'points' && (name === 'ArrowUp' || name === 'ArrowDown' || name === '+' || name === '=' || name === '-')) { emit('command', cursor.get(), shift, name); return true; }
      if (name === 'Escape' || (name === 'Tab' && mode === 'menu')) { emit('close', cursor.get(), shift, name); return true; }
      if (name === 'Enter' || name === ' ' || (name === 'F2' && mode === 'table')) return choose(cursor.get(), shift);
      if (mode === 'table' && (name === 'Delete' || name === 'Backspace' || name === 'Tab')) {
        emit('command', cursor.get(), shift, name); return true;
      }
      if (mode === 'table' && command && (name === 'ArrowLeft' || name === 'ArrowRight' || name === 'ArrowUp' || name === 'ArrowDown')) {
        emit('command', cursor.get(), shift, name); return true;
      }
      if (mode === 'menu' && name === 'ArrowLeft') {
        if (expanded.get() < 0) return false;
        return changeExpansion(-1, name);
      }
      if (mode === 'menu' && name === 'ArrowRight') {
        if (kind(cursor.get()) !== 2) return false;
        return changeExpansion(cursor.get(), name);
      }
      let next = -1;
      if (name === 'Home') next = seek(0, 1, count(), false);
      else if (name === 'End') next = seek(count() - 1, -1, count(), false);
      else if (name === 'ArrowUp' || name === 'ArrowLeft') next = seek(cursor.get() - 1, -1, count(), mode === 'menu' || mode === 'tablist');
      else if (name === 'ArrowDown' || name === 'ArrowRight') next = seek(cursor.get() + 1, 1, count(), mode === 'menu' || mode === 'tablist');
      else if (name === 'PageUp') next = seek(max(0, cursor.get() - (mode === 'points' ? 4 : 20)), -1, count(), false);
      else if (name === 'PageDown') next = seek(min(count() - 1, cursor.get() + (mode === 'points' ? 4 : 20)), 1, count(), false);
      else return false;
      if (next >= 0) select(next, shift, true);
      else if (!shift && kind(cursor.get()) > 0) select(cursor.get(), false, true);
      return true;
    };
    def.expose.method('setItems', (next: number[]) => {
      if (disposed || next.length > 1000000 || !validItems(next, 0, next.length)) return false;
      ++epoch; items = next;
      if (current) sync(current);
      return true;
    });
    def.expose.method('choose', (index: number, extend: boolean, basis: string) => {
      if (basis !== (current?.props.get().revision ?? '')) return false;
      return choose(index, extend);
    });
    def.expose.method('selectIndex', (index: number, extend: boolean, basis: string) => {
      if (basis !== (current?.props.get().revision ?? '')) return false;
      return select(index, extend, false);
    });
    // Native virtualized row callbacks carry their accepted collection revision.
    // Selection must not let a suspended callback command a replacement row.
    def.expose.method('keyAt', (index: number, name: string, shift: boolean, command: boolean, basis: string) => {
      if (!current || basis !== (current.props.get().revision ?? '') || !ready() || kind(index) === 0) return false;
      const mode = current.props.get().mode ?? 'listbox';
      const handled = name === 'Escape' || name === 'Enter' || name === ' ' || name === 'Home' || name === 'End'
        || name === 'ArrowLeft' || name === 'ArrowRight' || name === 'ArrowUp' || name === 'ArrowDown'
        || name === 'PageUp' || name === 'PageDown' || (name === 'Tab' && (mode === 'points' || mode === 'table' || mode === 'menu'))
        || (mode === 'table' && (name === 'F2' || name === 'Delete' || name === 'Backspace'))
        || (mode === 'points' && (name === '+' || name === '=' || name === '-'))
        || (mode === 'searchlist' && command && (name === 'f' || name === 'F'));
      if (!handled) return false;
      const ticket = ++epoch;
      cursor.set(index, 'reason: addressed collection key');
      if (ticket !== epoch || !ready()) return false;
      emit('select', index, false, '');
      if (ticket !== epoch || !ready() || basis !== (current?.props.get().revision ?? '')) return false;
      return key(name, shift, command);
    });
    def.expose.method('selectAndFocus', (index: number, extend: boolean, basis: string) => {
      if (!current || basis !== (current.props.get().revision ?? '')) return false;
      if (!select(index, extend, true) || basis !== (current?.props.get().revision ?? '')) return false;
      focus.focusSelf();
      return true;
    });
    def.expose.method('key', key);
    // Search and form fields are native descendants outside the collection's
    // own focus target. Their host only reports origin and normalized keys;
    // this source decides which shortcuts may leave the text editor.
    def.expose.method('keyFrom', (origin: string, name: string, shift: boolean, command: boolean, basis: string) => {
      if (!current || basis !== (current.props.get().revision ?? '')) return false;
      if (origin === 'form') return name === 'Escape' ? key(name, shift, command) : false;
      if (origin === 'search' && name !== 'ArrowUp' && name !== 'ArrowDown' && name !== 'Enter' && name !== 'Escape'
        && !(command && (name === 'f' || name === 'F'))) return false;
      return key(name, shift, command);
    });
    def.expose.method('dismiss', () => { if (ready()) emit('close', cursor.get(), false, 'outside'); });
    def.expose.method('focusSelf', () => { if (ready()) focus.focusSelf(); });
    def.event.on('key.down', (_run, event) => {
      if (key(event.key ?? '', event.shiftKey === true, event.ctrlKey === true || event.metaKey === true)) event.control.requestDefaultActionPrevention();
    });
    const sync = (run: RunHandle<Props>) => {
      if (disposed) return;
      const ticket = ++epoch; current = run;
      const props = run.props.get();
      role.set(props.mode === 'points' || props.mode === 'searchlist' ? 'listbox' : props.mode ?? 'listbox', 'reason: collection role');
      if (epoch !== ticket || disposed) return;
      table.set(props.mode === 'table', 'reason: collection table semantics');
      if (epoch !== ticket || disposed) return;
      rows.set(count(), 'reason: collection rows');
      if (epoch !== ticket || disposed) return;
      columns.set(props.columns ?? 0, 'reason: collection columns');
      if (epoch !== ticket || disposed) return;
      label.set(props.label ?? '', 'reason: collection name');
      if (epoch !== ticket || disposed) return;
      disabled.set(props.disabled ?? false, 'reason: collection policy');
      if (epoch !== ticket || disposed) return;
      focus.setDisabled(disabled.get());
      if (epoch !== ticket || disposed) return;
      const requested = props.cursor ?? 0;
      const accepted = Number.isFinite(requested) && requested % 1 === 0 ? max(0, min(count() - 1, requested)) : 0;
      cursor.set(kind(accepted) > 0 ? accepted : max(0, seek(0, 1, count(), false)), 'reason: accepted collection cursor');
      if (epoch !== ticket || disposed) return;
      expanded.set(props.expanded ?? -1, 'reason: accepted expanded item');
      if (epoch !== ticket || disposed) return;
      if (attached) run.update();
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((run) => { if (!disposed) { attached = true; sync(run); if (run.props.get().autoFocus) focus.focusSelf(); } });
    def.props.watch(['mode', 'label', 'cursor', 'expanded', 'revision', 'disabled', 'autoFocus', 'columns'], sync);
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; current = null; });
    def.lifecycle.onBeforeDispose(() => { ++epoch; disposed = true; attached = false; current = null; });
    def.feedback.style.use(tw('w-full h-full'));
    return (renderer) => renderer.slot();
  },
});
