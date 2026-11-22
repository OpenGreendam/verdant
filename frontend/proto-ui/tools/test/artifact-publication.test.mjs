import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFile, spawn } from 'node:child_process';
import { existsSync, lstatSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { promisify } from 'node:util';
import { setTimeout } from 'node:timers/promises';
import { acquireProducerLock, assertOwnedInventory, publishBatch, recoverPublication } from '../lib/artifact-publication.mjs';

const hash = (value) => createHash('sha256').update(value).digest('hex');
const stateDirectory = (lock) => lstatSync(lock).isDirectory()
  ? lock : `${lock}.state-${JSON.parse(readFileSync(lock, 'utf8')).id}`;
const moduleUrl = new URL('../lib/artifact-publication.mjs', import.meta.url).href;
function childProcess(t, script) {
  const child = spawn(process.execPath, ['--input-type=module', '-e', script]);
  t.after(() => child.kill());
  let output = '';
  child.stdout.on('data', (data) => { output += data; });
  child.stderr.on('data', (data) => { output += data; });
  const completion = new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (code, signal) => resolve({ code, signal, output }));
  });
  return { child, completion };
}
async function waitForFile(filename) {
  for (let attempt = 0; !existsSync(filename) && attempt < 400; attempt += 1) await setTimeout(5);
  assert.equal(existsSync(filename), true, `child reached ${path.basename(filename)}`);
}
function fixture(t) {
  const root = mkdtempSync(path.join(os.tmpdir(), 'verdant-publication-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const lockDirectory = path.join(root, '.producer.lock');
  const unit = (name, contents) => {
    const artifacts = [{ path: 'Component.rs', kind: 'source', contents }];
    return { outDirectory: path.join(root, name), receiptPath: path.join(root, `${name}.json`), artifacts, receipt: { artifacts: artifacts.map((artifact) => ({ path: artifact.path, kind: artifact.kind, sha256: hash(artifact.contents) })) } };
  };
  const options = { root, lockDirectory };
  const release = acquireProducerLock(lockDirectory);
  t.after(release);
  return { root, lockDirectory, unit, options, release };
}

test('publication owns the exact receipt inventory and preserves extra consumer files', (t) => {
  const { unit, options } = fixture(t);
  const original = unit('one', 'old');
  publishBatch([original], options);
  writeFileSync(path.join(original.outDirectory, 'human-notes.md'), 'keep me');
  assert.throws(() => publishBatch([unit('one', 'new')], options), /compiler-owned inventory/);
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'old');
  assert.equal(readFileSync(path.join(original.outDirectory, 'human-notes.md'), 'utf8'), 'keep me');
});

test('modified generated files and symlinks are never overwritten', (t) => {
  const { root, unit, options } = fixture(t);
  const original = unit('one', 'old');
  publishBatch([original], options);
  writeFileSync(path.join(original.outDirectory, 'Component.rs'), 'manual edit');
  assert.throws(() => publishBatch([unit('one', 'new')], options), /modified compiler artifact/);
  rmSync(path.join(original.outDirectory, 'Component.rs'));
  writeFileSync(path.join(root, 'outside'), 'old');
  symlinkSync(path.join(root, 'outside'), path.join(original.outDirectory, 'Component.rs'));
  assert.throws(() => assertOwnedInventory(original.outDirectory, original.receiptPath), /Symbolic artifact/);
  assert.equal(readFileSync(path.join(root, 'outside'), 'utf8'), 'old');
});

test('an incomplete multi-unit replacement rolls all artifacts and receipts back', (t) => {
  const { root, unit, options, lockDirectory } = fixture(t);
  const original = [unit('one', 'old one'), unit('two', 'old two')];
  publishBatch(original, options);
  const receipts = original.map((entry) => readFileSync(entry.receiptPath, 'utf8'));
  let renames = 0;
  assert.throws(() => publishBatch([unit('one', 'new one'), unit('two', 'new two')], {
    ...options,
    rename(source, target) {
      if (++renames === 6) throw new Error('simulated disk failure');
      renameSync(source, target);
    },
  }), /simulated disk failure/);
  original.forEach((entry, index) => {
    assert.equal(readFileSync(path.join(entry.outDirectory, 'Component.rs'), 'utf8'), entry.artifacts[0].contents);
    assert.equal(readFileSync(entry.receiptPath, 'utf8'), receipts[index]);
  });
  assert.equal(existsSync(path.join(stateDirectory(lockDirectory), 'journal.json')), false);
  assert.equal(readdirSync(root).some((filename) => filename.includes('.producer-')), false);
});

test('the lock rejects concurrent publishers and cannot recover a running owner', (t) => {
  const { root, lockDirectory } = fixture(t);
  assert.throws(() => acquireProducerLock(lockDirectory), /producer is locked/);
  assert.throws(() => recoverPublication(lockDirectory, root), /still running/);
});

test('interrupted publication recovery restores backups before releasing its lock', (t) => {
  const { root, lockDirectory, unit, options, release } = fixture(t);
  const original = unit('one', 'old');
  publishBatch([original], options);
  release();
  mkdirSync(lockDirectory);
  // A dead PID is checked rather than trusting the presence of a journal.
  writeFileSync(path.join(lockDirectory, 'owner.json'), JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const backup = `${original.outDirectory}.producer-interrupted.backup`;
  const staging = `${original.outDirectory}.producer-interrupted.staging`;
  renameSync(original.outDirectory, backup);
  mkdirSync(original.outDirectory);
  writeFileSync(path.join(original.outDirectory, 'Component.rs'), 'partly published');
  writeFileSync(path.join(lockDirectory, 'journal.json'), JSON.stringify({ schema: 'verdant.proto-ui.publication.v1', committed: false, replacements: [{ target: 'one', staging: path.basename(staging), backup: path.basename(backup), existed: true }] }));
  assert.equal(recoverPublication(lockDirectory, root), true);
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'old');
  assert.equal(existsSync(lockDirectory), false);
});

test('invalid emitted paths cannot escape unique staging or replace old output', (t) => {
  const { root, unit, options } = fixture(t);
  const original = unit('one', 'old');
  publishBatch([original], options);
  const replacement = unit('one', 'new');
  replacement.artifacts.push({ path: '../escape', kind: 'source', contents: 'bad' });
  assert.throws(() => publishBatch([replacement], options), /escapes its declared root/);
  assert.equal(existsSync(path.join(root, 'escape')), false);
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'old');
  assert.equal(readdirSync(root).some((filename) => filename.includes('.producer-')), false);
});

