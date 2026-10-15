// Cargo's builds of rust-js crates (ADR 0101). Cargo owns resolution; this
// adapter does not infer dependencies from source files or run build scripts.
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, renameSync, rmdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { promisify } from "node:util";

import { parseManifest } from "./manifest.js";
import { commit, fingerprint, removable } from "./publish.js";

/** rust-js's target (ADR 0090), which Cargo checks its crates for: the compiler's `TARGET`. */
export const target = "wasm32-unknown-unknown";

const execute = promisify(execFile);

/** Cargo's environment of a check through rust-js: rust-js as the wrapper
 * of every crate, compiling those `crates` names, the app's own, and
 * rustc's shim, the React the react crate is built for, the app,
 * `RUST_JS_APP`, a program no crate of the build uses (ADR 0360), and
 * where `@rust-js/typescript` is, `@rust-js/build`'s own, which prints each
 * module's `.d.ts` (ADRs 0207, 0412).
 * @param {string} compiler
 * @param {string | undefined} react
 * @param {{ app?: string, crates: string[], dependencies?: { dir: string }[] }} build
 * @returns {NodeJS.ProcessEnv} */
export function cargoEnv(compiler, react, { app, crates, dependencies = [] }) {
  const env = {
    ...process.env,
    RUSTC_WRAPPER: resolve(compiler),
    RUSTC: rustcShim(resolve(compiler)),
    RUST_JS_CRATES: JSON.stringify(crates),
    RUST_JS_DEPENDENCIES: JSON.stringify(dependencies.map(({ dir }) => dir)),
    RUST_JS_TYPESCRIPT: createRequire(import.meta.url).resolve("@rust-js/typescript"),
  };
  if (react) env.RUST_JS_REACT = react;
  else delete env.RUST_JS_REACT;
  if (app) env.RUST_JS_APP = app;
  else delete env.RUST_JS_APP;
  return env;
}

/** What a check builds: the app, the package it's asked for, or else its
 * manifest's own, as Cargo says, a workspace's root without one having
 * none; and the crates rust-js compiles, the app's own, each of Cargo's
 * packages of a path, not a registry's or git's: the workspace's members,
 * a path dependency, and a crate an npm package has, which the patch names
 * (ADR 0414), by its directory. A binding crate, one whose
 * `[package.metadata.rust-js]` says `bindings = true`, is rustc's: what it
 * binds is JS's own, and rust-js reads its metadata (ADR 0024). And the
 * crates of crates.io's or git's, `dependencies`, each by its directory,
 * name and version, which rust-js compiles where it can, its types alone
 * where it can't (ADR 0415).
 * @param {{ manifestPath: string, packageName?: string, toolchain?: string, offline?: boolean }} options
 * @returns {Promise<{ app?: string, crates: string[], dependencies: { dir: string, name: string, version: string }[] }>} */
export async function cargoBuild({ manifestPath, packageName, toolchain, offline = false }) {
  const manifest = resolve(manifestPath);
  const args = [...(toolchain ? [`+${toolchain}`] : []), "metadata", "--format-version=1", "--manifest-path", manifest, ...(offline ? ["--offline"] : [])];
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), maxBuffer: 64 * 1024 * 1024 }).catch((error) => {
    throw new Error(`cargo metadata failed:\n${error.stderr ?? error.message}`);
  });
  const { packages } = JSON.parse(stdout);
  const app = packageName ?? packages.find((pkg) => realpathSync(pkg.manifest_path) === realpathSync(manifest))?.name;
  const compiled = packages.filter((pkg) => pkg.metadata?.["rust-js"]?.bindings !== true);
  const crates = compiled
    .filter((pkg) => pkg.source === null)
    .map((pkg) => realpathSync(dirname(pkg.manifest_path)))
    .sort();
  const dependencies = compiled
    .filter((pkg) => pkg.source !== null)
    .map((pkg) => ({ dir: realpathSync(dirname(pkg.manifest_path)), name: pkg.name, version: pkg.version }))
    .sort((a, b) => a.dir.localeCompare(b.dir));
  return { app, crates, dependencies };
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
export async function editorCheck({ manifestPath, toolchain, compiler, react, targetDir }) {
  if (!exactToolchain(toolchain)) throw new Error("Cargo builds require an exact toolchain pin");
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "check", "--message-format=json", "--target", target, "--manifest-path", manifest];
  const env = { ...cargoEnv(compiler, react, await cargoBuild({ manifestPath: manifest, toolchain })), CARGO_TARGET_DIR: targetDir };
  return new Promise((done, failed) => {
    spawn("cargo", args, { cwd: dirname(manifest), env, stdio: ["ignore", "inherit", "inherit"] })
      .on("error", failed)
      .on("close", (code) => done(code ?? 1));
  });
}

