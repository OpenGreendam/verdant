import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';
const upstream = process.env.PROTO_UI_UPSTREAM;
const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
let createRuntimeSession, caps;
const prototypes = {};
if (upstream) {
  const require = createRequire(path.join(upstream, 'package.json'));
  const { register } = await import(pathToFileURL(require.resolve('tsx/esm/api')).href);
  register({ tsconfig: path.join(upstream, 'tsconfig.json') });
  const load = (name) => import(pathToFileURL(path.join(upstream, 'packages', name, 'src/index.ts')).href);
  ({ createRuntimeSession } = await load('runtime'));
  caps = Object.assign({}, ...await Promise.all(['anatomy', 'context', 'expose-state', 'expose-event', 'focus', 'event', 'as-trigger', 'overlay', 'scroll'].map(name => load(`modules/${name}`))));
  for (const name of ['action-control', 'collection-control', 'overlay-control', 'scroll-region']) prototypes[name] = (await import(pathToFileURL(path.join(repository, `frontend/proto-ui/src/${name}.proto.ts`)).href)).default;
}
const runtimeTest = (name, work) => test(name, { skip: !upstream && 'set PROTO_UI_UPSTREAM to the pinned checkout' }, work);
async function fixture(t, name, initial = {}) {
  let props = { revision: 'document:1', disabled: false, ...initial };
  const prototype = prototypes[name];
  const events = []; const scrollRequests = []; let scrollDisposed = 0; let exposes; let onEvent = () => {};
  const target = new EventTarget(); const globalTarget = new EventTarget(); const token = {};
  const session = createRuntimeSession(prototype, {
    prototypeName: prototype.name, getRawProps: () => props,
    schedule: work => work(), commit: (_children, signal) => signal?.done(),
    onRuntimeReady(wiring) {
      wiring.attach('as-trigger', [[caps.AS_TRIGGER_INSTANCE_CAP, token], [caps.AS_TRIGGER_PARENT_CAP, () => null], [caps.AS_TRIGGER_GET_PROTO_CAP, () => prototype], [caps.AS_TRIGGER_MERGE_GROUP_CAP, () => {}], [caps.AS_TRIGGER_GET_GROUP_EVENT_TARGET_CAP, () => target]]);
      wiring.attach('anatomy', [[caps.ANATOMY_INSTANCE_TOKEN_CAP, token], [caps.ANATOMY_PARENT_CAP, () => null], [caps.ANATOMY_GET_PROTO_CAP, () => prototype]]);
      wiring.attach('context', [[caps.CONTEXT_INSTANCE_TOKEN_CAP, token], [caps.CONTEXT_PARENT_CAP, () => null]]);
      wiring.attach('event', [[caps.EVENT_ROOT_TARGET_CAP, () => target], [caps.EVENT_GLOBAL_TARGET_CAP, () => globalTarget]]);
      wiring.attach('scroll', [[caps.SCROLL_SURFACE_HOST_CAP, {
        support: { system: true, composed: false },
        attach(connection) {
          const axis = { position: 0, visibleRatio: 0.5, canScrollBefore: false, canScrollAfter: true, atEnd: false };
          connection.onFacts({ axes: 'vertical', horizontal: axis, vertical: axis, scrolling: false, projection: 'system', endFollow: { state: 'off', requestStatus: 'idle' } });
          return { update() {}, request(request) { scrollRequests.push(request); }, dispose() { scrollDisposed++; } };
        },
      }]]);
      wiring.attach('overlay', [[caps.OVERLAY_TARGET_HOST_CAP, { root: () => target, resolve: candidate => candidate === target ? target : null }], [caps.OVERLAY_MODAL_CAP, { lock() {}, unlock() {} }], [caps.OVERLAY_LAYER_SCHEDULER_CAP, { attach: () => () => {} }]]);
      wiring.attach('focus', [[caps.FOCUS_INSTANCE_TOKEN_CAP, token], [caps.FOCUS_PARENT_CAP, () => null], [caps.FOCUS_ROOT_TARGET_CAP, () => target], [caps.FOCUS_REQUEST_FOCUS_CAP, () => true]]);
      wiring.attach('expose-state', [[caps.EXPOSE_STATE_SET_EXPOSES_CAP, next => { exposes = next; }]]);
      wiring.attach('expose-event', [[caps.EXPOSE_EVENT_SINK_CAP, (name, payload) => { events.push({ name, payload }); onEvent(name, payload); }]]);
    },
  });
  t.after(() => session.dispose()); await session.mount();
  return { session, events, scrollRequests, get scrollDisposed() { return scrollDisposed; }, get x() { return exposes; },
    call: (method, ...args) => session.invokeInCallbackScope(() => exposes[method](...args)),
    update(patch) { props = { ...props, ...patch }; session.controller.applyRawProps(props); },
    onEvent(callback) { onEvent = callback; },
    event(name, detail = {}) { const event = new CustomEvent(name, { cancelable: true, detail }); target.dispatchEvent(event); return event; },
    semantics: () => session.caps.getPort('a11y').getSnapshot(),
  };
}