test('publication rejects symlink ancestors for outputs and receipts', (t) => {
  const { root, unit, options } = fixture(t);
  const outside = mkdtempSync(path.join(os.tmpdir(), 'verdant-outside-'));
  t.after(() => rmSync(outside, { recursive: true, force: true }));
  symlinkSync(outside, path.join(root, 'generated'));
  const unsafe = unit('generated/one', 'must remain inside');
  assert.throws(() => publishBatch([unsafe], options), /Symbolic compiler-owned path/);
  assert.deepEqual(readdirSync(outside), []);
  const receipt = unit('safe', 'inside');
  receipt.receiptPath = path.join(root, 'generated/receipt.json');
  assert.throws(() => publishBatch([receipt], options), /Symbolic compiler-owned path/);
  assert.deepEqual(readdirSync(outside), []);
});

test('staging interruptions are journaled and recovery removes partial staging', (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  mkdirSync(lockDirectory);
  writeFileSync(path.join(lockDirectory, 'owner.json'), JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const staging = path.join(root, 'one.producer-interrupted.staging');
  mkdirSync(staging);
  writeFileSync(path.join(staging, 'partial.rs'), 'partial compiler output');
  writeFileSync(path.join(lockDirectory, 'journal.json'), JSON.stringify({ schema: 'verdant.proto-ui.publication.v1', committed: false, replacements: [{ target: 'one', staging: path.basename(staging), backup: 'one.producer-interrupted.backup', existed: false }] }));
  assert.equal(recoverPublication(lockDirectory, root), true);
  assert.equal(existsSync(staging), false);
  assert.equal(existsSync(path.join(root, 'one')), false);
});

test('a process killed during its first staging write leaves a recoverable inventory', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, unit, options, release } = fixture(t);
  const original = unit('one', 'old');
  publishBatch([original], options);
  release();
  const moduleUrl = new URL('../lib/artifact-publication.mjs', import.meta.url).href;
  const replacement = unit('one', 'replacement');
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const write = fs.writeFileSync;
    fs.writeFileSync = function(file, ...args) {
      const result = write.call(this, file, ...args);
      if (String(file).includes('.producer-') && String(file).endsWith('/Component.rs')) process.kill(process.pid, 'SIGKILL');
      return result;
    };
    syncBuiltinESMExports();
    const {acquireProducerLock, publishBatch} = await import(${JSON.stringify(moduleUrl)});
    acquireProducerLock(${JSON.stringify(lockDirectory)}, ${JSON.stringify(root)});
    publishBatch([${JSON.stringify(replacement)}], ${JSON.stringify(options)});`;
  const child = spawn(process.execPath, ['--input-type=module', '-e', script]);
  t.after(() => child.kill());
  let errors = '';
  child.stderr.on('data', (data) => { errors += data; });
  const status = await new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (code, signal) => resolve({ code, signal }));
  });
  assert.equal(status.signal, 'SIGKILL', errors);
  assert.equal(existsSync(path.join(stateDirectory(lockDirectory), 'journal.json')), true);
  assert.equal(recoverPublication(lockDirectory, root), true);
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'old');
  assert.equal(readdirSync(root).some((filename) => filename.includes('.producer-')), false);
});

test('concurrent recovery cannot delete the restored original', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, unit, options, release } = fixture(t);
  const original = unit('one', 'original survives');
  publishBatch([original], options);
  release();
  mkdirSync(lockDirectory);
  const ownerFile = path.join(lockDirectory, 'owner.json');
  writeFileSync(ownerFile, JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const backup = `${original.outDirectory}.producer-interrupted.backup`;
  renameSync(original.outDirectory, backup);
  mkdirSync(original.outDirectory);
  writeFileSync(path.join(original.outDirectory, 'Component.rs'), 'partial replacement');
  writeFileSync(path.join(lockDirectory, 'journal.json'), JSON.stringify({ schema: 'verdant.proto-ui.publication.v1', committed: false, replacements: [{ target: 'one', staging: 'one.producer-interrupted.staging', backup: path.basename(backup), existed: true }] }));
  const marker = path.join(root, 'first-recovery-claimed');
  const moduleUrl = new URL('../lib/artifact-publication.mjs', import.meta.url).href;
  const firstScript = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const read = fs.readFileSync;
    fs.readFileSync = function(file, ...args) {
      if (String(file) === ${JSON.stringify(ownerFile)}) {
        fs.writeFileSync(${JSON.stringify(marker)}, 'claimed');
        Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
      }
      return read.call(this, file, ...args);
    };
    syncBuiltinESMExports();
    const {recoverPublication} = await import(${JSON.stringify(moduleUrl)});
    console.log(recoverPublication(${JSON.stringify(lockDirectory)}, ${JSON.stringify(root)}));`;
  const first = spawn(process.execPath, ['--input-type=module', '-e', firstScript]);
  t.after(() => first.kill());
  let errors = '';
  first.stderr.on('data', (data) => { errors += data; });
  const completion = new Promise((resolve, reject) => {
    first.on('error', reject);
    first.on('close', (status) => status === 0 ? resolve() : reject(new Error(errors)));
  });
  for (let attempt = 0; !existsSync(marker) && attempt < 200; attempt += 1) await setTimeout(5);
  assert.equal(existsSync(marker), true, 'first recovery must hold its exclusive claim');
  const secondScript = `import {recoverPublication} from ${JSON.stringify(moduleUrl)};
    try { recoverPublication(${JSON.stringify(lockDirectory)}, ${JSON.stringify(root)}); process.exit(2); }
    catch(error) { if (!error.message.includes('already in progress')) throw error; }`;
  await promisify(execFile)(process.execPath, ['--input-type=module', '-e', secondScript]);
  await completion;
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'original survives');
  assert.equal(existsSync(backup), false);
});

