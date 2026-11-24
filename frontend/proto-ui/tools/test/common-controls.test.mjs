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
runtimeTest('action reentrant accepted props are not overwritten by suspended sync', async t => {
  const f = await fixture(t, 'action-control', { label: 'Before', selected: false, role: 'tab' });
  const stop = f.x.selected.subscribe(event => { if (event.type === 'next' && event.next === true) f.update({ selected: false, disabled: true, label: 'After', revision: 'document:3' }); });
  f.update({ selected: true, label: 'Suspended', revision: 'document:2' }); stop();
  assert.equal(f.x.selected.get(), false); assert.equal(f.x.disabled.get(), true); assert.equal(f.semantics().name.value, 'After');
});
runtimeTest('action retires a held press when the same label receives new domain authority', async t => {
  const f = await fixture(t, 'action-control', { label: 'Remove' });
  f.event('press.start'); assert.equal(f.x.pressed.get(), true);
  f.update({ revision: 'document:2' }); assert.equal(f.x.pressed.get(), false);
  f.event('press.end'); f.event('press.commit'); assert.equal(f.events.length, 0);
  f.event('press.start'); f.event('press.end'); f.event('press.commit');
  assert.deepEqual(f.events, [{ name: 'activate', payload: { selected: true, revision: 'document:2', origin: 'press', key: '', shiftKey: false, ctrlKey: false, metaKey: false, altKey: false } }]);
  f.event('key.up', { key: 'Enter' }); f.event('press.commit'); assert.equal(f.events.length, 1);
  f.event('host:verdant-action-activate'); assert.equal(f.events.length, 2, 'assistive activation is a fresh source request');
});
runtimeTest('action live accessible text preserves an admitted press without changing command authority', async t => {
  const f=await fixture(t,'action-control',{label:'Position 0',description:'Playback stopped'});
  f.event('press.start');f.update({label:'Position 1',description:'Playback running'});
  assert.equal(f.x.pressed.get(),true);assert.equal(f.semantics().name.value,'Position 1');
  assert.equal(f.semantics().description.value,'Playback running');
  f.event('press.end');f.event('press.commit');
  assert.deepEqual(f.events,[{name:'activate',payload: { selected: true, revision: 'document:1', origin: 'press', key: '', shiftKey: false, ctrlKey: false, metaKey: false, altKey: false }}]);
  f.event('press.start');f.update({label:'Position 2',revision:'document:2'});
  f.event('press.commit');assert.equal(f.events.length,1,'new command authority still retires a held press');
});
runtimeTest('action reentrant retirement cannot revive a suspended press or emit a replacement command', async t => {
  const f = await fixture(t, 'action-control', { label: 'Remove' });
  const stop = f.x.pressed.subscribe(event => {
    if (event.type === 'next' && event.next === true) f.update({ revision: 'document:2', disabled: true });
  });
  f.event('press.start'); stop();
  assert.equal(f.x.pressed.get(), false); assert.equal(f.x.disabled.get(), true);
  f.update({ disabled: false }); f.event('press.commit'); assert.equal(f.events.length, 0);
  f.event('press.start'); f.event('press.cancel'); f.event('press.commit'); assert.equal(f.events.length, 0);
});
runtimeTest('action focus loss retires contact while hover follows the physical pointer route', async t => {
  const f=await fixture(t,'action-control');
  f.event('pointer.enter');f.event('press.start');f.event('host:blur');
  assert.equal(f.x.pressed.get(),false);assert.equal(f.x.hovered.get(),true);
  f.event('press.commit');assert.equal(f.events.length,0);
  f.event('pointer.leave');assert.equal(f.x.hovered.get(),false);
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

runtimeTest('scroll region owns keyboard page/edge requests and releases its native lease on detach', async t => {
  const f = await fixture(t, 'scroll-region', { label: 'References', trapFocus: true });
  assert.equal(f.x.scopeActive.get(), true);
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
runtimeTest('action projects expansion only for declared disclosure controls', async t => {
  const f = await fixture(t, 'action-control', { expandable: true, expanded: false, label: 'Section' });
  assert.deepEqual(f.semantics().states, { disabled: false, expanded: false });
  f.update({ expanded: true }); assert.equal(f.semantics().states.expanded, true);
  f.update({ expandable: false }); assert.deepEqual(f.semantics().states, { disabled: false });
});


runtimeTest('action Enter default response is source-owned and pointer/Space retain ordinary activation', async t => {
  const f = await fixture(t, 'action-control', { submitOnEnter: true, label: 'Choice' });
  assert.equal(f.event('key.down', { key: 'Enter' }).defaultPrevented, true);
  assert.deepEqual(f.events, [{ name: 'submit', payload: { revision: 'document:1' } }]);
  f.event('press.start', { key: 'Enter' }); assert.equal(f.x.pressed.get(), false);
  assert.equal(f.event('press.commit', { key: 'Enter' }).defaultPrevented, true);
  assert.deepEqual(f.events, [{ name: 'submit', payload: { revision: 'document:1' } }]);
  f.event('key.up', { key: 'Enter' });
  // Space remains available to the host's press protocol; suppressing key.down
  // would prevent the native compiler from producing its semantic press.
  assert.equal(f.event('key.down', { key: ' ' }).defaultPrevented, false);
  f.event('press.start', { key: ' ' });
  assert.equal(f.event('press.commit', { key: ' ' }).defaultPrevented, true);
  assert.equal(f.events.at(-1).name, 'activate');
  f.event('key.up', { key: ' ' });
  f.event('press.start'); f.event('press.commit'); assert.equal(f.events.at(-1).payload.origin, 'press');
  f.update({ disabled: true, revision: 'document:2' });
  const n = f.events.length; f.event('key.down', { key: 'Enter' }); f.event('press.commit');
  assert.equal(f.events.length, n);
});

runtimeTest('track list context requests validate hit facts and rejection restores accepted selection', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'tracklist', cursor: 0 });
  f.call('setItems', [1, 1, 1]);
  assert.equal(f.semantics().role, 'listbox');
  assert.equal(f.call('contextAt', 2, 'old', 4, 5, false), false);
  assert.equal(f.call('contextAt', 2, 'document:1', Infinity, 5, false), false);
  assert.equal(f.call('contextAt', 2, 'document:1', 4, 5, false), true);
  assert.deepEqual(f.events.at(-1), { name:'context', payload:{ index:2, x:4, y:5, keyboard:false, revision:'document:1' } });
  assert.equal(f.call('reconcile'), true); assert.equal(f.x.cursor.get(), 0);
  f.call('key', 'F10', true, false);
  assert.deepEqual(f.events.at(-1), { name:'context', payload:{ index:0, x:0, y:0, keyboard:true, revision:'document:1' } });
  const stop = f.x.cursor.subscribe(event => { if (event.type === 'next' && event.next === 1) f.update({ cursor:0, disabled:true, revision:'document:2' }); });
  const count = f.events.length;
  assert.equal(f.call('contextAt', 1, 'document:1', 4, 5, false), false); stop();
  assert.equal(f.events.length, count); assert.equal(f.x.cursor.get(), 0);
});

runtimeTest('collection page and edge navigation request its declared scroll engine', async t => {
  const f = await fixture(t, 'collection-control', { mode:'table', cursor:0 });
  f.call('setItems', new Array(100).fill(1));
  f.call('key','PageDown',false,false); assert.equal(f.x.cursor.get(),20);
  f.call('key','Home',false,false); assert.equal(f.x.cursor.get(),0);
  f.call('key','End',false,false); assert.equal(f.x.cursor.get(),99);
  assert.deepEqual(f.scrollRequests,[
    {kind:'page',axis:'vertical',direction:'after'},
    {kind:'to',axis:'vertical',position:0},
    {kind:'to-end',axis:'vertical'},
  ]);
});
runtimeTest('nested scroll regions can delegate their focus scope to the surrounding overlay', async t => {
  const f = await fixture(t, 'scroll-region', { trapFocus:false });
  assert.equal(f.x.scopeActive.get(),false); assert.equal(f.call('key','Tab',false),false);
  f.call('key','PageDown',false); assert.equal(f.scrollRequests.length,1);
  f.update({trapFocus:true}); assert.equal(f.x.scopeActive.get(),true);
  f.update({trapFocus:false}); assert.equal(f.x.scopeActive.get(),false);
});

runtimeTest('action context ownership validates native facts and routes keyboard through the same policy', async t => {
  const f = await fixture(t,'action-control',{contextMenu:true});
  assert.equal(f.call('requestContext',12,34,false,'old'),false);
  assert.equal(f.call('requestContext',Infinity,34,false,'document:1'),false);
  assert.equal(f.call('requestContext',12,34,false,'document:1'),true);
  assert.deepEqual(f.events.at(-1),{name:'context',payload:{x:12,y:34,keyboard:false,selected:false,revision:'document:1',origin:'press',key:'',shiftKey:false,ctrlKey:false,metaKey:false,altKey:false}});
  assert.equal(f.event('key.down',{key:'F10',shiftKey:true}).defaultPrevented,true);
  assert.deepEqual(f.events.at(-1),{name:'context',payload:{selected:false,revision:'document:1',origin:'keyboard',key:'F10',shiftKey:true,ctrlKey:false,metaKey:false,altKey:false}});
  f.update({disabled:true,revision:'document:2'});const n=f.events.length;
  assert.equal(f.call('requestContext',12,34,false,'document:2'),false);
  f.event('key.down',{key:'ContextMenu'});assert.equal(f.events.length,n);
});
runtimeTest('scroll axis policy handles horizontal panes without inventing a second engine', async t => {
  const f=await fixture(t,'scroll-region',{axes:'horizontal',trapFocus:false});
  f.call('key','ArrowRight',false);f.call('key','PageDown',false);f.call('key','Home',false);
  assert.deepEqual(f.scrollRequests,[{kind:'by',axis:'horizontal',delta:40},{kind:'page',axis:'horizontal',direction:'after'},{kind:'to',axis:'horizontal',position:0}]);
});

runtimeTest('filtered collection owns result reconciliation, search routing and focus commands', async t => {
  const f = await fixture(t, 'collection-control', { mode:'filterlist', cursor:-1 });
  assert.equal(f.events.at(-1).payload.index, -1, 'empty results clear accepted selection through the source');
  f.events.length = 0;
  f.call('setItems', [1, 1, 1]);
  assert.equal(f.semantics().role, 'listbox');
  assert.equal(f.x.cursor.get(), 0);
  assert.deepEqual(f.events.map(event => [event.name, event.payload.key]), [['select', 'reconcile'], ['command', 'reveal']]);
  f.events.length = 0;
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, true, 'document:1', false), true);
  assert.equal(f.x.cursor.get(), 1);
  assert.deepEqual(f.events.map(event => [event.name, event.payload.index, event.payload.key]), [['select', 1, ''], ['command', 1, 'reveal']]);
  assert.equal(f.call('filterKey', 'Enter', false, false, false, true, 'document:1', false), true);
  assert.equal(f.events.at(-1).name, 'activate');
  assert.equal(f.call('filterKey', 'f', false, true, false, false, 'document:1', false), true);
  assert.equal(f.events.at(-1).payload.key, 'focus-filter');
  assert.equal(f.call('filterKey', 'Escape', false, false, false, true, 'document:1', false), true);
  assert.equal(f.events.at(-1).name, 'close');
  f.update({ revision:'document:2', cursor:-1 });
  assert.equal(f.events.filter(event => event.name === 'select').at(-1).payload.revision, 'document:2');
  assert.equal(f.x.cursor.get(), 0);
  f.events.length = 0; f.update({ disabled:true, cursor:-1 });
  assert.equal(f.events.length, 0, 'disabled reconciliation cannot publish a selection');
});