runtimeTest('action pointer and assistive activation share accepted selection and disabled policy', async t => {
  const f = await fixture(t, 'action-control', { label: 'Mute', role: 'switch', selected: false, selectable: true });
  f.event('press.start'); assert.equal(f.x.pressed.get(), true);
  assert.equal(f.event('press.commit').defaultPrevented, true);
  assert.deepEqual(f.events, [{ name: 'activate', payload: { selected: true, revision: 'document:1', origin: 'press', key: '', shiftKey: false, ctrlKey: false, metaKey: false, altKey: false } }]);
  assert.equal(f.x.selected.get(), false);
  assert.deepEqual(f.semantics().states, { disabled: false, checked: false });
  f.update({ selected: true, revision: 'document:2' });
  f.event('host:verdant-action-activate'); assert.equal(f.events.at(-1).payload.selected, false);
  f.update({ disabled: true }); f.event('press.start'); f.event('press.commit'); f.call('requestActivate');
  assert.equal(f.events.length, 2); assert.equal(f.x.pressed.get(), false); assert.equal(f.semantics().states.disabled, true);
});
runtimeTest('action preserves source modifiers and separates keyboard assistive and method activation', async t => {
  const f = await fixture(t, 'action-control');
  assert.equal(f.event('press.commit', { key: 'Enter', shiftKey: true, altKey: true }).defaultPrevented, true);
  assert.deepEqual(f.events.at(-1).payload, { selected: true, revision: 'document:1', origin: 'keyboard', key: 'Enter', shiftKey: true, ctrlKey: false, metaKey: false, altKey: true });
  f.event('press.commit', { ctrlKey: true, metaKey: true });
  assert.deepEqual(f.events.at(-1).payload, { selected: true, revision: 'document:1', origin: 'press', key: '', shiftKey: false, ctrlKey: true, metaKey: true, altKey: false });
  f.event('host:verdant-action-activate'); assert.equal(f.events.at(-1).payload.origin, 'assistive');
  f.call('requestActivate'); assert.equal(f.events.at(-1).payload.origin, 'programmatic');
});
runtimeTest('context-menu actions share mouse keyboard assistive policy without making left click open the menu', async t => {
  const f = await fixture(t, 'action-control', { contextMenu: true, contextOnly: true });
  assert.equal(f.event('context.menu', { shiftKey: true }).defaultPrevented, true);
  assert.equal(f.events.at(-1).name, 'context'); assert.equal(f.events.at(-1).payload.shiftKey, true);
  assert.equal(f.event('key.down', { key: 'F10', shiftKey: true }).defaultPrevented, true);
  assert.equal(f.events.at(-1).payload.origin, 'keyboard');
  assert.equal(f.event('key.down', { key: 'ContextMenu' }).defaultPrevented, true);
  f.event('host:verdant-action-activate'); assert.equal(f.events.at(-1).name, 'context'); assert.equal(f.events.at(-1).payload.origin, 'assistive');
  f.event('press.commit'); assert.equal(f.events.at(-1).name, 'activate');
  f.event('press.commit', { key: 'Enter' }); assert.equal(f.events.at(-1).name, 'context');
  f.update({ disabled: true }); const count = f.events.length;
  f.event('context.menu'); f.event('key.down', { key: 'ContextMenu' }); assert.equal(f.events.length, count);
});
runtimeTest('action reentrant accepted props are not overwritten by suspended sync', async t => {
  const f = await fixture(t, 'action-control', { label: 'Before', selected: false, role: 'tab' });
  const stop = f.x.selected.subscribe(event => { if (event.type === 'next' && event.next === true) f.update({ selected: false, disabled: true, label: 'After', revision: 'document:3' }); });
  f.update({ selected: true, label: 'Suspended', revision: 'document:2' }); stop();
  assert.equal(f.x.selected.get(), false); assert.equal(f.x.disabled.get(), true); assert.equal(f.semantics().name.value, 'After');
});
runtimeTest('action keyboard group navigation proposes selection and detached roots cannot act', async t => {
  const f = await fixture(t, 'action-control', { role: 'tab', selected: true, position: 0, count: 3 });
  assert.equal(f.event('key.down', { key: 'ArrowLeft' }).defaultPrevented, true);
  assert.deepEqual(f.events[0], { name: 'navigate', payload: { index: 2, revision: 'document:1' } });
  await f.session.unmount(); assert.equal(f.call('requestActivate'), false); assert.equal(f.events.length, 1);
});
runtimeTest('menu traversal skips unavailable rows, opens submenus and rejects stale callbacks', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'menu', label: 'Menu', cursor: 0, expanded: -1 });
  assert.equal(f.call('setItems', [1, 0, 2, 1]), true);
  assert.equal(f.event('key.down', { key: 'ArrowDown' }).defaultPrevented, true); assert.equal(f.x.cursor.get(), 2);
  f.event('key.down', { key: 'ArrowRight' }); assert.equal(f.x.expanded.get(), 2); assert.equal(f.events.at(-1).name, 'expand');
  f.event('key.down', { key: 'ArrowLeft' }); assert.equal(f.x.expanded.get(), -1);
  f.update({ revision: 'document:2', cursor: 3 });
  const n = f.events.length; assert.equal(f.call('choose', 0, false, 'document:1'), false); assert.equal(f.events.length, n);
  f.call('choose', 3, false, 'document:2'); assert.equal(f.events.at(-1).name, 'activate');
  f.event('key.down', { key: 'Escape' }); assert.equal(f.events.at(-1).name, 'close');
});
runtimeTest('collection validates typed manifests and large unavailable spans without recursion overflow', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'table', cursor: 0 });
  assert.equal(f.call('setItems', [1, 9]), false);
  const items = new Array(12000).fill(0); items[11999] = 1;
  assert.equal(f.call('setItems', items), true); assert.equal(f.x.cursor.get(), 11999);
  f.event('key.down', { key: 'Home' }); assert.equal(f.x.cursor.get(), 11999);
  f.event('key.down', { key: 'ArrowLeft', ctrlKey: true, shiftKey: true });
  assert.deepEqual(f.events.at(-1), { name: 'command', payload: { index: 11999, extend: true, key: 'ArrowLeft', revision: 'document:1' } });
});
runtimeTest('collection activation emits once even with synchronous owner acceptance and reentrant props settle', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'table', cursor: 0 });
  f.call('setItems', [1, 1, 1]);
  f.onEvent((name, payload) => { if (name === 'activate') f.update({ cursor: payload.index, revision: 'document:2' }); });
  f.call('choose', 1, false, 'document:1'); assert.equal(f.events.filter(e => e.name === 'activate').length, 1); assert.equal(f.x.cursor.get(), 1);
  const stop = f.x.cursor.subscribe(event => { if (event.type === 'next' && event.next === 2) f.update({ cursor: 0, disabled: true, revision: 'document:4' }); });
  f.update({ cursor: 2, disabled: false, revision: 'document:3' }); stop();
  assert.equal(f.x.cursor.get(), 0); assert.equal(f.semantics().states.disabled, true);
  const n = f.events.length; f.event('key.down', { key: 'Enter' }); assert.equal(f.events.length, n);
});

