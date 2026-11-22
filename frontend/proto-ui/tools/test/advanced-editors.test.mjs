import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';

// Actual authored prototype in the pinned portable Runtime. The native GPUI
// consumer tests remain a separate evidence layer; this is not an OS input test.
const upstream = process.env.PROTO_UI_UPSTREAM;
const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
let prototypes;
let createRuntimeSession;
let caps;
if (upstream) {
  const require = createRequire(path.join(upstream, 'package.json'));
  const { register } = await import(pathToFileURL(require.resolve('tsx/esm/api')).href);
  register({ tsconfig: path.join(upstream, 'tsconfig.json') });
  prototypes = Object.fromEntries(await Promise.all(['editor-surface','performance-key','drag-source','graph-point'].map(async (name) => [name, (await import(pathToFileURL(path.join(repository, `frontend/proto-ui/src/${name}.proto.ts`)).href)).default])));
  const load = (name) => import(pathToFileURL(path.join(upstream, 'packages', name, 'src/index.ts')).href);
  ({ createRuntimeSession } = await load('runtime'));
  caps = Object.assign({}, ...await Promise.all([
    'anatomy', 'context', 'expose-state', 'expose-event', 'focus', 'event', 'as-trigger',
  ].map((name) => load(`modules/${name}`))));
}
const runtimeTest = (name, work) => test(name, { skip: !upstream && 'set PROTO_UI_UPSTREAM to the pinned checkout' }, work);
async function fixture(t, name, initial = {}, mount = true) {
  const prototype = prototypes[name];
  let props = { note: 60, label: 'Editor', disabled: false, revision: 'document:1', threshold: 4, ...initial };
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
      wiring.attach('as-trigger', [[caps.AS_TRIGGER_INSTANCE_CAP, token], [caps.AS_TRIGGER_PARENT_CAP, () => null], [caps.AS_TRIGGER_GET_PROTO_CAP, () => prototype], [caps.AS_TRIGGER_GET_GROUP_EVENT_TARGET_CAP, () => target]]);
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
    presses: () => events.filter((event) => event.name === 'pressedChange').map((event) => event.payload),
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
      const event = new CustomEvent(`host:verdant-note-${name}`, { cancelable: true });
      target.dispatchEvent(event);
      return event;
    },
    event(name) { target.dispatchEvent(new Event(name)); },
    semantics: () => session.caps.getPort('a11y').getSnapshot(),
  };
}


runtimeTest('editor source admits only its live contact and threshold, then settles once', async (t) => {
  const f = await fixture(t, 'editor-surface', {}, false);
  assert.equal(f.call('begin', 'Mouse', 0, 0), false);
  await f.session.mount();
  assert.equal(f.call('begin', 'Mouse', 0, 0), true);
  assert.equal(f.call('begin', 'Touch(0)', 0, 0), false);
  assert.equal(f.call('sample', 'Touch(0)', 10, 0), false);
  assert.equal(f.call('sample', 'Mouse', 3, 0), false);
  assert.equal(f.x.phase.get(), 'pressed');
  assert.equal(f.call('sample', 'Mouse', 3, 4), true);
  assert.equal(f.x.phase.get(), 'dragging');
  assert.equal(f.call('finish', 'Touch(0)', 3, 4), false);
  assert.equal(f.call('finish', 'Mouse', 5, 8), true);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 'Mouse', 5, 8), false);
  assert.deepEqual(f.cancels(), []);
});

runtimeTest('editor cancellation identifies the retired contact and authority, never its replacement', async t => {
  const f = await fixture(t, 'editor-surface');
  f.call('begin', 'Mouse', 0, 0);
  const first = f.x.lease.get();
  assert.ok(first > 0);
  f.update({ revision: 'document:2' });
  assert.deepEqual(f.cancels(), [{ pointer: 'Mouse', revision: 'document:1', lease: first }]);
  assert.equal(f.x.lease.get(), 0);
  f.call('begin', 'Mouse', 0, 0);
  const second = f.x.lease.get();
  assert.ok(second > first);
  f.call('cancel');
  assert.deepEqual(f.cancels().at(-1), { pointer: 'Mouse', revision: 'document:2', lease: second });
});