runtimeTest('filtered routing leaves editing, modifiers, composition and replaced views authoritative', async t => {
  const f = await fixture(t, 'collection-control', { mode:'filterlist' });
  f.call('setItems', [1, 1]); f.events.length = 0;
  for (const key of [' ', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageDown', 'Backspace', 'Delete', 'Tab', 'x']) {
    assert.equal(f.call('filterKey', key, false, false, false, true, 'document:1', false), false);
  }
  for (const key of ['ArrowUp', 'ArrowDown', 'Enter', 'Escape', 'f']) {
    assert.equal(f.call('filterKey', key, false, key === 'f', true, true, 'document:1', false), false);
  }
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, false, 'document:1', false), false);
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, true, 'document:1', true), false);
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, true, 'old', false), false);
  assert.equal(f.events.length, 0);
  await f.session.unmount();
  assert.equal(f.call('filterKey', 'Enter', false, false, false, true, 'document:1', false), false);
});

runtimeTest('collection reveals measured rows by its nearest edge and rejects stale geometry', async t => {
  const f = await fixture(t, 'collection-control', { mode:'filterlist' });
  f.call('setItems', [1, 1]);
  assert.equal(f.call('revealGeometry', 0, -20, 20, 0, 100, 'document:1'), true);
  assert.equal(f.call('revealGeometry', 0, 90, 130, 0, 100, 'document:1'), true);
  assert.equal(f.call('revealGeometry', 0, 10, 30, 0, 100, 'document:1'), true);
  assert.equal(f.call('revealGeometry', 0, -10, 110, 0, 100, 'document:1'), true);
  assert.deepEqual(f.scrollRequests, [{kind:'by',axis:'vertical',delta:-20}, {kind:'by',axis:'vertical',delta:30}]);
  for (const args of [[1,110,140,0,100,'document:1'], [0,110,140,0,100,'old'], [0,NaN,140,0,100,'document:1'], [0,110,140,0,0,'document:1'], [0,140,110,0,100,'document:1']]) {
    assert.equal(f.call('revealGeometry', ...args), false);
  }
  assert.equal(f.scrollRequests.length, 2);
  await f.session.unmount();
  assert.equal(f.call('revealGeometry', 0, 110, 140, 0, 100, 'document:1'), false);
});