test('a killed recovery can be resumed without deleting another recovery claim', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, unit, options, release } = fixture(t);
  const original = unit('one', 'original survives recovery interruption');
  publishBatch([original], options);
  release();
  mkdirSync(lockDirectory);
  writeFileSync(path.join(lockDirectory, 'owner.json'), JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const backup = `${original.outDirectory}.producer-interrupted.backup`;
  renameSync(original.outDirectory, backup);
  mkdirSync(original.outDirectory);
  writeFileSync(path.join(original.outDirectory, 'Component.rs'), 'partial replacement');
  writeFileSync(path.join(lockDirectory, 'journal.json'), JSON.stringify({ schema: 'verdant.proto-ui.publication.v1', committed: false, replacements: [{ target: 'one', staging: 'one.producer-interrupted.staging', backup: path.basename(backup), existed: true }] }));
  const moduleUrl = new URL('../lib/artifact-publication.mjs', import.meta.url).href;
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const rename = fs.renameSync;
    fs.renameSync = function(source, target) {
      if (String(source) === ${JSON.stringify(backup)}) process.kill(process.pid, 'SIGKILL');
      return rename.call(this, source, target);
    };
    syncBuiltinESMExports();
    const {recoverPublication} = await import(${JSON.stringify(moduleUrl)});
    recoverPublication(${JSON.stringify(lockDirectory)}, ${JSON.stringify(root)});`;
  const child = spawn(process.execPath, ['--input-type=module', '-e', script]);
  t.after(() => child.kill());
  let errors = '';
  child.stderr.on('data', (data) => { errors += data; });
  const status = await new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (code, signal) => resolve({ code, signal }));
  });
  assert.equal(status.signal, 'SIGKILL', errors);
  assert.equal(existsSync(original.outDirectory), false, 'interruption occurs after replacement removal and before backup restore');
  assert.equal(existsSync(backup), true);
  assert.equal(recoverPublication(lockDirectory, root), true);
  assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), 'original survives recovery interruption');
  assert.equal(existsSync(lockDirectory), false);
});

test('a late recovery cannot recreate the lock removed by an earlier recovery', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  mkdirSync(lockDirectory);
  writeFileSync(path.join(lockDirectory, 'owner.json'), JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const claims = path.join(lockDirectory, 'recovery-claims');
  const ready = path.join(root, 'late-recovery-ready');
  const resume = path.join(root, 'late-recovery-resume');
  const moduleUrl = new URL('../lib/artifact-publication.mjs', import.meta.url).href;
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const mkdir = fs.mkdirSync;
    fs.mkdirSync = function(filename, ...args) {
      if (String(filename) === ${JSON.stringify(claims)}) {
        fs.writeFileSync(${JSON.stringify(ready)}, 'ready');
        while (!fs.existsSync(${JSON.stringify(resume)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 5);
      }
      return mkdir.call(this, filename, ...args);
    };
    syncBuiltinESMExports();
    const {recoverPublication} = await import(${JSON.stringify(moduleUrl)});
    if (recoverPublication(${JSON.stringify(lockDirectory)}, ${JSON.stringify(root)}) !== false) throw new Error('late recovery must be a no-op');`;
  const child = spawn(process.execPath, ['--input-type=module', '-e', script]);
  t.after(() => child.kill());
  let errors = '';
  child.stderr.on('data', (data) => { errors += data; });
  const completion = new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (status) => status === 0 ? resolve() : reject(new Error(errors)));
  });
  for (let attempt = 0; !existsSync(ready) && attempt < 200; attempt += 1) await setTimeout(5);
  assert.equal(existsSync(ready), true);
  assert.equal(recoverPublication(lockDirectory, root), true);
  writeFileSync(resume, 'resume');
  await completion;
  assert.equal(existsSync(lockDirectory), false);
});

