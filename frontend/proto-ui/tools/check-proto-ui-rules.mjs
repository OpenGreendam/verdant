#!/usr/bin/env node
// Regression tests for TOML admission and Cargo graph validation. These are
// fixture tests; a successful run alone is not evidence about a real Cargo graph.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  findCargoGpuiGraphViolations, findWorkspaceGpuiViolations,
  parseCargoManifest, parseDependencyEntries, readWorkspaceGpuiIdentity,
} from './lib/workspace-gpui.mjs';

const fixtures = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../fixtures/workspace-gpui');
const read = (name) => readFileSync(path.join(fixtures, name), 'utf8');
const revision = 'e5f2f9db75e97613232b5c1946333e61fa3828ab';
const repository = 'https://github.com/cyjin-yl/z3rm';
const workspaceManifest = `[workspace.dependencies]\ngpui = { git = "${repository}", rev = "${revision}" }\n`;
const admitted = {
  crateManifest: read('crate-manifest.workspace.toml'),
  sdkManifest: read('sdk-manifest.workspace.toml'),
  provenance: JSON.parse(read('provenance.workspace.json')),
  workspaceManifest,
};
let passed = 0;
function test(name, callback) {
  try {
    callback();
    passed += 1;
    console.log(`ok ${name}`);
  } catch (error) {
    console.error(`FAIL ${name}\n${error.stack}`);
    process.exitCode = 1;
  }
}
function admission(name, input, expected) {
  test(name, () => assert.deepEqual(findWorkspaceGpuiViolations(input), expected));
}
const insert = (manifest, kind, body) => manifest.includes(`[${kind}]\n`)
  ? manifest.replace(`[${kind}]\n`, `[${kind}]\n${body}\n`)
  : `${manifest}\n[${kind}]\n${body}\n`;
const rejected = (table, alias = 'gpui_legacy', label = 'the generated crate manifest') =>
  `${label} does not inherit workspace gpui in ${table}.${alias}`;

admission('compiler-shaped workspace manifests', admitted, []);
admission('duplicate TOML keys are invalid, not a second parsed declaration', {
  ...admitted, crateManifest: read('crate-manifest.duplicate.toml'),
}, ['the generated crate manifest is not valid TOML']);
admission('pinned component GPUI in ordinary and development tables', {
  ...admitted, crateManifest: read('crate-manifest.pinned.toml'),
}, [rejected('dependencies', 'gpui'), rejected('dev-dependencies', 'gpui')]);
admission('pinned shared SDK GPUI', {
  ...admitted, sdkManifest: read('sdk-manifest.pinned.toml'),
}, [rejected('dependencies', 'gpui', 'the shared native SDK manifest')]);
admission('compiler provenance must inherit workspace', {
  ...admitted, provenance: JSON.parse(read('provenance.pinned.json')),
}, ['the compiler provenance pins gpui instead of the workspace: 62e5991dd0f0c8a3af8d5e7e9c4652490d468db8']);
admission('comments, spaces and single quotes do not change inheritance', {
  ...admitted, crateManifest: admitted.crateManifest.replace('gpui = { workspace = true }',
    "'gpui' = {workspace=true} # a legal Cargo dependency"),
}, []);
admission('multiline feature arrays and dependency subtables', {
  ...admitted, crateManifest: admitted.crateManifest.replace('gpui = { workspace = true }\n', '')
    + '\n[dependencies.gpui]\nworkspace = true\nfeatures = [\n  "test-support", # comment\n]\n',
}, []);
admission('ordinary non-GPUI revision pins remain valid', {
  ...admitted, crateManifest: insert(admitted.crateManifest, 'dependencies',
    `unrelated = { git = "https://example.com/other", rev = "${'a'.repeat(40)}" }`),
}, []);
admission('metadata mentioning gpui does not declare a dependency', {
  ...admitted, crateManifest: admitted.crateManifest
    + '\n[package.metadata.notes]\ngpui = "another package in prose"\n',
}, []);

