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
  prototype = (await import(pathToFileURL(path.join(repository, 'frontend/proto-ui/src/normalized-knob.proto.ts')).href)).default;
  const load = (name) => import(pathToFileURL(path.join(upstream, 'packages', name, 'src/index.ts')).href);
  ({ createRuntimeSession } = await load('runtime'));
  caps = Object.assign({}, ...await Promise.all([
    'anatomy', 'context', 'expose-state', 'expose-event', 'focus', 'event',
  ].map((name) => load(`modules/${name}`))));
}
const runtimeTest = (name, work) => test(name, { skip: !upstream && 'set PROTO_UI_UPSTREAM to the pinned checkout' }, work);
async function fixture(t, initial = {}, mount = true) {
  let props = { value: 0.5, defaultValue: 0.25, step: 0.01, label: 'Gain', disabled: false, revision: 'document:1', ...initial };
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
      const event = new CustomEvent(`host:verdant-knob-${name}`, { cancelable: true });
      target.dispatchEvent(event);
      return event;
    },
    setValue(value) {
      const event = Object.assign(new Event('host:verdant-knob-set-value'), { value });
      target.dispatchEvent(event);
      return event;
    },
    semantics: () => session.caps.getPort('a11y').getSnapshot(),
  };
}

runtimeTest('knob final up sample is proposed with its revision and rejected control stays accepted', async (t) => {
  const f = await fixture(t);
  assert.equal(f.call('begin', false, false), true);
  assert.equal(f.call('sample', 18, true, false), true);
  assert.equal(f.x.value.get(), 0.6);
  assert.equal(f.call('finish', 36, true, false), true);
  assert.deepEqual(f.commits(), [{ value: 0.7, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
});

runtimeTest('knob synchronous owner acceptance survives commit return', async (t) => {
  const f = await fixture(t);
  f.onEvent((name, payload) => { if (name === 'valueCommit') f.update({ value: payload.value, revision: 'document:2' }); });
  f.call('begin', false, false);
  f.call('finish', 90, true, true);
  assert.deepEqual(f.commits(), [{ value: 0.55, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.55);
  f.call('begin', true, false);
  assert.deepEqual(f.commits()[1], { value: 0.25, revision: 'document:2' });
  assert.equal(f.x.value.get(), 0.25);
});

runtimeTest('knob reentrant cancellation cannot overwrite newer authoritative props', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  f.onEvent((name) => { if (name === 'interactionCancel') f.update({ value: 0.9, revision: 'document:3' }); });
  f.update({ value: 0.8, revision: 'document:2' });
  assert.equal(f.x.value.get(), 0.9);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 100, true, false), false);
  assert.deepEqual(f.commits(), []);
  assert.deepEqual(f.cancels(), [{ revision: 'document:2' }]);
});

runtimeTest('knob keyboard takeover cancels preview and steps accepted truth', async (t) => {
  const f = await fixture(t, { step: 0.1 });
  f.call('begin', false, false);
  f.call('sample', 54, true, false);
  assert.equal(f.x.value.get(), 0.8);
  assert.equal(f.key('ArrowRight', { shiftKey: true }).defaultPrevented, true);
  assert.deepEqual(f.commits(), [{ value: 0.51, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.cancels().length, 1);
});

runtimeTest('knob cancel callback supersedes a suspended keyboard request', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.onEvent((name) => { if (name === 'interactionCancel') f.update({ value: 0.9, revision: 'document:2' }); });
  f.key('ArrowRight');
  assert.equal(f.x.value.get(), 0.9);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob phase observer cancellation prevents stale preview and commit', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  const unsubscribe = f.x.phase.subscribe((event) => {
    if (event.type === 'next' && f.x.phase.get() === 'dragging') f.call('cancel');
  });
  t.after(unsubscribe);
  assert.equal(f.call('sample', 54, true, false), false);
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob publication observer changes cannot leak an older commit proposal', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  let replaced = false;
  const unsubscribe = f.x.value.subscribe((event) => {
    if (event.type === 'next' && f.x.value.get() === 0.5 && !replaced) {
      replaced = true;
      f.update({ value: 0.9, revision: 'document:2' });
    }
  });
  t.after(unsubscribe);
  assert.equal(f.call('finish', 36, true, false), false);
  assert.equal(f.x.value.get(), 0.9);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob focus callbacks may disable the owner before a gesture begins', async (t) => {
  const f = await fixture(t);
  f.onFocus(() => f.update({ disabled: true }));
  assert.equal(f.call('begin', false, false), false);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.x.disabled.get(), true);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob exposed state and a11y follow label, disabled, bounds, and authoritative value', async (t) => {
  const f = await fixture(t);
  assert.equal(f.semantics().role, 'slider');
  assert.deepEqual(f.semantics().name, { kind: 'text', value: 'Gain' });
  assert.deepEqual(f.semantics().states, { valueMin: 0, valueMax: 1, valueNow: 0.5, disabled: false });
  assert.deepEqual(Object.keys(f.semantics().actions).sort(), ['decrement', 'increment', 'setValue']);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  f.update({ label: 'Cutoff' });
  assert.equal(f.x.phase.get(), 'dragging');
  assert.equal(f.x.value.get(), 0.6);
  f.update({ disabled: true, label: 'Locked', value: 2 });
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.x.value.get(), 1);
  assert.equal(f.semantics().name.value, 'Locked');
  assert.equal(f.semantics().states.disabled, true);
  assert.equal(f.semantics().states.valueNow, 1);
  f.key('Home');
  assert.equal(f.call('begin', true, false), false);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob assistive increment and decrement use authoritative value and one commit', async (t) => {
  const f = await fixture(t, { step: 0.1 });
  f.onEvent((name, payload) => { if (name === 'valueCommit') f.update({ value: payload.value, revision: `document:${f.commits().length + 1}` }); });
  f.call('begin', false, false);
  f.call('sample', 54, true, false);
  assert.equal(f.x.value.get(), 0.8);
  f.action('increment');
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.x.value.get(), 0.6);
  assert.deepEqual(f.commits(), [{ value: 0.6, revision: 'document:1' }]);
  assert.equal(f.cancels().length, 1);
  f.action('decrement');
  assert.equal(f.x.value.get(), 0.5);
  assert.deepEqual(f.commits()[1], { value: 0.5, revision: 'document:2' });
});

