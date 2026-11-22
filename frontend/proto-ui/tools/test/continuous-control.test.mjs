import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';

// Actual authored prototype in the pinned portable Runtime. The native GPUI
// consumer tests remain a separate evidence layer; this is not an OS input test.
const upstream = process.env.PROTO_UI_UPSTREAM;
const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
let prototype;
let createRuntimeSession;
let caps;
if (upstream) {
  const require = createRequire(path.join(upstream, 'package.json'));
  const { register } = await import(pathToFileURL(require.resolve('tsx/esm/api')).href);
  register({ tsconfig: path.join(upstream, 'tsconfig.json') });
  prototype = (await import(pathToFileURL(path.join(repository, 'frontend/proto-ui/src/continuous-control.proto.ts')).href)).default;
  const load = (name) => import(pathToFileURL(path.join(upstream, 'packages', name, 'src/index.ts')).href);
  ({ createRuntimeSession } = await load('runtime'));
  caps = Object.assign({}, ...await Promise.all([
    'anatomy', 'context', 'expose-state', 'expose-event', 'focus', 'event',
  ].map((name) => load(`modules/${name}`))));
}
const runtimeTest = (name, work) => test(name, { skip: !upstream && 'set PROTO_UI_UPSTREAM to the pinned checkout' }, work);
async function fixture(t, initial = {}, mount = true) {
  let props = { value: 0.5, min: 0, max: 1, defaultValue: 0.25, step: 0.01, fineStep: 0.001, label: 'Gain', disabled: false, revision: 'document:1', ...initial };
  let exposes;
  let onEvent = () => {};
  let onFocus = () => {};
  const events = [];
  const token = {};
  const target = new EventTarget();
  const globalTarget = new EventTarget();
  const session = createRuntimeSession(prototype, {
    prototypeName: prototype.name,
    getRawProps: () => props,
    schedule: (work) => work(),
    commit: (_children, signal) => signal?.done(),
    onRuntimeReady(wiring) {
      wiring.attach('anatomy', [
        [caps.ANATOMY_INSTANCE_TOKEN_CAP, token],
        [caps.ANATOMY_PARENT_CAP, () => null],
        [caps.ANATOMY_GET_PROTO_CAP, () => prototype],
      ]);
      wiring.attach('context', [
        [caps.CONTEXT_INSTANCE_TOKEN_CAP, token],
        [caps.CONTEXT_PARENT_CAP, () => null],
      ]);
      wiring.attach('event', [
        [caps.EVENT_ROOT_TARGET_CAP, () => target],
        [caps.EVENT_GLOBAL_TARGET_CAP, () => globalTarget],
      ]);
      wiring.attach('focus', [
        [caps.FOCUS_INSTANCE_TOKEN_CAP, token],
        [caps.FOCUS_PARENT_CAP, () => null],
        [caps.FOCUS_ROOT_TARGET_CAP, () => target],
        [caps.FOCUS_REQUEST_FOCUS_CAP, () => { onFocus(); return true; }],
      ]);
      wiring.attach('expose-state', [[caps.EXPOSE_STATE_SET_EXPOSES_CAP, (next) => { exposes = next; }]]);
      wiring.attach('expose-event', [[caps.EXPOSE_EVENT_SINK_CAP, (name, payload) => {
        events.push({ name, payload });
        onEvent(name, payload);
      }]]);
    },
  });
  t.after(() => session.dispose());
  if (mount) await session.mount();
  return {
    session, target, events,
    get x() { return exposes; },
    commits: () => events.filter((event) => event.name === 'valueCommit').map((event) => event.payload),
    cancels: () => events.filter((event) => event.name === 'interactionCancel').map((event) => event.payload),
    call: (name, ...args) => session.invokeInCallbackScope(() => exposes[name](...args)),
    update(patch) {
      props = { ...props, ...patch };
      session.controller.applyRawProps(props);
    },
    onEvent(callback) { onEvent = callback; },
    onFocus(callback) { onFocus = callback; },
    key(key, modifiers = {}) {
      const event = new CustomEvent('key.down', { cancelable: true, detail: { key, ...modifiers } });
      target.dispatchEvent(event);
      return event;
    },
    action(name) {
      const event = new CustomEvent(`host:verdant-scalar-${name}`, { cancelable: true });
      target.dispatchEvent(event);
      return event;
    },
    setValue(value) {
      const event = Object.assign(new Event('host:verdant-scalar-set-value'), { value });
      target.dispatchEvent(event);
      return event;
    },
    semantics: () => session.caps.getPort('a11y').getSnapshot(),
  };
}