runtimeTest('table semantics and addressed row commands reject stale and reentrant authority', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'table', columns: 5 });
  f.call('setItems', [1, 1, 1]);
  assert.deepEqual(f.semantics().states, { disabled: false, rowCount: 3, columnCount: 5 });
  assert.equal(f.call('keyAt', 2, 'ArrowUp', false, true, 'old'), false);
  assert.equal(f.events.length, 0);
  f.onEvent((name) => { if (name === 'select') f.update({ revision: 'document:2', cursor: 0 }); });
  assert.equal(f.call('keyAt', 2, 'ArrowUp', false, true, 'document:1'), false);
  assert.equal(f.events.filter(e => e.name === 'command').length, 0);
  assert.equal(f.x.cursor.get(), 0);
});
runtimeTest('curve point traversal restores endpoints and closes on tab boundaries', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'points', cursor: 0 });
  f.call('setItems', [1, 1, 1, 1, 1, 1]);
  assert.deepEqual(f.semantics().states, { disabled: false });
  f.call('key', 'PageDown', false, false); assert.equal(f.x.cursor.get(), 4);
  f.call('key', 'Tab', false, false); assert.equal(f.x.cursor.get(), 5);
  f.call('key', 'Tab', false, false); assert.equal(f.events.at(-1).name, 'close');
  f.call('key', 'Home', false, false); assert.equal(f.x.cursor.get(), 0);
  f.call('key', 'Tab', true, false); assert.equal(f.events.at(-1).name, 'close');
  await f.session.unmount(); assert.equal(f.call('keyAt', 0, 'ArrowRight', false, false, 'document:1'), false);
});

