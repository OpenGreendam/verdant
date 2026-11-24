#!/usr/bin/env node
// Compile the pinned source graph into verbatim compiler-owned Rust artifacts.
//
// --check requires a fresh pinned compilation; --check-retained proves only the
// retained input/artifact/receipt bindings and never reports fresh compilation.
// All offline checks precede upstream discovery, in both modes.
//
// Usage: node frontend/proto-ui/tools/build-proto-ui.mjs
//   [--pin <file>] [--upstream <dir>] [--check | --check-retained]
//   [--recover-publication]
//
// Exit: 0 success; 2 upstream revision; 3 upstream source state; 4 invalid pin;
// 5 unavailable/inexact toolchain; 6 compiler rejection; 7 retained/fresh drift;
// 8 GPUI/SDK admission; 9 unsafe publication/lock; 10 absent upstream;
// 11 absent retained artifacts. Exit 10 is a failure, never a successful check.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { findRustDataViolations, findWorkspaceGpuiViolations, parseCargoManifest, readWorkspaceGpuiIdentity } from './lib/workspace-gpui.mjs';
import { acquireProducerLock, assertNoSymlinkAncestors, assertOwnedInventory, insidePath, publishBatch, recoverPublication } from './lib/artifact-publication.mjs';

export const EXIT = { ok: 0, revision: 2, sourceState: 3, pin: 4, toolchain: 5, compile: 6, drift: 7, admission: 8, replace: 9, upstreamAbsent: 10, artifactsAbsent: 11 };
export class ProducerError extends Error {
  constructor(code, message) { super(message); this.code = code; }
}
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const defaultRoot = path.resolve(scriptDirectory, '../../..');
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const slash = (value) => value.split(path.sep).join('/');
const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);
const artifactText = (artifacts, filename) => artifacts.find((artifact) => artifact.path === filename)?.contents;
const json = (filename) => JSON.parse(readFileSync(filename, 'utf8'));
const artifactJson = (artifacts, filename) => JSON.parse(artifactText(artifacts, filename));
const fail = (code, message) => { throw new ProducerError(code, message); };
const isDataUnit = (unit) => unit.target.profile === 'rust-data-v1';
function checked(code, work) {
  try { return work(); } catch (error) { if (error instanceof ProducerError) throw error; fail(code, error.message); }
}

export function parseArguments(argv, root = defaultRoot) {
  const options = { pin: path.join(root, 'frontend/proto-ui/compiler.pin.json'), mode: 'publish' };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = () => {
      const next = argv[++index];
      if (!next || next.startsWith('--')) fail(EXIT.pin, `${argument} needs a value.`);
      return path.resolve(next);
    };
    if (argument === '--pin') options.pin = value();
    else if (argument === '--upstream') options.upstream = value();
    else if (['--check', '--check-retained', '--recover-publication'].includes(argument)) {
      if (options.mode !== 'publish') fail(EXIT.pin, 'Choose one producer mode.');
      options.mode = argument.slice(2);
    } else fail(EXIT.pin, `Unknown argument: ${argument}`);
  }
  return options;
}

