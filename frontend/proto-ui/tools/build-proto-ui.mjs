#!/usr/bin/env node
// Verdant Proto UI compiler producer.
//
// Compiles one authored Proto UI unit with the pinned upstream Proto UI
// compiler and publishes the compiler-owned artifact set into the pinned
// output directory. The generated Rust is copied verbatim: this producer never
// rewrites compiler output.
//
// The producer refuses every upstream source state other than the one named by
// the explicit pin, and it admits only artifact sets that inherit a single
// GPUI dependency from the embedding Cargo workspace.
//
// Usage:
//   node frontend/proto-ui/tools/build-proto-ui.mjs [--pin <file>] [--upstream <dir>] [--source <file>]
//        [--out <dir>] [--receipt <file>] [--check]
//
// Exit codes:
//   0  artifacts published, or the published artifacts match a fresh compilation
//   2  the upstream revision differs from the pin
//   3  the upstream source state differs from the pin
//   4  the pin or an argument is invalid
//   5  the compiler toolchain is unavailable
//   6  the compiler rejected the authored source
//   7  the published artifacts differ from a fresh compilation
//   8  the emitted artifacts are not a single-workspace GPUI artifact set
//   9  refused to replace a directory that is not a compiler artifact set
//  10  the pinned upstream checkout is absent
//  11  no published artifacts exist yet
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { findWorkspaceGpuiViolations } from './lib/workspace-gpui.mjs';

const EXIT = {
  ok: 0,
  revision: 2,
  sourceState: 3,
  pin: 4,
  toolchain: 5,
  compile: 6,
  drift: 7,
  admission: 8,
  replace: 9,
  upstreamAbsent: 10,
  artifactsAbsent: 11,
};

class ProducerError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDirectory, '../../..');
const sha256 = (value) => createHash('sha256').update(value).digest('hex');

function parseArguments(argv) {
  const options = {
    pin: path.join(repoRoot, 'frontend/proto-ui/compiler.pin.json'),
    check: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const next = () => {
      index += 1;
      const value = argv[index];
      if (typeof value !== 'string' || value === '') {
        throw new ProducerError(EXIT.pin, `${argument} needs a value.`);
      }
      return value;
    };
    if (argument === '--check') options.check = true;
    else if (argument === '--pin') options.pin = path.resolve(next());
    else if (argument === '--upstream') options.upstream = path.resolve(next());
    else if (argument === '--source') options.source = path.resolve(next());
    else if (argument === '--out') options.out = path.resolve(next());
    else if (argument === '--receipt') options.receipt = path.resolve(next());
    else throw new ProducerError(EXIT.pin, `Unknown argument: ${argument}`);
  }
  return options;
}

function loadPin(file) {
  let pin;
  try {
    pin = JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    throw new ProducerError(EXIT.pin, `Unreadable pin ${file}: ${error.message}`);
  }
  const requireRevision = (value, label) => {
    if (typeof value !== 'string' || !/^[0-9a-f]{40}$/.test(value)) {
      throw new ProducerError(EXIT.pin, `Pin ${label} is not a full 40-hex revision: ${value}`);
    }
  };
  if (!pin || typeof pin !== 'object') throw new ProducerError(EXIT.pin, 'The pin is not an object.');
  if (pin.schema !== 'verdant.proto-ui.compiler-pin.v1') {
    throw new ProducerError(EXIT.pin, `Unsupported pin schema: ${pin.schema}`);
  }
  if (!pin.upstream || typeof pin.upstream.repository !== 'string') {
    throw new ProducerError(EXIT.pin, 'The pin does not name an upstream repository.');
  }
  requireRevision(pin.upstream.revision, 'upstream.revision');
  requireRevision(pin.gpu?.workspaceGpuiRevision, 'gpu.workspaceGpuiRevision');
  requireRevision(pin.gpu?.upstreamGpuiRevision, 'gpu.upstreamGpuiRevision');
  if (pin.target?.profile !== 'gpui-source-v1') {
    throw new ProducerError(EXIT.pin, `Unsupported target profile: ${pin.target?.profile}`);
  }
  if (pin.target?.gpuiDependency !== 'workspace') {
    throw new ProducerError(EXIT.pin, 'The pin must request the workspace GPUI dependency.');
  }
  for (const key of ['source', 'output.directory', 'output.receipt']) {
    const value = key.split('.').reduce((node, part) => node?.[part], pin);
    if (typeof value !== 'string' || value === '') {
      throw new ProducerError(EXIT.pin, `The pin does not name ${key}.`);
    }
  }
  return pin;
}

