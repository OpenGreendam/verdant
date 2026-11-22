// The compiler owns exactly its receipt inventory, never arbitrary neighbours.
// A producer lock also covers readers, so no cooperating check observes a
// partially replaced batch. Backups and the journal survive an interrupted
// publication; --recover-publication restores the previous complete batch.
import { createHash, randomUUID } from 'node:crypto';
import {
  closeSync, existsSync, linkSync, lstatSync, mkdirSync, openSync, readFileSync, readdirSync, renameSync,
  rmSync, writeFileSync,
} from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const slash = (value) => value.split(path.sep).join('/');
const lockSchema = 'verdant.proto-ui.producer-lock.v2';
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function insidePath(root, relative) {
  if (typeof relative !== 'string' || relative === '' || relative.includes('\\') || relative.includes('\0')) {
    throw new Error(`Invalid artifact path: ${String(relative)}`);
  }
  const absolute = path.resolve(root, relative);
  const normalized = slash(path.relative(root, absolute));
  if (normalized !== relative || normalized === '..' || normalized.startsWith('../') || path.isAbsolute(relative)) {
    throw new Error(`Path escapes its declared root: ${relative}`);
  }
  return absolute;
}

export function assertNoSymlinkAncestors(root, filename) {
  const relative = slash(path.relative(root, filename));
  insidePath(root, relative);
  let current = root;
  for (const segment of ['', ...relative.split('/')]) {
    if (segment) current = path.join(current, segment);
    try {
      if (lstatSync(current).isSymbolicLink()) throw new Error(`Symbolic compiler-owned path is refused: ${current}`);
    } catch (error) {
      if (error.code === 'ENOENT') return;
      throw error;
    }
  }
}

function ownedPath(root, relative) {
  const filename = insidePath(root, relative);
  assertNoSymlinkAncestors(root, filename);
  return filename;
}