export function loadPin(filename, root = defaultRoot) {
  const pin = checked(EXIT.pin, () => json(filename));
  if (pin?.schema !== 'verdant.proto-ui.compiler-pin.v3') fail(EXIT.pin, `Unsupported pin schema: ${pin?.schema}`);
  for (const [label, value] of [['upstream.revision', pin.upstream?.revision], ['gpu.workspaceGpuiRevision', pin.gpu?.workspaceGpuiRevision], ['gpu.upstreamGpuiRevision', pin.gpu?.upstreamGpuiRevision]]) {
    if (!/^[0-9a-f]{40}$/.test(value ?? '')) fail(EXIT.pin, `Pin ${label} must be a full 40-hex revision.`);
  }
  if (typeof pin.upstream.repository !== 'string' || !pin.upstream.repository.startsWith('https://')) fail(EXIT.pin, 'Pin upstream.repository must be an HTTPS URL.');
  if (pin.target !== undefined) fail(EXIT.pin, 'Each compilation unit must declare its own target.');
  if (!pin.gpu.workspace || pin.gpu.workspace.name !== 'gpui' || pin.gpu.workspace.rev !== pin.gpu.workspaceGpuiRevision || typeof pin.gpu.workspace.git !== 'string' || !Array.isArray(pin.gpu.workspace.features) || typeof pin.gpu.workspace.defaultFeatures !== 'boolean') fail(EXIT.pin, 'Pin must describe the complete workspace GPUI identity.');
  if (!/^pnpm@\d+\.\d+\.\d+$/.test(pin.packageVersions?.packageManager ?? '') || pin.packageVersions.nodeEngine !== '24.x') fail(EXIT.pin, 'Pin requires Node 24.x and an exact pnpm version.');
  if (!/^[0-9a-f]{64}$/.test(pin.toolchain?.lockSha256 ?? '') || !pin.toolchain.packages?.typescript || !pin.toolchain.packages?.tsx) fail(EXIT.pin, 'Pin must bind the upstream lockfile and executed compiler package versions.');
  if (!Array.isArray(pin.units) || pin.units.length === 0) fail(EXIT.pin, 'Pin has no compilation units.');
  const names = new Set();
  const outputs = [];
  const receipts = new Set();
  const dataPackages = new Set();
  for (const unit of pin.units) {
    if (!/^[a-z][a-z0-9-]*$/.test(unit.name) || names.has(unit.name)) fail(EXIT.pin, `Invalid or duplicate unit: ${unit.name}`);
    names.add(unit.name);
    if (unit.componentName !== undefined || unit.nativeSdkPath !== undefined) fail(EXIT.pin, `${unit.name} must bind all compilation options under target.`);
    const target = unit.target;
    if (target?.profile === 'gpui-source-v1') {
      if (target.gpuiDependency !== 'workspace' || !/^[A-Z][A-Za-z0-9]*$/.test(target.componentName)) fail(EXIT.pin, `${unit.name} must name its workspace GPUI component target.`);
      if (Object.keys(target).some((key) => !['profile', 'gpuiDependency', 'componentName', 'nativeSdkPath'].includes(key))) fail(EXIT.pin, `${unit.name} has unknown native target options.`);
    } else if (target?.profile === 'rust-data-v1') {
      if (!/^[a-z][a-z0-9-]{0,63}$/.test(target.crateName) || dataPackages.has(target.crateName)) fail(EXIT.pin, `${unit.name} needs a distinct stable data crate name.`);
      dataPackages.add(target.crateName);
      if (!Array.isArray(target.methods) || !target.methods.length || target.methods.some((name) => !/^[A-Za-z][A-Za-z0-9]*$/.test(name)) || new Set(target.methods).size !== target.methods.length) fail(EXIT.pin, `${unit.name} must select distinct pure source methods.`);
      if (!same(target.dependencies, [])) fail(EXIT.pin, `${unit.name} must require a standard-library-only data artifact.`);
      if (target.sourceLicense !== undefined && (typeof target.sourceLicense !== 'string' || !target.sourceLicense.trim() || /[\u0000-\u001f\u007f-\u009f\uD800-\uDFFF]/u.test(target.sourceLicense))) fail(EXIT.pin, `${unit.name} has invalid source license metadata.`);
      if (Object.keys(target).some((key) => !['profile', 'crateName', 'methods', 'dependencies', 'sourceLicense'].includes(key))) fail(EXIT.pin, `${unit.name} has unknown data target options.`);
    } else fail(EXIT.pin, `${unit.name} has an unsupported compilation target.`);
    checked(EXIT.pin, () => {
      // Include the repository and inventory roots, even when a generated
      // directory does not exist yet. Do not follow linked entry/output paths.
      assertNoSymlinkAncestors(root, insidePath(root, unit.source));
      assertNoSymlinkAncestors(root, insidePath(root, unit.output?.directory));
      insidePath(root, unit.output?.receipt);
    });
    if (!unit.source.startsWith('frontend/proto-ui/src/') || !unit.source.endsWith('.proto.ts')) fail(EXIT.pin, `Unit source must be an authored Proto UI entry: ${unit.source}`);
    if (!unit.output.directory.startsWith('frontend/proto-ui/generated/') || !unit.output.receipt.startsWith('frontend/proto-ui/receipts/')) fail(EXIT.pin, 'Unit outputs and receipts must stay in their dedicated directories.');
    if (outputs.some((output) => output === unit.output.directory || output.startsWith(`${unit.output.directory}/`) || unit.output.directory.startsWith(`${output}/`)) || receipts.has(unit.output.receipt)) fail(EXIT.pin, 'Compilation outputs overlap.');
    outputs.push(unit.output.directory); receipts.add(unit.output.receipt);
  }
  // Every top-level authored entry is a build unit. Imported prototype modules
  // live in subdirectories and remain part of each unit's source-graph receipt.
  // An added consumer must never escape fresh/retained checks by being omitted
  // from this list while Cargo's workspace glob quietly includes its output.
  const sources = new Set(pin.units.map((unit) => unit.source));
  const authored = path.join(root, 'frontend/proto-ui/src');
  for (const file of readdirSync(authored, { withFileTypes: true })) {
    if (!file.name.endsWith('.proto.ts')) continue;
    if (!file.isFile()) fail(EXIT.pin, `Authored entry must be a regular file: ${file.name}`);
    if (!sources.has(`frontend/proto-ui/src/${file.name}`)) {
      fail(EXIT.pin, `Authored entry is absent from the compiler pin: ${file.name}`);
    }
  }
  const generated = path.join(root, 'frontend/proto-ui/generated');
  if (existsSync(generated)) for (const file of readdirSync(generated, { withFileTypes: true })) {
    // Cargo's workspace glob can include a linked crate that Dirent reports
    // as neither a file nor a directory; dangling links are not valid outputs.
    if (file.isSymbolicLink()) fail(EXIT.pin, `Symbolic generated inventory entry is refused: ${file.name}`);
    if (file.isDirectory() && existsSync(path.join(generated, file.name, 'Cargo.toml'))
      && !outputs.includes(`frontend/proto-ui/generated/${file.name}`)) {
      fail(EXIT.pin, `Generated Cargo consumer is absent from the compiler pin: ${file.name}`);
    }
  }
  const sdk = pin.sharedSdk;
  if (!sdk || !names.has(sdk.owner) || sdk.package !== 'proto-ui-gpui-native' || !/^\d+\.\d+\.\d+$/.test(sdk.version) || !Array.isArray(sdk.features) || !/^[0-9a-f]{64}$/.test(sdk.contentSha256)) fail(EXIT.pin, 'Pin must bind the shared native SDK owner, ABI and content.');
  checked(EXIT.pin, () => insidePath(root, sdk.directory));
  const owner = pin.units.find((unit) => unit.name === sdk.owner);
  if (isDataUnit(owner)) fail(EXIT.pin, 'A Rust data unit cannot own a native SDK.');
  for (const unit of pin.units) {
    if (isDataUnit(unit)) continue;
    if (unit === owner) {
      if (unit.target.nativeSdkPath !== undefined) fail(EXIT.pin, 'The SDK owner must emit its SDK.');
    } else {
      const expected = slash(path.relative(path.resolve(root, unit.output.directory), path.resolve(root, owner.output.directory, sdk.directory)));
      if (unit.target.nativeSdkPath !== expected) fail(EXIT.pin, `${unit.name} must reference its pinned shared SDK owner at ${expected}.`);
    }
  }
  return pin;
}