runtimeTest('horizontal collections own page, edge and measured strip reveal on one axis', async t => {
  const f = await fixture(t, 'collection-control', { mode:'tracklist', orientation:'horizontal' });
  f.call('setItems', new Array(40).fill(1));
  f.call('key','PageDown',false,false); assert.equal(f.x.cursor.get(),20);
  f.call('key','Home',false,false); f.call('key','End',false,false);
  assert.equal(f.call('revealGeometry',39,150,180,0,100,'document:1'),true);
  assert.deepEqual(f.scrollRequests,[{kind:'page',axis:'horizontal',direction:'after'}, {kind:'to',axis:'horizontal',position:0},
    {kind:'to-end',axis:'horizontal'}, {kind:'by',axis:'horizontal',delta:80}]);
});

runtimeTest('reentrant result acceptance retires a suspended page and reveal proposal', async t => {
  const f = await fixture(t, 'collection-control', { mode:'filterlist' });
  f.call('setItems', new Array(50).fill(1)); f.events.length = 0;
  f.onEvent(name => { if (name === 'select') f.update({ cursor:0, revision:'document:2', disabled:true }); });
  assert.equal(f.call('key','PageDown',false,false),true);
  assert.equal(f.x.cursor.get(),0); assert.equal(f.scrollRequests.length,0);
  assert.equal(f.events.filter(event => event.name === 'command').length,0);
});