runtimeTest('audio keyboard semantics share editing authority, cancellation and lifecycle', async t => {
  const f = await fixture(t, 'editor-surface', { commands: 'audio' });
  assert.equal(f.call('audioKey', 'Delete', false, false, false, true, 'document:1'), '');
  assert.equal(f.call('audioKey', 'Delete', false, false, false, false, 'stale'), '');
  assert.equal(f.call('audioKey', 'r', false, false, true, false, 'document:1'), '');
  f.call('begin', 'Mouse', 0, 0);
  assert.equal(f.call('audioKey', 'Delete', true, false, false, false, 'document:1'), 'ripple-delete');
  assert.equal(f.x.phase.get(), 'idle'); assert.equal(f.cancels().length, 1);
  for (const [key, secondary, command] of [['i', true, 'split'], ['a', true, 'select-all'], ['j', true, 'join'], ['v', false, 'select-tool'], ['r', false, 'range-tool'], ['Escape', false, 'escape']]) {
    assert.equal(f.call('audioKey', key, false, secondary, false, false, 'document:1'), command);
  }
  f.onEvent(name => { if (name === 'interactionCancel') f.update({ revision: 'new-document', disabled: true }); });
  f.call('begin', 'Mouse', 0, 0);
  assert.equal(f.call('audioKey', 'Escape', false, false, false, false, 'document:1'), '');
  await f.session.unmount();
  assert.equal(f.call('audioKey', 'a', false, true, false, false, 'new-document'), '');
});

runtimeTest('editor activation stops when cancellation observers acquire newer authority', async t => {
  const f = await fixture(t, 'editor-surface');
  f.call('begin', 'Mouse', 0, 0);
  f.onEvent(name => { if (name === 'interactionCancel') f.update({ revision: 'document:2', disabled: true }); });
  assert.equal(f.call('activate'), false);
  assert.equal(f.x.phase.get(), 'idle');
});

runtimeTest('editor nonfinite samples, Escape, blur and detach retire contacts without terminal acceptance', async (t) => {
  const f = await fixture(t, 'editor-surface');
  assert.equal(f.call('begin', 'Mouse', NaN, 0), false);
  f.call('begin', 'Mouse', 0, 0);
  assert.equal(f.call('sample', 'Mouse', Infinity, 0), false);
  assert.equal(f.x.phase.get(), 'idle');
  f.call('begin', 'Mouse', 0, 0);
  assert.equal(f.key('Escape').defaultPrevented, true);
  f.call('begin', 'Mouse', 0, 0);
  f.event('host:blur');
  f.call('begin', 'Mouse', 0, 0);
  await f.session.unmount();
  assert.equal(f.call('finish', 'Mouse', 10, 10), false);
  assert.equal(f.call('begin', 'Mouse', 0, 0), false);
  assert.equal(f.x.pointer.get(), '');
  assert.equal(f.cancels().length, 4);
});

runtimeTest('editor authority publication rejects stale work and survives reentrant replacement', async (t) => {
  const f = await fixture(t, 'editor-surface');
  f.call('begin', 'Mouse', 0, 0);
  f.onEvent((name) => { if (name === 'interactionCancel') f.update({ revision: 'document:3', label: 'New owner', disabled: true }); });
  f.update({ revision: 'document:2', label: 'Superseded' });
  assert.equal(f.semantics().name.value, 'New owner');
  assert.equal(f.semantics().states.disabled, true);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 'Mouse', 10, 10), false);
  assert.equal(f.call('begin', 'Mouse', 0, 0), false);
  assert.equal(f.cancels().length, 1);
});

runtimeTest('editor focus and phase observers cannot continue suspended admission', async (t) => {
  const f = await fixture(t, 'editor-surface');
  f.onFocus(() => f.update({ disabled: true }));
  assert.equal(f.call('begin', 'Mouse', 0, 0), false);
  assert.equal(f.x.phase.get(), 'idle');
  f.onFocus(() => {});
  f.update({ disabled: false });
  f.call('begin', 'Mouse', 0, 0);
  const unsubscribe = f.x.phase.subscribe((event) => {
    if (event.type === 'next' && f.x.phase.get() === 'dragging') f.call('cancel');
  });
  t.after(unsubscribe);
  assert.equal(f.call('sample', 'Mouse', 30, 40), false);
  assert.equal(f.x.phase.get(), 'idle');
  assert.equal(f.call('finish', 'Mouse', 30, 40), false);
});

