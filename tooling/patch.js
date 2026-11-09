// The patch an app's Cargo needs to find the crates its npm packages have:
// each package with `"rust-js": { "crate": ".." }`, found as Node finds a
// package, from the app and from each such package, where its package
// manager put it, all in one `node_modules`, npm's and bun's way, or in a
// store, and linked, pnpm's. Cargo, and an editor's rust-analyzer, are told
// where each is by `[patch.crates-io]`, in the app's `.cargo/config.toml`,
// between rust-js's lines, which `rust-js build` and the editor's check
// write each time they run Cargo (ADR 0412).
//
// And rust-js's own wasm-bindgen, js-sys and web-sys, which an app's
// dependencies use in place of crates.io's, each a folder of `compat/`
// beside this file, the one catalog: named where Cargo's graph has one of
// them, and only then, as Cargo warns of a patch it doesn't use and writes
// it in the app's `Cargo.lock` (`settleCompat`, ADR 1112).
import { existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const BEGIN = "# rust-js: begin";
const END = "# rust-js: end";
const COMPAT = "# rust-js's own, which the app's dependencies use in place of crates.io's:";
const compatDir = fileURLToPath(new URL("./compat/", import.meta.url));

/**
 * rust-js's own crates an app's dependencies use in place of crates.io's,
 * by name: each its folder of `compat/`, its version its Cargo.toml's
 * (ADR 1112).
 * @returns {Map<string, { dir: string, version: string }>}
 */
export function compatCrates() {
  const crates = new Map();
  for (const name of readdirSync(compatDir).sort()) {
    const manifest = join(compatDir, name, "Cargo.toml");
    if (!existsSync(manifest)) continue;
    const version = /^version\s*=\s*"([^"]*)"/m.exec(readFileSync(manifest, "utf8"))?.[1];
    if (!version) throw new Error(`${manifest} names no version`);
    crates.set(name, { dir: join(compatDir, name), version });
  }
  return crates;
}

/**
 * Write the patch of the app at `app`, and return the crates it names, each
 * with where it is, from the app, and its package: its name, which JS
 * imports it by, and its directory, its real path (ADR 0414). Throws,
 * writing nothing, if a crate is installed twice, which would be two of each
 * of its types, a package's crate is another version than it is, or a
 * package is installed under another name than its own. With `compat`, it
 * names rust-js's own crates, or not; without, as it did (ADR 1112).
 * @param {string} app
 * @param {boolean} [compat]
 * @returns {{ crate: string, path: string, name: string, dir: string }[]}
 */
export function writePatch(app, compat) {
  const root = realpathSync(app);
  const crates = existsSync(join(root, "package.json")) ? installedCrates(root) : new Map();
  const config = join(root, ".cargo", "config.toml");
  const before = existsSync(config) ? readFileSync(config, "utf8") : "";
  // rust-js's own crates, as the patch had them, until Cargo's graph says
  // otherwise (`settleCompat`).
  const withCompat = compat ?? before.includes(COMPAT);
  const lines = [...crates].sort(([x], [y]) => x.localeCompare(y)).map(([crate, { dir }]) => `${crate} = { path = ${JSON.stringify(shown(root, dir))} }`);
  if (withCompat) {
    lines.push(COMPAT, ...[...compatCrates()].map(([crate, { dir }]) => `${crate} = { path = ${JSON.stringify(shown(root, dir))} }`));
  }
  const rest = withoutBlock(before);
  if (/^\s*\[\s*patch\s*\.\s*("crates-io"|crates-io)\s*\]/m.test(rest)) {
    throw new Error(`.cargo/config.toml has a [patch.crates-io] of its own, and TOML has one of a table, which rust-js writes: move its entries to Cargo.toml's [patch.crates-io], which Cargo takes with it`);
  }
  if (lines.length === 0) {
    if (rest.trim()) writeFileSync(config, rest);
    else rmSync(config, { force: true });
    return [];
  }
  const block = [
    BEGIN,
    "# Where Cargo finds each crate an npm package has: written by rust-js from",
    "# what's installed, as each build runs. Edits here are written over.",
    "[patch.crates-io]",
    ...lines,
    END,
    "",
  ].join("\n");
  mkdirSync(dirname(config), { recursive: true });
  writeFileSync(config, rest.trim() ? `${rest.trimEnd()}\n\n${block}` : block);
  return [...crates].map(([crate, { dir, name }]) => ({ crate, path: shown(root, dir), name, dir }));
}

/**
 * The patch of the app whose Cargo.toml is in `from` or a directory above
 * it, made to name rust-js's own crates where Cargo's graph has one,
 * `needed`, and not where it has none: whether it changed, so the graph is
 * read again. Of no patch yet, the workspace's, `root`'s (ADR 1112).
 * @param {string} from
 * @param {string} root
 * @param {boolean} needed
 * @returns {boolean}
 */