runtimeTest('scroll nearest reveal validates measured geometry against the accepted item authority', async t => {
  const f=await fixture(t,'scroll-region',{trapFocus:false,revealIndex:3,revealRevision:'lane:3'});
  assert.equal(f.call('revealGeometry',3,300,340,0,200,'old'),false);
  assert.equal(f.call('revealGeometry',2,300,340,0,200,'lane:3'),false);
  assert.equal(f.call('revealGeometry',3,NaN,340,0,200,'lane:3'),false);
  assert.equal(f.call('revealGeometry',3,300,340,0,200,'lane:3'),true);
  assert.deepEqual(f.scrollRequests,[{kind:'by',axis:'vertical',delta:140}]);
  f.call('revealGeometry',3,20,60,0,200,'lane:3');
  f.call('revealGeometry',3,-10,300,0,200,'lane:3');
  assert.equal(f.scrollRequests.length,1,'visible and oversized spanning items do not oscillate');
  f.update({axes:'horizontal',revealRevision:'lane:4'});
  f.call('revealGeometry',3,-40,-10,0,200,'lane:4');
  assert.deepEqual(f.scrollRequests.at(-1),{kind:'by',axis:'horizontal',delta:-40});
  await f.session.unmount();assert.equal(f.call('revealGeometry',3,300,340,0,200,'lane:4'),false);
});

runtimeTest('action preserves source modifiers and separates keyboard assistive and method activation', async t => {
  const f = await fixture(t, 'action-control');
  f.event('press.start', { key: 'Enter', shiftKey: true, altKey: true });
  assert.equal(f.event('press.commit', { key: 'Enter', shiftKey: true, altKey: true }).defaultPrevented, true);
  assert.deepEqual(f.events.at(-1).payload, { selected: true, revision: 'document:1', origin: 'keyboard', key: 'Enter', shiftKey: true, ctrlKey: false, metaKey: false, altKey: true });
  f.event('press.start', { ctrlKey: true, metaKey: true });
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
  f.event('press.start');
  f.event('press.commit'); assert.equal(f.events.at(-1).name, 'activate');
  f.event('press.start', { key: 'Enter' });
  f.event('press.commit', { key: 'Enter' }); assert.equal(f.events.at(-1).name, 'context');
  f.update({ disabled: true }); const count = f.events.length;
  f.event('context.menu'); f.event('key.down', { key: 'ContextMenu' }); assert.equal(f.events.length, count);
});

