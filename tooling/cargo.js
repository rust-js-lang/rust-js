// Cargo's builds of rust-js crates (ADR 0101), and the experimental
// local-library planning before them (ADR 0085). Cargo owns resolution; this
// adapter does not infer dependencies from source files or run build scripts.
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { promisify } from "node:util";

import { commit, fingerprint } from "./publish.js";

const execute = promisify(execFile);

/** Cargo's environment of a check through rust-js: the workspace's
 * wrapper, rustc's shim, and the React the react crate is built for. */
function cargoEnv(compiler, react) {
  const env = { ...process.env, RUSTC_WORKSPACE_WRAPPER: resolve(compiler), RUSTC: rustcShim(resolve(compiler)) };
  if (react) env.RUST_JS_REACT = react;
  else delete env.RUST_JS_REACT;
  return env;
}

/**
 * The check an editor runs, rust-analyzer's `check.overrideCommand` (ADR
 * 0222): the app's `cargo check`, as its build's, through rust-js, which
 * reads inside JSX as a plain rustc doesn't, Cargo's JSON messages printed as
 * they come. In a target directory of its own, `targetDir`, it neither waits
 * for the dev server's check nor makes it check again.
 * @param {{ manifestPath: string, toolchain: string, compiler: string, react?: string, targetDir: string }} options
 * @returns {Promise<number>} Cargo's exit code
 */
export function editorCheck({ manifestPath, toolchain, compiler, react, targetDir }) {
  if (!exactToolchain(toolchain)) throw new Error("Cargo builds require an exact toolchain pin");
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "check", "--message-format=json", "--target", "wasm32-unknown-unknown", "--manifest-path", manifest];
  const env = { ...cargoEnv(compiler, react), CARGO_TARGET_DIR: targetDir };
  return new Promise((done, failed) => {
    spawn("cargo", args, { cwd: dirname(manifest), env, stdio: ["ignore", "inherit", "inherit"] })
      .on("error", failed)
      .on("close", (code) => done(code ?? 1));
  });
}

/** Cargo's rustc, for every crate it compiles: rust-js's `--rustc`, rustc
 * with rust-js's tool known (ADR 0112). The binding crates need it, which
 * the workspace's wrapper runs for only when they're its members, and a
 * plain rustc doesn't know the tool. The same pinned rustc, for the rest. */
export function rustcShim(compiler) {
  const dir = join(tmpdir(), "rust-js", createHash("sha256").update(compiler).digest("hex").slice(0, 16));
  const shim = join(dir, "rustc");
  mkdirSync(dir, { recursive: true });
  // Written whole, then moved into place: every build with this compiler
  // shares the shim, and one rewriting it in place while another's Cargo
  // runs it fails both, Linux's ETXTBSY. A Cargo running it keeps the file
  // it opened; the next one runs the new.
  const writing = `${shim}.${process.pid}`;
  writeFileSync(writing, `#!/bin/sh\nexec ${JSON.stringify(compiler)} --rustc "$@"\n`, { mode: 0o755 });
  renameSync(writing, shim);
  return shim;
}

/** One exact toolchain: a release, `1.99.0`, or a dated nightly (ADR 0109). */
const exactToolchain = toolchain => /^(\d+\.\d+\.\d+|nightly-\d{4}-\d{2}-\d{2})$/.test(toolchain ?? "");

