import assert from 'node:assert/strict';
import test from 'node:test';
import { findRustDataViolations } from '../lib/workspace-gpui.mjs';
import { headlessSessionContract, inspectHeadlessSessionGraph } from '../lib/headless-session.mjs';

const pin = {
  schema: 'verdant.proto-ui.compiler-pin.v3',
  sharedSdk: { owner: 'pitch-gesture-surface' },
  units: [
    ['pitch-gesture-surface', 'gpui-source-v1'],
    ['normalized-knob', 'gpui-source-v1'],
    ['note-gesture-planner', 'rust-data-v1', 'verdant-note-gesture-data'],
    ['control-gesture-planner', 'rust-data-v1', 'verdant-control-gesture-data'],
  ].map(([name, profile, crateName]) => ({ name, target: { profile, crateName }, output: { directory: `frontend/proto-ui/generated/${name}` } })),
};
const contract = headlessSessionContract(pin, '/work/frontend');
const packageOf = (id, name, manifest, source = null) => ({ id, name, manifest_path: manifest, source });
const edge = (pkg, name = pkg, kind = null, target = null) => ({ name, pkg, dep_kinds: [{ kind, target }] });
function add(metadata, pkg, dependencies = []) {
  metadata.packages.push(pkg);
  metadata.resolve.nodes.push({ id: pkg.id, dependencies: dependencies.map((dependency) => dependency.pkg), deps: dependencies, features: [] });
}
function connect(metadata, from, to, name = to, kind = null, target = null) {
  const node = metadata.resolve.nodes.find((node) => node.id === from);
  node.deps.push(edge(to, name, kind, target));
  node.dependencies.push(to);
}
function graph() {
  const metadata = { workspace_root: '/work/frontend', workspace_members: ['session', 'protocol', 'note', 'control', 'native'], packages: [], resolve: { nodes: [] } };
  add(metadata, packageOf('session', contract.session.name, contract.session.manifest), [edge('protocol'), edge('domain')]);
  add(metadata, packageOf('protocol', contract.protocol.name, contract.protocol.manifest), [edge('note'), edge('control')]);
  add(metadata, packageOf('note', contract.data[0].name, contract.data[0].manifest));
  add(metadata, packageOf('control', contract.data[1].name, contract.data[1].manifest));
  add(metadata, packageOf('domain', 'verdant-domain', '/work/frontend/crates/verdant-domain/Cargo.toml'), [edge('serde'), edge('windows', 'windows-sys', null, 'cfg(windows)')]);
  add(metadata, packageOf('serde', 'serde', '/cargo/serde/Cargo.toml', 'registry+crates.io'));
  add(metadata, packageOf('windows', 'windows-sys', '/cargo/windows-sys/Cargo.toml', 'registry+crates.io'));
  add(metadata, packageOf('native', 'generated-gpui-a', `${contract.nativeDirectories[0]}/Cargo.toml`), [edge('sdk'), edge('gpui')]);
  add(metadata, packageOf('sdk', 'proto-ui-gpui-native', `${contract.nativeDirectories[0]}/.proto-ui/gpui-native/Cargo.toml`), [edge('gpui'), edge('image')]);
  add(metadata, packageOf('gpui', 'gpui', '/cargo/gpui/Cargo.toml', 'git+https://example.com/gpui?rev=pin#pin'));
  add(metadata, packageOf('image', 'image', '/cargo/image/Cargo.toml', 'registry+crates.io'));
  return metadata;
}
const inspect = (metadata) => inspectHeadlessSessionGraph(metadata, contract);

test('the session closure excludes unrelated native workspace members while preserving target filesystem dependencies', () => {
  const result = inspect(graph());
  assert.deepEqual(result.violations, []);
  assert.deepEqual(result.reachable.map((pkg) => pkg.name).sort(), [
    'serde', 'verdant-control-gesture-data', 'verdant-domain', 'verdant-note-gesture-data', 'verdant-session', 'verdant-ui-protocol', 'windows-sys',
  ]);
  assert.deepEqual(result.data, ['note', 'control']);
});