function git(upstream, args) {
  try {
    return execFileSync('git', ['-C', upstream, ...args], { encoding: 'utf8' }).trim();
  } catch (error) {
    throw new ProducerError(EXIT.sourceState, `git ${args.join(' ')} failed in ${upstream}: ${error.message}`);
  }
}

function verifyUpstreamSource(upstream, pin) {
  const head = git(upstream, ['rev-parse', 'HEAD']);
  if (head !== pin.upstream.revision) {
    throw new ProducerError(
      EXIT.revision,
      `The pinned upstream revision ${pin.upstream.revision} is not the checked-out revision ${head}. ` +
        'Point the pin at the revision that is actually checked out, or check out the pinned revision.',
    );
  }
  const modified = git(upstream, ['status', '--porcelain', '--untracked-files=all']);
  if (modified) {
    throw new ProducerError(EXIT.sourceState, `The pinned compiler checkout has local changes:\n${modified}`);
  }
}

function verifyPackageVersions(upstream, pin) {
  const version = readFileSync(path.join(upstream, 'VERSION'), 'utf8').trim();
  if (version !== pin.packageVersions.protoUi) {
    throw new ProducerError(
      EXIT.sourceState,
      `The upstream package version ${version} does not match the pinned ${pin.packageVersions.protoUi}.`,
    );
  }
  const manifest = JSON.parse(readFileSync(path.join(upstream, 'package.json'), 'utf8'));
  if (manifest.packageManager !== pin.packageVersions.packageManager) {
    throw new ProducerError(
      EXIT.sourceState,
      `The upstream package manager ${manifest.packageManager} does not match the pinned ${pin.packageVersions.packageManager}.`,
    );
  }
  if (manifest.engines?.node !== pin.packageVersions.nodeEngine) {
    throw new ProducerError(
      EXIT.sourceState,
      `The upstream Node engine ${manifest.engines?.node} does not match the pinned ${pin.packageVersions.nodeEngine}.`,
    );
  }
}

function verifyToolchain(upstream) {
  if (typeof process.features.typescript !== 'string') {
    throw new ProducerError(
      EXIT.toolchain,
      `Node ${process.version} cannot run TypeScript sources at all; use Node 24 or newer.`,
    );
  }
  const require = createRequire(path.join(upstream, 'packages/compiler/package.json'));
  try {
    require.resolve('typescript');
  } catch {
    throw new ProducerError(
      EXIT.toolchain,
      `The pinned upstream checkout has no installed compiler dependencies. Install them with ` +
        `pnpm install --frozen-lockfile in ${upstream}.`,
    );
  }
  try {
    createRequire(path.join(upstream, 'package.json')).resolve('tsx/esm/api');
  } catch {
    throw new ProducerError(
      EXIT.toolchain,
      `The pinned upstream checkout has no TypeScript loader. Install it with ` +
        `pnpm install --frozen-lockfile in ${upstream}.`,
    );
  }
}

function readWorkspaceGpuiRevision() {
  const manifest = readFileSync(path.join(repoRoot, 'frontend/Cargo.toml'), 'utf8');
  const match = /^\s*gpui\s*=\s*\{[^}]*rev\s*=\s*"([0-9a-f]{40})"/m.exec(manifest);
  if (!match) throw new ProducerError(EXIT.admission, 'frontend/Cargo.toml does not pin a gpui revision.');
  return match[1];
}

async function registerTypeScriptLoader(upstream) {
  const apiPath = createRequire(path.join(upstream, 'package.json')).resolve('tsx/esm/api');
  const { register } = await import(pathToFileURL(apiPath).href);
  register();
}

async function compileUnit(upstream, sourcePath, pin) {
  const compilerUrl = pathToFileURL(path.join(upstream, 'packages/compiler/src/compile.ts')).href;
  const compiler = await import(compilerUrl);
  const source = readFileSync(sourcePath, 'utf8');
  const compiled = compiler.compilePrototype(source, {
    fileName: path.basename(sourcePath),
    profile: pin.target.profile,
    componentName: pin.target.componentName,
    gpuiDependency: pin.target.gpuiDependency,
  });
  if (!compiled.ok) {
    throw new ProducerError(
      EXIT.compile,
      `The compiler rejected ${path.relative(repoRoot, sourcePath)}:\n${JSON.stringify(compiled.diagnostics, null, 2)}`,
    );
  }
  return {
    compilation: compiled.value,
    artifacts: compiler.compilationArtifacts(compiled.value),
    diffCompilation: compiler.diffCompilation,
  };
}