for (const stage of ['partial owner', 'published owner']) {
  test(`SIGKILL during ${stage} initialization never exposes an incomplete active lock`, { timeout: 10000 }, async (t) => {
    const { root, lockDirectory, unit, options, release } = fixture(t);
    const original = unit('one', 'original before owner initialization');
    publishBatch([original], options);
    release();
    const script = `import fs from 'node:fs'; import path from 'node:path'; import {syncBuiltinESMExports} from 'node:module';
      const open=fs.openSync, write=fs.writeFileSync, link=fs.linkSync, paths=new Map();
      fs.openSync=function(file,...args){const fd=open.call(this,file,...args);paths.set(fd,String(file));return fd;};
      fs.writeFileSync=function(file,...args){
        if (${JSON.stringify(stage)}==='partial owner' && path.basename(paths.get(file)??String(file))==='owner.json') {
          write.call(this,file,'{');process.kill(process.pid,'SIGKILL');
        }
        return write.call(this,file,...args);
      };
      fs.linkSync=function(source,target){const result=link.call(this,source,target);
        if (${JSON.stringify(stage)}==='published owner' && String(target)===${JSON.stringify(lockDirectory)}) process.kill(process.pid,'SIGKILL');
        return result;
      };
      syncBuiltinESMExports();
      const {acquireProducerLock}=await import(${JSON.stringify(moduleUrl)});
      acquireProducerLock(${JSON.stringify(lockDirectory)},${JSON.stringify(root)});`;
    const status = await childProcess(t, script).completion;
    assert.equal(status.signal, 'SIGKILL', status.output);
    assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), original.artifacts[0].contents);
    if (stage === 'partial owner') {
      assert.equal(existsSync(lockDirectory), false, 'partial owner bytes have never become the public lock');
      const orphan = readdirSync(root).filter((name) => name.startsWith('.producer.lock.state-'));
      assert.equal(orphan.length, 1);
      const privateOwner = path.join(root, orphan[0], 'owner.json');
      assert.equal(readFileSync(privateOwner, 'utf8'), '{');
      assert.equal(recoverPublication(lockDirectory, root), false);
      const nextRelease = acquireProducerLock(lockDirectory, root);
      nextRelease();
      assert.equal(readFileSync(privateOwner, 'utf8'), '{', 'unpublished private candidates are not guessed away');
    } else {
      const owner = JSON.parse(readFileSync(lockDirectory, 'utf8'));
      assert.equal(owner.schema, 'verdant.proto-ui.producer-lock.v2');
      assert.equal(readFileSync(path.join(stateDirectory(lockDirectory), 'owner.json'), 'utf8'), readFileSync(lockDirectory, 'utf8'));
      assert.equal(recoverPublication(lockDirectory, root), true);
      assert.equal(readdirSync(root).some((name) => name.startsWith('.producer.lock.state-')), false);
    }
    assert.equal(existsSync(lockDirectory), false);
  });
}