runtimeTest('explicit current-row selection selects an initial row and collapses a range while hover stays deduplicated', async t => {
  for (const mode of ['table', 'listbox']) {
    const f = await fixture(t, 'collection-control', { mode, cursor: 0 });
    f.call('setItems', [1, 1, 1]);
    assert.equal(f.call('selectIndex', 0, false, 'document:1'), true);
    assert.deepEqual(f.events, [], 'an unchanged hover cursor does not select the first row');
    assert.equal(f.call('selectAndFocus', 0, false, 'document:1'), true);
    assert.equal(f.call('selectAndFocus', 2, true, 'document:1'), true);
    assert.equal(f.call('selectAndFocus', 2, false, 'document:1'), true);
    assert.deepEqual(f.events.map(({ name, payload }) => [name, payload.index, payload.extend]), [
      ['select', 0, false], ['select', 2, true], ['select', 2, false],
    ], 'plain click at the current range endpoint must be a new selection intent');
    assert.equal(f.call('selectIndex', 2, false, 'document:1'), true);
    assert.equal(f.events.length, 3, 'repeat hover remains deduplicated after explicit selection');
    assert.equal(f.call('selectAndFocus', 2, false, 'stale'), false);
    f.update({ disabled: true });
    assert.equal(f.call('selectAndFocus', 2, false, 'document:1'), false);
    assert.equal(f.events.length, 3, 'explicit intent retains revision and disabled guards');
  }
});

runtimeTest('current-row keyboard navigation remains an explicit selection intent at boundaries', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'table', cursor: 0 });
  f.call('setItems', [1, 1, 1]);
  const keys = [
    { key: 'Home' },
    { key: 'End', shiftKey: true },
    { key: 'End' },
    { key: 'Home', shiftKey: true },
    { key: 'ArrowUp' },
  ];
  for (const key of keys) assert.equal(f.event('key.down', key).defaultPrevented, true);
  assert.deepEqual(f.events.map(({ name, payload }) => [name, payload.index, payload.extend]), [
    ['select', 0, false], ['select', 2, true], ['select', 2, false],
    ['select', 0, true], ['select', 0, false],
  ], 'Home/End and an unmodified edge arrow must collapse selection even without cursor movement');
  assert.equal(f.call('selectIndex', 0, false, 'document:1'), true);
  assert.equal(f.events.length, keys.length, 'hover does not gain keyboard semantics');
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

runtimeTest('collection unhandled addressed keys leave cursor and selection unchanged', async t => {
  const f = await fixture(t, 'collection-control', { mode:'table', cursor:0 });
  f.call('setItems', [1,1,1]);
  assert.equal(f.call('keyAt',2,'x',false,false,'document:1'),false);
  assert.equal(f.x.cursor.get(),0); assert.deepEqual(f.events,[]);
  assert.equal(f.call('keyAt',2,'F2',false,false,'document:1'),true);
  assert.deepEqual(f.events.map(e=>[e.name,e.payload.index]),[['select',2],['activate',2]]);
});
runtimeTest('action consumes a committed press once without requiring accepted revision changes', async t => {
  const f = await fixture(t,'action-control');
  f.event('press.start'); f.event('press.commit'); f.event('press.commit');
  assert.equal(f.events.length,1); assert.equal(f.x.pressed.get(),false);
  f.event('host:verdant-action-activate'); assert.equal(f.events.length,2);
});
runtimeTest('ordinary scroll regions leave Tab free and retain child page fallback', async t => {
  const f=await fixture(t,'scroll-region');
  assert.equal(f.x.scopeActive.get(),false); assert.equal(f.x.focused.get(),false);
  assert.equal(f.event('key.down',{key:'Tab'}).defaultPrevented,false);
  assert.equal(f.call('keyInput','PageDown',false,false,false),true);
  assert.deepEqual(f.scrollRequests,[{kind:'page',axis:'vertical',direction:'after'}]);
  f.update({trapFocus:true,tabStop:false}); assert.equal(f.x.scopeActive.get(),true);
  f.update({trapFocus:false}); assert.equal(f.x.scopeActive.get(),false);
  assert.equal(f.call('keyInput','Tab',false,false,false),false);
});
runtimeTest('scroll native key facts preserve modified shortcuts and independent trap policy', async t => {
  const f=await fixture(t,'scroll-region',{axes:'both',tabStop:false});
  for(const key of ['PageDown','End','ArrowDown',' '])for(const modifier of ['ctrlKey','metaKey','altKey']) {
    assert.equal(f.event('key.down',{key,[modifier]:true}).defaultPrevented,false);
  }
  assert.deepEqual(f.scrollRequests,[]);
  assert.equal(f.call('keyInput','End',false,true,false),false);
  assert.equal(f.call('keyInput','End',false,false,true),false);
  f.event('host:focus');
  assert.equal(f.event('key.down',{key:'PageDown',shiftKey:true}).defaultPrevented,true);
  assert.deepEqual(f.scrollRequests,[{kind:'page',axis:'horizontal',direction:'after'}]);
});
runtimeTest('submitted Enter retains physical-tail ownership across reentrant command replacement', async t => {
  const f=await fixture(t,'action-control',{submitOnEnter:true});
  f.onEvent(name=>{if(name==='submit')f.update({submitOnEnter:false,revision:'document:2'});});
  f.event('press.start');assert.equal(f.x.pressed.get(),true);
  assert.equal(f.event('key.down',{key:'Enter'}).defaultPrevented,true);
  assert.equal(f.x.pressed.get(),false);
  f.event('press.start',{key:'Enter'});assert.equal(f.x.pressed.get(),false);
  assert.equal(f.event('press.commit',{key:'Enter'}).defaultPrevented,true);
  f.event('key.down',{key:'Enter',repeat:true});f.event('press.start',{key:'Enter'});f.event('press.commit',{key:'Enter'});
  assert.deepEqual(f.events,[{name:'submit',payload:{revision:'document:1'}}]);
  f.event('key.up',{key:'Enter'});
  assert.equal(f.event('key.down',{key:'Enter'}).defaultPrevented,false);
  f.event('press.start',{key:'Enter'});f.event('press.commit',{key:'Enter',shiftKey:true});
  assert.deepEqual(f.events.at(-1),{name:'activate',payload:{selected:true,revision:'document:2',origin:'keyboard',key:'Enter',shiftKey:true,ctrlKey:false,metaKey:false,altKey:false}});
});