for (const kind of ['dependencies', 'dev-dependencies', 'build-dependencies']) {
  for (const target of [null, 'cfg(unix)', 'x86_64-pc-windows-msvc']) {
    const table = target ? `target.${JSON.stringify(target)}.${kind}` : kind;
    const inject = (body) => target
      ? `${admitted.crateManifest}\n[${table}]\n${body}\n`
      : insert(admitted.crateManifest, kind, body);
    for (const shape of ['inline', 'subtable', 'dotted']) {
      let manifest;
      if (shape === 'inline') manifest = inject('gpui_legacy = { package = "gpui", version = "=0.2.2" }');
      else if (shape === 'dotted') manifest = inject('gpui_legacy.package = "gpui"\ngpui_legacy.version = "=0.2.2"');
      else manifest = admitted.crateManifest + `\n[${table}.gpui_legacy]\npackage = "gpui"\nversion = "=0.2.2"\n`;
      test(`${table} ${shape} probe is legal TOML`, () => parseCargoManifest(manifest));
      admission(`reject ${table} package alias in ${shape} syntax`, {
        ...admitted, crateManifest: manifest,
      }, [rejected(table)]);
    }
  }
}

admission('bare GPUI version in a build table', {
  ...admitted, crateManifest: insert(admitted.crateManifest, 'build-dependencies', 'gpui = "=0.2.2"'),
}, [rejected('build-dependencies', 'gpui')]);
admission('SDK aliases and target development tables are inspected', {
  ...admitted, sdkManifest: `${admitted.sdkManifest}\n[target.'cfg(unix)'.dev-dependencies]\ngpui_legacy = { package = "gpui", version = "=0.2.2" }\n`,
}, [
  'the shared native SDK manifest declares gpui in target."cfg(unix)".dev-dependencies.gpui_legacy',
  rejected('target."cfg(unix)".dev-dependencies', 'gpui_legacy', 'the shared native SDK manifest'),
]);
admission('an inherited workspace alias cannot hide another source', {
  ...admitted,
  workspaceManifest: `${workspaceManifest}gpui_legacy = { package = "gpui", version = "=0.2.2" }\n`,
  crateManifest: insert(admitted.crateManifest, 'dependencies', 'gpui_legacy = { workspace = true }'),
}, ['the generated crate manifest inherits a different GPUI source in dependencies.gpui_legacy']);
admission('a workspace alias to the same pinned source is permitted', {
  ...admitted,
  workspaceManifest: `${workspaceManifest}gpui_legacy = { package = "gpui", git = "${repository}", rev = "${revision}" }\n`,
  crateManifest: insert(admitted.crateManifest, 'dependencies', 'gpui_legacy = { workspace = true }'),
}, []);
admission('inheritance cannot also select a version', {
  ...admitted, crateManifest: admitted.crateManifest.replace('gpui = { workspace = true }',
    'gpui = { workspace = true, version = "=0.2.2" }'),
}, ['the generated crate manifest adds non-inherited gpui keys in dependencies.gpui: version']);
admission('missing inherited dependency fails', {
  ...admitted, workspaceManifest: '[workspace.dependencies]\nserde = "1"\n',
}, [
  'the generated crate manifest inherits missing workspace dependency in dependencies.gpui',
  'the generated crate manifest inherits missing workspace dependency in dev-dependencies.gpui',
  'the shared native SDK manifest inherits missing workspace dependency in dependencies.gpui',
]);

test('workspace identity reads only [workspace.dependencies]', () => {
  assert.deepEqual(readWorkspaceGpuiIdentity(`[package.metadata]\ngpui = { rev = "${'a'.repeat(40)}" }\n${workspaceManifest}`), {
    name: 'gpui', git: repository, rev: revision, features: [], defaultFeatures: true,
  });
  assert.throws(() => readWorkspaceGpuiIdentity(`[dependencies]\ngpui = { git = "${repository}", rev = "${revision}" }`), /workspace.dependencies/);
});
test('workspace feature/source identity is explicit', () => {
  assert.deepEqual(readWorkspaceGpuiIdentity(`${workspaceManifest.replace(' }', ', features = ["z", "a"], default-features = false }')}`), {
    name: 'gpui', git: repository, rev: revision, features: ['a', 'z'], defaultFeatures: false,
  });
  assert.throws(() => readWorkspaceGpuiIdentity(workspaceManifest.replace(' }', ', path = "elsewhere" }')), /cannot also declare path/);
});
test('entry API preserves actual package, dependency kind, and target', () => {
  const entries = parseDependencyEntries('[target.\'cfg(unix)\'.build-dependencies.alias]\npackage = "gpui"\nversion = "0.2"');
  assert.deepEqual(entries.map(({ name, package: pkg, kind, table }) => ({ name, package: pkg, kind, table })), [
    { name: 'alias', package: 'gpui', kind: 'build-dependencies', table: 'target."cfg(unix)".build-dependencies' },
  ]);
});