for (const [kind, target] of [[null, null], ['build', null], ['dev', null], [null, 'cfg(windows)'], ['build', 'cfg(unix)'], ['dev', 'cfg(target_os = "macos")']]) {
  test(`headless closure rejects aliased transitive GPUI on ${kind ?? 'normal'} / ${target ?? 'all targets'}`, () => {
    const metadata = graph();
    connect(metadata, 'domain', 'gpui', 'innocent-alias', kind, target);
    const violations = inspect(metadata).violations;
    assert.equal(violations.length, 1);
    assert.match(violations[0], /native\/image support gpui \(gpui\)/);
    assert.ok(violations[0].includes('verdant-session --domain [normal]--> verdant-domain --innocent-alias'));
    assert.ok(violations[0].includes(kind ?? 'normal'));
    if (target) assert.ok(violations[0].includes(target));
  });
}

test('a renamed native component is rejected by its pinned output path', () => {
  const metadata = graph();
  const pkg = metadata.packages.find((pkg) => pkg.id === 'native');
  pkg.name = 'pure-looking-helper';
  connect(metadata, 'protocol', 'native');
  assert.ok(inspect(metadata).violations.some((message) => message.includes('a pinned native output pure-looking-helper')));
});

test('an aliased shared SDK remains native even when copied outside the generated owner', () => {
  const metadata = graph();
  metadata.packages.find((pkg) => pkg.id === 'sdk').manifest_path = '/elsewhere/copied-sdk/Cargo.toml';
  connect(metadata, 'protocol', 'sdk', 'helper');
  assert.ok(inspect(metadata).violations.some((message) => message.includes('native/image support proto-ui-gpui-native')));
});

test('image support is prohibited even without a GPUI edge', () => {
  const metadata = graph();
  connect(metadata, 'domain', 'image', 'pixels');
  assert.deepEqual(inspect(metadata).violations, [
    'Headless session reaches native/image support image (image): verdant-session --domain [normal]--> verdant-domain --pixels [normal]--> image',
  ]);
});

test('required std-only data packages must really be reachable', () => {
  const metadata = graph();
  const protocol = metadata.resolve.nodes.find((node) => node.id === 'protocol');
  protocol.deps = protocol.deps.filter((dependency) => dependency.pkg !== 'note');
  protocol.dependencies = protocol.dependencies.filter((id) => id !== 'note');
  assert.deepEqual(inspect(metadata).violations, ['Headless session does not reach required generated-protocol package verdant-note-gesture-data (note).']);
});

test('the expected package name at a registry or different path cannot impersonate a generated data unit', () => {
  for (const replace of [
    (pkg) => { pkg.source = 'registry+crates.io'; },
    (pkg) => { pkg.manifest_path = '/elsewhere/Cargo.toml'; },
    (pkg) => { pkg.name = 'unrelated-package'; },
  ]) {
    const metadata = graph();
    replace(metadata.packages.find((pkg) => pkg.id === 'note'));
    assert.deepEqual(inspect(metadata).violations, [`Expected one local workspace package verdant-note-gesture-data at ${contract.data[0].manifest}.`]);
  }
});

test('generated Rust data packages cannot gain even a non-native support dependency', () => {
  const metadata = graph();
  connect(metadata, 'control', 'serde');
  assert.deepEqual(inspect(metadata).violations, ['The pinned std-only Rust data package resolves dependencies: verdant-control-gesture-data (control).']);
});

test('--no-deps and missing session roots cannot prove headlessness', () => {
  const withoutGraph = graph();
  withoutGraph.resolve = null;
  assert.deepEqual(inspect(withoutGraph).violations, ['Headless evidence requires full Cargo metadata with a resolve graph (--no-deps is insufficient).']);
  const withoutRoot = graph();
  withoutRoot.workspace_members = withoutRoot.workspace_members.filter((id) => id !== 'session');
  assert.deepEqual(inspect(withoutRoot).violations, [`Expected one local workspace package verdant-session at ${contract.session.manifest}.`]);
});

test('an incomplete modern dependency list cannot conceal Cargo dependency identities', () => {
  const metadata = graph();
  metadata.resolve.nodes.find((node) => node.id === 'protocol').dependencies.push('gpui');
  const violations = inspect(metadata).violations;
  assert.ok(violations.includes('Cargo dependency identities disagree or are incomplete for protocol.'));
  assert.ok(violations.some((message) => message.includes('native/image support gpui')));
});

