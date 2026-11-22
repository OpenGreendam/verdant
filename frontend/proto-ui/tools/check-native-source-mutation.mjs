#!/usr/bin/env node
// A source mutation must change a compiled Rust consumer's observed result.
// Preparation needs Node 24 + the pinned compiler. Execution can run in the
// native build image with its Cargo cache; the generated Rust is never edited.
//
//   node .../check-native-source-mutation.mjs --prepare <directory> --upstream <checkout>
//   node .../check-native-source-mutation.mjs --run <prepared-directory>
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runProducer } from './build-proto-ui.mjs';
import { findCargoGpuiGraphViolations, parseCargoManifest, readWorkspaceGpuiIdentity } from './lib/workspace-gpui.mjs';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const hash = (value) => createHash('sha256').update(value).digest('hex');
const readJson = (filename) => JSON.parse(readFileSync(filename, 'utf8'));
const writeJson = (filename, value) => writeFileSync(filename, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
const recordJson = (filename, value) => {
  if (existsSync(filename)) assert.deepEqual(readJson(filename), value, `Refusing to replace different evidence: ${filename}`);
  else writeJson(filename, value);
};
const consumerName = 'verdant-proto-source-mutation-consumer';
const variantValues = { baseline: 0.55, variant: 0.6 };
const consumerSource = `#[cfg(test)]
mod tests {
    use control_planner::ControlGesturePlannerHandle;
    use serde_json::json;

    #[test]
    fn compiled_source_controls_fine_drag() {
        let result = ControlGesturePlannerHandle::call_data_method(
            "dragNormalized", vec![json!(0.5), json!(90.0), json!(180.0), json!(true)],
        );
        let value = match result {
            Ok(result) => result.as_f64().expect("compiled numeric result"),
            Err(_) => panic!("compiled source method failed"),
        };
        println!("verdant-source-mutation-result={value}");
        let expected: f64 = std::env::var("VERDANT_MUTATION_EXPECTED")
            .expect("experiment expected value").parse().expect("finite expectation");
        assert!((value - expected).abs() < 1e-12,
            "source mutation native expectation: observed {value}, expected {expected}");
    }
}
`;

function makeConsumer(root) {
  const directory = path.join(root, 'frontend/crates/proto-source-mutation-consumer');
  mkdirSync(path.join(directory, 'src'), { recursive: true });
  const manifest = parseCargoManifest(readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/Cargo.toml'), 'utf8'));
  writeFileSync(path.join(directory, 'Cargo.toml'), `[package]\nname = "${consumerName}"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n\n[dependencies]\ncontrol-planner = { package = "${manifest.package.name}", path = "../../proto-ui/generated/control-gesture-planner" }\nserde_json.workspace = true\n`, { flag: 'wx' });
  writeFileSync(path.join(directory, 'src/lib.rs'), consumerSource, { flag: 'wx' });
}

async function prepare(directory, upstream) {
  if (existsSync(directory)) throw new Error(`Refusing to replace a prepared experiment: ${directory}`);
  await runProducer(['--check', '--upstream', upstream], repository);
  mkdirSync(directory, { recursive: true });
  const results = {};
  for (const [name, expected] of Object.entries(variantValues)) {
    const root = path.join(directory, name);
    mkdirSync(path.join(root, 'frontend'), { recursive: true });
    for (const filename of ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']) cpSync(path.join(repository, 'frontend', filename), path.join(root, 'frontend', filename));
    for (const filename of ['compiler.pin.json', 'src', 'generated', 'receipts']) cpSync(path.join(repository, 'frontend/proto-ui', filename), path.join(root, 'frontend/proto-ui', filename), { recursive: true });
    if (name === 'variant') {
      const filename = path.join(root, 'frontend/proto-ui/src/control-gesture-planner.proto.ts');
      const source = readFileSync(filename, 'utf8');
      const original = 'const rate = fine ? 0.1 : 1;';
      assert.equal(source.split(original).length, 2, 'fine-rate source mutation must match exactly once');
      writeFileSync(filename, source.replace(original, 'const rate = fine ? 0.2 : 1;'));
      await runProducer(['--upstream', upstream], root);
    } else await runProducer(['--check', '--upstream', upstream], root);
    makeConsumer(root);
    const generated = readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/Component.rs'));
    const pin = readJson(path.join(root, 'frontend/proto-ui/compiler.pin.json'));
    results[name] = { expected, generatedSha256: hash(generated), upstream: pin.upstream, sharedSdk: pin.sharedSdk };
  }
  assert.notEqual(results.baseline.generatedSha256, results.variant.generatedSha256);
  assert.deepEqual(results.baseline.sharedSdk, results.variant.sharedSdk);
  writeJson(path.join(directory, 'experiment.json'), { schema: 'verdant.proto-ui.native-source-mutation.v1', method: 'dragNormalized', arguments: [0.5, 90, 180, true], consumerSha256: hash(consumerSource), variants: results });
  console.log(JSON.stringify({ prepared: directory, nativeExecution: false, variants: results }, null, 2));
}

function cargo(root, args, { expected, failure = false } = {}) {
  const result = spawnSync(process.env.CARGO ?? 'cargo', args, {
    cwd: path.join(root, 'frontend'), encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
    env: { ...process.env, ...(expected === undefined ? {} : { VERDANT_MUTATION_EXPECTED: String(expected) }) },
  });
  if (result.error) throw result.error;
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  if ((!failure && result.status !== 0) || (failure && result.status === 0)) throw new Error(`cargo ${args.join(' ')} exited ${result.status}:\n${output}`);
  if (failure && !output.includes('source mutation native expectation')) throw new Error(`The negative consumer failed for an unrelated reason:\n${output}`);
  return { status: result.status, stdout: result.stdout ?? '', output };
}

async function run(directory) {
  const plan = readJson(path.join(directory, 'experiment.json'));
  assert.equal(plan.schema, 'verdant.proto-ui.native-source-mutation.v1');
  assert.equal(plan.consumerSha256, hash(consumerSource));
  const observations = {};
  for (const [name, expected] of Object.entries(variantValues)) {
    const root = path.join(directory, name);
    assert.equal(plan.variants[name].expected, expected);
    assert.equal(hash(readFileSync(path.join(root, 'frontend/proto-ui/generated/control-gesture-planner/Component.rs'))), plan.variants[name].generatedSha256);
    assert.equal(hash(readFileSync(path.join(root, 'frontend/crates/proto-source-mutation-consumer/src/lib.rs'))), hash(consumerSource));
    await runProducer(['--check-retained'], root);
    // Add the disposable consumer/workspace package identities while retaining
    // the copied dependency lock. This unlocked operation is confined to the
    // experiment; the subsequent graph and executions must be --locked.
    cargo(root, ['metadata', '--format-version', '1', '--all-features', '--offline']);
    const metadata = JSON.parse(cargo(root, ['metadata', '--format-version', '1', '--all-features', '--locked', '--offline']).stdout);
    const identity = readWorkspaceGpuiIdentity(readFileSync(path.join(root, 'frontend/Cargo.toml'), 'utf8'));
    const violations = findCargoGpuiGraphViolations(metadata, identity);
    assert.deepEqual(violations, [], `${name} native consumer graph must contain the one pinned GPUI and SDK`);
    recordJson(path.join(directory, `${name}-cargo-graph.json`), { gpui: metadata.packages.filter((pkg) => pkg.name === 'gpui').map(({ id, source }) => ({ id, source })), sharedSdk: metadata.packages.filter((pkg) => pkg.name === 'proto-ui-gpui-native').map(({ id, source }) => ({ id, source })) });
    const execution = cargo(root, ['test', '--locked', '--offline', '-p', consumerName, '--', '--nocapture'], { expected });
    const match = /verdant-source-mutation-result=([0-9.]+)/.exec(execution.output);
    assert.ok(match, `${name} native consumer must report its actual result`);
    const observed = Number(match[1]);
    assert.ok(Math.abs(observed - expected) < 1e-12);
    observations[name] = { observed, expected, exitCode: execution.status };
    if (name === 'variant') {
      const negative = cargo(root, ['test', '--locked', '--offline', '-p', consumerName, '--', '--nocapture'], { expected: variantValues.baseline, failure: true });
      observations.oldExpectationRejected = { expected: variantValues.baseline, exitCode: negative.status };
    }
  }
  const result = { schema: 'verdant.proto-ui.native-source-mutation-result.v1', nativeExecution: true, observations, variants: plan.variants };
  recordJson(path.join(directory, 'result.json'), result);
  console.log(JSON.stringify(result, null, 2));
}

try {
  const args = process.argv.slice(2);
  if (args[0] === '--prepare' && args[1] && args[2] === '--upstream' && args[3] && args.length === 4) await prepare(path.resolve(args[1]), path.resolve(args[3]));
  else if (args[0] === '--run' && args[1] && args.length === 2) await run(path.resolve(args[1]));
  else throw new Error('Use --prepare <directory> --upstream <checkout>, or --run <prepared-directory>.');
} catch (error) { console.error(error?.stack ?? String(error)); process.exitCode = 1; }