/** Cargo's rustc, for every crate it compiles: rust-js's `--rustc`, rustc
 * with rust-js's tool known (ADR 0112). The binding crates need it, which
 * the wrapper gives to rustc, and a plain rustc doesn't know the tool. The
 * same pinned rustc, for the rest. */
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
 * declarations. With `check` too, nothing is written: a JS or `.d.ts`
 * file beside the Rust that isn't what it compiles to is an error, which
 * names each (ADR 0412).
 * `files` is each crate's modules' JS, by its name, where it's served from.
 * `targetDir` is where Cargo builds, its own `target/` without one.
 * @param {{ manifestPath: string, toolchain: string, compiler: string, packageName?: string, features?: string[], noDefaultFeatures?: boolean, offline?: boolean, react?: string, inSource?: boolean, check?: boolean, routes?: string[], targetDir?: string }} options
 * `manifests` is each crate's, validated, as the compiler wrote it.
 * @returns {Promise<{ js: string, crates: Map<string, { js: string, manifest: string }>, manifests: Map<string, any>, files: Map<string, string[]> }>}
 */
export async function checkCargo({ manifestPath, toolchain, compiler, packageName, features = [], noDefaultFeatures = false, offline = false, react, inSource = false, check = false, routes = [], packages = [], targetDir }) {
  if (!exactToolchain(toolchain)) throw new Error("Cargo builds require an exact toolchain pin");
  const manifest = resolve(manifestPath);
  const args = [`+${toolchain}`, "check", "--message-format=json", "--target", target, "--manifest-path", manifest];
  if (packageName) args.push("-p", packageName);
  if (features.length) args.push("--features", features.join(","));
  if (noDefaultFeatures) args.push("--no-default-features");
  if (offline) args.push("--offline");
  const build = await cargoBuild({ manifestPath: manifest, packageName, toolchain, offline });
  const env = { ...cargoEnv(compiler, react, build), ...(targetDir ? { CARGO_TARGET_DIR: targetDir } : {}) };
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), env, maxBuffer: 64 * 1024 * 1024 }).catch((error) => {
    const messages = String(error.stdout ?? "").split("\n").filter(Boolean).map(line => JSON.parse(line));
    // A package's crate is quiet: what it warns of is its author's, which
    // the app can't act on, as ReScript has a dependency's (ADR 0414).
    const quiet = (m) => m.message.level === "warning" && packageOf(packages, realpathSync(m.manifest_path));
    const rendered = messages.filter(m => m.reason === "compiler-message" && !quiet(m)).map(m => m.message.rendered).join("");
    throw new Error(`cargo check failed:\n${rendered}${error.stderr ?? ""}`);
  });
  const crates = new Map();
  let js;
  let compiled = false;
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
    const built = existsSync(library) ? parseManifest(readFileSync(library, "utf8")) : { artifacts: [{ file: library }] };
    // What it published, as it published it, for a build Cargo has as done
    // too: Cargo checks its own outputs, not rust-js's.
    const changed = built.artifacts.find(({ file, hash }) => !existsSync(file) || fingerprint(readFileSync(file)) !== hash);
    if (changed) {
      const name = packageNameOf(message.package_id);
      throw new Error(`${changed.file} isn't what rust-js wrote for Cargo's build of ${name}, which has it as done: `
        + `\`cargo clean -p ${name} --target ${target}\` to build it again`);
    }
    // The crate root's module: `lib.jsx` when it has JSX (ADR 0075); none
    // for a root of `mod`s alone, an app's list of its files (ADR 0382).
    ledger ??= join(dirname(dirname(marker)), "rust-js-in-source.json");
    const entry = { js: built.modules.find(module => module.module.length === 0)?.file, manifest: library };
    crates.set(message.target.name, entry);
    if (packageName ? packageNameOf(message.package_id) === packageName : message.manifest_path === manifest) {
      compiled = true;
      js = entry.js;
    }
  }
  if (!compiled) throw new Error(`rust-js compiled no library of ${packageName ?? manifest}`);
  // Each with its crate's name, as Cargo has it: the app's manifest, a
  // program's, has no library to say it (ADR 0360).
  const validated = new Map([...crates].map(([crate, { manifest }]) => [crate, parseManifest(readFileSync(manifest, "utf8"))]));
  const manifests = [...validated].map(([crate, manifest]) => ({ crate, ...manifest }));
  // Each crate's modules' JS, where it's served from.
  const served = (where) => new Map(manifests.map(({ crate, modules }) => [crate, modules.map((module) => where(module.file))]));
  if (!inSource) return { js, crates, manifests: validated, files: served((file) => file) };
  // A crate of crates.io's JS is the app's, where its Cargo.toml is (ADR 0415).
  const vendored = build.dependencies.map((dependency) => ({ ...dependency, to: join(dirname(manifest), "rust-js", "crates", `${dependency.name}-${dependency.version}`) }));
  const moved = writeInSource(manifests, ledger, routes, { check, packages, vendored });
  for (const entry of crates.values()) entry.js = moved.get(entry.js);
  return { js: moved.get(js), crates, manifests: validated, files: served((file) => moved.get(file)) };
}