export function listRegularFiles(directory, prefix = '') {
  if (lstatSync(directory).isSymbolicLink()) throw new Error(`Symbolic artifact directory: ${directory}`);
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    const filename = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Symbolic artifact is not compiler-owned: ${filename}`);
    if (entry.isDirectory()) files.push(...listRegularFiles(filename, relative));
    else if (entry.isFile()) files.push(relative);
    else throw new Error(`Non-regular artifact is not compiler-owned: ${filename}`);
  }
  return files.sort();
}

export function assertOwnedInventory(directory, receiptPath, root) {
  if (root) {
    assertNoSymlinkAncestors(root, directory);
    assertNoSymlinkAncestors(root, receiptPath);
  }
  if (!existsSync(directory)) {
    if (existsSync(receiptPath)) throw new Error(`Receipt exists without its artifact directory: ${receiptPath}`);
    return;
  }
  if (!existsSync(receiptPath)) throw new Error(`Refusing to replace artifacts without their receipt: ${directory}`);
  if (lstatSync(receiptPath).isSymbolicLink()) throw new Error(`Symbolic receipt is not compiler-owned: ${receiptPath}`);
  const receipt = JSON.parse(readFileSync(receiptPath, 'utf8'));
  if (!Array.isArray(receipt.artifacts) || receipt.artifacts.length === 0) throw new Error(`No compiler inventory in ${receiptPath}`);
  const expected = receipt.artifacts.map((artifact) => artifact.path).sort();
  if (new Set(expected).size !== expected.length) throw new Error(`Duplicate receipt inventory in ${receiptPath}`);
  for (const artifact of receipt.artifacts) {
    insidePath(directory, artifact.path);
    if (!/^[0-9a-f]{64}$/.test(artifact.sha256)) throw new Error(`Invalid artifact digest: ${artifact.path}`);
  }
  const actual = listRegularFiles(directory);
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`Refusing to replace ${directory}: files differ from its compiler-owned inventory. Extra: ${actual.filter((file) => !expected.includes(file)).join(', ')}; missing: ${expected.filter((file) => !actual.includes(file)).join(', ')}`);
  }
  for (const artifact of receipt.artifacts) {
    if (sha256(readFileSync(insidePath(directory, artifact.path))) !== artifact.sha256) {
      throw new Error(`Refusing to overwrite a modified compiler artifact: ${artifact.path}`);
    }
  }
}

function writeCompleteFile(filename, contents) {
  const fd = openSync(filename, 'wx');
  try { writeFileSync(fd, contents); }
  finally { closeSync(fd); }
}

function privateStatePath(lockPath, owner, root) {
  if (owner?.schema !== lockSchema || !uuid.test(owner.id ?? '') ||
      !Number.isSafeInteger(owner.pid) || owner.pid < 1 || typeof owner.host !== 'string' || !owner.host) {
    throw new Error('Invalid publication lock identity.');
  }
  // The owner cannot redirect cleanup to an arbitrary path from its JSON.
  return ownedPath(root, slash(path.relative(root, `${lockPath}.state-${owner.id}`)));
}

function readLock(lockPath, root) {
  assertNoSymlinkAncestors(root, lockPath);
  const stat = lstatSync(lockPath);
  // Complete legacy directory locks remain recoverable. Incomplete/unknown
  // legacy locks are never replaced by a new acquisition or guessed away.
  if (stat.isDirectory()) return { path: lockPath, directory: lockPath, legacy: true };
  if (!stat.isFile()) throw new Error('Publication lock is not a regular owner record.');
  const contents = readFileSync(lockPath, 'utf8');
  const owner = JSON.parse(contents);
  const directory = privateStatePath(lockPath, owner, root);
  return { path: lockPath, directory, legacy: false, owner, contents };
}

function isCurrentLock(lock) {
  try {
    return lstatSync(lock.path).isFile() && readFileSync(lock.path, 'utf8') === lock.contents;
  } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}

function hasFileLockSuccessor(lockPath, root) {
  try {
    const successor = readLock(lockPath, root);
    return !successor.legacy && readFileSync(ownedPath(root,
      slash(path.relative(root, path.join(successor.directory, 'owner.json')))), 'utf8') === successor.contents;
  } catch { return false; }
}

function assertPrivateInventory(directory) {
  // These are protocol metadata, not an invitation to recursively delete any
  // extra files someone placed in the private state or recovery directory.
  const files = new Set(['owner.json', 'journal.json', 'journal.initial.json', 'journal.json.committed']);
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isFile() && files.has(entry.name)) continue;
    if (entry.name === 'recovery-claims' && entry.isDirectory()) {
      for (const claim of readdirSync(path.join(directory, entry.name), { withFileTypes: true })) {
        if (!claim.isFile() || !(/^[1-9][0-9]*\.json$/.test(claim.name) ||
          /^candidate-[0-9a-f-]{36}\.json$/.test(claim.name) ||
          /^released-[1-9][0-9]*-[0-9a-f-]{36}$/.test(claim.name))) {
          throw new Error(`Unknown publication recovery metadata: ${claim.name}`);
        }
      }
      continue;
    }
    throw new Error(`Unknown publication lock metadata: ${entry.name}`);
  }
}

function removeLock(lock) {
  assertPrivateInventory(lock.directory);
  if (lock.legacy) {
    rmSync(lock.directory, { recursive: true });
    return;
  }
  if (!isCurrentLock(lock)) throw new Error('Publication lock owner changed; cleanup refused.');
  // Unpublish first, after all artifact recovery is complete. A killed cleanup
  // may leave its unique private directory, but never an active lock pointing
  // to a missing state directory. Later owners use different directories.
  rmSync(lock.path);
  rmSync(lock.directory, { recursive: true });
}

export function acquireProducerLock(lockDirectory, root = path.dirname(lockDirectory)) {
  assertNoSymlinkAncestors(root, lockDirectory);
  const owner = { schema: lockSchema, pid: process.pid, host: os.hostname(), id: randomUUID() };
  const directory = privateStatePath(lockDirectory, owner, root);
  const contents = `${JSON.stringify(owner)}\n`;
  mkdirSync(directory);
  try {
    const ownerPath = path.join(directory, 'owner.json');
    writeCompleteFile(ownerPath, contents);
    // link is both atomic publication and create-only admission. Unlike a
    // directory rename, it cannot replace even an existing empty legacy lock.
    linkSync(ownerPath, lockDirectory);
  } catch (error) {
    assertPrivateInventory(directory);
    rmSync(directory, { recursive: true });
    if (error.code === 'EEXIST') throw new Error(`Proto UI producer is locked at ${lockDirectory}. If its owner stopped, run --recover-publication; do not delete an active lock.`);
    throw error;
  }
  const lock = { path: lockDirectory, directory, contents, owner, legacy: false };
  let released = false;
  return () => {
    if (released || !isCurrentLock(lock)) return;
    // An unsuccessful rollback deliberately keeps the journal and lock.
    if (existsSync(path.join(directory, 'journal.json'))) return;
    removeLock(lock);
    released = true;
  };
}

function checkedJournal(lockDirectory, root) {
  const journal = JSON.parse(readFileSync(path.join(lockDirectory, 'journal.json'), 'utf8'));
  if (journal.schema !== 'verdant.proto-ui.publication.v1' || !Array.isArray(journal.replacements)) throw new Error('Invalid publication recovery journal.');
  for (const replacement of journal.replacements) {
    for (const key of ['target', 'staging', 'backup']) ownedPath(root, replacement[key]);
    const target = path.dirname(replacement.target);
    if (path.dirname(replacement.staging) !== target || path.dirname(replacement.backup) !== target ||
        !path.basename(replacement.staging).includes('.producer-') || !replacement.backup.endsWith('.backup')) {
      throw new Error('Publication recovery paths are not sibling staging/backup paths.');
    }
  }
  return journal;
}

function rollback(lockDirectory, root) {
  const journal = checkedJournal(lockDirectory, root);
  for (const replacement of [...journal.replacements].reverse()) {
    const target = insidePath(root, replacement.target);
    const backup = insidePath(root, replacement.backup);
    const staging = insidePath(root, replacement.staging);
    if (existsSync(backup)) {
      rmSync(target, { force: true, recursive: true });
      renameSync(backup, target);
    } else if (!replacement.existed) {
      rmSync(target, { force: true, recursive: true });
    }
    rmSync(staging, { force: true, recursive: true });
  }
  rmSync(path.join(lockDirectory, 'journal.json'));
}

function ownerStopped(owner) {
  if (owner.host !== os.hostname() || !Number.isSafeInteger(owner.pid) || owner.pid < 1) throw new Error('Cannot establish that the publication owner has stopped on this host.');
  try { process.kill(owner.pid, 0); return false; }
  catch (error) { if (error.code === 'ESRCH') return true; throw error; }
}

function acquireRecoveryClaim(lockDirectory, root, legacy = false) {
  // Immutable numbered claims avoid the stale-lock unlink race: only one
  // process can hard-link a fully written owner record at the next generation.
  // A killed recovery is superseded only after its PID has stopped. Previous
  // claims are never removed individually, so a second reaper cannot unlink a
  // new owner's claim after observing the same old, dead owner.
  const claims = ownedPath(root, slash(path.relative(root, path.join(lockDirectory, 'recovery-claims'))));
  // Never recreate the ordinary lock after another recovery has removed it.
  try { mkdirSync(claims); }
  catch (error) { if (error.code !== 'EEXIST') throw error; }
  const owner = { pid: process.pid, host: os.hostname(), id: randomUUID() };
  const candidate = path.join(claims, `candidate-${owner.id}.json`);
  writeFileSync(candidate, `${JSON.stringify(owner)}\n`, { flag: 'wx' });
  try {
    for (;;) {
      const generations = readdirSync(claims).filter((name) => /^[1-9][0-9]*\.json$/.test(name)).map((name) => Number(name.slice(0, -5)));
      const latest = generations.length ? Math.max(...generations) : 0;
      if (!Number.isSafeInteger(latest + 1)) throw new Error('Recovery claim generation is invalid.');
      if (latest) {
        const previous = JSON.parse(readFileSync(ownedPath(root, slash(path.relative(root, path.join(claims, `${latest}.json`)))), 'utf8'));
        if (typeof previous.id !== 'string' || !/^[0-9a-f-]{36}$/.test(previous.id)) throw new Error('Invalid recovery owner record.');
        const released = existsSync(path.join(claims, `released-${latest}-${previous.id}`));
        if (!released && !ownerStopped(previous)) throw new Error('Publication recovery is already in progress.');
      }
      const next = latest + 1;
      try { linkSync(candidate, path.join(claims, `${next}.json`)); }
      catch (error) { if (error.code === 'EEXIST') continue; throw error; }
      return () => {
        // A release marker is specific to this immutable generation and owner;
        // it cannot release a new producer/recovery that reused the pathname.
        try { writeFileSync(path.join(claims, `released-${next}-${owner.id}`), '', { flag: 'wx' }); }
        catch (error) {
          if (error.code !== 'ENOENT' && error.code !== 'EEXIST' &&
              !(error.code === 'ENOTDIR' && legacy &&
                (!existsSync(lockDirectory) || hasFileLockSuccessor(lockDirectory, root)))) throw error;
        }
      };
    }
  } finally {
    try { rmSync(candidate, { force: true }); }
    catch (error) {
      if (!(error.code === 'ENOTDIR' && legacy &&
          (!existsSync(lockDirectory) || hasFileLockSuccessor(lockDirectory, root)))) throw error;
    }
  }
}

export function recoverPublication(lockDirectory, root) {
  assertNoSymlinkAncestors(root, lockDirectory);
  if (!existsSync(lockDirectory)) return false;
  let lock;
  try { lock = readLock(lockDirectory, root); }
  catch (error) { if (error.code === 'ENOENT' && !existsSync(lockDirectory)) return false; throw error; }
  let release;
  try { release = acquireRecoveryClaim(lock.directory, root, lock.legacy); }
  catch (error) {
    if (['ENOENT', 'ENOTDIR'].includes(error.code) && (!existsSync(lockDirectory) ||
        (!lock.legacy && !isCurrentLock(lock)) || (lock.legacy && hasFileLockSuccessor(lockDirectory, root)))) return false;
    throw error;
  }
  try {
    const ownerPath = ownedPath(root, slash(path.relative(root, path.join(lock.directory, 'owner.json'))));
    const contents = readFileSync(ownerPath, 'utf8');
    if (!lock.legacy && (contents !== lock.contents || !isCurrentLock(lock))) throw new Error('Publication lock identity differs from its private owner record.');
    const owner = JSON.parse(contents);
    if (!ownerStopped(owner)) throw new Error(`Publication owner ${owner.pid} is still running; recovery refused.`);
    assertPrivateInventory(lock.directory);
    if (existsSync(path.join(lock.directory, 'journal.json'))) {
      const journal = checkedJournal(lock.directory, root);
      if (journal.committed) {
        for (const replacement of journal.replacements) {
          rmSync(insidePath(root, replacement.backup), { recursive: true, force: true });
          rmSync(insidePath(root, replacement.staging), { recursive: true, force: true });
        }
        rmSync(path.join(lock.directory, 'journal.json'));
      } else rollback(lock.directory, root);
    }
    removeLock(lock);
    return true;
  } finally { release(); }
}

export function publishBatch(units, { root, lockDirectory, rename = renameSync } = {}) {
  assertNoSymlinkAncestors(root, lockDirectory);
  const lock = readLock(lockDirectory, root);
  if (lock.legacy || lock.owner.pid !== process.pid || lock.owner.host !== os.hostname() ||
      readFileSync(ownedPath(root, slash(path.relative(root, path.join(lock.directory, 'owner.json')))), 'utf8') !== lock.contents) {
    throw new Error('Publication requires this process\'s complete owner record.');
  }
  assertPrivateInventory(lock.directory);
  for (const unit of units) assertOwnedInventory(unit.outDirectory, unit.receiptPath, root);
  const replacements = [];
  const journalPath = path.join(lock.directory, 'journal.json');
  const add = (target, staging) => {
    assertNoSymlinkAncestors(root, target);
    assertNoSymlinkAncestors(root, staging);
    const replacement = {
      target: slash(path.relative(root, target)), staging: slash(path.relative(root, staging)),
      backup: slash(path.relative(root, `${target}.producer-${randomUUID()}.backup`)), existed: existsSync(target),
    };
    replacements.push(replacement);
    return replacement;
  };
  const plans = units.map((unit) => ({
    unit,
    output: add(unit.outDirectory, `${unit.outDirectory}.producer-${randomUUID()}.staging`),
    receipt: add(unit.receiptPath, `${unit.receiptPath}.producer-${randomUUID()}.staging`),
  }));
  const journal = { schema: 'verdant.proto-ui.publication.v1', committed: false, replacements };
  // Record all planned staging paths before the first write. A process killed
  // during staging can therefore recover without orphaned untracked outputs.
  const initialJournal = path.join(lock.directory, 'journal.initial.json');
  writeCompleteFile(initialJournal, `${JSON.stringify(journal)}\n`);
  try { linkSync(initialJournal, journalPath); }
  finally { rmSync(initialJournal); }
  try {
    for (const { unit, output, receipt } of plans) {
      const staging = ownedPath(root, output.staging);
      mkdirSync(path.dirname(staging), { recursive: true });
      mkdirSync(staging);
      const seen = new Set();
      for (const artifact of unit.artifacts) {
        if (seen.has(artifact.path)) throw new Error(`Duplicate emitted artifact: ${artifact.path}`);
        seen.add(artifact.path);
        const filename = insidePath(staging, artifact.path);
        mkdirSync(path.dirname(filename), { recursive: true });
        writeFileSync(filename, artifact.contents, { flag: 'wx' });
      }
      mkdirSync(path.dirname(unit.receiptPath), { recursive: true });
      const stagedReceipt = ownedPath(root, receipt.staging);
      writeFileSync(stagedReceipt, `${JSON.stringify(unit.receipt, null, 2)}\n`, { flag: 'wx' });
    }
    for (const replacement of replacements) {
      const target = insidePath(root, replacement.target);
      if (replacement.existed) rename(target, insidePath(root, replacement.backup));
      rename(insidePath(root, replacement.staging), target);
    }
    // The complete batch is now visible. A crash during cleanup finishes the
    // cleanup instead of attempting a rollback with already removed backups.
    const committedJournal = `${journalPath}.committed`;
    writeFileSync(committedJournal, `${JSON.stringify({ ...journal, committed: true })}\n`, { flag: 'wx' });
    renameSync(committedJournal, journalPath);
    for (const replacement of replacements) rmSync(insidePath(root, replacement.backup), { recursive: true, force: true });
    rmSync(journalPath);
  } catch (error) {
    if (existsSync(journalPath)) {
      if (!checkedJournal(lock.directory, root).committed) rollback(lock.directory, root);
    } else {
      for (const replacement of replacements) rmSync(insidePath(root, replacement.staging), { recursive: true, force: true });
    }
    throw error;
  }
}