/** @param {{ manifestPath: string, toolchain: string, target: string, packageName?: string, features?: string[], noDefaultFeatures?: boolean }} options */
export async function planCargoLibraries({ manifestPath, toolchain, target, packageName, features = [], noDefaultFeatures = false }) {
  if (!exactToolchain(toolchain)) throw new Error("Cargo planning requires an exact toolchain pin");
  if (typeof target !== "string" || !target) throw new Error("Cargo planning requires an explicit target triple");
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "metadata", "--format-version=1", "--frozen", "--manifest-path", manifest, "--filter-platform", target];
  if (features.length) args.push("--features", features.join(","));
  if (noDefaultFeatures) args.push("--no-default-features");
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), maxBuffer: 64 * 1024 * 1024 });
  const metadata = JSON.parse(stdout);
  const packages = new Map(metadata.packages.map(pkg => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  const selected = packageName
    ? metadata.workspace_members.filter(id => packages.get(id).name === packageName)
    : metadata.resolve.root ? [metadata.resolve.root] : [];
  if (selected.length !== 1) throw new Error("Select one Cargo workspace library with packageName");
  const ordered = [];
  const visited = new Set();
  const visiting = new Set();
  const pending = [{ id: selected[0] }];
  while (pending.length) {
    const { id, library: completed } = pending.pop();
    if (completed) {
      visiting.delete(id);
      visited.add(id);
      ordered.push(completed);
      continue;
    }
    if (visited.has(id)) continue;
    if (visiting.has(id)) throw new Error(`Cyclic Cargo library dependency: ${id}`);
    const pkg = packages.get(id), node = nodes.get(id);
    if (!pkg || !node) throw new Error(`Cargo metadata is missing package ${id}`);
    if (pkg.source !== null) throw new Error(`Cargo JS planning currently supports only local path libraries: ${pkg.name}`);
    if (pkg.targets.some(t => t.kind.includes("custom-build"))) throw new Error(`Cargo JS planning does not support build scripts: ${pkg.name}`);
    if (pkg.targets.some(t => t.kind.includes("proc-macro"))) throw new Error(`Cargo JS planning does not support procedural macros: ${pkg.name}`);
    const libraries = pkg.targets.filter(t => t.kind.includes("lib"));
    if (libraries.length !== 1) throw new Error(`Cargo JS planning requires one ordinary library target: ${pkg.name}`);
    const library = libraries[0];
    const dependencies = node.deps.filter(dep => dep.dep_kinds.some(kind => kind.kind === null))
      .map(dep => ({ name: dep.name, packageId: dep.pkg })).sort((a, b) => a.name.localeCompare(b.name));
    visiting.add(id);
    pending.push({ id, library: {
      id, name: pkg.name, crateName: library.name, edition: library.edition,
      manifestPath: pkg.manifest_path, sourcePath: library.src_path,
      features: [...node.features].sort(), dependencies,
    } });
    for (const dependency of [...dependencies].reverse()) pending.push({ id: dependency.packageId });
  }
  return { root: selected[0], toolchain, target, workspaceRoot: metadata.workspace_root, libraries: ordered };
}

/**
 * `cargo check` of a workspace for rust-js's target, with rust-js as Cargo's
 * workspace wrapper (ADR 0101), and where each crate rust-js compiled has its
 * JS: beside the metadata Cargo keeps for that build of it, so a feature set
 * built before is the JS it was. Cargo reports each crate it built or found
 * fresh; the `.rust-js` beside its metadata says where its manifest is. `js` is
 * the selected package's, or the manifest's own package's. `react` is the
 * React release the react crate is checked for (ADR 0043), or its latest.
 * With `inSource`, each module's JS is written beside its Rust too, as a
 * project commits it (`writeInSource`), and `js` and `crates` are those;
 * `routes` are directories where a file is a route, which get no
 * declarations.
 * `files` is every module's JS, where it's served from.
 * @param {{ manifestPath: string, toolchain: string, compiler: string, packageName?: string, features?: string[], noDefaultFeatures?: boolean, offline?: boolean, react?: string, inSource?: boolean, routes?: string[] }} options
 * @returns {Promise<{ js: string, crates: Map<string, { js: string, manifest: string }>, files: string[] }>}
 */
export async function checkCargo({ manifestPath, toolchain, compiler, packageName, features = [], noDefaultFeatures = false, offline = false, react, inSource = false, routes = [] }) {
  if (!exactToolchain(toolchain)) throw new Error("Cargo builds require an exact toolchain pin");
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "check", "--message-format=json", "--target", "wasm32-unknown-unknown", "--manifest-path", manifest];
  if (packageName) args.push("-p", packageName);
  if (features.length) args.push("--features", features.join(","));
  if (noDefaultFeatures) args.push("--no-default-features");
  if (offline) args.push("--offline");
  const env = cargoEnv(compiler, react);
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), env, maxBuffer: 64 * 1024 * 1024 }).catch((error) => {
    const messages = String(error.stdout ?? "").split("\n").filter(Boolean).map(line => JSON.parse(line));
    const rendered = messages.filter(m => m.reason === "compiler-message").map(m => m.message.rendered).join("");
    throw new Error(`cargo check failed:\n${rendered}${error.stderr ?? ""}`);
  });
  const crates = new Map();
  let js;
  // What's written in source, and how, beside Cargo's builds: `cargo clean`
  // forgets it, and then nothing in source is taken as this build's.
  let ledger;
  for (const line of stdout.split("\n").filter(Boolean)) {
    const message = JSON.parse(line);
    if (message.reason !== "compiler-artifact") continue;
    const metadata = message.filenames.find(file => file.endsWith(".rmeta"));
    const marker = metadata?.replace(/\.rmeta$/, ".rust-js");
    if (!marker || !existsSync(marker)) continue;
    // Its own manifest, then those of the libraries it was compiled with.
    const library = readFileSync(marker, "utf8").split("\n")[0];
    const built = existsSync(library) ? JSON.parse(readFileSync(library, "utf8")) : { artifacts: [{ file: library }] };
    // What it published, as it published it, for a build Cargo has as done
    // too: Cargo checks its own outputs, not rust-js's.
    const changed = built.artifacts.find(({ file, hash }) => !existsSync(file) || fingerprint(readFileSync(file)) !== hash);
    if (changed) {
      const name = packageNameOf(message.package_id);
      throw new Error(`${changed.file} isn't what rust-js wrote for Cargo's build of ${name}, which has it as done: `
        + `\`cargo clean -p ${name} --target wasm32-unknown-unknown\` to build it again`);
    }
    // The crate root's module: `lib.jsx` when it has JSX (ADR 0075).
    ledger ??= join(dirname(dirname(marker)), "rust-js-in-source.json");
    const entry = { js: built.modules.find(module => module.module.length === 0).file, manifest: library };
    crates.set(message.target.name, entry);
    if (packageName ? packageNameOf(message.package_id) === packageName : message.manifest_path === manifest) js = entry.js;
  }
  if (!js) throw new Error(`rust-js compiled no library of ${packageName ?? manifest}`);
  const manifests = [...crates.values()].map(({ manifest }) => JSON.parse(readFileSync(manifest, "utf8")));
  if (!inSource) return { js, crates, files: manifests.flatMap(({ modules }) => modules.map((module) => module.file)) };
  const moved = writeInSource(manifests, ledger, routes);
  for (const entry of crates.values()) entry.js = moved.get(entry.js);
  return { js: moved.get(js), crates, files: [...moved.values()] };
}