runtimeTest('knob assistive actions cannot outlive disabled, detached or reentrant owners', async (t) => {
  const f = await fixture(t);
  f.update({ disabled: true });
  f.action('increment');
  assert.deepEqual(f.commits(), []);
  f.update({ disabled: false });
  f.call('begin', false, false);
  f.onEvent((name) => { if (name === 'interactionCancel') f.update({ value: 0.9, revision: 'document:2' }); });
  f.action('decrement');
  assert.equal(f.x.value.get(), 0.9);
  assert.deepEqual(f.commits(), []);
  await f.session.unmount();
  f.action('increment');
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob assistive value setter cancels preview and commits one accepted proposal', async (t) => {
  const f = await fixture(t);
  f.onEvent((name, payload) => { if (name === 'valueCommit') f.update({ value: payload.value, revision: 'document:2' }); });
  f.call('begin', false, false);
  f.call('sample', 54, true, false);
  f.setValue(0.63);
  assert.deepEqual(f.commits(), [{ value: 0.63, revision: 'document:1' }]);
  assert.deepEqual(f.cancels(), [{ revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.63);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 72, true, false), false);
  assert.equal(f.commits().length, 1);
});

runtimeTest('knob assistive value rejection leaves authority intact and malformed input has no effect', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  for (const invalid of [undefined, null, '0.63', NaN, Infinity, -Infinity]) f.setValue(invalid);
  assert.equal(f.x.phase.get(), 'dragging');
  assert.equal(f.x.value.get(), 0.6);
  assert.deepEqual(f.commits(), []);
  assert.deepEqual(f.cancels(), []);
  f.setValue(0.63);
  assert.deepEqual(f.commits(), [{ value: 0.63, revision: 'document:1' }]);
  assert.equal(f.x.value.get(), 0.5);
  f.setValue(2);
  assert.deepEqual(f.commits()[1], { value: 1, revision: 'document:1' });
  assert.equal(f.x.value.get(), 0.5);
});