runtimeTest('modal response policy, accepted visibility and scope follow one source lifecycle', async t => {
  const f = await fixture(t, 'overlay-control', { label: 'Confirm', role: 'alertdialog', choices: 3, defaultChoice: 2, cancelChoice: 0 });
  assert.equal(f.x.open.get(), true); assert.equal(f.x.scopeActive.get(), true);
  assert.equal(f.semantics().role, 'alertdialog');
  assert.equal(f.event('key.down', { key: 'Enter' }).defaultPrevented, true);
  assert.deepEqual(f.events.at(-1), { name: 'response', payload: { index: 2, revision: 'document:1' } });
  assert.equal(f.call('choose', 3, 'document:1'), false); assert.equal(f.call('choose', 0.5, 'document:1'), false);
  f.update({ open: false, revision: 'document:2' });
  assert.equal(f.x.open.get(), false); assert.equal(f.x.scopeActive.get(), false);
  assert.equal(f.call('choose', 0, 'document:1'), false);
  const n = f.events.length; f.event('key.down', { key: 'Escape' }); assert.equal(f.events.length, n);
  f.update({ open: true, defaultChoice: -1, cancelChoice: -1, revision: 'document:3' });
  assert.equal(f.x.scopeActive.get(), true); assert.equal(f.call('key', 'Enter'), false); assert.equal(f.call('key', 'Escape'), false);
  await f.session.unmount(); assert.equal(f.x.scopeActive.get(), false); assert.equal(f.call('choose', 0, 'document:3'), false);
});
runtimeTest('reentrant modal visibility acceptance cannot reopen a dismissed scope', async t => {
  const f = await fixture(t, 'overlay-control', { open: false });
  const stop = f.x.open.subscribe(event => { if (event.type === 'next' && event.next) f.update({ open: false, revision: 'document:3' }); });
  f.update({ open: true, revision: 'document:2' }); stop();
  assert.equal(f.x.open.get(), false); assert.equal(f.x.scopeActive.get(), false);
});

runtimeTest('scroll region owns focused page/edge requests without trapping ordinary document focus', async t => {
  const f = await fixture(t, 'scroll-region', { label: 'References' });
  assert.equal(f.x.scopeActive.get(), false);
  assert.equal(f.call('key', 'ArrowDown', false), false);
  assert.deepEqual(f.scrollRequests, []);
  f.event('host:focus');
  for (const [name, shift] of [['ArrowDown', false], ['PageUp', false], [' ', true], ['Home', false], ['End', false]]) f.call('key', name, shift);
  assert.deepEqual(f.scrollRequests, [
    { kind: 'by', axis: 'vertical', delta: 40 },
    { kind: 'page', axis: 'vertical', direction: 'before' },
    { kind: 'page', axis: 'vertical', direction: 'before' },
    { kind: 'to', axis: 'vertical', position: 0 },
    { kind: 'to-end', axis: 'vertical' },
  ]);
  await f.session.unmount(); assert.equal(f.x.scopeActive.get(), false); assert.equal(f.scrollDisposed, 1);
  assert.equal(f.call('key', 'End', false), false);
});

