// Host publication for a successful WASI build. The manifest is committed last.
import { copyFileSync, existsSync, mkdirSync, readFileSync, realpathSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { parseManifest } from "./manifest.js";

export function fingerprint(bytes) {
  let hash = 0xcbf29ce484222325n;
  for (const byte of bytes) hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
  return hash.toString(16).padStart(16, "0");
}

// A path with its symlinks resolved, as the native compiler's `absolute`
// resolves them, so aliases of one file are one path; one that doesn't
// exist yet, through the part of it that does.
function real(path) {
  if (existsSync(path)) return realpathSync(path);
  const parent = dirname(resolve(path));
  return parent === resolve(path) ? parent : join(real(parent), basename(path));
}

// What an older build wrote that this one may remove is the compiler's to
// say, its manifest's `stale` (ADR 0375). Of each, what only the disk shows,
// which a WASI compiler can't see.
function staleArtifacts(manifest, writes) {
  return removable(manifest.stale ?? [], [manifest.input, ...manifest.sources, ...writes.keys()]);
}

/**
 * Of `candidates`, files an older build wrote, each as its `hash` says,
 * those this one removes: still there and as written, so no person's edit,
 * and none of `kept`, what this build reads or writes, by any name, a link
 * to one. The one rule, of a build's own output and of its copy in source.
 * @param {{ file: string, hash: string }[]} candidates
 * @param {Iterable<string>} kept
 * @returns {string[]}
 */
export function removable(candidates, kept) {
  const keeps = new Set([...kept].map(real));
  return candidates.filter(({ file, hash }) => existsSync(file)
    && !keeps.has(real(file))
    && fingerprint(readFileSync(real(file))) === hash)
    .map(({ file }) => real(file));
}

export function publishArtifacts(manifestPath, manifest, files) {
  parseManifest(JSON.stringify(manifest));
  for (const artifact of manifest.artifacts) {
    const bytes = files.get(artifact.file);
    if (!bytes || fingerprint(bytes) !== artifact.hash) throw new Error(`Missing or inconsistent artifact: ${artifact.file}`);
  }
  if (files.size !== manifest.artifacts.length) throw new Error("Unexpected artifacts outside manifest");
  // Whether the older manifest at `manifestPath` is this compilation's is
  // the compiler's to say, which a host gives it to (ADR 0375).
  const writes = new Map(files);
  // What's stale is told the publisher, and isn't what the build wrote.
  const { stale: _, ...published } = manifest;
  writes.set(manifestPath, Buffer.from(JSON.stringify(published, null, 2) + "\n"));
  refuseOverlaps(writes.keys(), [manifest.input, ...manifest.sources]);
  commit(writes, staleArtifacts(manifest, writes), manifestPath);
}

/**
 * Throws, before any file changes, where a write resolves to a file the
 * build reads, an output or the manifest that's a link to its Rust, or two
 * writes resolve to one file: a write through such a link would replace
 * the source with what was built from it.
 * @param {Iterable<string>} writes
 * @param {Iterable<string>} reads
 */
export function refuseOverlaps(writes, reads) {
  const read = new Set([...reads].map(real));
  const written = new Map();
  for (const path of writes) {
    const to = real(path);
    if (read.has(to)) throw new Error(`Refusing to write ${path}: it resolves to ${to}, which this build reads`);
    if (written.has(to)) throw new Error(`Refusing to write ${path}: it resolves to ${to}, as ${written.get(to)} does`);
    written.set(to, path);
  }
}

/**
 * Replace each of `writes`' files with its bytes, and remove `stale`, all
 * or none of them for an ordinary I/O failure: every byte is staged before
 * any file changes, each file replaced is kept until all are, and `last`,
 * the record of what was written, is written last. A file whose bytes are
 * already these keeps its mtime.
 * @param {Map<string, Buffer | string>} writes
 * @param {string[]} stale
 * @param {string} [last]
 * @param {(path: string) => boolean} [inPlace] whether a file replaced is
 * written in place, as a bundler watching it sees; one that isn't, an npm
 * package's, whose package manager's store may share it by a hard link with
 * other projects, is replaced by a new file, so the store's stays as it is
 * (ADR 0414).
 */
export function commit(writes, stale, last, inPlace = () => true) {
  const id = randomUUID();
  const staged = [];
  const changed = [];
  try {
    // Stage every byte before replacing any output. Unchanged files keep mtimes.
    for (const [path, data] of writes) {
      if (existsSync(path) && readFileSync(path).equals(Buffer.from(data))) continue;
      mkdirSync(dirname(path), { recursive: true });
      const stage = `${path}.${id}.stage`;
      staged.push(stage);
      writeFileSync(stage, data, { flag: "wx" });
      changed.push({ path, data, stage, backup: `${path}.${id}.backup`, saved: false, installed: false });
    }
    // Stale files are backed up too, so an ordinary I/O failure can roll back.
    const lastWrite = changed.find(change => change.path === last);
    const order = [...changed.filter(change => change !== lastWrite),
      ...stale.map((file) => ({ path: file, backup: `${file}.${id}.backup`, saved: false, installed: false })),
      ...(lastWrite ? [lastWrite] : [])];
    changed.splice(0, changed.length, ...order);
    for (const change of changed) {
      // One replaced is kept by a copy, and written in place, as an
      // editor saves a file: a bundler watching it, Next.js's Turbopack,
      // sees it change, where it can miss one renamed over it, and never
      // sees it missing (ADR 0192). A new one is renamed into place, and a
      // stale one moved away, as it's removed.
      const replaced = change.stage && existsSync(change.path) && inPlace(change.path);
      if (replaced) {
        copyFileSync(change.path, change.backup);
        change.saved = true;
        writeFileSync(change.path, change.data);
        change.installed = true;
      } else {
        if (existsSync(change.path)) {
          renameSync(change.path, change.backup);
          change.saved = true;
        }
        if (change.stage) {
          renameSync(change.stage, change.path);
          change.installed = true;
        }
      }
    }
  } catch (error) {
    const recovery = [];
    for (const change of [...changed].reverse()) {
      try {
        if (change.saved) renameSync(change.backup, change.path);
        else if (change.installed) rmSync(change.path);
      } catch (failure) { recovery.push(`${change.path}: ${failure}; backup: ${change.backup}`); }
    }
    throw new Error(`${error}${recovery.length ? `\nRecovery failures:\n${recovery.join("\n")}` : ""}`);
  } finally {
    for (const stage of staged) rmSync(stage, { force: true });
  }
  for (const change of changed) if (change.saved) rmSync(change.backup);
}