test('missing package records, resolve nodes, and edge kinds fail closed', () => {
  const withoutNode = graph();
  withoutNode.resolve.nodes = withoutNode.resolve.nodes.filter((node) => node.id !== 'serde');
  assert.ok(inspect(withoutNode).violations.includes('Headless dependency has an incomplete resolve node: serde'));
  const withoutPackage = graph();
  withoutPackage.packages = withoutPackage.packages.filter((pkg) => pkg.id !== 'serde');
  assert.ok(inspect(withoutPackage).violations.includes('Headless dependency has no package record: serde'));
  const withoutKinds = graph();
  withoutKinds.resolve.nodes.find((node) => node.id === 'domain').deps[0].dep_kinds = [];
  assert.ok(inspect(withoutKinds).violations.includes('Cargo dependency kind/target evidence is incomplete for domain -> serde.'));
});

test('cycles terminate and diagnostics use the shortest dependency path', () => {
  const metadata = graph();
  connect(metadata, 'domain', 'session');
  connect(metadata, 'domain', 'image');
  connect(metadata, 'session', 'image', 'direct');
  assert.deepEqual(inspect(metadata).violations, ['Headless session reaches native/image support image (image): verdant-session --direct [normal]--> image']);
});

test('native output path matching respects directory boundaries', () => {
  const metadata = graph();
  add(metadata, packageOf('helper', 'headless-helper', `${contract.nativeDirectories[0]}-helpers/Cargo.toml`));
  connect(metadata, 'domain', 'helper');
  assert.deepEqual(inspect(metadata).violations, []);
});

test('the pin must declare real Rust data targets before admitting a headless graph', () => {
  const previous = structuredClone(pin);
  previous.units.find((unit) => unit.name === 'note-gesture-planner').target.profile = 'gpui-source-v1';
  assert.throws(() => headlessSessionContract(previous, '/work/frontend'), /must be a pinned Rust data unit/);
  const unsafe = structuredClone(pin);
  unsafe.units[0].output.directory = 'frontend/../elsewhere';
  assert.throws(() => headlessSessionContract(unsafe, '/work/frontend'), /Invalid pinned output directory/);
});

test('Windows manifest paths are matched by identity instead of slash or drive casing', () => {
  const windowsContract = headlessSessionContract(pin, 'C:\\repo\\frontend');
  const metadata = graph();
  metadata.workspace_root = 'C:\\repo\\frontend';
  for (const pkg of metadata.packages) {
    pkg.manifest_path = pkg.manifest_path.replace('/work/frontend', 'C:/REPO/frontend').replaceAll('/', '\\');
  }
  // Registry manifests also have an absolute drive on Windows.
  for (const pkg of metadata.packages.filter((pkg) => pkg.source !== null)) pkg.manifest_path = `C:${pkg.manifest_path}`;
  assert.deepEqual(inspectHeadlessSessionGraph(metadata, windowsContract).violations, []);
});

const manifest = '[package]\nname = "verdant-note-gesture-data"\nversion = "0.1.0"\n[dependencies]\n';
const provenance = { format: 'proto-ui-rust-data', backend: 'rust-data-v1', profile: 'rust-data-v1', dependencies: [], methods: [{ name: 'planNoteGesture' }] };
const dataInput = { crateManifest: manifest, provenance, expectedPackageName: 'verdant-note-gesture-data', expectedMethods: ['planNoteGesture'] };
test('the Rust data admission binds package, backend, methods, and an empty dependency closure', () => {
  assert.deepEqual(findRustDataViolations(dataInput), []);
  assert.deepEqual(findRustDataViolations({ ...dataInput, expectedMethods: ['wrong'] }), ['the Rust data methods differ from the pinned exposed-method order']);
  assert.deepEqual(findRustDataViolations({ ...dataInput, provenance: { ...provenance, nativeSdk: {} } }), ['the Rust data provenance must not declare a native SDK']);
  assert.deepEqual(findRustDataViolations({ ...dataInput, provenance: { ...provenance, dependencies: [{ name: 'gpui' }] } }), ['the std-only Rust data provenance must declare no dependencies']);
});

for (const table of ['dependencies', 'dev-dependencies', 'build-dependencies', 'target."cfg(unix)".dependencies', 'target."cfg(windows)".build-dependencies', 'target."cfg(unix)".dev-dependencies']) {
  test(`std-only admission rejects an aliased dependency in ${table}`, () => {
    const crateManifest = `${manifest.replace('[dependencies]\n', '')}\n[${table}.helper]\npackage = "gpui"\nversion = "0.2"\n`;
    assert.deepEqual(findRustDataViolations({ ...dataInput, crateManifest }), [`the std-only Rust data crate declares a dependency in ${table}.helper`]);
  });
}
