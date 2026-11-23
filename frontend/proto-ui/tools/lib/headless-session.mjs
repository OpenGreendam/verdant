// Headless is a property of the session's resolved dependency closure. The
// native app may share the Cargo workspace without belonging to that closure.
import path from 'node:path';

const record = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const normalizePath = (value) => {
  if (typeof value !== 'string' || value.includes('\0')) throw new Error('Cargo manifest paths must be strings without NUL bytes.');
  const normalized = path.posix.normalize(value.replaceAll('\\', '/')).replace(/\/$/, '');
  if (/^[A-Za-z]:\//.test(normalized) || normalized.startsWith('//')) return normalized.toLowerCase();
  if (!normalized.startsWith('/')) throw new Error(`Cargo metadata path is not absolute: ${value}`);
  return normalized;
};
const under = (filename, directory) => filename === directory || filename.startsWith(`${directory}/`);
const sameIds = (left, right) => JSON.stringify([...new Set(left)].sort()) === JSON.stringify([...new Set(right)].sort());

// This list describes the native/image support already excluded by the design.
// Generated native units and their SDK are additionally identified by the pin's
// actual output directories, so renaming a generated package cannot evade it.
const nativeSupportPackages = new Set([
  'gpui', 'gpui_platform', 'gpui_macros', 'proto-ui-gpui-native',
  'image', 'image-webp', 'imageproc', 'resvg', 'usvg', 'tiny-skia', 'tiny-skia-path',
  'font-kit', 'fontdb', 'cosmic-text', 'taffy', 'winit',
  'wgpu', 'wgpu-core', 'wgpu-hal', 'blade-graphics', 'blade-macros',
  'accesskit', 'accesskit_consumer', 'accesskit_winit', 'skia-safe', 'skia-bindings',
]);

/** Resolve the producer's unit ownership against Cargo's own workspace root. */
export function headlessSessionContract(pin, workspaceRoot) {
  const workspace = normalizePath(workspaceRoot);
  if (pin?.schema !== 'verdant.proto-ui.compiler-pin.v3' || !Array.isArray(pin.units)) {
    throw new Error('A v3 compiler pin with per-unit targets is required for headless evidence.');
  }
  const directory = (unit) => {
    const output = unit.output?.directory;
    if (typeof output !== 'string' || !output.startsWith('frontend/')
      || output.includes('\\') || output.split('/').some((part) => part === '..' || part === '.')) {
      throw new Error(`Invalid pinned output directory for ${unit.name}.`);
    }
    return normalizePath(`${workspace}/${output.slice('frontend/'.length)}`);
  };
  const data = ['note-gesture-planner', 'control-gesture-planner'].map((name) => {
    const units = pin.units.filter((unit) => unit.name === name);
    if (units.length !== 1 || units[0].target?.profile !== 'rust-data-v1'
      || typeof units[0].target.crateName !== 'string' || !units[0].target.crateName) {
      throw new Error(`${name} must be a pinned Rust data unit.`);
    }
    return { name: units[0].target.crateName, manifest: `${directory(units[0])}/Cargo.toml` };
  });
  const nativeDirectories = pin.units.filter((unit) => unit.target?.profile === 'gpui-source-v1').map(directory);
  if (nativeDirectories.length === 0 || !pin.units.some((unit) => unit.name === pin.sharedSdk?.owner
    && unit.target?.profile === 'gpui-source-v1')) {
    throw new Error('The pin must identify the native units and their shared SDK owner.');
  }
  return {
    workspace,
    session: { name: 'verdant-session', manifest: `${workspace}/crates/verdant-session/Cargo.toml` },
    protocol: { name: 'verdant-ui-protocol', manifest: `${workspace}/crates/verdant-ui-protocol/Cargo.toml` },
    data,
    nativeDirectories,
  };
}

/** Traverse every resolved normal/build/dev/target edge from the real session. */
export function inspectHeadlessSessionGraph(metadata, contract) {
  const violations = [];
  const result = { violations, session: null, protocol: null, data: [], reachable: [] };
  if (!record(metadata) || !Array.isArray(metadata.packages)
    || !Array.isArray(metadata.workspace_members) || !Array.isArray(metadata.resolve?.nodes)) {
    violations.push('Headless evidence requires full Cargo metadata with a resolve graph (--no-deps is insufficient).');
    return result;
  }
  const packages = new Map();
  const nodes = new Map();
  for (const pkg of metadata.packages) {
    if (!record(pkg) || typeof pkg.id !== 'string' || typeof pkg.name !== 'string') {
      violations.push('Cargo metadata contains an invalid package identity.');
      continue;
    }
    if (packages.has(pkg.id)) violations.push(`Cargo metadata repeats package identity: ${pkg.id}`);
    packages.set(pkg.id, pkg);
  }
  for (const node of metadata.resolve.nodes) {
    if (!record(node) || typeof node.id !== 'string') {
      violations.push('Cargo metadata contains an invalid resolve node.');
      continue;
    }
    if (nodes.has(node.id)) violations.push(`Cargo metadata repeats resolve node: ${node.id}`);
    nodes.set(node.id, node);
  }
  const members = new Set(metadata.workspace_members);
  const locate = (expected) => {
    const candidates = [...packages.values()].filter((pkg) => {
      try { return normalizePath(pkg.manifest_path) === normalizePath(expected.manifest); }
      catch { return false; }
    });
    if (candidates.length !== 1 || candidates[0].name !== expected.name
      || candidates[0].source !== null || !members.has(candidates[0].id)) {
      violations.push(`Expected one local workspace package ${expected.name} at ${expected.manifest}.`);
      return null;
    }
    return candidates[0];
  };
  const session = locate(contract.session);
  const protocol = locate(contract.protocol);
  const data = contract.data.map(locate);
  if (!session) return result;
  result.session = session.id;
  result.protocol = protocol?.id ?? null;
  result.data = data.filter(Boolean).map((pkg) => pkg.id);
  const visited = new Set();
  const pending = [session.id];
  const previous = new Map();
  const describePath = (id) => {
    const chain = [];
    let current = id;
    while (current !== session.id && previous.has(current)) {
      const edge = previous.get(current);
      const kinds = (edge.dep_kinds ?? []).map(({ kind, target }) => `${kind ?? 'normal'}${target ? `, ${target}` : ''}`).join('; ');
      chain.unshift(`--${edge.name} [${kinds}]--> ${packages.get(current)?.name ?? current}`);
      current = edge.parent;
    }
    return `${session.name} ${chain.join(' ')}`.trim();
  };
  for (let index = 0; index < pending.length; index += 1) {
    const id = pending[index];
    if (visited.has(id)) continue;
    visited.add(id);
    const pkg = packages.get(id);
    const node = nodes.get(id);
    if (!pkg) violations.push(`Headless dependency has no package record: ${id}`);
    if (!node || !Array.isArray(node.deps) || !Array.isArray(node.dependencies)) {
      violations.push(`Headless dependency has an incomplete resolve node: ${id}`);
      continue;
    }
    const deps = node.deps.filter((dep) => record(dep) && typeof dep.pkg === 'string');
    if (deps.length !== node.deps.length || !sameIds(node.dependencies, deps.map((dep) => dep.pkg))) {
      violations.push(`Cargo dependency identities disagree or are incomplete for ${id}.`);
    }
    for (const dep of deps) {
      if (typeof dep.name !== 'string' || !Array.isArray(dep.dep_kinds) || dep.dep_kinds.length === 0
        || dep.dep_kinds.some((kind) => !record(kind) || ![null, 'build', 'dev'].includes(kind.kind)
          || (kind.target !== null && typeof kind.target !== 'string'))) {
        violations.push(`Cargo dependency kind/target evidence is incomplete for ${id} -> ${dep.pkg}.`);
      }
      if (dep.pkg !== session.id && !previous.has(dep.pkg)) previous.set(dep.pkg, { ...dep, parent: id });
      pending.push(dep.pkg);
    }
    // Do not silently discard an edge from Cargo's redundant identity list if
    // a supplied metadata snapshot contains a shortened modern `deps` list.
    for (const dependency of node.dependencies) {
      if (typeof dependency !== 'string') {
        violations.push(`Cargo dependency identity is invalid for ${id}.`);
        continue;
      }
      pending.push(dependency);
    }
    if (!pkg) continue;
    let nativeOutput = false;
    try {
      nativeOutput = contract.nativeDirectories.some((directory) => under(normalizePath(pkg.manifest_path), normalizePath(directory)));
    } catch { violations.push(`Headless dependency has an invalid manifest path: ${id}`); }
    if (nativeOutput || nativeSupportPackages.has(pkg.name)) {
      violations.push(`Headless session reaches ${nativeOutput ? 'a pinned native output' : 'native/image support'} ${pkg.name} (${id}): ${describePath(id)}`);
    }
    result.reachable.push({ id, name: pkg.name, source: pkg.source, manifest: pkg.manifest_path });
  }
  for (const pkg of [protocol, ...data].filter(Boolean)) {
    if (!visited.has(pkg.id)) violations.push(`Headless session does not reach required generated-protocol package ${pkg.name} (${pkg.id}).`);
  }
  for (const pkg of data.filter(Boolean)) {
    if (nodes.get(pkg.id)?.deps?.length !== 0) violations.push(`The pinned std-only Rust data package resolves dependencies: ${pkg.name} (${pkg.id}).`);
  }
  result.reachable.sort((left, right) => left.id.localeCompare(right.id));
  return result;
}
