import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';

const upstream = process.env.PROTO_UI_UPSTREAM;
const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
test('vendored Base Input preserves the exact recorded official source', () => {
  const directory = path.join(repository, 'frontend/proto-ui/src/upstream-input');
  const record = JSON.parse(readFileSync(path.join(directory, 'provenance.json'), 'utf8'));
  assert.equal(record.repository, 'https://github.com/Proto-UI/Proto-UI');
  assert.match(record.revision, /^[0-9a-f]{40}$/);
  assert.deepEqual(record.files.map(file => file.path), ['root.proto.ts', 'types.ts']);
  for (const file of record.files) {
    const bytes = readFileSync(path.join(directory, file.path));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256);
    assert.equal(file.upstreamPath, `packages/prototypes/base/src/input/${file.path}`);
    if (upstream) assert.deepEqual(bytes, execFileSync('git', ['show', `${record.revision}:${file.upstreamPath}`], { cwd: upstream }));
  }
  const entry = readFileSync(path.join(repository, 'frontend/proto-ui/src/text-field.proto.ts'), 'utf8');
  assert.match(entry, /const inherited = asInputRoot\(\)/);
  assert.doesNotMatch(entry, /asInputEditor/);
});
let prototype, createRuntimeSession, caps, dom;
if (upstream) {
  const require = createRequire(path.join(upstream, 'package.json'));
  const { register } = await import(pathToFileURL(require.resolve('tsx/esm/api')).href);
  register({ tsconfig: path.join(upstream, 'tsconfig.json') });
  const { Window } = await import(pathToFileURL(require.resolve('happy-dom')).href);
  dom = new Window();
  for (const name of ['window','document','HTMLElement','HTMLInputElement','HTMLTextAreaElement','Event','CustomEvent','InputEvent','CompositionEvent']) {
    globalThis[name] = name === 'window' ? dom : name === 'document' ? dom.document : dom[name];
  }
  prototype = (await import(pathToFileURL(path.join(repository, 'frontend/proto-ui/src/text-field.proto.ts')).href)).default;
  const load = (name) => import(pathToFileURL(path.join(upstream, 'packages', name, 'src/index.ts')).href);
  ({ createRuntimeSession } = await load('runtime'));
  caps = Object.assign({}, ...await Promise.all(['anatomy','context','expose-state','expose-event','focus','event','text-control'].map(name => load(`modules/${name}`))));
}
const runtimeTest = (name, work) => test(name, { skip: !upstream && 'set PROTO_UI_UPSTREAM to the pinned compiler checkout' }, work);
async function fixture(t, initial = {}) {
  const target = document.createElement('input');
  const boundary = document.createElement('div');
  const props = { defaultValue:'a😀候', ariaLabel:'Lyric', submitOnEnter:true, cancelOnEscape:true, advanceOnTab:true, ...initial };
  let exposes;
  const events = [];
  const token = {};
  const session = createRuntimeSession(prototype, {
    prototypeName: prototype.name, getRawProps: () => props,
    schedule: work => work(), commit: (_children, signal) => signal?.done(),
    onRuntimeReady(wiring) {
      wiring.attach('anatomy', [[caps.ANATOMY_INSTANCE_TOKEN_CAP,token],[caps.ANATOMY_PARENT_CAP,()=>null],[caps.ANATOMY_GET_PROTO_CAP,()=>prototype]]);
      wiring.attach('context', [[caps.CONTEXT_INSTANCE_TOKEN_CAP,token],[caps.CONTEXT_PARENT_CAP,()=>null]]);
      wiring.attach('event', [[caps.EVENT_ROOT_TARGET_CAP,()=>boundary],[caps.EVENT_GLOBAL_TARGET_CAP,()=>window]]);
      wiring.attach('focus', [[caps.FOCUS_INSTANCE_TOKEN_CAP,token],[caps.FOCUS_PARENT_CAP,()=>null],[caps.FOCUS_ROOT_TARGET_CAP,()=>boundary],[caps.FOCUS_REQUEST_FOCUS_CAP,()=>true]]);
      wiring.attach('expose-state', [[caps.EXPOSE_STATE_SET_EXPOSES_CAP,next=>{exposes=next;}]]);
      wiring.attach('expose-event', [[caps.EXPOSE_EVENT_SINK_CAP,(name,payload)=>{events.push({name,payload});}]]);
      wiring.attach('text-control', [[caps.TEXT_CONTROL_HOST_CAP,caps.createWebTextControlHost(()=>target)],
        [caps.TEXT_CONTROL_RUN_IN_CALLBACK_CAP,work=>session.invokeInCallbackScope(work)]]);
    },
  });
  t.after(()=>session.dispose());
  await session.mount();
  return { session,target,events,get x(){return exposes;},
    call: (name,...args) => session.invokeInCallbackScope(()=>exposes[name](...args)),
    key(key,modifiers={}) { const event=new CustomEvent('key.down',{cancelable:true,detail:{key,...modifiers}});boundary.dispatchEvent(event);return event; },
    named: name=>events.filter(event=>event.name===name).map(event=>event.payload),
  };
}