runtimeTest('editor wheel modifier priority, directional policy and pinch admission are source-owned', async (t) => {
  const f = await fixture(t, 'editor-surface');
  assert.deepEqual(f.call('wheel', 2, 3, false, false, false, false), {kind:'pan',x:-2,y:-3,amount:0});
  assert.deepEqual(f.call('wheel', 0, 3, false, true, false, true), {kind:'pan',x:-3,y:0,amount:0});
  assert.equal(f.call('wheel', 2, 3, true, true, true, false).kind, 'zoom-x');
  assert.equal(f.call('wheel', 2, 3, false, false, true, false).kind, 'zoom-y');
  assert.equal(f.call('wheel', 0, 3, false, false, false, true).kind, 'none');
  assert.equal(f.call('wheel', NaN, 3, false, false, false, false).kind, 'none');
  assert.equal(f.call('pinch', -1), false);
  assert.equal(f.call('pinch', 1.2), true);
  f.update({disabled:true});
  assert.equal(f.call('pinch', 1.2), false);
  assert.equal(f.call('wheel', 2, 3, true, false, false, false).kind, 'none');
});

runtimeTest('performance source suppresses repeat and keeps independent pointer and typing contacts', async (t) => {
  const f = await fixture(t, 'performance-key');
  assert.equal(f.call('keyDown', 's'), false);
  assert.equal(f.call('keyDown', 'a'), true);
  assert.equal(f.call('keyDown', 'a'), true);
  assert.equal(f.presses().length, 1);
  f.event('press.start');
  f.call('keyUp', 'a');
  assert.equal(f.x.pressed.get(), true);
  f.event('press.end');
  assert.equal(f.x.pressed.get(), false);
  assert.deepEqual(f.presses().map(x => x.pressed), [true, false]);
  assert.equal(f.semantics().role, 'button');
  assert.equal(f.semantics().states.pressed, false);
});

runtimeTest('performance source cancels on owner replacement, disable, blur, Escape and detach', async (t) => {
  const f = await fixture(t, 'performance-key');
  f.call('keyDown', 'a');
  f.update({revision:'document:2'});
  assert.equal(f.x.pressed.get(), false);
  f.call('keyDown', 'a');
  f.event('host:blur');
  f.call('keyDown', 'a');
  assert.equal(f.key('Escape').defaultPrevented, true);
  f.call('keyDown', 'a');
  f.update({disabled:true});
  assert.equal(f.call('keyDown', 'a'), false);
  f.update({disabled:false});
  f.call('keyDown', 'a');
  await f.session.unmount();
  assert.equal(f.x.pressed.get(), false);
  assert.equal(f.call('keyDown', 'a'), false);
  assert.deepEqual(f.presses().map(x => x.pressed), [true,false,true,false,true,false,true,false,true,false]);
});

runtimeTest('performance publication cannot leak a stale note-on after a reentrant policy change', async (t) => {
  const f = await fixture(t, 'performance-key');
  const unsubscribe = f.x.pressed.subscribe((event) => {
    if(event.type === 'next' && f.x.pressed.get()) f.update({disabled:true,revision:'document:2'});
  });
  t.after(unsubscribe);
  f.call('keyDown', 'a');
  assert.equal(f.x.pressed.get(), false);
  assert.deepEqual(f.presses().map(x=>x.pressed), [false]);
});

runtimeTest('performance assistive activation toggles a sustained contact or triggers a momentary audition', async (t) => {
  const f = await fixture(t, 'performance-key');
  f.action('toggle');
  assert.equal(f.x.pressed.get(), true);
  f.action('toggle');
  assert.equal(f.x.pressed.get(), false);
  f.update({momentary:true});
  f.action('toggle');
  assert.equal(f.x.pressed.get(), false);
  assert.equal(f.events.filter(e=>e.name==='trigger').length, 1);
  f.update({disabled:true});
  f.action('toggle');
  assert.equal(f.events.filter(e=>e.name==='trigger').length, 1);
});