const artifactText = (artifacts, artifactPath) =>
  artifacts.find((artifact) => artifact.path === artifactPath)?.contents;
const artifactJson = (artifacts, artifactPath) => {
  const contents = artifactText(artifacts, artifactPath);
  if (typeof contents !== 'string') return undefined;
  try {
    return JSON.parse(contents);
  } catch {
    return undefined;
  }
};

function buildReceipt({ pin, artifacts, sourcePath }) {
  return {
    schema: 'verdant.proto-ui.receipt.v1',
    unit: path.basename(pin.output.directory),
    source: path.relative(repoRoot, sourcePath),
    sourceSha256: sha256(readFileSync(sourcePath)),
    upstream: {
      repository: pin.upstream.repository,
      revision: pin.upstream.revision,
    },
    packageVersions: pin.packageVersions,
    target: pin.target,
    gpu: pin.gpu,
    artifacts: artifacts
      .map((artifact) => ({ path: artifact.path, kind: artifact.kind, sha256: sha256(artifact.contents) }))
      .sort((left, right) => left.path.localeCompare(right.path)),
  };
}

function publishArtifacts({ artifacts, outDirectory }) {
  if (existsSync(outDirectory)) {
    const entries = readdirSync(outDirectory);
    if (!entries.includes('provenance.json')) {
      throw new ProducerError(
        EXIT.replace,
        `Refusing to replace ${outDirectory}: it is not a compiler artifact directory.`,
      );
    }
  }
  const staging = `${outDirectory}.staging`;
  rmSync(staging, { recursive: true, force: true });
  mkdirSync(staging, { recursive: true });
  for (const artifact of artifacts) {
    const target = path.join(staging, artifact.path);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, artifact.contents);
  }
  rmSync(outDirectory, { recursive: true, force: true });
  renameSync(staging, outDirectory);
}

async function verifyPublished({ compilation, diffCompilation, outDirectory, receipt, receiptPath }) {
  if (!existsSync(outDirectory)) {
    throw new ProducerError(
      EXIT.artifactsAbsent,
      `No published Proto UI artifacts in ${outDirectory}. Publish them before checking them.`,
    );
  }
  const diff = await diffCompilation(compilation, outDirectory);
  if (!diff.ok) {
    throw new ProducerError(
      EXIT.drift,
      `The published artifacts are not a compiler artifact set:\n${JSON.stringify(diff.diagnostics, null, 2)}`,
    );
  }
  const drift = diff.value.changes.filter(
    (change) => change.status !== 'unchanged' || change.consumerModified,
  );
  if (drift.length > 0) {
    throw new ProducerError(
      EXIT.drift,
      `The published artifacts differ from a fresh compilation:\n${JSON.stringify(drift, null, 2)}`,
    );
  }
  if (!existsSync(receiptPath)) {
    throw new ProducerError(EXIT.drift, `The published receipt ${receiptPath} is absent.`);
  }
  const expected = `${JSON.stringify(receipt, null, 2)}\n`;
  if (readFileSync(receiptPath, 'utf8') !== expected) {
    throw new ProducerError(
      EXIT.drift,
      `The published receipt ${receiptPath} does not describe the pinned compilation.`,
    );
  }
}

