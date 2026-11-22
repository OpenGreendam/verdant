import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFile, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { promisify } from 'node:util';
import { setTimeout } from 'node:timers/promises';
import { acquireProducerLock, assertOwnedInventory, publishBatch, recoverPublication } from '../lib/artifact-publication.mjs';

const hash = (value) => createHash('sha256').update(value).digest('hex');
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
  assert.equal(existsSync(path.join(lockDirectory, 'journal.json')), false);
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
  assert.equal(existsSync(path.join(lockDirectory, 'journal.json')), true);
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