runtimeTest('transfer source leases are single-use, scoped to authority and cannot accept after cancellation', async (t) => {
  const f = await fixture(t, 'drag-source');
  const first = f.call('begin');
  assert.ok(first > 0);
  assert.equal(f.x.phase.get(), 'dragging');
  assert.equal(f.call('begin'), 0);
  assert.equal(f.call('accept', first + 1, 'document:1'), false);
  assert.equal(f.call('accept', first, 'document:0'), false);
  assert.equal(f.call('accept', first, 'document:1'), true);
  assert.equal(f.call('accept', first, 'document:1'), false);
  const second = f.call('begin');
  assert.ok(second > first);
  assert.equal(f.call('cancel', first), false, 'late transport Drop must preserve the replacement');
  assert.equal(f.x.phase.get(), 'dragging');
  f.call('cancel', second);
  assert.equal(f.call('accept', second, 'document:1'), false);
});

runtimeTest('transfer source retires replaced, disabled and detached authorities', async (t) => {
  const f = await fixture(t, 'drag-source');
  const first = f.call('begin');
  f.update({revision:'document:2'});
  assert.equal(f.call('accept', first, 'document:1'), false);
  const second = f.call('begin');
  f.update({disabled:true});
  assert.equal(f.call('accept', second, 'document:2'), false);
  assert.equal(f.call('begin'), 0);
  f.update({disabled:false});
  const third = f.call('begin');
  await f.session.unmount();
  assert.equal(f.call('accept', third, 'document:2'), false);
  assert.equal(f.call('begin'), 0);
  assert.deepEqual(f.events.filter(e=>e.name==='cancelled').map(e=>e.payload.lease), [first,second,third]);
});

runtimeTest('transfer keyboard and assistive activation select a target while drag Escape retires ownership', async (t) => {
  const f = await fixture(t, 'drag-source');
  f.event('host:verdant-drag-choose');
  f.event('press.commit');
  assert.equal(f.events.filter(e=>e.name==='choose').length, 2);
  const token=f.call('begin');
  f.event('host:verdant-drag-choose');
  assert.equal(f.events.filter(e=>e.name==='choose').length, 2);
  assert.equal(f.key('Escape').defaultPrevented, true);
  assert.equal(f.call('accept', token, 'document:1'), false);
  assert.equal(f.semantics().role, 'button');
  assert.equal(f.semantics().actions.activate.event, 'host:verdant-drag-choose');
});


runtimeTest('graph point owns two-axis threshold, fine mode and atomic final-up proposal', async (t) => {
  const f=await fixture(t,'graph-point',{x:0.5,y:0.5});
  assert.equal(f.call('begin',false,false),true);
  assert.equal(f.call('sample',1,1,100,100,false),true);
  assert.equal(f.x.phase.get(),'pressed');
  assert.equal(f.x.x.get(),0.5);
  f.call('sample',20,-30,100,100,false);
  assert.equal(f.x.x.get(),0.7);
  assert.equal(f.x.y.get(),0.8);
  f.call('finish',40,-50,100,100,true);
  assert.deepEqual(f.events.filter(e=>e.name==='valueCommit').map(e=>e.payload),[{x:0.54,y:0.55,reset:false,revision:'document:1'}]);
  assert.equal(f.x.phase.get(),'idle');
  assert.equal(f.x.x.get(),0.5);
  assert.equal(f.x.y.get(),0.5);
  assert.equal(f.call('finish',40,-50,100,100,true),false);
});

runtimeTest('graph clicks select without writes, reset proposes once and boundaries follow supplied axes', async (t) => {
  const f=await fixture(t,'graph-point',{x:0.5,y:0.5,minX:0.2,maxX:0.7,minY:0.1,maxY:0.9});
  f.call('begin',false,false);f.call('finish',1,1,100,100,false);
  assert.equal(f.events.filter(e=>e.name==='selected').length,1);
  assert.equal(f.events.filter(e=>e.name==='valueCommit').length,0);
  f.call('begin',true,false);
  assert.equal(f.events.filter(e=>e.name==='valueCommit')[0].payload.reset,true);
  f.call('begin',false,false);f.call('finish',100,-100,100,100,false);
  assert.deepEqual(f.events.filter(e=>e.name==='valueCommit')[1].payload,{x:0.7,y:0.9,reset:false,revision:'document:1'});
});

