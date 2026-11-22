#!/usr/bin/env node
// This is a Cargo resolve-graph gate, not a Cargo.lock text or --no-deps check.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { findCargoGpuiGraphViolations, readWorkspaceGpuiIdentity } from './lib/workspace-gpui.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
let manifest = path.resolve(here, '../../Cargo.toml');
let metadataFile;
let offline = false;
try {
  const args = process.argv.slice(2);
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === '--offline') offline = true;
    else if (argument === '--manifest' || argument === '--metadata') {
      const value = args[++index];
      if (!value || value.startsWith('--')) throw new Error(`${argument} requires a file.`);
      if (argument === '--manifest') manifest = path.resolve(value);
      else metadataFile = path.resolve(value);
    } else throw new Error(`Unknown argument: ${argument}`);
  }
  const expected = readWorkspaceGpuiIdentity(readFileSync(manifest, 'utf8'));
  const cargoArgs = ['metadata', '--format-version', '1', '--locked', '--all-features', '--manifest-path', manifest];
  if (offline) cargoArgs.push('--offline');
  // No --filter-platform: target-specific dependencies must be included too.
  const metadata = JSON.parse(metadataFile ? readFileSync(metadataFile, 'utf8')
    : execFileSync('cargo', cargoArgs, {
      cwd: path.dirname(manifest), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'inherit'],
    }));
  const violations = findCargoGpuiGraphViolations(metadata, expected);
  if (violations.length > 0) throw new Error(violations.join('\n'));
  console.log(JSON.stringify({
    evidence: 'cargo-resolve-graph',
    command: metadataFile ? ['retained-metadata', metadataFile] : ['cargo', ...cargoArgs],
    workspace: expected,
    gpui: metadata.packages.filter((pkg) => pkg.name === 'gpui').map(({ id, source }) => ({ id, source })),
    sharedSdk: metadata.packages.filter((pkg) => pkg.name === 'proto-ui-gpui-native').map(({ id }) => id),
  }, null, 2));
} catch (error) {
  console.error(`Workspace GPUI graph check failed: ${error.message}`);
  process.exitCode = 1;
}