runtimeTest('scroll region owns focused page/edge requests without trapping ordinary document focus', async t => {
  const f = await fixture(t, 'scroll-region', { label: 'References' });
  assert.equal(f.x.scopeActive.get(), false);
  assert.equal(f.event('key.down', { key: 'ArrowDown' }).defaultPrevented, false);
  assert.deepEqual(f.scrollRequests, []);
  f.event('host:focus');
  for (const [key, shiftKey] of [['ArrowDown', false], ['PageUp', false], [' ', true], ['Home', false], ['End', false]]) {
    assert.equal(f.event('key.down', { key, shiftKey }).defaultPrevented, true);
  }
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
  assert.equal(f.event('key.down', { key: 'ArrowDown' }).defaultPrevented, false);
  assert.equal(f.event('key.down', { key: 'ArrowRight' }).defaultPrevented, true);
  assert.deepEqual(f.scrollRequests, [{ kind: 'by', axis: 'horizontal', delta: 40 }]);
  f.update({ trapFocus: false, axes: 'both' });
  assert.equal(f.x.scopeActive.get(), false);
  f.event('host:focus');
  assert.equal(f.event('key.down', { key: 'ArrowDown' }).defaultPrevented, true);
  assert.deepEqual(f.scrollRequests.at(-1), { kind: 'by', axis: 'vertical', delta: 40 });
  f.event('host:blur');
  const count = f.scrollRequests.length;
  for (const key of ['Home', 'End', 'ArrowDown', ' ']) {
    assert.equal(f.event('key.down', { key }).defaultPrevented, false);
  }
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

runtimeTest('search list routes text shortcuts through source without consuming input or stale field callbacks', async t => {
  const f = await fixture(t, 'collection-control', { mode: 'filterlist', cursor: 0 });
  f.call('setItems', [1, 1, 1]);
  assert.equal(f.semantics().role, 'listbox');
  f.events.length = 0; // Source reconciliation/reveal has already completed.
  for (const key of [' ', 'Home', 'End', 'ArrowLeft', 'Delete']) {
    assert.equal(f.call('filterKey', key, false, false, false, true, 'document:1', false), false);
  }
  assert.deepEqual(f.events, []);
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, true, 'document:1', false), true);
  assert.equal(f.events.filter(event => event.name === 'select').at(-1).payload.index, 1);
  assert.equal(f.events.at(-1).payload.key, 'reveal');
  assert.equal(f.x.cursor.get(), 1);
  assert.equal(f.call('filterKey', 'Enter', false, false, false, true, 'document:1', false), true);
  assert.equal(f.events.at(-1).name, 'activate');
  assert.equal(f.call('filterKey', 'f', false, true, false, false, 'document:1', false), true);
  assert.equal(f.events.at(-1).payload.key, 'focus-filter');
  assert.equal(f.call('filterKey', 'ArrowDown', false, false, false, false, 'document:1', false), false);
  f.update({ revision: 'filtered', cursor: 0 });
  const before = f.events.length;
  assert.equal(f.call('filterKey', 'Enter', false, false, false, true, 'document:1', false), false);
  assert.equal(f.events.length, before);
  // Form dismissal belongs to the enclosing compiled Overlay, while its text
  // control and the filtered collection retain their independent authority.
  const form = await fixture(t, 'overlay-control', { revision: 'filtered', cancelChoice: 0, choices: 2 });
  assert.equal(form.event('key.down', { key: 'Escape' }).defaultPrevented, true);
  assert.deepEqual(form.events, [{ name: 'response', payload: { index: 0, revision: 'filtered' } }]);
  assert.equal(f.events.length, before);
});