runtimeTest('graph keyboard and assistive routes cancel preview and use accepted atomic coordinates', async (t) => {
  const f=await fixture(t,'graph-point',{x:0.4,y:0.6});
  f.call('begin',false,false);f.call('sample',30,-20,100,100,false);
  assert.equal(f.key('ArrowRight',{shiftKey:true}).defaultPrevented,true);
  assert.equal(f.x.phase.get(),'idle');
  assert.deepEqual(f.events.filter(e=>e.name==='valueCommit').at(-1).payload,{x:0.401,y:0.6,reset:false,revision:'document:1'});
  assert.equal(f.x.axis.get(),'x');
  f.event('host:verdant-point-axis');
  assert.equal(f.x.axis.get(),'y');
  f.event('host:verdant-point-increment');
  assert.deepEqual(f.events.filter(e=>e.name==='valueCommit').at(-1).payload,{x:0.4,y:0.61,reset:false,revision:'document:1'});
  f.target.dispatchEvent(Object.assign(new Event('host:verdant-point-set-value'),{value:0.25}));
  assert.deepEqual(f.events.filter(e=>e.name==='valueCommit').at(-1).payload,{x:0.4,y:0.25,reset:false,revision:'document:1'});
  assert.equal(f.semantics().role,'slider');
  assert.deepEqual(Object.keys(f.semantics().actions).sort(),['activate','decrement','increment','setValue']);
});

runtimeTest('graph authority and invalid samples retire a preview and prevent stale commits', async (t) => {
  const f=await fixture(t,'graph-point');
  f.call('begin',false,false);f.call('sample',30,-20,100,100,false);
  f.update({x:0.8,revision:'document:2'});
  assert.equal(f.x.phase.get(),'idle');assert.equal(f.x.x.get(),0.8);
  assert.equal(f.call('finish',30,-20,100,100,false),false);
  f.call('begin',false,false);
  assert.equal(f.call('finish',NaN,3,100,100,false),false);
  assert.equal(f.events.filter(e=>e.name==='valueCommit').length,0);
  f.call('begin',false,false);
  await f.session.unmount();
  assert.equal(f.x.phase.get(),'idle');
  assert.equal(f.call('begin',false,false),false);
  assert.equal(f.events.filter(e=>e.name==='valueCommit').length,0);
});

runtimeTest('graph state observers and synchronous acceptance cannot leak an old controlled proposal', async (t) => {
  const f=await fixture(t,'graph-point');
  f.onEvent((name,payload)=>{if(name==='valueCommit')f.update({x:payload.x,y:payload.y,revision:'document:2'});});
  f.call('begin',false,false);f.call('finish',20,-30,100,100,false);
  assert.equal(f.x.x.get(),0.7);assert.equal(f.x.y.get(),0.8);
  const unsub=f.x.phase.subscribe(event=>{if(event.type==='next'&&f.x.phase.get()==='dragging')f.update({disabled:true,x:0.1,y:0.2,revision:'document:3'});});
  t.after(unsub);
  f.call('begin',false,false);
  assert.equal(f.call('finish',20,-30,100,100,false),false);
  assert.equal(f.x.x.get(),0.1);assert.equal(f.x.y.get(),0.2);
  assert.equal(f.events.filter(e=>e.name==='valueCommit').length,1);
  assert.equal(f.semantics().states.disabled,true);
});

runtimeTest('graph source owns contextual and precise-entry requests under current authority', async (t) => {
  const f=await fixture(t,'graph-point',{hasX:false,hasY:true});
  assert.equal(f.call('openContext'),true);
  f.key('Enter');
  assert.deepEqual(f.events.filter(e=>e.name==='exact').map(e=>e.payload),[{axis:'y',revision:'document:1'}]);
  assert.equal(f.events.filter(e=>e.name==='context').length,1);
  f.update({disabled:true});
  assert.equal(f.call('openContext'),false);f.key('Enter');
  assert.equal(f.events.filter(e=>e.name==='exact').length,1);
});