runtimeTest('scroll opt-in trap and axes update without capturing descendant editing keys', async t => {
  const f = await fixture(t, 'scroll-region', { trapFocus: true, axes: 'horizontal' });
  assert.equal(f.x.scopeActive.get(), true);
  f.event('host:focus');
  assert.equal(f.call('key', 'ArrowDown', false), false);
  assert.equal(f.call('key', 'ArrowRight', false), true);
  assert.deepEqual(f.scrollRequests, [{ kind: 'by', axis: 'horizontal', delta: 40 }]);
  f.update({ trapFocus: false, axes: 'both' });
  assert.equal(f.x.scopeActive.get(), false);
  f.event('host:focus');
  f.call('key', 'ArrowDown', false);
  assert.deepEqual(f.scrollRequests.at(-1), { kind: 'by', axis: 'vertical', delta: 40 });
  f.event('host:blur');
  const count = f.scrollRequests.length;
  assert.equal(f.call('key', 'Home', false), false);
  assert.equal(f.scrollRequests.length, count);
});
runtimeTest('compound scroll content can opt out of Tab without becoming inert', async t => {
  const f = await fixture(t, 'scroll-region', { tabStop: false });
  assert.equal(f.session.caps.getPort('focus').getFocusableConfig().navParticipation, 'none');
  f.event('host:focus'); f.call('key', 'PageDown', false);
  assert.deepEqual(f.scrollRequests, [{ kind: 'page', axis: 'vertical', direction: 'after' }]);
  f.update({ tabStop: true });
  assert.equal(f.session.caps.getPort('focus').getFocusableConfig().navParticipation, 'auto');
  f.update({ tabStop: false });
  await f.session.unmount(); await f.session.mount();
  assert.equal(f.session.caps.getPort('focus').getFocusableConfig().navParticipation, 'none');
});
runtimeTest('action projects expansion only for declared disclosure controls', async t => {
  const f = await fixture(t, 'action-control', { expandable: true, expanded: false, label: 'Section' });
  assert.deepEqual(f.semantics().states, { disabled: false, expanded: false });
  f.update({ expanded: true }); assert.equal(f.semantics().states.expanded, true);
  f.update({ expandable: false }); assert.deepEqual(f.semantics().states, { disabled: false });
});

runtimeTest('menu radio selection is checked and modified tab keys preserve host shortcuts', async t => {
  const radio = await fixture(t, 'action-control', { role: 'menuitemradio', selected: true, selectable: true });
  assert.deepEqual(radio.semantics().states, { disabled: false, checked: true });
  const tab = await fixture(t, 'action-control', { role: 'tab', selected: true, position: 0, count: 3 });
  for (const modifier of ['ctrlKey', 'altKey', 'metaKey']) {
    assert.equal(tab.event('key.down', { key: 'ArrowRight', [modifier]: true }).defaultPrevented, false);
  }
  assert.deepEqual(tab.events, []);
});

runtimeTest('collection rejects fractional indices and reentrant submenu changes retire older expansion', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'menu', cursor: 0 });
  f.call('setItems', [2, 1, 1]);
  for (const index of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, 3]) {
    assert.equal(f.call('choose', index, false, 'document:1'), false);
    assert.equal(f.call('selectIndex', index, false, 'document:1'), false);
  }
  assert.deepEqual(f.events, []);
  const unsubscribe = f.x.expanded.subscribe(event => {
    if (event.type === 'next' && event.next === 0) f.update({ disabled: true, expanded: -1, revision: 'new-menu' });
  });
  f.call('key', 'ArrowRight', false, false);
  unsubscribe();
  assert.equal(f.x.expanded.get(), -1);
  assert.deepEqual(f.events, []);
});

runtimeTest('search list routes text shortcuts through source without consuming input or stale field callbacks', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'searchlist', cursor: 0 });
  f.call('setItems', [1, 1, 1]);
  assert.equal(f.semantics().role, 'listbox');
  for (const key of [' ', 'Home', 'End', 'ArrowLeft', 'Delete']) {
    assert.equal(f.call('keyFrom', 'search', key, false, false, 'document:1'), false);
  }
  assert.deepEqual(f.events, []);
  assert.equal(f.call('keyFrom', 'search', 'ArrowDown', false, false, 'document:1'), true);
  assert.equal(f.events.at(-1).name, 'select');
  assert.equal(f.x.cursor.get(), 1);
  assert.equal(f.call('keyFrom', 'search', 'Enter', false, false, 'document:1'), true);
  assert.equal(f.events.at(-1).name, 'activate');
  assert.equal(f.call('keyFrom', 'root', 'f', false, true, 'document:1'), true);
  assert.equal(f.events.at(-1).payload.key, 'Find');
  const before = f.events.length;
  assert.equal(f.call('keyFrom', 'form', 'ArrowDown', false, false, 'document:1'), false);
  f.update({ revision: 'filtered', cursor: 0 });
  assert.equal(f.call('keyFrom', 'search', 'Enter', false, false, 'document:1'), false);
  assert.equal(f.events.length, before);
  assert.equal(f.call('keyFrom', 'form', 'Escape', false, false, 'filtered'), true);
  assert.equal(f.events.at(-1).name, 'close');
});