for (const stage of ['partial journal', 'published journal']) {
  test(`SIGKILL during ${stage} initialization preserves a recoverable publication`, { timeout: 10000 }, async (t) => {
    const { root, lockDirectory, unit, options, release } = fixture(t);
    const original = unit('one', 'original before journal initialization');
    publishBatch([original], options);
    const originalReceipt = readFileSync(original.receiptPath, 'utf8');
    release();
    const replacement = unit('one', 'replacement must not become visible');
    const script = `import fs from 'node:fs'; import path from 'node:path'; import {syncBuiltinESMExports} from 'node:module';
      const open=fs.openSync, write=fs.writeFileSync, link=fs.linkSync, paths=new Map();
      fs.openSync=function(file,...args){const fd=open.call(this,file,...args);paths.set(fd,String(file));return fd;};
      fs.writeFileSync=function(file,...args){
        if (${JSON.stringify(stage)}==='partial journal' && ['journal.initial.json','journal.json'].includes(path.basename(paths.get(file)??String(file)))) {
          write.call(this,file,'{');process.kill(process.pid,'SIGKILL');
        }
        return write.call(this,file,...args);
      };
      fs.linkSync=function(source,target){const result=link.call(this,source,target);
        if (${JSON.stringify(stage)}==='published journal' && path.basename(String(target))==='journal.json') process.kill(process.pid,'SIGKILL');
        return result;
      };
      syncBuiltinESMExports();
      const {acquireProducerLock,publishBatch}=await import(${JSON.stringify(moduleUrl)});
      acquireProducerLock(${JSON.stringify(lockDirectory)},${JSON.stringify(root)});
      publishBatch([${JSON.stringify(replacement)}],${JSON.stringify(options)});`;
    const status = await childProcess(t, script).completion;
    assert.equal(status.signal, 'SIGKILL', status.output);
    const directory = stateDirectory(lockDirectory);
    const journal = path.join(directory, 'journal.json');
    if (stage === 'partial journal') {
      assert.equal(existsSync(journal), false, 'partial bytes never replace the recoverable journal');
      assert.equal(readFileSync(path.join(directory, 'journal.initial.json'), 'utf8'), '{');
    } else {
      const record = JSON.parse(readFileSync(journal, 'utf8'));
      assert.equal(record.replacements.length, 2);
      assert.equal(record.committed, false);
    }
    assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), original.artifacts[0].contents);
    assert.equal(readFileSync(original.receiptPath, 'utf8'), originalReceipt);
    assert.equal(recoverPublication(lockDirectory, root), true);
    assert.equal(readFileSync(path.join(original.outDirectory, 'Component.rs'), 'utf8'), original.artifacts[0].contents);
    assert.equal(readFileSync(original.receiptPath, 'utf8'), originalReceipt);
    assert.equal(existsSync(lockDirectory), false);
    assert.equal(existsSync(directory), false);
  });
}