runtimeTest('alert response facts let the source choose suggested and cancel actions', async t => {
  const f = await fixture(t, 'overlay-control', {
    role: 'alertdialog', choices: 4, responseKinds: [2, 0, 1, 3],
    // Explicit indices are ignored once the host supplies response facts.
    defaultChoice: 0, cancelChoice: 3,
  });
  assert.equal(f.x.defaultChoice.get(), 3); assert.equal(f.x.cancelChoice.get(), 0);
  assert.equal(f.call('choose', 1, 'document:1'), false, 'unavailable choices reject pointer/assistive requests too');
  assert.equal(f.call('requestResponse', 'Enter', 'document:1'), true);
  assert.equal(f.events.at(-1).payload.index, 3);
  assert.equal(f.call('requestResponse', 'Escape', 'document:1'), true);
  assert.equal(f.events.at(-1).payload.index, 0);
  f.update({ responseKinds: [2, 0, 1, 1] });
  assert.equal(f.x.defaultChoice.get(), 2, 'without an explicit suggestion the first ordinary response is suggested');
  f.update({ responseKinds: [2, 0, 0, 2] });
  assert.equal(f.x.defaultChoice.get(), -1);
  assert.equal(f.call('requestResponse', 'Enter', 'document:1'), false);
});

runtimeTest('overlay rejects malformed response facts and stale semantic child requests', async t => {
  const f = await fixture(t, 'overlay-control', { choices: 2, responseKinds: [2, 1] });
  for (const responseKinds of [[1], [2, 4], [2, 1.5], [2, NaN], '21', { 0: 2, 1: 1, length: 2 }]) {
    f.update({ responseKinds });
    assert.equal(f.call('choose', 1, 'document:1'), false);
    assert.equal(f.call('requestResponse', 'Enter', 'document:1'), false);
    assert.equal(f.x.defaultChoice.get(), -1);
  }
  assert.deepEqual(f.events, []);
  f.update({ responseKinds: [2, 1], revision: 'document:2' });
  assert.equal(f.call('requestResponse', 'Enter', 'document:1'), false);
  assert.equal(f.call('requestResponse', 'Escape', 'document:1'), false);
  assert.equal(f.call('requestResponse', 'Enter', 'document:2'), true);
  assert.deepEqual(f.events, [{ name: 'response', payload: { index: 1, revision: 'document:2' } }]);
  f.update({ disabled: true });
  assert.equal(f.call('requestResponse', 'Escape', 'document:2'), false);
  await f.session.unmount();
  assert.equal(f.call('requestResponse', 'Enter', 'document:2'), false);
});

runtimeTest('reentrant response policy updates cannot restore a retired alert choice', async t => {
  const f = await fixture(t, 'overlay-control', { choices: 3, responseKinds: [2, 1, 0] });
  const stop = f.x.defaultChoice.subscribe(event => {
    if (event.type === 'next' && event.next === 2) f.update({ responseKinds: [2, 0, 0], open: false, revision: 'document:3' });
  });
  f.update({ responseKinds: [2, 0, 3], revision: 'document:2' }); stop();
  assert.equal(f.x.defaultChoice.get(), -1); assert.equal(f.x.open.get(), false);
  assert.equal(f.x.scopeActive.get(), false);
  assert.equal(f.call('requestResponse', 'Enter', 'document:2'), false);
  assert.deepEqual(f.events, []);
});