/**
 * Each module's JS, and its map, beside its Rust, as ReScript writes it in
 * source and a project commits it (ADRs 0041, 0273), where the compiler
 * says, its manifest's `in_source`: `src/api.rs`'s is `src/api.js`, or
 * `.jsx`, and an inline `mod inner { .. }`'s is where its file would be.
 * It's an error if that's another module's, before anything's written. An import of another crate's module is of its
 * copy, where it is; any other stays as written. A file is written only if
 * it changed. What's written is published as a build's output is (ADR
 * 0091): every byte prepared first, then all of it or none. `ledger` records
 * what was written beside each crate's Rust, and how; a file it records
 * that this build doesn't write goes, if it's still as written: an edited
 * one is a person's, and one rust-js wrote some other way isn't this
 * build's. Where each JS went.
 * @param {{ crate?: string, library?: { name: string }, modules: { file: string, map?: string, types?: string, source: string, module: string[], in_source?: string, links?: { start: number, end: number, file: string }[], type_links?: { start: number, end: number, file: string }[] }[] }[]} manifests
 * @param {string} ledger
 * @param {string[]} [routes] directories where each file is a route,
 * Next.js's `pages/`: a module's there gets no declarations, which
 * Turbopack would take as a route of its own (ADR 0276).
 * @param {{ check?: boolean, packages?: { name: string, dir: string }[], vendored?: { dir: string, to: string }[] }} [options] `packages`,
 * the npm packages whose crates the build compiles, as the patch has them:
 * what's in one is the package's (ADR 0414), imported by its name, replaced
 * and never written through, as a store may share it, and not the app's to
 * check. `vendored`, the crates of crates.io's or git's it compiles: each
 * one's JS is where `to` says, in the app, its Rust being Cargo's, not the
 * app's to write beside (ADR 0415).
 * @returns {Map<string, string>}
 */