test('simultaneous acquire publishes exactly one complete owner', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  const go = path.join(root, 'acquire-go'), finish = path.join(root, 'acquire-finish');
  const children = [1, 2].map((id) => {
    const ready = path.join(root, `acquire-ready-${id}`);
    const won = path.join(root, `acquire-won-${id}`), lost = path.join(root, `acquire-lost-${id}`);
    const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
      const link=fs.linkSync;
      fs.linkSync=function(source,target){
        if (String(target)===${JSON.stringify(lockDirectory)}) {
          fs.writeFileSync(${JSON.stringify(ready)},'ready');
          while(!fs.existsSync(${JSON.stringify(go)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,5);
        }
        return link.call(this,source,target);
      };
      syncBuiltinESMExports();
      const {acquireProducerLock}=await import(${JSON.stringify(moduleUrl)});
      try {
        const release=acquireProducerLock(${JSON.stringify(lockDirectory)},${JSON.stringify(root)});
        fs.writeFileSync(${JSON.stringify(won)},String(process.pid));
        while(!fs.existsSync(${JSON.stringify(finish)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,5);
        release();release();
      } catch(error) {
        if(!error.message.includes('producer is locked')) throw error;
        fs.writeFileSync(${JSON.stringify(lost)},'locked');
      }`;
    return { ...childProcess(t, script), ready, won, lost };
  });
  await Promise.all(children.map(({ ready }) => waitForFile(ready)));
  writeFileSync(go, 'go');
  for (let attempt = 0; children.some(({ won, lost }) => !existsSync(won) && !existsSync(lost)) && attempt < 400; attempt += 1) await setTimeout(5);
  const winners = children.filter(({ won }) => existsSync(won));
  assert.equal(winners.length, 1);
  assert.equal(children.filter(({ lost }) => existsSync(lost)).length, 1);
  assert.equal(JSON.parse(readFileSync(lockDirectory, 'utf8')).pid, Number(readFileSync(winners[0].won, 'utf8')));
  assert.equal(readdirSync(root).filter((name) => name.startsWith('.producer.lock.state-')).length, 1, 'loser removes only its own unpublished candidate');
  writeFileSync(finish, 'finish');
  for (const child of children) {
    const result = await child.completion;
    assert.equal(result.code, 0, result.output);
  }
  assert.equal(existsSync(lockDirectory), false);
});

test('an old release is idempotent and cannot remove a later owner', (t) => {
  const { lockDirectory, root, release } = fixture(t);
  release();
  const nextRelease = acquireProducerLock(lockDirectory, root);
  const nextOwner = readFileSync(lockDirectory, 'utf8');
  release();
  assert.equal(readFileSync(lockDirectory, 'utf8'), nextOwner);
  nextRelease();
  nextRelease();
  assert.equal(existsSync(lockDirectory), false);
});

test('a late file-lock recovery cannot remove a new producer at the same public path', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  const owner = childProcess(t, `import {acquireProducerLock} from ${JSON.stringify(moduleUrl)};
    acquireProducerLock(${JSON.stringify(lockDirectory)},${JSON.stringify(root)});process.kill(process.pid,'SIGKILL');`);
  assert.equal((await owner.completion).signal, 'SIGKILL');
  const claims = path.join(stateDirectory(lockDirectory), 'recovery-claims');
  const ready = path.join(root, 'file-recovery-ready'), resume = path.join(root, 'file-recovery-resume');
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const mkdir=fs.mkdirSync;
    fs.mkdirSync=function(filename,...args){
      if(String(filename)===${JSON.stringify(claims)}) {
        fs.writeFileSync(${JSON.stringify(ready)},'ready');
        while(!fs.existsSync(${JSON.stringify(resume)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,5);
      }
      return mkdir.call(this,filename,...args);
    };
    syncBuiltinESMExports();
    const {recoverPublication}=await import(${JSON.stringify(moduleUrl)});
    if(recoverPublication(${JSON.stringify(lockDirectory)},${JSON.stringify(root)})!==false) throw new Error('obsolete recovery must be a no-op');`;
  const late = childProcess(t, script);
  await waitForFile(ready);
  assert.equal(recoverPublication(lockDirectory, root), true);
  const nextRelease = acquireProducerLock(lockDirectory, root);
  const nextOwner = readFileSync(lockDirectory, 'utf8');
  writeFileSync(resume, 'resume');
  const status = await late.completion;
  assert.equal(status.code, 0, status.output);
  assert.equal(readFileSync(lockDirectory, 'utf8'), nextOwner);
  nextRelease();
});

test('legacy recovery can hand its public path to a new file lock before returning', { timeout: 10000 }, async (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  mkdirSync(lockDirectory);
  writeFileSync(path.join(lockDirectory, 'owner.json'), JSON.stringify({ pid: 2147483647, host: os.hostname() }));
  const ready = path.join(root, 'legacy-handoff-ready'), resume = path.join(root, 'legacy-handoff-resume');
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const remove=fs.rmSync;
    fs.rmSync=function(filename,...args){
      const result=remove.call(this,filename,...args);
      if(String(filename)===${JSON.stringify(lockDirectory)}) {
        fs.writeFileSync(${JSON.stringify(ready)},'ready');
        while(!fs.existsSync(${JSON.stringify(resume)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,5);
      }
      return result;
    };
    syncBuiltinESMExports();
    const {recoverPublication}=await import(${JSON.stringify(moduleUrl)});
    if(recoverPublication(${JSON.stringify(lockDirectory)},${JSON.stringify(root)})!==true) throw new Error('completed recovery must succeed');`;
  const recovery = childProcess(t, script);
  await waitForFile(ready);
  const nextRelease = acquireProducerLock(lockDirectory, root);
  const nextOwner = readFileSync(lockDirectory, 'utf8');
  writeFileSync(resume, 'resume');
  const status = await recovery.completion;
  assert.equal(status.code, 0, status.output);
  assert.equal(readFileSync(lockDirectory, 'utf8'), nextOwner, 'the legacy release marker cannot touch the new public file');
  nextRelease();
});

test('unknown legacy locks, forged state identities and extra private files remain intact', (t) => {
  const { root, lockDirectory, release } = fixture(t);
  release();
  mkdirSync(lockDirectory);
  assert.throws(() => acquireProducerLock(lockDirectory, root), /producer is locked/);
  assert.throws(() => recoverPublication(lockDirectory, root), /ENOENT/);
  assert.equal(lstatSync(lockDirectory).isDirectory(), true);
  rmSync(lockDirectory, { recursive: true }); // Remove only this test's known empty fixture.
  const forged = JSON.stringify({ schema: 'verdant.proto-ui.producer-lock.v2', pid: 2147483647, host: os.hostname(), id: '../outside' });
  writeFileSync(lockDirectory, forged);
  assert.throws(() => acquireProducerLock(lockDirectory, root), /producer is locked/);
  assert.throws(() => recoverPublication(lockDirectory, root), /Invalid publication lock identity/);
  assert.equal(readFileSync(lockDirectory, 'utf8'), forged);
  rmSync(lockDirectory);
  const nextRelease = acquireProducerLock(lockDirectory, root);
  const humanFile = path.join(stateDirectory(lockDirectory), 'human-notes.md');
  writeFileSync(humanFile, 'preserve unknown content');
  assert.throws(nextRelease, /Unknown publication lock metadata/);
  assert.equal(readFileSync(humanFile, 'utf8'), 'preserve unknown content');
  assert.equal(existsSync(lockDirectory), true);
  rmSync(humanFile);
  nextRelease();
});