/**
 * Each module's JS, and its map, beside its Rust, as ReScript writes it in
 * source and a project commits it (ADRs 0041, 0273): `src/api.rs`'s is
 * `src/api.js`, or `.jsx`. A module that isn't a file of its own, an
 * inline `mod inner { .. }`, is where its file would be: `src/inner.js`
 * of `src/lib.rs`'s, `src/api/inner.js` of `src/api.rs`'s, and it's an
 * error if that's another module's, before anything's written. An import of another crate's module is of its
 * copy, where it is; any other stays as written. A file is written only if
 * it changed. What's written is published as a build's output is (ADR
 * 0091): every byte prepared first, then all of it or none. `ledger` records
 * what was written beside each crate's Rust, and how; a file it records
 * that this build doesn't write goes, if it's still as written: an edited
 * one is a person's, and one rust-js wrote some other way isn't this
 * build's. Where each JS went.
 * @param {{ library: { name: string }, modules: { file: string, map?: string, types?: string, source: string, module: string[], located?: boolean }[] }[]} manifests
 * @param {string} ledger
 * @param {string[]} [routes] directories where each file is a route,
 * Next.js's `pages/`: a module's there gets no declarations, which
 * Turbopack would take as a route of its own (ADR 0276).
 * @returns {Map<string, string>}
 */
export function writeInSource(manifests, ledger, routes = []) {
  const routed = (file) => routes.some((dir) => !relative(resolve(dir), file).startsWith(".."));
  const moved = new Map();
  // `mod root { .. }` of a `[lib] path = "src/root.rs"` is where the root
  // is, and two crates' inline `mod helper` of `sources/alpha.rs` and
  // `sources/beta.rs` are in one place: none is written over another, as
  // rust-js's own output refuses `mod lib` of `lib.rs`.
  const byDestination = new Map();
  const named = ({ crate, module }, other) => {
    const own = module.length === 0 ? "the crate root" : `module \`${module.join("::")}\``;
    if (crate === other.crate) return own;
    return `crate \`${crate}\`'s ${module.length === 0 ? "root" : own}`;
  };
  for (const { library, modules } of manifests) {
    const crateRoot = modules.find((m) => m.module.length === 0).source;
    for (const { file, source, module, located } of modules) {
      const extension = file.endsWith(".jsx") ? ".jsx" : ".js";
      // The module whose file `source` is, by where it is: `src/api.rs`
      // and `src/api/mod.rs` are `api`'s.
      const own = source === crateRoot ? [] : relative(dirname(crateRoot), source).replace(/\.rs$/, "").split(sep);
      if (own.at(-1) === "mod") own.pop();
      // Or a `#[path]` module's file, wherever it is (ADR 0273).
      const written = (own.length === module.length && own.every((name, i) => name === module[i])) || located;
      moved.set(file, written ? source.replace(/\.rs$/, extension) : join(dirname(crateRoot), ...module) + extension);
    }
    for (const { file, module } of [...modules].sort((a, b) => a.module.length - b.module.length)) {
      const to = moved.get(file);
      const here = { crate: library.name, module };
      const other = byDestination.get(to);
      if (other) throw new Error(`rust-js can't write the JS in source: ${named(other, here)} and ${named(here, other)} would both be ${to}; rename the module`);
      byDestination.set(to, here);
    }
  }
  // Every file's bytes, each crate's, before any is written: a read or a
  // map that fails, fails here.
  const writes = new Map();
  const crates = manifests.map(({ modules }) => {
    const files = [];
    for (const { file, map, types } of modules) {
      const to = moved.get(file);
      writes.set(to, Buffer.from(relocated(readFileSync(file, "utf8"), file, to, moved)));
      files.push(to);
      // Its declarations, `Tag.d.ts` beside `Tag.jsx` (ADR 0196).
      if (types && existsSync(types) && !routed(to)) {
        const typesTo = to.replace(/\.jsx?$/, ".d.ts");
        writes.set(typesTo, readFileSync(types));
        files.push(typesTo);
      }
      if (map && existsSync(map)) {
        const json = JSON.parse(readFileSync(map, "utf8"));
        json.file = basename(to);
        json.sources = (json.sources ?? []).map((path) => relative(dirname(to), resolve(dirname(map), path)));
        writes.set(`${to}.map`, Buffer.from(JSON.stringify(json)));
        files.push(`${to}.map`);
      }
    }
    return { root: dirname(modules.find((module) => module.module.length === 0).source), files };
  });
  // What this build wrote before beside each of its crates' Rust, by the
  // ledger, that it doesn't now: gone, if no one has changed it since.
  const recorded = existsSync(ledger) ? JSON.parse(readFileSync(ledger, "utf8")) : { version: 1, crates: {} };
  const stale = crates.flatMap(({ root }) => (recorded.crates[root] ?? [])
    .filter(({ file, hash }) => !writes.has(file) && existsSync(file) && fingerprint(readFileSync(file)) === hash)
    .map(({ file }) => file));
  for (const { root, files } of crates) {
    recorded.crates[root] = files.map((file) => ({ file, hash: fingerprint(writes.get(file)) }));
  }
  writes.set(ledger, Buffer.from(JSON.stringify(recorded, null, 2) + "\n"));
  commit(writes, stale, ledger);
  return moved;
}