export function writeInSource(manifests, ledger, routes = [], { check = false, packages = [], vendored = [] } = {}) {
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
  for (const { crate, library, modules } of manifests) {
    for (const { file, in_source, links } of modules) {
      // A manifest of an older compiler's says neither where its JS goes nor
      // where it names another's, and Cargo has its build as done.
      if (in_source == null || links === undefined) {
        throw new Error(`${file} was built by an older rust-js, whose manifest doesn't say where its JS goes in source: `
          + `\`cargo clean --target ${target}\` to build it again`);
      }
      const crate = vendored.find(({ dir }) => in_source.startsWith(dir + sep));
      moved.set(file, crate ? join(crate.to, relative(crate.dir, in_source)) : in_source);
    }
    for (const { file, module } of [...modules].sort((a, b) => a.module.length - b.module.length)) {
      const to = moved.get(file);
      const here = { crate: library?.name ?? crate, module };
      const other = byDestination.get(to);
      if (other) throw new Error(`rust-js can't write the JS in source: ${named(other, here)} and ${named(here, other)} would both be ${to}; rename the module`);
      byDestination.set(to, here);
    }
  }
  // Every file's bytes, each crate's, before any is written: a read or a
  // map that fails, fails here.
  const writes = new Map();
  // A crate of crates.io's module is written where the app imports it, from
  // its own or such a module the app imports: one no module of the app's
  // reaches, serde_json's crates', which rustc builds, isn't (ADR 0415).
  const all = manifests.flatMap(({ modules }) => modules);
  const vendoredFile = (file) => vendored.some(({ to }) => moved.get(file)?.startsWith(to + sep));
  const reached = new Set();
  const pending = all.filter(({ file }) => !vendoredFile(file));
  while (pending.length) {
    const { links = [], type_links = [] } = /** @type {typeof all[number]} */ (pending.pop());
    for (const { file } of [...links, ...type_links]) {
      if (!vendoredFile(file) || reached.has(file)) continue;
      reached.add(file);
      const module = all.find((m) => m.file === file);
      if (module) pending.push(module);
    }
  }
  const crates = manifests.map(({ input, modules }) => {
    const files = [];
    for (const { file, map, types, links, type_links } of modules) {
      if (vendoredFile(file) && !reached.has(file)) continue;
      const to = moved.get(file);
      const js = relinked(readFileSync(file), links, to, moved, packages).toString("utf8")
        .replace(/\/\/# sourceMappingURL=\S+(\s*)$/, `//# sourceMappingURL=${basename(to)}.map$1`);
      writes.set(to, Buffer.from(js));
      files.push(to);
      // Its declarations, `Tag.d.ts` beside `Tag.jsx` (ADR 0196), naming
      // what they import where it's copied too.
      if (types && existsSync(types) && !routed(to)) {
        const typesTo = to.replace(/\.jsx?$/, ".d.ts");
        writes.set(typesTo, relinked(readFileSync(types), type_links ?? [], typesTo, moved, packages));
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
    // The app's crates of crates.io's are one, `rust-js/crates/`: one no
    // longer built, another version's, is what goes (ADR 0415).
    const crate = vendored.find(({ dir }) => input.startsWith(dir + sep));
    return { root: crate ? dirname(crate.to) : dirname(input), files };
  });
  // What this build wrote before beside each of its crates' Rust, by the
  // ledger, that it doesn't now: gone, if no one has changed it since.
  const recorded = existsSync(ledger) ? JSON.parse(readFileSync(ledger, "utf8")) : { version: 1, crates: {} };
  const reads = manifests.flatMap((manifest) => [manifest.input, ...(manifest.sources ?? [])].filter(Boolean));
  const roots = [...new Set(crates.map(({ root }) => root))];
  const stale = removable(roots.flatMap((root) => recorded.crates[root] ?? []), [...writes.keys(), ...reads]);
  for (const root of roots) {
    const files = crates.filter((crate) => crate.root === root).flatMap((crate) => crate.files);
    recorded.crates[root] = files.map((file) => ({ file, hash: fingerprint(writes.get(file)) }));
  }
  // Checked, not written: each committed file that isn't what it compiles
  // to, or that's missing. A map, which a project doesn't commit, isn't, nor
  // a package's, which isn't the app's.
  if (check) {
    const outdated = [...writes].filter(([file, bytes]) => !file.endsWith(".map") && !packageOf(packages, file)
      && !(existsSync(file) && readFileSync(file).equals(Buffer.from(bytes))));
    if (outdated.length) {
      const files = outdated.map(([file]) => `  ${relative(process.cwd(), file)}`).join("\n");
      throw new Error(`rust-js: the JS beside the Rust isn't what it compiles to; run \`rust-js build\`:\n${files}`);
    }
    return moved;
  }
  writes.set(ledger, Buffer.from(JSON.stringify(recorded, null, 2) + "\n"));
  commit(writes, stale, ledger, (file) => !packageOf(packages, file));
  // A crate of crates.io's another version's, or gone, leaves no directory
  // behind, as its JS was all rust-js's (ADR 0415).
  for (const file of stale) {
    const root = vendored.map(({ to }) => dirname(to)).find((dir) => file.startsWith(dir + sep));
    for (let dir = dirname(file); root && dir.startsWith(root + sep) && existsSync(dir) && readdirSync(dir).length === 0; dir = dirname(dir)) {
      rmdirSync(dir);
    }
  }
  return moved;
}

/**
 * The npm package of `packages` that `file` is in, if one is: a crate's
 * there is the package's (ADR 0414).
 * @param {{ name: string, dir: string }[]} packages
 * @param {string} file
 */
export function packageOf(packages, file) {
  return packages.find(({ dir }) => file.startsWith(dir + sep));
}

/**
 * `bytes`, a file rust-js wrote, for where it's copied, `to`: each
 * specifier the compiler found in it by a parse, its manifest's `links`
 * (ADR 0101), that names a file copied too names the copy, from `to`;
 * after a directive, in an `export .. from` or in a declaration alike. One
 * in another npm package than `to` is by the package's name, as Node finds
 * it wherever its package manager put it, `@acme/greet/src/lib.js` (ADR
 * 0414). Nothing else changes: a string that reads like an import is the
 * program's.
 * @param {Buffer} bytes
 * @param {{ start: number, end: number, file: string }[]} links
 * @param {string} to
 * @param {Map<string, string>} moved
 * @param {{ name: string, dir: string }[]} packages
 */
function relinked(bytes, links, to, moved, packages) {
  const parts = [];
  let at = 0;
  for (const { start, end, file } of links) {
    const target = moved.get(file);
    if (!target) continue;
    const home = packageOf(packages, target);
    const path = relative(dirname(to), target);
    const specifier = home && home !== packageOf(packages, to)
      ? imported(home, `./${relative(home.dir, target).split(sep).join("/")}`)
      : path.startsWith(".") ? path : `./${path}`;
    parts.push(bytes.subarray(at, start), Buffer.from(specifier));
    at = end;
  }
  parts.push(bytes.subarray(at));
  return Buffer.concat(parts);
}

/**
 * The specifier a package's file, `subpath` in it, `./src/lib.js`, is
 * imported by: the package's name and the path, `@acme/greet/src/lib.js`,
 * which its `exports`, if it has them, export as that file, as Node reads
 * them; else an error that says what to export (ADR 0414).
 * @param {{ name: string, dir: string }} home
 * @param {string} subpath
 */
function imported(home, subpath) {
  const specifier = `${home.name}${subpath.slice(1)}`;
  const { exports } = JSON.parse(readFileSync(join(home.dir, "package.json"), "utf8"));
  if (exports === undefined || exportsFile(exports, subpath)) return specifier;
  throw new Error(`${home.name}'s \`exports\` hides ${subpath}, which the app's JS imports, \`${specifier}\`: export it, \`"./src/*": "./src/*"\``);
}

/** Whether `exports`, a package's, export `subpath` as that file: by its
 * own key, or a pattern's, `"./src/*": "./src/*"`, of any condition. Ones
 * with no subpath export only the package's root. */
function exportsFile(exports, subpath) {
  if (typeof exports !== "object" || exports === null || Array.isArray(exports)) return false;
  /** @returns {string[]} */
  const targets = (value) => typeof value === "string" ? [value]
    : Array.isArray(value) ? value.flatMap(targets)
    : value && typeof value === "object" ? Object.values(value).flatMap(targets) : [];
  return Object.entries(exports).some(([key, value]) => {
    if (!key.startsWith(".")) return false;
    const star = key.indexOf("*");
    if (star < 0) return key === subpath && targets(value).includes(subpath);
    const before = key.slice(0, star), after = key.slice(star + 1);
    if (subpath.length < before.length + after.length || !subpath.startsWith(before) || !subpath.endsWith(after)) return false;
    const middle = subpath.slice(before.length, subpath.length - after.length);
    return targets(value).some((target) => target.replaceAll("*", middle) === subpath);
  });
}

/**
 * The workspace a Cargo manifest is of, a member's or the root's, and where
 * Cargo builds it: what Cargo reads, and what it writes (ADR 0101), where each of its
 * packages is, each one's library's root, `src/lib.rs` or its `[lib] path`,
 * and `own`, the package the manifest is of, none of a workspace's root
 * without one.
 * @param {{ manifestPath: string, toolchain?: string, offline?: boolean, targetDir?: string }} options
 * @returns {Promise<{ root: string, target: string, packages: Map<string, string>, libs: Map<string, string>, own?: string }>}
 */
export async function cargoWorkspace({ manifestPath, toolchain, offline = false, targetDir }) {
  const manifest = resolve(manifestPath);
  // Cargo's own Rust, without a pin: a workspace's metadata is the same.
  const args = [...(toolchain ? [`+${toolchain}`] : []), "metadata", "--no-deps", "--format-version=1", "--manifest-path", manifest, ...(offline ? ["--offline"] : [])];
  const env = { ...process.env, ...(targetDir ? { CARGO_TARGET_DIR: targetDir } : {}) };
  const { stdout } = await execute("cargo", args, { cwd: dirname(manifest), env, maxBuffer: 64 * 1024 * 1024 }).catch((error) => {
    throw new Error(`cargo metadata failed:\n${error.stderr ?? error.message}`);
  });
  const metadata = JSON.parse(stdout);
  const packages = new Map(metadata.packages.map((pkg) => [pkg.name, dirname(pkg.manifest_path)]));
  const libs = new Map(metadata.packages.flatMap((pkg) => {
    const lib = pkg.targets.find((target) => target.kind.includes("lib"));
    return lib ? [[pkg.name, lib.src_path]] : [];
  }));
  const own = metadata.packages.find((pkg) => realpathSync(pkg.manifest_path) === realpathSync(manifest))?.name;
  return { root: metadata.workspace_root, target: metadata.target_directory, packages, libs, own };
}

/** The package's name in a Cargo package ID: `path+file:///dir#name@1.0`, or `path+file:///dir/name#1.0`. */
function packageNameOf(id) {
  const [url, fragment = ""] = id.split("#");
  return fragment.includes("@") ? fragment.slice(0, fragment.lastIndexOf("@")) : url.slice(url.lastIndexOf("/") + 1);
}