function git(upstream, args) {
  return checked(EXIT.sourceState, () => execFileSync('git', ['-C', upstream, ...args], { encoding: 'utf8' }).trim());
}
function verifyUpstreamSource(upstream, pin) {
  const head = git(upstream, ['rev-parse', 'HEAD']);
  if (head !== pin.upstream.revision) fail(EXIT.revision, `Pinned upstream ${pin.upstream.revision} differs from checkout ${head}.`);
  const modified = git(upstream, ['status', '--porcelain', '--untracked-files=all']);
  if (modified) fail(EXIT.sourceState, `Pinned compiler checkout has local changes:\n${modified}`);
}

export function verifyToolchain(upstream, pin) {
  if (!process.version.startsWith('v24.') || typeof process.features.typescript !== 'string') fail(EXIT.toolchain, `Node ${process.version} does not satisfy pinned Node 24.x TypeScript execution.`);
  const manifest = checked(EXIT.sourceState, () => json(path.join(upstream, 'package.json')));
  if (readFileSync(path.join(upstream, 'VERSION'), 'utf8').trim() !== pin.packageVersions.protoUi || manifest.packageManager !== pin.packageVersions.packageManager || manifest.engines?.node !== pin.packageVersions.nodeEngine) fail(EXIT.sourceState, 'Upstream version/package-manager/Node declarations differ from the compiler pin.');
  const lock = readFileSync(path.join(upstream, 'pnpm-lock.yaml'));
  if (sha256(lock) !== pin.toolchain.lockSha256) fail(EXIT.sourceState, 'Upstream lockfile content differs from the compiler pin.');
  return checked(EXIT.toolchain, () => {
    const require = createRequire(path.join(upstream, 'package.json'));
    const compilerRequire = createRequire(path.join(upstream, 'packages/compiler/package.json'));
    const installed = {
      typescript: JSON.parse(readFileSync(compilerRequire.resolve('typescript/package.json'), 'utf8')).version,
      tsx: JSON.parse(readFileSync(require.resolve('tsx/package.json'), 'utf8')).version,
    };
    if (!same(installed, pin.toolchain.packages)) throw new Error(`Executed compiler packages ${JSON.stringify(installed)} differ from pinned ${JSON.stringify(pin.toolchain.packages)}.`);
    const installedLock = readFileSync(path.join(upstream, 'node_modules/.pnpm/lock.yaml'));
    if (!lock.equals(installedLock)) throw new Error('Installed dependency lock differs from pnpm-lock.yaml; run a full pnpm install --frozen-lockfile.');
    const modules = require('yaml').parse(readFileSync(path.join(upstream, 'node_modules/.modules.yaml'), 'utf8'));
    if (modules.packageManager !== pin.packageVersions.packageManager) throw new Error(`Installed dependencies were produced by ${modules.packageManager}, expected ${pin.packageVersions.packageManager}.`);
    let manager;
    let runner = 'pnpm';
    try {
      manager = execFileSync('pnpm', ['--version'], { cwd: upstream, encoding: 'utf8' }).trim();
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    // CI may install the exact executable directly. Do not force Corepack to
    // download a redundant copy during an otherwise offline verification.
    if (`pnpm@${manager}` !== pin.packageVersions.packageManager) {
      manager = execFileSync('corepack', [pin.packageVersions.packageManager, '--version'], { cwd: upstream, encoding: 'utf8' }).trim();
      runner = `corepack ${pin.packageVersions.packageManager}`;
    }
    if (`pnpm@${manager}` !== pin.packageVersions.packageManager) throw new Error(`Executed pnpm ${manager} differs from ${pin.packageVersions.packageManager}.`);
    return { node: process.version, packageManager: `pnpm@${manager}`, runner, packages: installed, lockSha256: sha256(installedLock) };
  });
}

function verifyWorkspace(root, pin) {
  const manifest = checked(EXIT.admission, () => readFileSync(path.join(root, 'frontend/Cargo.toml'), 'utf8'));
  const identity = checked(EXIT.admission, () => readWorkspaceGpuiIdentity(manifest));
  if (!same(identity, pin.gpu.workspace)) fail(EXIT.admission, `frontend/Cargo.toml GPUI identity ${JSON.stringify(identity)} differs from pin ${JSON.stringify(pin.gpu.workspace)}.`);
  return manifest;
}

const records = (artifacts) => artifacts.map((artifact) => ({ path: artifact.path, kind: artifact.kind, sha256: sha256(artifact.contents) })).sort((a, b) => a.path.localeCompare(b.path));
function sdkIdentity(pin, ownerArtifacts) {
  // Match the upstream SDK's own inventory identity: paths are SDK-relative,
  // with deterministic code-point ordering rather than a machine locale.
  const prefix = `${pin.sharedSdk.directory}/`;
  const artifacts = records(ownerArtifacts).filter((artifact) => artifact.path.startsWith(prefix)).map((artifact) => ({ ...artifact, path: artifact.path.slice(prefix.length) })).sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const contentSha256 = sha256(JSON.stringify(artifacts));
  if (contentSha256 !== pin.sharedSdk.contentSha256) fail(EXIT.admission, 'Shared SDK content differs from its pinned identity.');
  const sdkManifest = artifactText(ownerArtifacts, `${pin.sharedSdk.directory}/Cargo.toml`);
  const manifest = checked(EXIT.admission, () => parseCargoManifest(sdkManifest));
  if (manifest.package?.name !== pin.sharedSdk.package || manifest.package?.version !== pin.sharedSdk.version) fail(EXIT.admission, 'Shared SDK package/ABI version differs from the pin.');
  for (const feature of pin.sharedSdk.features) if (!Object.hasOwn(manifest.features ?? {}, feature)) fail(EXIT.admission, `Shared SDK is missing ABI feature ${feature}.`);
  return { ...pin.sharedSdk, contentSha256 };
}

function verifyUnitAdmission(pin, unit, artifacts, ownerArtifacts, workspaceManifest, root) {
  const provenance = artifactJson(artifacts, 'provenance.json');
  if (isDataUnit(unit)) {
    const violations = findRustDataViolations({ crateManifest: artifactText(artifacts, 'Cargo.toml'), provenance, expectedPackageName: unit.target.crateName, expectedMethods: unit.target.methods });
    if (violations.length) fail(EXIT.admission, `${unit.name}:\n${violations.join('\n')}`);
    if (!same(provenance.methods?.map((method) => method.name), unit.target.methods)) fail(EXIT.admission, `${unit.name} does not expose precisely its pinned pure methods.`);
    if (artifacts.some((artifact) => artifact.path.startsWith('.proto-ui/') || artifact.path === 'Component.rs')) fail(EXIT.admission, `${unit.name} must not contain native component or SDK artifacts.`);
    return;
  }
  const sdkManifest = artifactText(ownerArtifacts, `${pin.sharedSdk.directory}/Cargo.toml`);
  const violations = findWorkspaceGpuiViolations({ crateManifest: artifactText(artifacts, 'Cargo.toml'), sdkManifest, provenance, workspaceManifest });
  if (violations.length) fail(EXIT.admission, `${unit.name}:\n${violations.join('\n')}`);
  const manifest = parseCargoManifest(artifactText(artifacts, 'Cargo.toml'));
  const dependency = manifest.dependencies?.[pin.sharedSdk.package];
  const owner = pin.units.find((entry) => entry.name === pin.sharedSdk.owner);
  const expectedIdentity = { schemaVersion: 1, name: pin.sharedSdk.package, version: pin.sharedSdk.version, dependencyMode: 'workspace', contentSha256: pin.sharedSdk.contentSha256, path: unit.target.nativeSdkPath ?? pin.sharedSdk.directory, bundled: unit.name === owner.name };
  if (!same(provenance.nativeSdk, expectedIdentity)) fail(EXIT.admission, `${unit.name} compiler SDK provenance differs from the shared identity.`);
  const actualSdk = path.resolve(root, unit.output.directory, dependency?.path ?? '');
  const expectedSdk = path.resolve(root, owner.output.directory, pin.sharedSdk.directory);
  if (actualSdk !== expectedSdk || dependency?.version !== `=${pin.sharedSdk.version}` || !same([...(dependency.features ?? [])].sort(), [...pin.sharedSdk.features].sort())) fail(EXIT.admission, `${unit.name} does not link the pinned shared SDK path/version/ABI features.`);
  if (unit.name !== owner.name && artifacts.some((artifact) => artifact.path.startsWith(`${pin.sharedSdk.directory}/`))) fail(EXIT.admission, `${unit.name} must not copy the shared SDK.`);
}

function receiptFor(pin, unit, artifacts, inputs, sharedSdk) {
  return {
    schema: 'verdant.proto-ui.receipt.v3', unit: unit.name, source: unit.source,
    sourceInputs: inputs, upstream: pin.upstream, packageVersions: pin.packageVersions,
    toolchainRequirements: pin.toolchain, target: unit.target,
    ...(isDataUnit(unit) ? {} : { gpu: pin.gpu, sharedSdk }), artifacts: records(artifacts),
  };
}
function readInput(root, input) {
  const filename = insidePath(root, input.path);
  try {
    const canonical = realpathSync(filename);
    const relative = slash(path.relative(root, canonical));
    insidePath(root, relative);
    return sha256(readFileSync(filename));
  } catch (error) {
    if (error.code === 'ENOENT' || error.code === 'ENOTDIR') return null;
    throw error;
  }
}

function verifyRetained(root, pin, workspaceManifest) {
  const retained = pin.units.map((unit) => checked(EXIT.drift, () => {
    const outDirectory = insidePath(root, unit.output.directory);
    const receiptPath = insidePath(root, unit.output.receipt);
    if (!existsSync(outDirectory) || !existsSync(receiptPath)) fail(EXIT.artifactsAbsent, `Retained artifacts/receipt are absent for ${unit.name}.`);
    assertOwnedInventory(outDirectory, receiptPath, root);
    const receipt = json(receiptPath);
    if (receipt.schema !== 'verdant.proto-ui.receipt.v3' || !Array.isArray(receipt.sourceInputs) || receipt.sourceInputs.length === 0) fail(EXIT.drift, `${unit.name} needs a v3 per-target source-graph receipt.`);
    const inputs = receipt.sourceInputs;
    if (new Set(inputs.map((input) => input.path)).size !== inputs.length || !inputs.some((input) => input.path === unit.source && typeof input.sha256 === 'string')) fail(EXIT.drift, `${unit.name} source-input inventory is invalid.`);
    for (const input of inputs) if (input.sha256 !== readInput(root, input)) fail(EXIT.drift, `Retained source/resolution input changed: ${input.path}`);
    const artifacts = receipt.artifacts.map((artifact) => ({ path: artifact.path, kind: artifact.kind, contents: readFileSync(insidePath(outDirectory, artifact.path), 'utf8') }));
    for (const required of [isDataUnit(unit) ? 'lib.rs' : 'Component.rs', 'Cargo.toml', 'provenance.json']) if (!artifacts.some((artifact) => artifact.path === required)) fail(EXIT.drift, `Required artifact missing: ${unit.name}/${required}`);
    const provenance = artifactJson(artifacts, 'provenance.json');
    for (const source of provenance.sourceFiles ?? []) {
      if (!inputs.some((input) => input.path === source.file && input.sha256 === source.sha256)) fail(EXIT.drift, `Compiler source is absent from receipt closure: ${source.file}`);
    }
    return { unit, outDirectory, receiptPath, receipt, artifacts, inputs };
  }));
  const owner = retained.find((entry) => entry.unit.name === pin.sharedSdk.owner);
  const sharedSdk = sdkIdentity(pin, owner.artifacts);
  for (const entry of retained) {
    verifyUnitAdmission(pin, entry.unit, entry.artifacts, owner.artifacts, workspaceManifest, root);
    if (!same(entry.receipt, receiptFor(pin, entry.unit, entry.artifacts, entry.inputs, sharedSdk))) fail(EXIT.drift, `${entry.unit.name} retained receipt differs from its pin/source/artifacts.`);
  }
  return retained;
}

async function compilerModules(upstream, pin) {
  const require = createRequire(path.join(upstream, 'package.json'));
  const { register } = await import(pathToFileURL(require.resolve('tsx/esm/api')).href);
  register();
  const load = (filename) => import(pathToFileURL(path.join(upstream, 'packages/compiler/src', filename)).href);
  const compiler = await load('compile.ts');
  const { TARGET_PROFILES } = await load('targets.ts');
  const profile = TARGET_PROFILES['gpui-source-v1'];
  const gpui = profile?.dependencies?.find((dependency) => dependency.name === 'gpui');
  const sdk = profile?.dependencies?.find((dependency) => dependency.name === pin.sharedSdk.package);
  if (profile?.version !== `git:${pin.gpu.upstreamGpuiRevision}` || gpui?.version !== `git:${pin.gpu.upstreamGpuiRevision}` || sdk?.version !== pin.sharedSdk.version) fail(EXIT.admission, 'Compiler target GPUI/SDK profile pins differ from the recorded upstream identity.');
  const { FileSourceGraph } = await load('source-loader.ts');
  const dataCompiler = await load('rust-data.ts');
  const { gpuiNativeSdkIdentity } = await load('gpui-native-sdk.ts');
  if (gpuiNativeSdkIdentity(true).contentSha256 !== pin.sharedSdk.contentSha256) fail(EXIT.admission, 'Compiler SDK content identity differs from the pinned shared SDK.');
  return { compiler, dataCompiler, FileSourceGraph };
}

async function compileUnit(root, pin, unit, { compiler, dataCompiler, FileSourceGraph }) {
  const graph = new FileSourceGraph(root);
  const snapshot = await graph.load(await graph.entry(insidePath(root, unit.source)));
  if (snapshot.failures.length) fail(EXIT.compile, `${unit.name} source graph rejected: ${snapshot.failures.map((error) => error.message).join('\n')}`);
  const inputs = [];
  for (const dependency of graph.dependencies) {
    const filename = path.resolve(root, dependency);
    const relative = slash(path.relative(root, filename));
    if (relative.startsWith('../') || path.isAbsolute(relative)) {
      // The loader probes ancestor node_modules for intrinsic imports. Absent
      // candidates outside the repository are not portable source artifacts.
      // Existing out-of-repository packages must be brought into the pinned
      // project instead of hashing machine-specific absolute locations.
      if (existsSync(filename)) fail(EXIT.compile, `Source resolution escaped the repository: ${dependency}`);
      continue;
    }
    inputs.push({ path: relative, sha256: readInput(root, { path: relative }) });
  }
  inputs.sort((a, b) => a.path.localeCompare(b.path));
  for (const [filename, source] of Object.entries(snapshot.files)) {
    if (!inputs.some((input) => input.path === filename && input.sha256 === sha256(source))) fail(EXIT.compile, `Source changed while capturing compilation inputs: ${filename}`);
  }
  const result = isDataUnit(unit)
    ? await dataCompiler.compileRustDataFile(unit.source, { root, sourceSnapshot: snapshot, methods: unit.target.methods, crateName: unit.target.crateName, sourceLicense: unit.target.sourceLicense })
    : await compiler.compileFile(unit.source, { root, sourceSnapshot: snapshot, ...unit.target });
  if (!result.ok) fail(EXIT.compile, `Compiler rejected ${unit.source}:\n${JSON.stringify(result.diagnostics, null, 2)}`);
  const artifacts = isDataUnit(unit) ? dataCompiler.rustDataCompilationArtifacts(result.value) : compiler.compilationArtifacts(result.value);
  return { unit, compilation: result.value, artifacts, inputs, sourceGraphIdentity: snapshot.identity, outDirectory: insidePath(root, unit.output.directory), receiptPath: insidePath(root, unit.output.receipt) };
}

export async function runProducer(argv, root = defaultRoot) {
  const options = parseArguments(argv, root);
  const lockDirectory = path.join(root, 'frontend/proto-ui/.producer.lock');
  if (options.mode === 'recover-publication') return { ok: true, mode: options.mode, recovered: checked(EXIT.replace, () => recoverPublication(lockDirectory, root)) };
  const pin = loadPin(options.pin, root);
  // This must happen before upstream existence checks, including --check.
  const workspaceManifest = verifyWorkspace(root, pin);
  const release = checked(EXIT.replace, () => acquireProducerLock(lockDirectory, root));
  try {
    if (options.mode === 'check' || options.mode === 'check-retained') verifyRetained(root, pin, workspaceManifest);
    if (options.mode === 'check-retained') return { ok: true, mode: 'retained-artifacts', freshCompilation: false, units: pin.units.map((unit) => unit.name), upstreamRevision: pin.upstream.revision };
    const upstream = options.upstream ?? path.resolve(root, '../Proto-UI');
    if (!existsSync(upstream)) fail(EXIT.upstreamAbsent, `Pinned upstream checkout ${upstream} is absent; fresh compilation is required. Use --check-retained only for explicitly offline retained-artifact evidence.`);
    verifyUpstreamSource(upstream, pin);
    const executedToolchain = verifyToolchain(upstream, pin);
    const modules = await compilerModules(upstream, pin);
    const compiled = [];
    for (const unit of pin.units) compiled.push(await compileUnit(root, pin, unit, modules));
    const owner = compiled.find((entry) => entry.unit.name === pin.sharedSdk.owner);
    const sharedSdk = sdkIdentity(pin, owner.artifacts);
    for (const entry of compiled) {
      // Validate the complete resolution snapshot as well as file bytes. This
      // catches package-export or nearer-candidate changes between loader reads
      // and receipt capture, even when the entry source itself is unchanged.
      const replayGraph = new modules.FileSourceGraph(root);
      const replay = await replayGraph.load(await replayGraph.entry(insidePath(root, entry.unit.source)));
      if (replay.failures.length || replay.identity !== entry.sourceGraphIdentity) fail(EXIT.compile, `Source/resolution graph changed during compilation: ${entry.unit.source}`);
      verifyUnitAdmission(pin, entry.unit, entry.artifacts, owner.artifacts, workspaceManifest, root);
      entry.receipt = receiptFor(pin, entry.unit, entry.artifacts, entry.inputs, sharedSdk);
      // Recheck inputs before publishing/checking; no mixed read generations.
      for (const input of entry.inputs) if (readInput(root, input) !== input.sha256) fail(EXIT.compile, `Source input changed during compilation: ${input.path}`);
    }
    if (options.mode === 'check') {
      for (const entry of compiled) {
        const diff = isDataUnit(entry.unit)
          ? await modules.dataCompiler.diffRustDataCompilation(entry.compilation, entry.outDirectory)
          : await modules.compiler.diffCompilation(entry.compilation, entry.outDirectory);
        if (!diff.ok) fail(EXIT.drift, `${entry.unit.name} artifact diff rejected: ${JSON.stringify(diff.diagnostics)}`);
        const changes = diff.value.changes.filter((change) => change.status !== 'unchanged' || change.consumerModified);
        if (changes.length || !same(json(entry.receiptPath), entry.receipt)) fail(EXIT.drift, `${entry.unit.name} differs from fresh compilation: ${JSON.stringify(changes)}`);
      }
    } else checked(EXIT.replace, () => publishBatch(compiled, { root, lockDirectory }));
    return { ok: true, mode: options.mode === 'check' ? 'fresh-compilation' : 'publish', freshCompilation: true, upstreamRevision: pin.upstream.revision, executedToolchain, sharedSdk, units: compiled.map((entry) => ({ name: entry.unit.name, target: entry.unit.target, packageName: parseCargoManifest(artifactText(entry.artifacts, 'Cargo.toml')).package.name, artifacts: entry.artifacts.length, sourceInputs: entry.inputs.length, outputDirectory: entry.unit.output.directory, receipt: entry.unit.output.receipt })) };
  } finally { release(); }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { console.log(JSON.stringify(await runProducer(process.argv.slice(2)), null, 2)); }
  catch (error) { console.error(error?.stack ?? String(error)); process.exit(error instanceof ProducerError ? error.code : EXIT.toolchain); }
}