runtimeTest('text source owns UTF16 selection policy and terminal requests', async t=>{
  const f=await fixture(t,{selectAllOnMount:true});
  assert.deepEqual(f.named('selectionRequest'),[{start:0,end:4,viewEpoch:1}]);
  assert.equal(f.call('selectRange',2,999),true);
  assert.deepEqual(f.named('selectionRequest').at(-1),{start:2,end:4,viewEpoch:1});
  f.key('Tab',{shiftKey:true});
  assert.deepEqual(f.named('advance'),[{value:'a😀候',forward:false,viewEpoch:1}]);
  f.key('Enter');
  assert.equal(f.named('submit').length,1);
  f.key('Escape');
  assert.equal(f.named('cancel').length,1);
});

runtimeTest('marked text cannot submit cancel advance or change selection', async t=>{
  const f=await fixture(t);
  f.target.dispatchEvent(new CompositionEvent('compositionstart',{data:'候'}));
  f.target.value='候选';
  f.target.dispatchEvent(new InputEvent('input',{data:'候选',inputType:'insertCompositionText',isComposing:true}));
  assert.equal(f.x.composing.get(),true);
  for (const method of ['requestSubmit','requestCancel','requestAdvance','selectAll']) assert.equal(f.call(method,true),false);
  for (const key of ['Enter','Escape','Tab']) assert.equal(f.key(key).defaultPrevented,true);
  for (const name of ['submit','cancel','advance','selectionRequest']) assert.deepEqual(f.named(name),[]);
  f.target.dispatchEvent(new CompositionEvent('compositionend',{data:'候选'}));
  f.key('Enter');
  assert.deepEqual(f.named('submit'),[{value:'候选',forward:true,viewEpoch:1}]);
});

runtimeTest('source replacement uses inherited lease reset and publishes no user input',async t=>{
  const f=await fixture(t);
  const count=f.events.length;
  assert.equal(f.call('resetValue','new\r\n😀'),true);
  assert.equal(f.x.value.get(),'new😀');
  assert.equal(f.target.value,'new😀');
  assert.equal(f.events.length,count);
  f.call('selectAll');
  assert.deepEqual(f.named('selectionRequest').at(-1),{start:0,end:5,viewEpoch:1});
  f.key('Enter');
  assert.equal(f.named('submit')[0].value,'new😀');
});

runtimeTest('detached text source retires requests until a new mounted view epoch',async t=>{
  const f=await fixture(t);
  await f.session.unmount();
  assert.equal(f.call('requestSubmit'),false);
  assert.equal(f.call('selectAll'),false);
  await f.session.mount();
  f.key('Enter');
  assert.deepEqual(f.named('submit'),[{value:'a😀候',forward:true,viewEpoch:3}]);
});

