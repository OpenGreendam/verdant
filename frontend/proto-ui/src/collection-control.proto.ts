import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asAccessible, asFocusable, asScrollSurface } from '@proto.ui/hooks';

type Props = {
  mode?: string; label?: string; description?: string; cursor?: number; orientation?: string;
  expanded?: number; revision?: string; disabled?: boolean; autoFocus?: boolean; columns?: number;
};

// Item kinds: 0 unavailable/header, 1 action, 2 submenu, 3/4 unchecked/checked
// Boolean, 5/6 unselected/selected choice. Host supplies accepted content facts;
// this instance owns traversal, expansion, selection proposals and dismissal.
export default definePrototype<Props>({
  name: 'verdant-collection-control',
  setup(def) {
    def.props.define({
      mode: { type: 'string', default: 'listbox' }, label: { type: 'string', default: '' },
      description: { type: 'string', default: '' },
      cursor: { type: 'number', default: 0 },
      orientation: { type: 'string', default: 'vertical' },
      expanded: { type: 'number', default: -1 }, revision: { type: 'string', default: '' },
      disabled: { type: 'boolean', default: false }, autoFocus: { type: 'boolean', default: false }, columns: { type: 'number', default: 0 },
    });
    const cursor = def.state.numberDiscrete('cursor', 0);
    const expanded = def.state.numberDiscrete('expanded', -1);
    const disabled = def.state.bool('disabled', false);
    const role = def.state.string('role', 'listbox');
    const label = def.state.string('label', '');
    const description = def.state.string('description', '');
    const table = def.state.bool('table', false);
    const rows = def.state.numberDiscrete('rows', 0);
    const columns = def.state.numberDiscrete('columns', 0);
    const focus = asFocusable(); focus.configure({ disabled: false });
    const scroll = asScrollSurface(); scroll.configure({ axes: 'both', projection: 'system' });
    const a11y = asAccessible(); a11y.role(role); a11y.name(label); a11y.description(description); a11y.state('disabled', disabled);
    a11y.state('rowCount', rows, { when: table }); a11y.state('columnCount', columns, { when: table });
    def.expose.state('scrollPosition', scroll.vertical.position); def.expose.state('scrollAtEnd', scroll.vertical.atEnd);
    def.expose.state('horizontalPosition', scroll.horizontal.position);
    def.expose.state('mode', role);
    def.expose.state('cursor', cursor); def.expose.state('expanded', expanded);
    def.expose.state('focused', focus.focused); def.expose.state('focusVisible', focus.focusVisible);
    def.expose.event('select', { payload: 'json' });
    def.expose.event('activate', { payload: 'json' });
    def.expose.event('selection', { payload: 'json' });
    def.expose.event('expand', { payload: 'json' });
    def.expose.event('close', { payload: 'json' });
    def.expose.event('command', { payload: 'json' });
    def.expose.event('context', { payload: 'json' });
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
      if (end - start === 1) return values[start] === 0 || values[start] === 1 || values[start] === 2
        || values[start] === 3 || values[start] === 4 || values[start] === 5 || values[start] === 6;
      const sum = start + end;
      const middle = (sum - sum % 2) / 2;
      return validItems(values, start, middle) && validItems(values, middle, end);
    };
    const emit = (name: 'select' | 'activate' | 'expand' | 'close' | 'command', index: number, extend: boolean, key: string) => {
      if (current) current.expose.emit(name, { index, extend, key, revision: current.props.get().revision ?? '' });
    };
    const requestReveal = (index: number) => {
      const owner = current;
      if (!owner || !ready()) return;
      const mode = owner.props.get().mode;
      if (mode === 'filterlist' || mode === 'tracklist') emit('command', index, false, 'reveal');
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
      // Explicit selection may choose an initially unselected row or collapse
      // a range; only repeated hover selection is redundant.
      if (!explicit && index === cursor.get() && !extend) return true;
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection selection');
      if (ticket !== epoch || !ready()) return false;
      emit('select', index, extend, '');
      if (ticket !== epoch || !ready()) return false;
      requestReveal(index);
      if (ticket !== epoch || !ready()) return false;
      if (current && ticket === epoch) current.update();
      return true;
    };
    const chooseSelection = (index: number, selected: boolean, extend: boolean): boolean => {
      const owner = current;
      const item = kind(index);
      if (!owner || !ready() || typeof selected !== 'boolean' || item < 3 || item > 6
        || (item >= 5 && !selected)) return false;
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection selection proposal cursor');
      if (!ready() || epoch !== ticket) return false;
      owner.expose.emit('selection', { index, selected, extend, revision: owner.props.get().revision ?? '' });
      return true;
    };
    const choose = (index: number, extend: boolean): boolean => {
      if (!ready() || kind(index) === 0) return false;
      if (kind(index) >= 3) return chooseSelection(index, kind(index) === 3 || kind(index) >= 5, extend);
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection activation cursor');
      if (!ready() || epoch !== ticket) return false;
      if (kind(index) === 2) {
        const next = expanded.get() === index ? -1 : index;
        return changeExpansion(next, '');
      } else emit('activate', index, extend, '');
      return true;
    };
    const contextAt = (index: number, basis: string, x: number, y: number, keyboard: boolean): boolean => {
      const owner = current;
      if (!owner || !ready() || basis !== (owner.props.get().revision ?? '') || kind(index) === 0
        || !Number.isFinite(index) || index % 1 !== 0 || !Number.isFinite(x) || !Number.isFinite(y)) return false;
      const ticket = ++epoch;
      cursor.set(index, 'reason: collection context cursor');
      if (ticket !== epoch || !ready()) return false;
      focus.focusSelf();
      if (ticket !== epoch || !ready()) return false;
      owner.expose.emit('context', { index, x, y, keyboard, revision: basis });
      return true;
    };
    const key = (name: string, shift: boolean, command: boolean): boolean => {
      if (!ready() || !current) return false;
      const mode = current.props.get().mode ?? 'listbox';
      if (mode === 'filterlist' && command && (name === 'f' || name === 'F')) { emit('command', cursor.get(), false, 'focus-filter'); return true; }
      if (mode === 'tracklist' && (name === 'ContextMenu' || (name === 'F10' && shift))) return contextAt(cursor.get(), current.props.get().revision ?? '', 0, 0, true);
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
      if (next >= 0 && !select(next, shift, true)) return true;
      else if (next < 0 && !shift && kind(cursor.get()) > 0 && !select(cursor.get(), false, true)) return true;
      if (!ready()) return true;
      const horizontal = current?.props.get().orientation === 'horizontal';
      if (name === 'PageUp') { if (horizontal) scroll.request({ kind: 'page', axis: 'horizontal', direction: 'before' }); else scroll.request({ kind: 'page', axis: 'vertical', direction: 'before' }); }
      else if (name === 'PageDown') { if (horizontal) scroll.request({ kind: 'page', axis: 'horizontal', direction: 'after' }); else scroll.request({ kind: 'page', axis: 'vertical', direction: 'after' }); }
      else if (name === 'Home') { if (horizontal) scroll.request({ kind: 'to', axis: 'horizontal', position: 0 }); else scroll.request({ kind: 'to', axis: 'vertical', position: 0 }); }
      else if (name === 'End') { if (horizontal) scroll.request({ kind: 'to-end', axis: 'horizontal' }); else scroll.request({ kind: 'to-end', axis: 'vertical' }); }
      return true;
    };
    // Search fields report native focus/composition facts. This source chooses
    // which keys belong to the collection; ordinary caret and editing keys
    // remain with the independently compiled TextControl.
    def.expose.method('filterKey', (name: string, shift: boolean, command: boolean, composing: boolean, fromFilter: boolean, basis: string, alt: boolean) => {
      const owner = current;
      if (!owner || !ready() || composing || alt || owner.props.get().mode !== 'filterlist' || basis !== (owner.props.get().revision ?? '')) return false;
      if (command && (name === 'f' || name === 'F')) return key(name, shift, true);
      if (!fromFilter || command) return false;
      if (name !== 'ArrowUp' && name !== 'ArrowDown' && name !== 'Enter' && name !== 'Escape') return false;
      return key(name, shift, false);
    });
    // These are measured coordinates from this exact viewport, not an item
    // identity or a native command. The source owns the reveal decision and
    // ScrollSurface request, including axis, containment and nearest edge.
    def.expose.method('revealGeometry', (index: number, start: number, end: number, viewportStart: number, viewportEnd: number, basis: string) => {
      const owner = current;
      if (!owner || !ready() || basis !== (owner.props.get().revision ?? '') || index !== cursor.get() || kind(index) === 0
        || !Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(viewportStart) || !Number.isFinite(viewportEnd)
        || end <= start || viewportEnd <= viewportStart) return false;
      if ((start >= viewportStart && end <= viewportEnd) || (start <= viewportStart && end >= viewportEnd)) return true;
      const leading = start - viewportStart; const trailing = end - viewportEnd;
      const leadingDistance = leading < 0 ? -leading : leading; const trailingDistance = trailing < 0 ? -trailing : trailing;
      const delta = leadingDistance < trailingDistance ? leading : trailing;
      if (owner.props.get().orientation === 'horizontal') scroll.request({ kind: 'by', axis: 'horizontal', delta });
      else scroll.request({ kind: 'by', axis: 'vertical', delta });
      return true;
    });
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
    // A child ActionControl proposal retains its value. Keyboard activation
    // reaches the same admission path using the accepted item facts above.
    def.expose.method('chooseSelection', (index: number, selected: boolean, basis: string) => {
      if (basis !== (current?.props.get().revision ?? '')) return false;
      return chooseSelection(index, selected, false);
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
        || (mode === 'filterlist' && command && (name === 'f' || name === 'F'))
        || (mode === 'tracklist' && (name === 'ContextMenu' || (name === 'F10' && shift)));
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
    def.expose.method('contextAt', contextAt);
    def.expose.method('reconcile', () => { const owner = current; if (!owner || !ready()) return false; sync(owner); return true; });
    def.expose.method('dismiss', () => { if (ready()) emit('close', cursor.get(), false, 'outside'); });
    def.expose.method('focusSelf', () => { if (ready()) focus.focusSelf(); });
    def.event.on('key.down', (_run, event) => {
      if (key(event.key ?? '', event.shiftKey === true, event.ctrlKey === true || event.metaKey === true)) event.control.requestDefaultActionPrevention();
    });
    const sync = (run: RunHandle<Props>) => {
      if (disposed) return;
      const ticket = ++epoch; current = run;
      const props = run.props.get();
      role.set(props.mode === 'points' || props.mode === 'tracklist' || props.mode === 'filterlist' ? 'listbox' : props.mode ?? 'listbox', 'reason: collection role');
      if (epoch !== ticket || disposed) return;
      table.set(props.mode === 'table', 'reason: collection table semantics');
      if (epoch !== ticket || disposed) return;
      rows.set(count(), 'reason: collection rows');
      if (epoch !== ticket || disposed) return;
      columns.set(props.columns ?? 0, 'reason: collection columns');
      if (epoch !== ticket || disposed) return;
      label.set(props.label ?? '', 'reason: collection name');
      if (epoch !== ticket || disposed) return;
      description.set(props.description ?? '', 'reason: collection description');
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
      // A filtered result manifest may retire the accepted row. This source
      // announces its repaired cursor, rather than having native rendering
      // invent a first-row selection before the compiled owner is mounted.
      if (ready() && props.mode === 'filterlist') {
        if (kind(cursor.get()) === 0) emit('select', -1, false, 'reconcile');
        else if (props.cursor !== cursor.get()) emit('select', cursor.get(), false, 'reconcile');
        if (epoch !== ticket || !ready()) return;
        if (kind(cursor.get()) > 0) requestReveal(cursor.get());
        if (epoch !== ticket || !ready()) return;
      }
      if (attached) run.update();
    };
    def.lifecycle.onCreated(sync);
    def.lifecycle.onMounted((run) => { if (!disposed) { attached = true; sync(run); if (run.props.get().autoFocus) focus.focusSelf(); } });
    def.props.watch(['mode', 'label', 'description', 'cursor', 'expanded', 'revision', 'disabled', 'autoFocus', 'columns', 'orientation'], sync);
    def.lifecycle.onUnmounted(() => { ++epoch; attached = false; current = null; });
    def.lifecycle.onBeforeDispose(() => { ++epoch; disposed = true; attached = false; current = null; });
    def.feedback.style.use(tw('w-full h-full'));
    return (renderer) => renderer.slot();
  },
});
