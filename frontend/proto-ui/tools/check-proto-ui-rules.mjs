#!/usr/bin/env node
// Fixture-driven check of the single-workspace GPUI admission rules.
//
// The fixtures are the compiler-shaped manifests the producer has to accept or
// reject. This check never touches the network, the upstream checkout or the
// Rust workspace; it only proves that the rules reject a second GPUI version.
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { findWorkspaceGpuiViolations } from './lib/workspace-gpui.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.resolve(scriptDirectory, '../fixtures/workspace-gpui');

const read = (name) => readFileSync(path.join(fixtures, name), 'utf8');
const readJson = (name) => JSON.parse(read(name));

const admitted = {
  crateManifest: read('crate-manifest.workspace.toml'),
  sdkManifest: read('sdk-manifest.workspace.toml'),
  provenance: readJson('provenance.workspace.json'),
};

const cases = [
  {
    name: 'the workspace-shaped artifact set is admitted',
    input: admitted,
    expected: [],
  },
  {
    name: 'a generated crate that pins gpui is rejected',
    input: { ...admitted, crateManifest: read('crate-manifest.pinned.toml') },
    expected: ['the generated crate manifest carries a gpui revision pin'],
  },
  {
    name: 'a generated crate that declares gpui twice is rejected',
    input: { ...admitted, crateManifest: read('crate-manifest.duplicate.toml') },
    expected: ['the generated crate manifest declares gpui more than once in dependencies'],
  },
  {
    name: 'a bundled native SDK that pins gpui is rejected',
    input: { ...admitted, sdkManifest: read('sdk-manifest.pinned.toml') },
    expected: ['the bundled native SDK manifest carries a gpui revision pin'],
  },
  {
    name: 'provenance that reports a pinned gpui is rejected',
    input: { ...admitted, provenance: readJson('provenance.pinned.json') },
    expected: ['the compiler provenance pins gpui instead of the workspace: 62e5991dd0f0c8a3af8d5e7e9c4652490d468db8'],
  },
];

let failures = 0;
for (const testCase of cases) {
  const violations = findWorkspaceGpuiViolations(testCase.input);
  const missing = testCase.expected.filter((expected) => !violations.includes(expected));
  if (missing.length > 0 || (testCase.expected.length === 0 && violations.length > 0)) {
    failures += 1;
    console.error(
      `FAIL ${testCase.name}: expected ${JSON.stringify(missing)} in ${JSON.stringify(violations)}`,
    );
    continue;
  }
  console.log(`ok ${testCase.name}`);
}

if (failures > 0) {
  console.error(`${failures} workspace GPUI rule case(s) failed.`);
  process.exit(1);
}
console.log('all workspace GPUI rule cases passed.');
