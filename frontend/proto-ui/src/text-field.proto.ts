import { definePrototype, tw, type RunHandle } from '@proto.ui/core';
import { asFocusable } from '@proto.ui/hooks';
import { asInputEditor, type InputRootProps } from './upstream-input/root.proto';

type Props = InputRootProps & {
  autoFocus?: boolean;
  selectAllOnMount?: boolean;
  submitOnEnter?: boolean;
  cancelOnEscape?: boolean;
  advanceOnTab?: boolean;
  commitOnBlur?: boolean;
  searchNavigation?: boolean;
  numeric?: boolean;
  numericMin?: number;
  numericMax?: number;
  numericStep?: number;
  numericDefault?: number;
};

// Base Input owns the editor, value/composition, focus and accessible textbox.
// This prototype adds application-boundary requests, not a second text engine.
export default definePrototype<Props>({
  name: 'verdant-text-field',
  modules: asInputEditor.modules,
  setup(def) {
    const inherited = asInputEditor();
    const state = inherited.stateHandles;
    if (!state) throw new Error('Input root state projection is required');
    const focus = asFocusable();
    const viewEpoch = def.state.numberDiscrete('viewEpoch', 0);
    def.expose.state('viewEpoch', viewEpoch);
    def.props.define({
      autoFocus: { type: 'boolean', default: false },
      selectAllOnMount: { type: 'boolean', default: false },
      submitOnEnter: { type: 'boolean', default: false },
      cancelOnEscape: { type: 'boolean', default: false },
      advanceOnTab: { type: 'boolean', default: false },
      commitOnBlur: { type: 'boolean', default: false },
      searchNavigation: { type: 'boolean', default: false },
      numeric: { type: 'boolean', default: false },
      numericMin: { type: 'number', default: 0 },
      numericMax: { type: 'number', default: 1 },
      numericStep: { type: 'number', default: 1 },
      numericDefault: { type: 'number', default: 0 },
    });
    def.feedback.style.use(tw('w-full h-full min-w-0'));
    def.expose.event('submit', { payload: 'json' });
    def.expose.event('cancel', { payload: 'json' });
    def.expose.event('advance', { payload: 'json' });
    def.expose.event('blurCommit', { payload: 'json' });
    def.expose.event('selectionRequest', { payload: 'json' });
    def.expose.event('replacementRequest', { payload: 'json' });
    def.expose.event('navigationRequest', { payload: 'json' });
    let run: RunHandle<Props> | null = null;
    let mounted = false;
    let disposed = false;
    const allowed = (): boolean => mounted && !disposed && !state.disabled.get() && !state.composing.get();
    const request = (kind: string, forward: boolean): boolean => {
      const current = run;
      if (!current || !allowed()) return false;
      const detail = { value: state.value.get(), forward, viewEpoch: viewEpoch.get() };
      if (kind === 'submit') current.expose.emit('submit', detail);
      else if (kind === 'cancel') current.expose.emit('cancel', detail);
      else if (kind === 'advance') current.expose.emit('advance', detail);
      else if (kind === 'blurCommit') current.expose.emit('blurCommit', detail);
      else return false;
      return true;
    };
    // UTF-16 selection policy stays in authored source; the SDK only applies
    // the bounded range to this live text owner and reveals its caret.
    const selectRange = (start: number, end: number): boolean => {
      const current = run;
      if (!current || !allowed() || !Number.isFinite(start) || !Number.isFinite(end)) return false;
      const length = state.value.get().length;
      let from = Math.floor(start);
      let to = Math.floor(end);
      if (from < 0) from = 0;
      if (to < from) to = from;
      if (from > length) from = length;
      if (to > length) to = length;
      const epoch = viewEpoch.get();
      focus.focusSelf();
      if (!allowed() || viewEpoch.get() !== epoch) return false;
      current.expose.emit('selectionRequest', { start: from, end: to, viewEpoch: epoch });
      return true;
    };
    def.expose.method('selectRange', selectRange);
    def.expose.method('selectAll', () => selectRange(0, state.value.get().length));
    def.expose.method('requestSubmit', () => request('submit', true));
    def.expose.method('requestCancel', () => request('cancel', false));
    def.expose.method('requestAdvance', (forward: boolean) => request('advance', forward));
    const replaceDraft = (value: string): boolean => {
      const current = run;
      if (!current || !allowed() || state.readOnly.get()) return false;
      current.expose.emit('replacementRequest', { value, start: 0, end: value.length, viewEpoch: viewEpoch.get() });
      return true;
    };
    const stepText = (direction: number): boolean => {
      const current = run;
      if (!current || !allowed() || !current.props.get().numeric) return false;
      const props = current.props.get();
      const text = state.value.get();
      let value = text.length === 0 ? (props.numericDefault ?? 0) : Number(text);
      if (!Number.isFinite(value)) value = props.numericDefault ?? 0;
      value += direction * (props.numericStep ?? 1);
      if (value < (props.numericMin ?? 0)) value = props.numericMin ?? 0;
      if (value > (props.numericMax ?? 1)) value = props.numericMax ?? 1;
      return replaceDraft(String(value));
    };
    def.expose.method('stepText', stepText);
    def.event.on('key.down', (_current, event) => {
      const current = run;
      if (!current || !mounted || disposed || state.disabled.get()) return;
      const key = event.key;
      if (state.composing.get()) {
        // The platform IME supplies text through TextControl. Application
        // terminal keys cannot confirm, cancel or move away from marked text.
        if (key === 'Enter' || key === 'Escape' || key === 'Tab') event.control.requestDefaultActionPrevention();
        return;
      }
      const props = current.props.get();
      const command = event.ctrlKey === true || event.metaKey === true;
      // A search field explicitly delegates only these collection requests.
      // The authored request precedes native single-line caret/Enter defaults;
      // ordinary editor keys and marked-text candidate navigation stay local.
      if (props.searchNavigation && !event.altKey
        && ((!command && (key === 'ArrowUp' || key === 'ArrowDown' || key === 'Enter' || key === 'Escape'))
          || (command && !event.shiftKey && (key === 'f' || key === 'F')))) {
        event.control.requestDefaultActionPrevention();
        current.expose.emit('navigationRequest', {
          key: key ?? '', value: state.value.get(), shift: event.shiftKey === true,
          command, viewEpoch: viewEpoch.get(),
        });
        return;
      }
      if (command || event.altKey) return;
      if (props.numeric && (key === 'ArrowUp' || key === 'ArrowDown')) {
        event.control.requestDefaultActionPrevention();
        stepText(key === 'ArrowUp' ? 1 : -1);
      } else if (props.numeric && key === 'Delete') {
        event.control.requestDefaultActionPrevention();
        replaceDraft('');
      } else if (key === 'Enter' && props.submitOnEnter) {
        event.control.requestDefaultActionPrevention();
        request('submit', true);
      } else if (key === 'Escape' && props.cancelOnEscape) {
        event.control.requestDefaultActionPrevention();
        request('cancel', false);
      } else if (key === 'Tab' && props.advanceOnTab) {
        event.control.requestDefaultActionPrevention();
        request('advance', event.shiftKey !== true);
      }
    });
    def.event.on('host:blur', () => {
      const current = run;
      if (current && current.props.get().commitOnBlur) request('blurCommit', true);
    });
    def.lifecycle.onMounted((current) => {
      if (disposed) return;
      run = current;
      mounted = true;
      viewEpoch.set(viewEpoch.get() + 1, 'reason: text view mounted');
      if (current.props.get().selectAllOnMount) selectRange(0, state.value.get().length);
      else if (current.props.get().autoFocus && !state.disabled.get()) focus.focusSelf();
    });
    def.lifecycle.onUnmounted(() => {
      mounted = false;
      run = null;
      viewEpoch.set(viewEpoch.get() + 1, 'reason: text view retired');
    });
    def.lifecycle.onBeforeDispose(() => {
      mounted = false;
      disposed = true;
      run = null;
    });
    return inherited.render;
  },
});