export function settleCompat(from, root, needed) {
  let dir = realpathSync(from);
  for (;;) {
    const config = join(dir, ".cargo", "config.toml");
    if (existsSync(config) && readFileSync(config, "utf8").includes(BEGIN)) break;
    if (dirname(dir) === dir) {
      dir = realpathSync(root);
      break;
    }
    dir = dirname(dir);
  }
  const config = join(dir, ".cargo", "config.toml");
  const text = existsSync(config) ? readFileSync(config, "utf8") : "";
  if (text.includes(COMPAT) === needed) return false;
  writePatch(dir, needed);
  return true;
}

/** `text` without rust-js's block, and the blank line rust-js put before it. */
function withoutBlock(text) {
  const start = text.indexOf(BEGIN);
  if (start < 0) return text;
  const end = text.indexOf(END, start);
  const after = end < 0 ? text.length : text.indexOf("\n", end) < 0 ? text.length : text.indexOf("\n", end) + 1;
  return text.slice(0, start).replace(/\n\n$/, "\n") + text.slice(after);
}

/**
 * Each crate the app's packages have, by its name, and its package's name
 * and directory: one, or an error naming each place and who asked for it.
 * @param {string} root the app's directory, its real path
 * @returns {Map<string, { name: string, dir: string }>}
 */
function installedCrates(root) {
  /** @type {Map<string, Map<string, { version: string, askers: Set<string> }>>} */
  const found = new Map();
  const seen = new Set();
  /** @type {{ from: string, name: string, asker: string }[]} */
  const queue = names(readJson(join(root, "package.json"))).map(name => ({ from: root, name, asker: "the app" }));
  while (queue.length) {
    const { from, name, asker } = /** @type {{ from: string, name: string, asker: string }} */ (queue.shift());
    const dir = lookup(from, name);
    if (!dir) continue;
    const pkg = readJson(join(dir, "package.json"));
    const crate = pkg["rust-js"]?.crate;
    if (typeof crate !== "string") continue;
    // JS imports it by its name, which is where Node finds it only if it's
    // installed under it, not an alias's, `"x": "npm:@acme/greet"`.
    if (pkg.name !== name) {
      throw new Error(`${pkg.name} is installed as ${name}, for ${asker}, and JS imports its crate's JS by its own name: install it as ${pkg.name}`);
    }
    const at = found.get(crate) ?? new Map();
    found.set(crate, at);
    const place = at.get(dir) ?? { version: pkg.version, askers: new Set() };
    place.askers.add(asker);
    at.set(dir, place);
    if (seen.has(dir)) continue;
    seen.add(dir);
    checkCrate(dir, pkg, crate);
    for (const next of names(pkg, true)) queue.push({ from: dir, name: next, asker: pkg.name });
  }
  const crates = new Map();
  for (const [crate, at] of found) {
    const places = [...at].map(([dir, { version, askers }]) => `${version} at ${shown(root, dir)}, for ${[...askers].join(", ")}`);
    if (at.size > 1) {
      throw new Error(`${crate} is installed twice, which would be two of each of its types: ${places.join("; ")}. Install one version, which each package asks for`);
    }
    const dir = [...at.keys()][0];
    crates.set(crate, { name: readJson(join(dir, "package.json")).name, dir });
  }
  return crates;
}

/** The packages `pkg` depends on: its own, and, for a package, its peers. */
function names(pkg, peers = false) {
  const fields = ["dependencies", "devDependencies", "optionalDependencies", ...(peers ? ["peerDependencies"] : [])];
  return [...new Set(fields.flatMap(field => Object.keys(pkg[field] ?? {})))];
}

/** Where Node finds package `name` from `from`, as its real path. */
function lookup(from, name) {
  for (let dir = from; ; dir = dirname(dir)) {
    if (basename(dir) !== "node_modules") {
      const candidate = join(dir, "node_modules", name, "package.json");
      if (existsSync(candidate)) return realpathSync(dirname(candidate));
    }
    if (dirname(dir) === dir) return undefined;
  }
}

/** A package's crate is the one it names, at the version it is. */
function checkCrate(dir, pkg, crate) {
  const manifest = join(dir, "Cargo.toml");
  if (!existsSync(manifest)) throw new Error(`${pkg.name} names a crate, ${crate}, and has no Cargo.toml`);
  const table = /^\[package\][^\S\n]*\n([\s\S]*?)(?=^\[|(?![\s\S]))/m.exec(readFileSync(manifest, "utf8"))?.[1] ?? "";
  const field = key => new RegExp(`^${key}\\s*=\\s*"([^"]*)"`, "m").exec(table)?.[1];
  if (field("name") !== crate) throw new Error(`${pkg.name} names its crate ${crate}, and its Cargo.toml ${field("name")}`);
  if (field("version") !== pkg.version) {
    throw new Error(`${pkg.name} is ${pkg.version}, and its crate, ${crate}, ${field("version")}: they're to be one version`);
  }
}

/** `dir`, as the app's config names it: from the app, with `/`. */
function shown(root, dir) {
  return relative(root, dir).split(sep).join("/");
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}