runtimeTest('knob assistive value cannot outlive disabled, detached or reentrant owners', async (t) => {
  const f = await fixture(t);
  f.update({ disabled: true });
  f.setValue(0.63);
  assert.deepEqual(f.commits(), []);
  f.update({ disabled: false });
  f.call('begin', false, false);
  f.onEvent((name) => { if (name === 'interactionCancel') f.update({ value: 0.9, revision: 'document:2' }); });
  f.setValue(0.63);
  assert.equal(f.x.value.get(), 0.9);
  assert.deepEqual(f.commits(), []);
  await f.session.unmount();
  f.setValue(0.63);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob observer cancellation cannot interrupt authoritative value and focus synchronization', async (t) => {
  const f = await fixture(t);
  const unsubscribe = f.x.disabled.subscribe((event) => {
    if (event.type === 'next' && f.x.disabled.get()) {
      f.call('cancel');
      assert.equal(f.call('begin', false, false), false);
      f.setValue(0.2);
    }
  });
  t.after(unsubscribe);
  f.update({ value: 0.9, revision: 'document:2', disabled: true });
  assert.equal(f.x.value.get(), 0.9);
  assert.equal(f.semantics().states.valueNow, 0.9);
  assert.equal(f.semantics().states.disabled, true);
  assert.equal(f.x.phase.get(), 'idle');
  assert.deepEqual(f.commits(), []);
  f.update({ disabled: false });
  assert.equal(f.call('begin', false, false), true);
  assert.equal(f.call('finish', 0, false, false), true);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob detach cancels exactly once and cannot accept input before remount', async (t) => {
  const f = await fixture(t, {}, false);
  assert.equal(f.call('begin', false, false), false);
  await f.session.mount();
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  await f.session.unmount();
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.cancels().length, 1);
  assert.equal(f.call('begin', false, false), false);
  await f.session.mount();
  assert.equal(f.call('begin', false, false), true);
  assert.equal(f.call('finish', 36, true, false), true);
  assert.deepEqual(f.commits(), [{ value: 0.7, revision: 'document:1' }]);
});

runtimeTest('knob disposal retires an active preview through unmount without another proposal', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  await f.session.dispose();
  assert.deepEqual(f.cancels(), [{ revision: 'document:1' }]);
  assert.deepEqual(f.commits(), []);
  // Native before-dispose additionally forbids State.set and RunHandle
  // callbacks; the explicit GPUI handle.dispose test checks that target rule.
});

runtimeTest('knob observer cancellation during detach cannot retain a provisional display', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 54, true, false);
  const unsubscribe = f.x.phase.subscribe((event) => {
    if (event.type === 'next' && f.x.phase.get() === 'idle') f.call('cancel');
  });
  t.after(unsubscribe);
  await f.session.unmount();
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.deepEqual(f.cancels(), [{ revision: 'document:1' }]);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob shortcuts clamp, reset, reject modifiers and normalize invalid step', async (t) => {
  const f = await fixture(t, { step: 0, defaultValue: 2 });
  f.key('ArrowRight');
  f.key('Home');
  f.key('End');
  f.key('PageUp');
  f.key('PageDown');
  assert.deepEqual(f.commits().map((entry) => entry.value), [0.51, 0, 1, 0.6, 0.4]);
  f.key('ArrowRight', { altKey: true });
  f.key('ArrowRight', { ctrlKey: true });
  f.key('ArrowRight', { metaKey: true });
  assert.equal(f.commits().length, 5);
  assert.equal(f.call('begin', false, true), true);
  assert.deepEqual(f.commits()[5], { value: 1, revision: 'document:1' });
  assert.equal(f.x.value.get(), 0.5);
});

runtimeTest('knob Escape and blur cancel without a commit or unaccepted value', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  assert.equal(f.key('Escape').defaultPrevented, true);
  assert.equal(f.x.value.get(), 0.5);
  f.call('begin', false, false);
  f.call('sample', 36, true, false);
  f.target.dispatchEvent(new Event('host:blur'));
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.cancels().length, 2);
  assert.deepEqual(f.commits(), []);
});

runtimeTest('knob press hover cancellation does not cancel the independent move lease', async (t) => {
  const f = await fixture(t);
  f.call('begin', false, false);
  f.call('sample', 18, true, false);
  // The pinned native press adapter emits press.cancel when a held pointer
  // leaves the root hitbox. A captured MoveGesture remains valid there; only
  // its lease cancellation calls the exposed cancel method. This tests the
  // prototype contract; the GPUI event-routing test exercises the real hitbox.
  f.target.dispatchEvent(new Event('pointer.leave'));
  f.target.dispatchEvent(new Event('press.cancel'));
  f.target.dispatchEvent(new Event('host:mouseleave'));
  assert.equal(f.x.phase.get(), 'dragging');
  assert.equal(f.call('sample', 36, true, false), true);
  assert.equal(f.call('finish', 54, true, false), true);
  assert.deepEqual(f.commits(), [{ value: 0.8, revision: 'document:1' }]);
  assert.deepEqual(f.cancels(), []);

  f.call('begin', false, false);
  f.call('sample', 36, true, false);
  assert.equal(f.call('cancel'), true);
  assert.equal(f.x.value.get(), 0.5);
  assert.equal(f.x.phase.get(), 'idle');
  assert.deepEqual(f.cancels(), [{ revision: 'document:1' }]);
  assert.equal(f.call('finish', 54, true, false), false);
  assert.equal(f.commits().length, 1);
});
