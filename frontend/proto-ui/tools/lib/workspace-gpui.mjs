// Admission rules for a single-workspace GPUI artifact set.
//
// One native desktop window must not contain two GPUI versions. The generated
// crate, the bundled Proto UI native SDK crate and the compiler's own
// dependency metadata therefore have to inherit the embedding Cargo
// workspace's `gpui` dependency instead of pinning a revision of their own.
//
// The rules are shared by the producer and by the fixture-driven rule check so
// that both judge the same bytes with the same code.

/** Parse the `[dependencies]` and `[dev-dependencies]` tables of a Cargo manifest. */
export function parseDependencyEntries(manifest) {
  const tables = { dependencies: [], 'dev-dependencies': [] };
  let current = null;
  for (const line of manifest.split('\n')) {
    const section = /^\s*\[([^\]]+)\]\s*$/.exec(line);
    if (section) {
      current = section[1].trim();
      continue;
    }
    if (current !== 'dependencies' && current !== 'dev-dependencies') continue;
    const entry = /^\s*([A-Za-z0-9_-]+)\s*=\s*(.+?)\s*$/.exec(line);
    if (entry) tables[current].push({ name: entry[1], value: entry[2] });
  }
  return tables;
}

const REVISION_PIN = /rev\s*=\s*"[0-9a-f]{40}"/;
const WORKSPACE_ONLY = /^\{\s*workspace\s*=\s*true\s*\}$/;

/**
 * Report every reason why the emitted artifacts are not a single-workspace
 * GPUI artifact set. An empty list means the set is admitted.
 */
export function findWorkspaceGpuiViolations({ crateManifest, sdkManifest, provenance }) {
  const violations = [];

  const checkManifest = (label, manifest, { devAllowed }) => {
    if (typeof manifest !== 'string' || manifest.trim() === '') {
      violations.push(`${label} is missing`);
      return;
    }
    if (REVISION_PIN.test(manifest)) {
      violations.push(`${label} carries a gpui revision pin`);
    }
    const { dependencies, 'dev-dependencies': devDependencies } = parseDependencyEntries(manifest);
    const tables = devAllowed
      ? [['dependencies', dependencies], ['dev-dependencies', devDependencies]]
      : [['dependencies', dependencies]];
    if (!devAllowed && devDependencies.some((entry) => entry.name === 'gpui')) {
      violations.push(`${label} declares gpui in dev-dependencies`);
    }
    for (const [table, entries] of tables) {
      const gpui = entries.filter((entry) => entry.name === 'gpui');
      if (table === 'dependencies' && gpui.length === 0) {
        violations.push(`${label} has no gpui dependency`);
      }
      if (gpui.length > 1) violations.push(`${label} declares gpui more than once in ${table}`);
      for (const entry of gpui) {
        if (!WORKSPACE_ONLY.test(entry.value) && !/^\{\s*workspace\s*=\s*true\s*,\s*features\s*=\s*\["test-support"\]\s*\}$/.test(entry.value)) {
          violations.push(`${label} pins gpui instead of inheriting the workspace: ${entry.value}`);
        }
      }
    }
  };

  checkManifest('the generated crate manifest', crateManifest, { devAllowed: true });
  checkManifest('the bundled native SDK manifest', sdkManifest, { devAllowed: false });

  if (!provenance || typeof provenance !== 'object') {
    violations.push('the compiler provenance manifest is missing');
  } else {
    const dependencies = Array.isArray(provenance.dependencies) ? provenance.dependencies : [];
    const gpui = dependencies.filter((entry) => entry && entry.name === 'gpui');
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