async function main(argv) {
  const options = parseArguments(argv);
  const pin = loadPin(options.pin);
  const sourcePath = options.source ?? path.resolve(repoRoot, pin.source);
  const outDirectory = options.out ?? path.resolve(repoRoot, pin.output.directory);
  const receiptPath = options.receipt ?? path.resolve(repoRoot, pin.output.receipt);
  if (options.check) {
    if (!existsSync(outDirectory) || !existsSync(receiptPath)) {
      throw new ProducerError(EXIT.artifactsAbsent, 'Published Proto UI artifacts or their receipt are absent.');
    }
    const retained = JSON.parse(readFileSync(receiptPath, 'utf8'));
    if (!Array.isArray(retained.artifacts) || retained.artifacts.length === 0) {
      throw new ProducerError(EXIT.drift, 'The retained Proto UI receipt has no artifact records.');
    }
    for (const required of ['Component.rs', 'Cargo.toml', 'provenance.json', '.proto-ui/gpui-native/Cargo.toml']) {
      if (!retained.artifacts.some((artifact) => artifact.path === required)) {
        throw new ProducerError(EXIT.drift, `Required retained artifact is unrecorded: ${required}`);
      }
    }
    const artifacts = retained.artifacts.map((artifact) => {
      const filename = path.resolve(outDirectory, artifact.path);
      if (!filename.startsWith(`${outDirectory}${path.sep}`)) {
        throw new ProducerError(EXIT.drift, `Invalid retained artifact path: ${artifact.path}`);
      }
      return { path: artifact.path, kind: artifact.kind, contents: readFileSync(filename, 'utf8') };
    });
    const expected = buildReceipt({ pin, artifacts, sourcePath });
    if (JSON.stringify(retained) !== JSON.stringify(expected)) {
      throw new ProducerError(EXIT.drift, 'Retained Proto UI artifacts, source or pin differ from their receipt.');
    }
    const violations = findWorkspaceGpuiViolations({
      crateManifest: artifactText(artifacts, 'Cargo.toml'),
      sdkManifest: artifactText(artifacts, '.proto-ui/gpui-native/Cargo.toml'),
      provenance: artifactJson(artifacts, 'provenance.json'),
    });
    if (violations.length) {
      throw new ProducerError(EXIT.admission, violations.join('\n'));
    }
  }
  const upstream = options.upstream ?? path.resolve(repoRoot, '../Proto-UI');
  if (!existsSync(upstream)) {
    throw new ProducerError(
      EXIT.upstreamAbsent,
      `The pinned upstream checkout ${upstream} is absent.`,
    );
  }
  verifyUpstreamSource(upstream, pin);
  verifyPackageVersions(upstream, pin);
  verifyToolchain(upstream);

  const workspaceRevision = readWorkspaceGpuiRevision();
  if (workspaceRevision !== pin.gpu.workspaceGpuiRevision) {
    throw new ProducerError(
      EXIT.admission,
      `frontend/Cargo.toml pins gpui ${workspaceRevision} but the pin records ${pin.gpu.workspaceGpuiRevision}.`,
    );
  }

  await registerTypeScriptLoader(upstream);
  const { compilation, artifacts, diffCompilation } = await compileUnit(upstream, sourcePath, pin);

  const violations = findWorkspaceGpuiViolations({
    crateManifest: artifactText(artifacts, 'Cargo.toml'),
    sdkManifest: artifactText(artifacts, '.proto-ui/gpui-native/Cargo.toml'),
    provenance: artifactJson(artifacts, 'provenance.json'),
  });
  if (violations.length > 0) {
    throw new ProducerError(
      EXIT.admission,
      `The emitted artifacts are not a single-workspace GPUI artifact set:\n- ${violations.join('\n- ')}`,
    );
  }

  const receipt = buildReceipt({ pin, artifacts, sourcePath });

  if (options.check) {
    await verifyPublished({ compilation, diffCompilation, outDirectory, receipt, receiptPath });
  } else {
    publishArtifacts({ artifacts, outDirectory });
    mkdirSync(path.dirname(receiptPath), { recursive: true });
    writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);
  }

  const crateManifest = artifactText(artifacts, 'Cargo.toml') ?? '';
  const packageName = /name\s*=\s*"([^"]+)"/.exec(crateManifest)?.[1] ?? 'unknown';
  console.log(
    JSON.stringify(
      {
        ok: true,
        mode: options.check ? 'check' : 'publish',
        unit: path.basename(pin.output.directory),
        upstreamRevision: pin.upstream.revision,
        packageName,
        componentName: pin.target.componentName,
        artifacts: artifacts.length,
        outputDirectory: path.relative(repoRoot, outDirectory),
        receipt: path.relative(repoRoot, receiptPath),
      },
      null,
      2,
    ),
  );
}

try {
  await main(process.argv.slice(2));
} catch (error) {
  if (error instanceof ProducerError) {
    console.error(error.message);
    process.exit(error.code);
  }
  console.error(error?.stack ?? String(error));
  process.exit(EXIT.toolchain);
}
