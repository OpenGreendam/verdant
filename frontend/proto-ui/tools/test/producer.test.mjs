import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { promisify } from 'node:util';
import { EXIT, ProducerError, loadPin, runProducer } from '../build-proto-ui.mjs';
import { parseCargoManifest } from '../lib/workspace-gpui.mjs';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..');
const json = (filename) => JSON.parse(readFileSync(filename, 'utf8'));
function fixture(t) {
  const root = mkdtempSync(path.join(os.tmpdir(), 'verdant-producer-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(path.join(root, 'frontend'), { recursive: true });
  cpSync(path.join(repository, 'frontend/Cargo.toml'), path.join(root, 'frontend/Cargo.toml'));
  for (const name of ['compiler.pin.json', 'src', 'generated', 'receipts']) {
    cpSync(path.join(repository, 'frontend/proto-ui', name), path.join(root, 'frontend/proto-ui', name), { recursive: true });
  }
  return root;
}
const code = (expected) => (error) => error instanceof ProducerError && error.code === expected;
const absentUpstream = (root) => ['--upstream', path.join(root, 'absent-upstream')];

test('retained evidence explicitly succeeds offline without claiming fresh compilation', async (t) => {
  const root = fixture(t);
  const result = await runProducer(['--check-retained', ...absentUpstream(root)], root);
  assert.equal(result.mode, 'retained-artifacts');
  assert.equal(result.freshCompilation, false);
  assert.ok(result.units.includes('note-gesture-planner'));
});

test('fresh checks fail when the pinned upstream is absent', async (t) => {
  const root = fixture(t);
  await assert.rejects(runProducer(['--check', ...absentUpstream(root)], root), code(EXIT.upstreamAbsent));
});

for (const [name, replace] of [
  ['revision', (text) => text.replaceAll('e5f2f9db75e97613232b5c1946333e61fa3828ab', 'a'.repeat(40))],
  ['source repository', (text) => text.replaceAll('https://github.com/cyjin-yl/z3rm', 'https://github.com/other/gpui')],
  ['default features', (text) => text.replace('gpui = { git', 'gpui = { default-features = false, git')],
  ['additive features', (text) => text.replace('gpui = { git', 'gpui = { features = ["test-support"], git')],
]) {
  test(`workspace ${name} is checked before missing upstream in both check modes`, async (t) => {
    const root = fixture(t);
    const manifest = path.join(root, 'frontend/Cargo.toml');
    writeFileSync(manifest, replace(readFileSync(manifest, 'utf8')));
    for (const mode of ['--check', '--check-retained']) {
      await assert.rejects(runProducer([mode, ...absentUpstream(root)], root), code(EXIT.admission));
    }
  });
}

test('retained checks reject changed source before looking for upstream', async (t) => {
  const root = fixture(t);
  const source = path.join(root, 'frontend/proto-ui/src/control-gesture-planner.proto.ts');
  writeFileSync(source, `${readFileSync(source, 'utf8')}\n// changed source\n`);
  await assert.rejects(runProducer(['--check', ...absentUpstream(root)], root), code(EXIT.drift));
});

test('shared consumers cannot redirect their SDK path or add another owner', (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const pin = json(filename);
  pin.units.find((unit) => unit.name === 'normalized-knob').target.nativeSdkPath = './.proto-ui/gpui-native';
  writeFileSync(filename, JSON.stringify(pin));
  assert.throws(() => loadPin(filename, root), code(EXIT.pin));
});

test('every application authored entry and generated Cargo consumer is pinned', (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const original = json(filename);
  const omitted = structuredClone(original);
  omitted.units = omitted.units.filter((unit) => unit.name !== 'action-control');
  writeFileSync(filename, JSON.stringify(omitted));
  assert.throws(() => loadPin(filename, root), (error) => code(EXIT.pin)(error) && /Authored entry.*action-control/.test(error.message));
  writeFileSync(filename, JSON.stringify(original));
  const orphan = path.join(root, 'frontend/proto-ui/generated/unpinned-control');
  mkdirSync(orphan);
  writeFileSync(path.join(orphan, 'Cargo.toml'), '[package]\nname="orphan"\nversion="0.1.0"\n');
  assert.throws(() => loadPin(filename, root), (error) => code(EXIT.pin)(error) && /Generated Cargo consumer.*unpinned-control/.test(error.message));
});

async function rejectsInventoryPath(root) {
  const invalid = (error) => code(EXIT.pin)(error) && /[Ss]ymbolic|regular file/.test(error.message);
  assert.throws(() => loadPin(path.join(root, 'frontend/proto-ui/compiler.pin.json'), root), invalid);
  // Both lanes must reject the path before consulting an upstream checkout.
  for (const mode of ['--check', '--check-retained']) {
    await assert.rejects(runProducer([mode, ...absentUpstream(root)], root), invalid);
  }
}

for (const [name, target] of [
  ['unpinned live source link', 'action-control.proto.ts'],
  ['unpinned dangling source link', 'absent.proto.ts'],
  ['pinned source link', 'helpers/original.proto.ts'],
  ['source entry directory', null],
]) {
  test(`inventory rejects ${name}`, async (t) => {
    const root = fixture(t);
    const source = path.join(root, 'frontend/proto-ui/src');
    const entry = path.join(source, name === 'pinned source link' ? 'action-control.proto.ts' : 'unpinned-control.proto.ts');
    if (name === 'pinned source link') {
      mkdirSync(path.join(source, 'helpers'));
      renameSync(entry, path.join(source, target));
    }
    if (target === null) mkdirSync(entry);
    else symlinkSync(target, entry, 'file');
    await rejectsInventoryPath(root);
  });
}

for (const [name, target] of [
  ['unpinned generated consumer link', '../../unregistered-consumer'],
  ['dangling generated consumer link', '../../absent-consumer'],
  ['pinned generated consumer link', '../../unregistered-consumer'],
]) {
  test(`inventory rejects ${name}`, async (t) => {
    const root = fixture(t);
    const generated = path.join(root, 'frontend/proto-ui/generated');
    const entry = path.join(generated, name.startsWith('pinned') ? 'action-control' : 'unpinned-control');
    if (name.startsWith('pinned')) renameSync(entry, path.join(root, 'frontend/unregistered-consumer'));
    else if (!name.startsWith('dangling')) {
      mkdirSync(path.join(root, 'frontend/unregistered-consumer/src'), { recursive: true });
      writeFileSync(path.join(root, 'frontend/unregistered-consumer/Cargo.toml'), '[package]\nname="orphan"\nversion="0.1.0"\n');
      writeFileSync(path.join(root, 'frontend/unregistered-consumer/src/lib.rs'), 'pub fn orphan() {}\n');
    }
    symlinkSync(target, entry, 'dir');
    await rejectsInventoryPath(root);
  });
}

for (const relative of ['frontend', 'frontend/proto-ui', 'frontend/proto-ui/src', 'frontend/proto-ui/generated']) {
  test(`inventory rejects a symbolic ${relative} root`, async (t) => {
    const root = fixture(t);
    const directory = path.join(root, relative);
    renameSync(directory, `${directory}-real`);
    symlinkSync(`${path.basename(directory)}-real`, directory, 'dir');
    await rejectsInventoryPath(root);
  });
}

for (const relative of ['frontend/proto-ui/src', 'frontend/proto-ui/generated']) {
  test(`inventory rejects a dangling ${relative} root`, async (t) => {
    const root = fixture(t);
    const directory = path.join(root, relative);
    rmSync(directory, { recursive: true });
    symlinkSync('absent-directory', directory, 'dir');
    await rejectsInventoryPath(root);
  });
}

test('inventory rejects a symbolic repository root', async (t) => {
  const root = fixture(t);
  const alias = path.join(root, 'repository-alias');
  symlinkSync('.', alias, 'dir');
  await rejectsInventoryPath(alias);
});

test('inventory permits ordinary helper files and nested vendored prototype sources', async (t) => {
  const root = fixture(t);
  const source = path.join(root, 'frontend/proto-ui/src');
  writeFileSync(path.join(source, 'helper.ts'), 'export const helper = 1;\n');
  mkdirSync(path.join(source, 'vendor/nested'), { recursive: true });
  writeFileSync(path.join(source, 'vendor/nested/imported.proto.ts'), 'export const helper = 2;\n');
  const pin = loadPin(path.join(root, 'frontend/proto-ui/compiler.pin.json'), root);
  assert.equal(pin.units.some((unit) => unit.source.endsWith('imported.proto.ts')), false);
  assert.equal((await runProducer(['--check-retained'], root)).freshCompilation, false);
});

test('inventory permits an absent generated root before first publication', (t) => {
  const root = fixture(t);
  rmSync(path.join(root, 'frontend/proto-ui/generated'), { recursive: true });
  assert.doesNotThrow(() => loadPin(path.join(root, 'frontend/proto-ui/compiler.pin.json'), root));
});

test('each data unit pins selected pure methods, a stable package and an empty dependency closure', (t) => {
  const root = fixture(t);
  const pin = loadPin(path.join(root, 'frontend/proto-ui/compiler.pin.json'), root);
  const data = pin.units.filter((unit) => unit.target.profile === 'rust-data-v1');
  assert.deepEqual(data.map((unit) => unit.target.crateName), ['verdant-note-gesture-data', 'verdant-control-gesture-data']);
  for (const unit of data) {
    const receipt = json(path.join(root, unit.output.receipt));
    const provenance = json(path.join(root, unit.output.directory, 'provenance.json'));
    assert.deepEqual(receipt.target, unit.target);
    assert.deepEqual(provenance.methods.map((method) => method.name), unit.target.methods);
    assert.deepEqual(provenance.dependencies, []);
    assert.equal(unit.target.sourceLicense, 'AGPL-3.0-or-later');
    assert.equal(provenance.sourceLicense, unit.target.sourceLicense);
    assert.equal(parseCargoManifest(readFileSync(path.join(root, unit.output.directory, 'Cargo.toml'), 'utf8')).package.license, unit.target.sourceLicense);
    assert.equal(receipt.sharedSdk, undefined);
    assert.equal(receipt.gpu, undefined);
    assert.equal(existsSync(path.join(root, unit.output.directory, 'Component.rs')), false);
    assert.equal(existsSync(path.join(root, unit.output.directory, 'lib.rs')), true);
  }
});

for (const [label, mutate] of [
  ['native SDK option', (unit) => { unit.target.nativeSdkPath = '../pitch-gesture-surface/.proto-ui/gpui-native'; }],
  ['missing selectors', (unit) => { unit.target.methods = []; }],
  ['duplicate selectors', (unit) => { unit.target.methods = ['planNoteGesture', 'planNoteGesture']; }],
  ['language dependency', (unit) => { unit.target.dependencies = ['serde_json']; }],
  ['empty source license', (unit) => { unit.target.sourceLicense = ''; }],
  ['multiline source license', (unit) => { unit.target.sourceLicense = 'AGPL-3.0-or-later\nMIT'; }],
  ['implicit target', (unit) => { delete unit.target; }],
]) {
  test(`data targets reject ${label} instead of falling back to a GPUI component`, (t) => {
    const root = fixture(t);
    const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
    const pin = json(filename);
    mutate(pin.units.find((unit) => unit.name === 'note-gesture-planner'));
    writeFileSync(filename, JSON.stringify(pin));
    assert.throws(() => loadPin(filename, root), code(EXIT.pin));
  });
}

test('retained checks refuse unrecorded generated files instead of silently accepting them', async (t) => {
  const root = fixture(t);
  writeFileSync(path.join(root, 'frontend/proto-ui/generated/note-gesture-planner/manual.rs'), '// preserve me');
  await assert.rejects(runProducer(['--check-retained'], root), code(EXIT.drift));
  assert.equal(existsSync(path.join(root, 'frontend/proto-ui/generated/note-gesture-planner/manual.rs')), true);
});

const upstream = process.env.PROTO_UI_UPSTREAM;
test('fresh source-graph recompilation tracks imported helpers and changes emitted data code', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const source = path.join(root, 'frontend/proto-ui/src/control-gesture-planner.proto.ts');
  const helper = path.join(root, 'frontend/proto-ui/src/drag-rate.ts');
  const before = readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/lib.rs'), 'utf8');
  const original = readFileSync(source, 'utf8');
  assert.ok(original.includes('const rate = fine ? 0.1 : 1;'));
  writeFileSync(source, `import { dragRate } from './drag-rate';\n${original.replace('const rate = fine ? 0.1 : 1;', 'const rate = dragRate(fine);')}`);
  writeFileSync(helper, 'export function dragRate(fine: boolean): number { return fine ? 0.2 : 1; }\n');
  const first = await runProducer(['--upstream', upstream], root);
  assert.equal(first.freshCompilation, true);
  const receipt = json(path.join(root, 'frontend/proto-ui/receipts/control-gesture-planner.json'));
  assert.ok(receipt.sourceInputs.some((input) => input.path === 'frontend/proto-ui/src/drag-rate.ts' && input.sha256));
  const changed = readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/lib.rs'), 'utf8');
  assert.notEqual(changed, before);
  assert.match(changed, /0\.2/);
  await runProducer(['--check', '--upstream', upstream], root);
  // Mutating only the imported file must invalidate the offline receipt too.
  writeFileSync(helper, 'export function dragRate(fine: boolean): number { return fine ? 0.3 : 1; }\n');
  await assert.rejects(runProducer(['--check-retained', ...absentUpstream(root)], root), code(EXIT.drift));
  await runProducer(['--upstream', upstream], root);
  const second = readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/lib.rs'), 'utf8');
  assert.notEqual(second, changed);
  assert.match(second, /0\.3/);
  await runProducer(['--check', '--upstream', upstream], root);
});

test('fresh data compilation rejects an unavailable selected method without omitting it', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const pin = json(filename);
  pin.units.find((unit) => unit.name === 'control-gesture-planner').target.methods.push('missingMethod');
  writeFileSync(filename, JSON.stringify(pin));
  await assert.rejects(runProducer(['--upstream', upstream], root), (error) => code(EXIT.compile)(error) && /PUI_RUST_DATA_PURITY/.test(error.message));
});