runtimeTest('continuous final sample commits once and controlled rejection restores accepted value', async (t) => {
  const f = await fixture(t);
  assert.equal(f.call('begin', 0.5, false, false, false), true);
  assert.equal(f.call('sample', 0.6, 0.51, true, false), true);
  assert.equal(f.x.value.get(), 0.6);
  assert.equal(f.call('finish', 0.8, 0.53, true, false), true);
  assert.deepEqual(f.commits(), [{ value: 0.8, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
});

runtimeTest('continuous cancellation carries the retired accepted value and original revision', async t => {
  const f = await fixture(t);
  f.call('begin', 0.5, false, false, false);
  f.call('sample', 0.7, 0.52, true, false);
  f.update({ value: 0.2, revision: 'document:2' });
  assert.deepEqual(f.cancels(), [{ value: 0.5, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.2);
  assert.equal(f.call('finish', 0.9, 0.53, true, false), false);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('continuous preview echo cannot mask a synchronous policy change during finish', async t => {
  for (const policy of [{ max: 2 }, { disabled: true }, { step: 0.2 }, { dragMode: 'relative' }]) {
    const f = await fixture(t, { previewEcho: true });
    f.call('begin', 0.5, false, false, false);
    f.onEvent((name, payload) => { if (name === 'valuePreview') f.update({ value: payload.value, ...policy }); });
    assert.equal(f.call('finish', 0.8, 0.53, true, false), false);
    assert.deepEqual(f.commits(), []);
    assert.equal(f.x.phase.get(), 'idle');
    assert.deepEqual(f.cancels(), [{ value: 0.5, revision: 'document:1' }]);
  }
});

runtimeTest('continuous plus minus and fine keys use the same authored numeric proposal', async t => {
  const f = await fixture(t);
  assert.equal(f.key('=').defaultPrevented, true);
  assert.equal(f.key('+', { shiftKey: true }).defaultPrevented, true);
  assert.equal(f.key('-').defaultPrevented, true);
  assert.deepEqual(f.commits().map(proposal => proposal.value), [0.51, 0.501, 0.49]);
  assert.equal(f.key('+', { ctrlKey: true }).defaultPrevented, false);
});

runtimeTest('continuous synchronous draft echoes cannot swallow the final release commit', async (t) => {
  const f = await fixture(t, { previewEcho: true });
  f.onEvent((name, payload) => {
    if (name === 'valuePreview') f.update({ value: payload.value });
    if (name === 'valueCommit') f.update({ value: payload.value, revision: 'document:2' });
  });
  f.call('begin', 0.5, false, false, false);
  assert.equal(f.call('sample', 0.6, 0.51, true, false), true);
  assert.equal(f.call('finish', 0.7, 0.52, true, false), true);
  assert.deepEqual(f.commits(), [{ value: 0.7, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.7);
});

runtimeTest('continuous numeric scrubbing owns rounding, physical steps and fine steps', async (t) => {
  const f = await fixture(t, { value: 120, min: 40, max: 300, step: 1, fineStep: 0.1, dragMode: 'steps', role: 'spinbutton' });
  f.call('begin', 0, false, false, false);
  f.call('sample', 2.5, 2.5, true, false);
  assert.equal(f.x.value.get(), 123);
  f.call('finish', -2.5, -2.5, true, true);
  assert.deepEqual(f.commits(), [{ value: 119.8, revision: 'document:1' }]);
  assert.equal(f.semantics().role, 'spinbutton');
});

runtimeTest('continuous keyboard and assistive takeovers cancel preview and use accepted physical units', async (t) => {
  const f = await fixture(t, { value: -8, min: -96, max: 6, step: 0.5, fineStep: 0.1 });
  f.call('begin', -8, false, false, false);
  f.call('sample', 2, -7, true, false);
  f.key('ArrowUp');
  assert.deepEqual(f.commits(), [{ value: -7.5, revision: 'document:1' }]);
  assert.equal(f.cancels().length, 1);
  f.action('decrement');
  assert.equal(f.commits()[1].value, -8.5);
  f.setValue(2.125);
  assert.equal(f.commits()[2].value, 2.125);
  f.setValue(Infinity);
  assert.equal(f.commits().length, 3);
});

runtimeTest('continuous authority change, invalid input and detach cannot commit stale preview', async (t) => {
  const f = await fixture(t);
  f.call('begin', 0.5, false, false, false);
  f.call('sample', 0.9, 0.54, true, false);
  f.update({ value: 0.4, revision: 'document:2' });
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 1, 0.6, true, false), false);
  assert.equal(f.x.value.get(), 0.4);
  f.call('begin', 0.4, false, false, false);
  assert.equal(f.call('sample', NaN, 0.4, true, false), false);
  assert.equal(f.x.phase.get(), 'idle');
  f.call('begin', 0.4, false, false, false);
  await f.session.unmount();
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 0.8, 0.44, true, false), false);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('continuous resize boundaries, reset and stepper share one source proposal', async (t) => {
  const f = await fixture(t, { value: 300, min: 260, max: 440, defaultValue: 300, step: 8, fineStep: 1, role: 'separator' });
  f.call('adjust', 1, false);
  f.key('End');
  f.key('Home');
  f.call('reset');
  assert.deepEqual(f.commits().map((p) => p.value), [308, 440, 260, 300]);
  f.call('begin', 300, false, false, false);
  f.call('sample', 400, 310, true, false);
  f.update({ max: 350 });
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.x.value.get(), 300);
});

runtimeTest('continuous double edit and tap activation preserve value without a commit', async (t) => {
  const f = await fixture(t, { doubleAction: 'edit', tapAction: 'activate' });
  f.call('begin', 0.5, false, false, true);
  assert.equal(f.events.at(-1).name, 'editRequest');
  f.call('begin', 0.5, false, false, false);
  f.call('finish', 0.5, 0.5, false, false);
  assert.equal(f.events.at(-1).name, 'activate');
  assert.deepEqual(f.commits(), []);
});

runtimeTest('continuous disabled instance never starts or requests a host write', async (t) => {
  const f = await fixture(t, { disabled: true });
  assert.equal(f.call('begin', 0.5, true, false, false), false);
  assert.equal(f.call('adjust', 1, false), false);
  f.key('ArrowUp'); f.action('increment'); f.setValue(0.7);
  assert.deepEqual(f.events, []);
});