const expected = readWorkspaceGpuiIdentity(workspaceManifest);
const source = `git+${repository}?rev=${revision}#${revision}`;
function graph() {
  const names = { app: 'verdant-app', a: 'generated-gpui-a', b: 'generated-gpui-b', sdk: 'proto-ui-gpui-native', gpui: 'gpui' };
  const edges = { app: ['a', 'b'], a: ['sdk', 'gpui'], b: ['sdk', 'gpui'], sdk: ['gpui'], gpui: [] };
  return {
    workspace_members: ['app', 'a', 'b'],
    packages: Object.entries(names).map(([id, name]) => ({ id, name, source: id === 'gpui' ? source : null, features: id === 'gpui' ? { default: [] } : {} })),
    resolve: { nodes: Object.entries(edges).map(([id, deps]) => ({ id, deps: deps.map((pkg) => ({ name: pkg, pkg, dep_kinds: [{ kind: null, target: null }] })), features: id === 'gpui' ? ['default'] : [] })) },
  };
}
const graphTest = (name, mutate, diagnostics) => test(name, () => {
  const metadata = graph();
  mutate(metadata);
  assert.deepEqual(findCargoGpuiGraphViolations(metadata, expected), diagnostics);
});
graphTest('two generated consumers share one GPUI and one SDK', () => {}, []);
graphTest('--no-deps is not graph evidence', (m) => { m.resolve = null; }, [
  'full Cargo metadata with workspace members and a resolve graph is required (--no-deps is insufficient)',
]);
graphTest('transitive aliased target GPUI creates a second package identity', (m) => {
  m.packages.push({ id: 'registry-gpui', name: 'gpui', source: 'registry+https://github.com/rust-lang/crates.io-index' });
  m.resolve.nodes.push({ id: 'registry-gpui', deps: [], features: [] });
  m.resolve.nodes.find((n) => n.id === 'sdk').deps.push({ name: 'legacy', pkg: 'registry-gpui', dep_kinds: [{ kind: 'build', target: 'cfg(windows)' }] });
}, [
  'Cargo resolves 2 reachable GPUI package identities; expected exactly one: gpui, registry-gpui',
  `Cargo GPUI source differs from workspace ${repository}@${revision}: registry+https://github.com/rust-lang/crates.io-index`,
]);
graphTest('same revision from another repository is another identity', (m) => {
  m.packages.find((p) => p.id === 'gpui').source = `git+https://example.com/fork?rev=${revision}#${revision}`;
}, [`Cargo GPUI source differs from workspace ${repository}@${revision}: git+https://example.com/fork?rev=${revision}#${revision}`]);
graphTest('resolved commit must match the declared revision', (m) => {
  m.packages.find((p) => p.id === 'gpui').source = `git+${repository}?rev=${revision}#${'a'.repeat(40)}`;
}, [`Cargo GPUI source differs from workspace ${repository}@${revision}: git+${repository}?rev=${revision}#${'a'.repeat(40)}`]);
graphTest('a local GPUI patch cannot masquerade as the pinned Git source', (m) => {
  m.packages.find((p) => p.id === 'gpui').source = null;
}, [`Cargo GPUI source differs from workspace ${repository}@${revision}: null`]);
graphTest('unreachable package records do not create a false second GPUI', (m) => {
  m.packages.push({ id: 'unreachable-gpui', name: 'gpui', source: 'registry+unused' });
}, []);
graphTest('two native SDK owners are rejected', (m) => {
  m.packages.push({ id: 'sdk-copy', name: 'proto-ui-gpui-native', source: null });
  m.resolve.nodes.push({ id: 'sdk-copy', deps: [{ name: 'gpui', pkg: 'gpui' }], features: [] });
  m.resolve.nodes.find((n) => n.id === 'b').deps.push({ name: 'native', pkg: 'sdk-copy' });
}, ['Cargo resolves 2 reachable Proto-UI native SDK identities; expected one shared SDK']);
graphTest('required default features are part of the graph check', (m) => {
  m.resolve.nodes.find((n) => n.id === 'gpui').features = [];
}, ['Cargo GPUI does not activate the workspace default features']);
test('workspace feature activation is checked', () => {
  assert.deepEqual(findCargoGpuiGraphViolations(graph(), { ...expected, features: ['test-support'] }), [
    'Cargo GPUI is missing workspace feature: test-support',
  ]);
});

if (process.exitCode) console.error('Workspace GPUI regression checks failed.');
else console.log(`${passed} workspace GPUI regression checks passed (fixtures; run check-cargo-gpui.mjs for the real graph).`);
