// Source admission and resolved Cargo graph checks are separate evidence.
// TOML admission covers every declared dependency table, including disabled
// targets. The graph check covers the package identities Cargo actually resolves.
import TOML from '@iarna/toml';

const dependencyKinds = ['dependencies', 'dev-dependencies', 'build-dependencies'];
const record = (value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const sortedStrings = (value, label) => {
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== 'string')) {
    throw new Error(`${label} must be an array of strings.`);
  }
  return [...new Set(value)].sort();
};

export function parseCargoManifest(manifest) {
  return TOML.parse(manifest);
}

/** All ordinary, build, development and target-specific dependency entries. */
export function parseDependencyEntries(manifest, workspaceManifest) {
  const parsed = typeof manifest === 'string' ? parseCargoManifest(manifest) : manifest;
  const workspace = typeof workspaceManifest === 'string'
    ? parseCargoManifest(workspaceManifest) : workspaceManifest;
  const entries = [];
  const add = (table, tablePath, kind) => {
    if (!record(table)) return;
    for (const [name, value] of Object.entries(table)) {
      const inherited = record(value) && value.workspace === true
        ? workspace?.workspace?.dependencies?.[name] : undefined;
      const packageName = record(value) && typeof value.package === 'string'
        ? value.package : record(inherited) && typeof inherited.package === 'string'
          ? inherited.package : name;
      entries.push({ name, package: packageName, value, inherited, kind, table: tablePath });
    }
  };
  for (const kind of dependencyKinds) add(parsed[kind], kind, kind);
  for (const [target, tables] of Object.entries(parsed.target ?? {})) {
    for (const kind of dependencyKinds) add(tables[kind], `target.${JSON.stringify(target)}.${kind}`, kind);
  }
  return entries;
}

/** Parse only the workspace's canonical GPUI declaration, never another rev. */
export function readWorkspaceGpuiIdentity(manifest) {
  const parsed = typeof manifest === 'string' ? parseCargoManifest(manifest) : manifest;
  const value = parsed?.workspace?.dependencies?.gpui;
  if (!record(value) || (value.package !== undefined && value.package !== 'gpui')) {
    throw new Error('frontend/Cargo.toml must declare gpui in [workspace.dependencies].');
  }
  if (typeof value.git !== 'string' || !/^https?:\/\//.test(value.git)
    || typeof value.rev !== 'string' || !/^[0-9a-f]{40}$/.test(value.rev)) {
    throw new Error('The workspace GPUI dependency must pin a Git repository and full 40-hex revision.');
  }
  for (const key of ['path', 'branch', 'tag', 'registry', 'registry-index', 'workspace']) {
    if (Object.hasOwn(value, key)) throw new Error(`The workspace GPUI dependency cannot also declare ${key}.`);
  }
  if (value['default-features'] !== undefined && typeof value['default-features'] !== 'boolean') {
    throw new Error('The workspace GPUI default-features must be a boolean.');
  }
  return {
    name: 'gpui',
    git: value.git,
    rev: value.rev,
    features: sortedStrings(value.features ?? [], 'The workspace GPUI features'),
    defaultFeatures: value['default-features'] ?? true,
  };
}