/**
 * `file`'s JS for where it's copied, `to`: each import declaration, which
 * rust-js writes before any code (src/to_oxc.rs), names the copy of what it
 * imports, and the source-map comment it ends with, the map beside it, by
 * its name now, where a root of another name, `src/App.rs`'s, was written
 * as the build's `lib.js`. Nothing else changes: a string that reads like
 * an import is the program's.
 * @param {string} text
 * @param {string} file
 * @param {string} to
 * @param {Map<string, string>} moved
 */
function relocated(text, file, to, moved) {
  const header = text.indexOf("\n") + 1;
  // `import { a } from "./x.js";`, `import x from "./x.js";` or
  // `import "./x.js";`, over lines or not, one after another.
  const declaration = /\s*import\b[^;"']*?(["'])([^"']+)\1\s*;/y;
  declaration.lastIndex = header;
  let end = header;
  const parts = [text.slice(0, header)];
  for (let found; (found = declaration.exec(text)); end = declaration.lastIndex) {
    const [whole, quote, spec] = found;
    const target = /^\.{1,2}\//.test(spec) ? moved.get(resolve(dirname(file), spec)) : undefined;
    if (!target) {
      parts.push(whole);
      continue;
    }
    const path = relative(dirname(to), target);
    const at = whole.lastIndexOf(`${quote}${spec}${quote}`);
    parts.push(`${whole.slice(0, at)}${quote}${path.startsWith(".") ? path : `./${path}`}${quote}${whole.slice(at + spec.length + 2)}`);
  }
  const rest = text.slice(end).replace(/\/\/# sourceMappingURL=\S+(\s*)$/, `//# sourceMappingURL=${basename(to)}.map$1`);
  return parts.join("") + rest;
}

/**
 * The workspace a Cargo manifest is of, a member's or the root's, and where
 * Cargo builds it: what Cargo reads, and what it writes (ADR 0101), and where each of its
 * packages is.
 * @param {{ manifestPath: string, toolchain: string, offline?: boolean }} options
 * @returns {Promise<{ root: string, target: string, packages: Map<string, string> }>}
 */
export async function cargoWorkspace({ manifestPath, toolchain, offline = false }) {
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "metadata", "--no-deps", "--format-version=1", "--manifest-path", manifest, ...(offline ? ["--offline"] : [])];
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), maxBuffer: 64 * 1024 * 1024 }).catch((error) => {
    throw new Error(`cargo metadata failed:\n${error.stderr ?? error.message}`);
  });
  const metadata = JSON.parse(stdout);
  const packages = new Map(metadata.packages.map((pkg) => [pkg.name, dirname(pkg.manifest_path)]));
  return { root: metadata.workspace_root, target: metadata.target_directory, packages };
}

/** The package's name in a Cargo package ID: `path+file:///dir#name@1.0`, or `path+file:///dir/name#1.0`. */
function packageNameOf(id) {
  const [url, fragment = ""] = id.split("#");
  return fragment.includes("@") ? fragment.slice(0, fragment.lastIndexOf("@")) : url.slice(url.lastIndexOf("/") + 1);
}