runtimeTest('radio and tab activation proposes selection idempotently without accepting it for the owner', async t => {
  for (const role of ['radio', 'menuitemradio', 'tab']) {
    const f = await fixture(t, 'action-control', { role, selected: false, selectable: true });
    for (let index = 0; index < 2; index++) f.call('requestActivate');
    assert.deepEqual(f.events.map(event => event.payload.selected), [true, true], role);
    assert.equal(f.x.selected.get(), false, 'the owner may reject a selection proposal');
    f.update({ selected: true });
    f.event('host:verdant-action-activate');
    f.event('press.start'); f.event('press.end'); f.event('press.commit');
    assert.deepEqual(f.events.map(event => event.payload.selected), [true, true, true, true], `${role} must not unselect its active member`);
    f.update({ disabled: true });
    assert.equal(f.call('requestActivate'), false);
    f.event('host:verdant-action-activate');
    assert.equal(f.events.length, 4, 'disabled exclusive controls cannot propose changes');
  }
});

runtimeTest('ordinary toggle roles keep controlled inversion while context preserves acceptance', async t => {
  for (const role of ['button', 'switch', 'checkbox', 'menuitemcheckbox']) {
    const f = await fixture(t, 'action-control', { role, selected: true, selectable: true, contextMenu: true });
    f.call('requestActivate'); f.call('requestActivate');
    assert.deepEqual(f.events.map(event => event.payload.selected), [false, false], `${role} proposals use accepted state until the owner changes it`);
    assert.equal(f.x.selected.get(), true);
    f.update({ selected: false }); f.call('requestActivate');
    assert.equal(f.events.at(-1).payload.selected, true);
    f.call('requestContext', 1, 1, false, 'document:1');
    assert.equal(f.events.at(-1).name, 'context');
    assert.equal(f.events.at(-1).payload.selected, false, 'opening a context menu never inverts selection');
  }
});

runtimeTest('overlay preserves child controlled selection proposals and legacy response indices', async t => {
  const form = await fixture(t, 'overlay-control', { choices: 3, defaultChoice: 2, cancelChoice: 0 });
  const checkbox = await fixture(t, 'action-control', { role: 'checkbox', selected: true, selectable: true });
  checkbox.onEvent((name, payload) => {
    if (name === 'activate') assert.equal(form.call('chooseSelection', 1, payload.selected, payload.revision), true);
  });
  checkbox.event('host:verdant-action-activate'); checkbox.event('host:verdant-action-activate');
  assert.deepEqual(form.events, [
    { name: 'selection', payload: { index: 1, selected: false, revision: 'document:1' } },
    { name: 'selection', payload: { index: 1, selected: false, revision: 'document:1' } },
  ], 'repeated proposals preserve the child source value rather than reconstructing a modal toggle');
  assert.equal(checkbox.x.selected.get(), true, 'the host still decides whether to accept the proposal');
  assert.equal(form.call('choose', 0, 'document:1'), true);
  assert.deepEqual(form.events.at(-1), { name: 'response', payload: { index: 0, revision: 'document:1' } });
  assert.equal(form.call('requestResponse', 'Enter', 'document:1'), true);
  assert.deepEqual(form.events.at(-1), { name: 'response', payload: { index: 2, revision: 'document:1' } });
});

runtimeTest('overlay selection shares choice validation and rejects stale or inactive authority', async t => {
  const f = await fixture(t, 'overlay-control', { choices: 3, responseKinds: [2, 1, 0] });
  for (const index of [-1, 2, 3, 0.5, NaN, Infinity]) assert.equal(f.call('chooseSelection', index, true, 'document:1'), false);
  for (const selected of [null, undefined, 'true', 0, {}]) assert.equal(f.call('chooseSelection', 1, selected, 'document:1'), false);
  f.update({ responseKinds: [2] });
  assert.equal(f.call('chooseSelection', 1, true, 'document:1'), false);
  f.update({ responseKinds: [2, 1, 0], revision: 'document:2' });
  assert.equal(f.call('chooseSelection', 1, true, 'document:1'), false);
  assert.equal(f.call('chooseSelection', 1, false, 'document:2'), true);
  assert.deepEqual(f.events, [{ name: 'selection', payload: { index: 1, selected: false, revision: 'document:2' } }]);
  f.update({ disabled: true });
  assert.equal(f.call('chooseSelection', 1, true, 'document:2'), false);
  f.update({ disabled: false, open: false });
  assert.equal(f.call('chooseSelection', 1, true, 'document:2'), false);
  f.update({ open: true }); await f.session.unmount();
  assert.equal(f.call('chooseSelection', 1, true, 'document:2'), false);
  assert.equal(f.events.length, 1);
});