test('the actual upstream target GPUI pin is checked, not just recorded', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const pin = json(filename);
  pin.gpu.upstreamGpuiRevision = 'b'.repeat(40);
  writeFileSync(filename, JSON.stringify(pin));
  await assert.rejects(runProducer(['--upstream', upstream], root), (error) => code(EXIT.admission)(error) && /Compiler target GPUI\/SDK profile/.test(error.message));
});

test('the actually installed compiler package versions are checked', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const pin = json(filename);
  pin.toolchain.packages.typescript = '0.0.0';
  writeFileSync(filename, JSON.stringify(pin));
  await assert.rejects(runProducer(['--upstream', upstream], root), (error) => code(EXIT.toolchain)(error) && /Executed compiler packages/.test(error.message));
});

test('the actual upstream lock content is checked before executing compiler code', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const filename = path.join(root, 'frontend/proto-ui/compiler.pin.json');
  const pin = json(filename);
  pin.toolchain.lockSha256 = 'c'.repeat(64);
  writeFileSync(filename, JSON.stringify(pin));
  await assert.rejects(runProducer(['--upstream', upstream], root), (error) => code(EXIT.sourceState)(error) && /lockfile content/.test(error.message));
});

test('a nearer resolution candidate appearing after the loader read is rejected', { skip: !upstream }, async (t) => {
  const root = fixture(t);
  const source = path.join(root, 'frontend/proto-ui/src/control-gesture-planner.proto.ts');
  const helper = path.join(root, 'frontend/proto-ui/src/drag-rate.ts');
  const nearer = path.join(root, 'frontend/proto-ui/src/drag-rate');
  const original = readFileSync(source, 'utf8');
  writeFileSync(source, `import { dragRate } from './drag-rate';\n${original.replace('const rate = fine ? 0.1 : 1;', 'const rate = dragRate(fine);')}`);
  writeFileSync(helper, 'export function dragRate(fine: boolean): number { return fine ? 0.2 : 1; }\n');
  const producerUrl = new URL('../build-proto-ui.mjs', import.meta.url).href;
  // The async graph loader first observes no extensionless candidate. Make it
  // appear just as the producer records synchronous input hashes. Entry/helper
  // bytes are unchanged, so only a full resolution-identity replay catches it.
  const script = `import fs from 'node:fs'; import {syncBuiltinESMExports} from 'node:module';
    const read = fs.readFileSync; let changed = false;
    fs.readFileSync = function(file, ...args) {
      if (!changed && String(file) === ${JSON.stringify(source)}) {
        changed = true;
        fs.writeFileSync(${JSON.stringify(nearer)}, 'export function dragRate(fine: boolean): number { return fine ? 0.9 : 1; }\\n');
      }
      return read.call(this, file, ...args);
    };
    syncBuiltinESMExports();
    const {runProducer, EXIT} = await import(${JSON.stringify(producerUrl)});
    try { await runProducer(['--upstream', ${JSON.stringify(upstream)}], ${JSON.stringify(root)}); throw new Error('mixed source graph was accepted'); }
    catch(error) { if (error.code !== EXIT.compile || !error.message.includes('Source/resolution graph changed')) throw error; }
    if (!changed) throw new Error('mutation hook did not run');`;
  await promisify(execFile)(process.execPath, ['--input-type=module', '-e', script], { maxBuffer: 4 * 1024 * 1024 });
});