runtimeTest('optional numeric text policy steps bounds and clears automatic value in source',async t=>{
  const f=await fixture(t,{defaultValue:'',numeric:true,numericMin:1,numericMax:32767,numericStep:1,numericDefault:480});
  assert.equal(f.key('ArrowUp').defaultPrevented,true);
  assert.deepEqual(f.named('replacementRequest').at(-1),{value:'481',start:0,end:3,viewEpoch:1});
  f.call('resetValue','32767');
  f.key('ArrowUp');
  assert.equal(f.named('replacementRequest').at(-1).value,'32767');
  f.call('resetValue','1');
  f.key('ArrowDown');
  assert.equal(f.named('replacementRequest').at(-1).value,'1');
  f.key('Delete');
  assert.deepEqual(f.named('replacementRequest').at(-1),{value:'',start:0,end:0,viewEpoch:1});
  f.target.dispatchEvent(new CompositionEvent('compositionstart',{data:'候'}));
  const count=f.named('replacementRequest').length;
  f.key('ArrowUp');f.key('Delete');
  assert.equal(f.named('replacementRequest').length,count);
});

runtimeTest('search text delegates finite collection keys before editing defaults and keeps IME local', async t => {
  const f = await fixture(t, { defaultValue:'query', searchNavigation:true, submitOnEnter:false, cancelOnEscape:false, advanceOnTab:false });
  for (const key of ['ArrowDown','ArrowUp','Enter','Escape']) assert.equal(f.key(key).defaultPrevented,true);
  assert.equal(f.key('f',{ctrlKey:true}).defaultPrevented,true);
  assert.deepEqual(f.named('navigationRequest').map(({key,value,shift,command,viewEpoch})=>({key,value,shift,command,viewEpoch})),
    ['ArrowDown','ArrowUp','Enter','Escape','f'].map(key=>({key,value:'query',shift:false,command:key==='f',viewEpoch:1})));
  const count=f.named('navigationRequest').length;
  for (const key of ['ArrowLeft','ArrowRight','Home','End','Delete','Backspace','Tab']) assert.equal(f.key(key).defaultPrevented,false);
  assert.equal(f.key('ArrowDown',{altKey:true}).defaultPrevented,false);
  assert.equal(f.key('ArrowDown',{ctrlKey:true}).defaultPrevented,false);
  assert.equal(f.key('f',{ctrlKey:true,shiftKey:true}).defaultPrevented,false);
  assert.equal(f.named('navigationRequest').length,count);
  assert.deepEqual(f.named('submit'),[]);
  assert.deepEqual(f.named('cancel'),[]);
  f.target.dispatchEvent(new CompositionEvent('compositionstart',{data:'候'}));
  f.target.value='候选';
  f.target.dispatchEvent(new InputEvent('input',{data:'候选',inputType:'insertCompositionText',isComposing:true}));
  for (const key of ['ArrowUp','ArrowDown','Enter','Escape','Tab']) f.key(key);
  assert.equal(f.named('navigationRequest').length,count);
  f.target.dispatchEvent(new CompositionEvent('compositionend',{data:'候选'}));
  f.key('ArrowDown',{shiftKey:true});
  assert.deepEqual(f.named('navigationRequest').at(-1),{key:'ArrowDown',value:'候选',shift:true,command:false,viewEpoch:1});
  await f.session.unmount();
  f.key('Enter');
  assert.equal(f.named('navigationRequest').length,count+1);
  const normal=await fixture(t,{defaultValue:'plain',submitOnEnter:false,cancelOnEscape:false,advanceOnTab:false});
  for (const key of ['ArrowDown','ArrowUp','Enter','Escape']) assert.equal(normal.key(key).defaultPrevented,false);
  assert.deepEqual(normal.named('navigationRequest'),[]);
});