export function findWorkspaceGpuiViolations({ crateManifest, sdkManifest, provenance, workspaceManifest }) {
  const violations = [];
  let workspace;
  if (workspaceManifest !== undefined) {
    try { workspace = parseCargoManifest(workspaceManifest); }
    catch { return ['the workspace manifest is not valid TOML']; }
  }

  const checkManifest = (label, manifest, { devAllowed }) => {
    if (typeof manifest !== 'string' || manifest.trim() === '') {
      violations.push(`${label} is missing`);
      return;
    }
    let entries;
    try { entries = parseDependencyEntries(manifest, workspace); }
    catch {
      violations.push(`${label} is not valid TOML`);
      return;
    }
    const gpui = entries.filter((entry) => entry.package === 'gpui');
    if (!gpui.some((entry) => entry.kind === 'dependencies')) {
      violations.push(`${label} has no gpui dependency`);
    }
    for (const entry of gpui) {
      const location = `${entry.table}.${entry.name}`;
      if (!devAllowed && entry.kind === 'dev-dependencies') {
        violations.push(`${label} declares gpui in ${location}`);
      }
      const value = entry.value;
      if (!record(value) || value.workspace !== true) {
        violations.push(`${label} does not inherit workspace gpui in ${location}`);
        continue;
      }
      // Cargo permits additive features and optional/public inheritance flags.
      // Source/package selectors in an inherited entry are not inheritance.
      const allowed = new Set(['workspace', 'features', 'optional', 'public', 'default-features']);
      const extra = Object.keys(value).filter((key) => !allowed.has(key)).sort();
      if (extra.length > 0) {
        violations.push(`${label} adds non-inherited gpui keys in ${location}: ${extra.join(', ')}`);
      }
      if (value.features !== undefined
        && (!Array.isArray(value.features) || value.features.some((feature) => typeof feature !== 'string'))) {
        violations.push(`${label} has invalid gpui features in ${location}`);
      }
      for (const key of ['optional', 'public', 'default-features']) {
        if (value[key] !== undefined && typeof value[key] !== 'boolean') {
          violations.push(`${label} has invalid gpui ${key} in ${location}`);
        }
      }
      if (workspace && !Object.hasOwn(workspace.workspace?.dependencies ?? {}, entry.name)) {
        violations.push(`${label} inherits missing workspace dependency in ${location}`);
      }
      if (workspace && entry.name !== 'gpui') {
        // A workspace alias is allowed only when it inherits the same source.
        const canonical = workspace.workspace?.dependencies?.gpui;
        const inherited = entry.inherited;
        if (!record(inherited) || !record(canonical)
          || inherited.git !== canonical.git || inherited.rev !== canonical.rev
          || ['version', 'path', 'branch', 'tag', 'registry', 'registry-index'].some((key) => inherited[key] !== canonical[key])) {
          violations.push(`${label} inherits a different GPUI source in ${location}`);
        }
      }
    }
  };

  checkManifest('the generated crate manifest', crateManifest, { devAllowed: true });
  checkManifest('the shared native SDK manifest', sdkManifest, { devAllowed: false });

  if (!record(provenance)) {
    violations.push('the compiler provenance manifest is missing');
  } else {
    const gpui = (Array.isArray(provenance.dependencies) ? provenance.dependencies : [])
      .filter((entry) => record(entry) && (entry.package ?? entry.name) === 'gpui');
    if (gpui.length === 0) violations.push('the compiler provenance reports no gpui dependency');
    if (gpui.length > 1) violations.push('the compiler provenance reports gpui more than once');
    for (const entry of gpui) {
      if (entry.version !== 'workspace') {
        violations.push(`the compiler provenance pins gpui instead of the workspace: ${entry.version}`);
      }
    }
  }
  return violations;
}

const canonicalRepository = (value) => {
  const url = new URL(value);
  url.pathname = url.pathname.replace(/\.git\/?$/, '').replace(/\/$/, '');
  url.search = '';
  url.hash = '';
  return url.href;
};

/** Validate a full, unfiltered `cargo metadata --locked --all-features` graph. */
export function findCargoGpuiGraphViolations(metadata, expected) {
  if (!record(metadata) || !Array.isArray(metadata.packages)
    || !Array.isArray(metadata.workspace_members) || metadata.workspace_members.length === 0
    || !Array.isArray(metadata.resolve?.nodes)) {
    return ['full Cargo metadata with workspace members and a resolve graph is required (--no-deps is insufficient)'];
  }
  const violations = [];
  const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const reachable = new Set();
  const pending = [...metadata.workspace_members];
  while (pending.length > 0) {
    const id = pending.pop();
    if (reachable.has(id)) continue;
    reachable.add(id);
    const node = nodes.get(id);
    if (!packages.has(id)) violations.push(`Cargo graph package is missing: ${id}`);
    if (!node || !Array.isArray(node.deps)) {
      violations.push(`Cargo resolve node is missing or incomplete: ${id}`);
      continue;
    }
    for (const dependency of node.deps) pending.push(dependency.pkg);
  }
  const gpui = [...reachable].map((id) => packages.get(id)).filter((pkg) => pkg?.name === 'gpui');
  if (gpui.length !== 1) {
    violations.push(`Cargo resolves ${gpui.length} reachable GPUI package identities; expected exactly one: ${gpui.map((pkg) => pkg.id).sort().join(', ')}`);
  }
  for (const pkg of gpui) {
    let validSource = false;
    try {
      if (typeof pkg.source === 'string' && pkg.source.startsWith('git+')) {
        const source = new URL(pkg.source.slice(4));
        validSource = canonicalRepository(source.href) === canonicalRepository(expected.git)
          && source.searchParams.get('rev') === expected.rev
          && [...source.searchParams.keys()].every((key) => key === 'rev')
          && source.hash === `#${expected.rev}`;
      }
    } catch { /* Malformed/missing source is rejected below. */ }
    if (!validSource) {
      violations.push(`Cargo GPUI source differs from workspace ${expected.git}@${expected.rev}: ${pkg.source}`);
    }
    const features = new Set(nodes.get(pkg.id)?.features ?? []);
    for (const feature of expected.features) {
      if (!features.has(feature)) violations.push(`Cargo GPUI is missing workspace feature: ${feature}`);
    }
    if (expected.defaultFeatures && Object.hasOwn(pkg.features ?? {}, 'default') && !features.has('default')) {
      violations.push('Cargo GPUI does not activate the workspace default features');
    }
  }
  const sdk = [...reachable].map((id) => packages.get(id)).filter((pkg) => pkg?.name === 'proto-ui-gpui-native');
  if (sdk.length !== 1) {
    violations.push(`Cargo resolves ${sdk.length} reachable Proto-UI native SDK identities; expected one shared SDK`);
  }
  return violations;
}
