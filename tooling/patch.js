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

import { crates as shippedCrates } from "./crates.js";

const BEGIN = "# rust-js: begin";
const END = "# rust-js: end";
const COMPAT = "# rust-js's own, which the app's dependencies use in place of crates.io's:";
const compatDir = fileURLToPath(new URL("./compat/", import.meta.url));

/**
 * rust-js's own crates an app's dependencies use in place of crates.io's,
 * by name: each its folder of `compat/`, its version its Cargo.toml's, and
 * the others of them it depends on, by version, through the patch, as
 * crates.io's does, and the binding crates it maps onto, `rust-js-builtins`
 * (ADR 1112).
 * @returns {Map<string, { dir: string, version: string, uses: string[], bindings: string[] }>}
 */
export function compatCrates() {
  const names = readdirSync(compatDir).filter((name) => existsSync(join(compatDir, name, "Cargo.toml"))).sort();
  const crates = new Map();
  for (const name of names) {
    const manifest = readFileSync(join(compatDir, name, "Cargo.toml"), "utf8");
    const version = /^version\s*=\s*"([^"]*)"/m.exec(manifest)?.[1];
    if (!version) throw new Error(`compat/${name}/Cargo.toml names no version`);
    const uses = names.filter((other) => other !== name && new RegExp(`^${other}\\s*=`, "m").test(manifest));
    const bindings = [...manifest.matchAll(/package = "(rust-js-[\w-]+)"/g)].map((m) => m[1]);
    crates.set(name, { dir: join(compatDir, name), version, uses, bindings });
  }
  return crates;
}

/**
 * Whether rust-js's own crates serve `dependent`, a crate that uses one,
 * its name and version: `compat/covered.txt` names it, and its release
 * (#1129).
 * @param {{ name: string, version: string }} dependent
 */
export function covered({ name, version }) {
  const lines = readFileSync(join(compatDir, "covered.txt"), "utf8").split("\n");
  return lines.some((line) => {
    const [crate, releases] = line.trim().split(/\s+/);
    return !line.startsWith("#") && crate === name && (version === releases || version.startsWith(`${releases}.`));
  });
}

/** Of rust-js's own crates, those `present` names and each they use. */
function withUsed(present) {
  const crates = compatCrates();
  const all = new Set();
  const add = (name) => {
    if (all.has(name) || !crates.has(name)) return;
    all.add(name);
    for (const used of crates.get(name).uses) add(used);
  };
  for (const name of present) add(name);
  return [...all].sort();
}

/** The names of rust-js's own crates a patch's text names. */
function patched(text) {
  const at = text.indexOf(COMPAT);
  if (at < 0) return [];
  const end = text.indexOf(END, at);
  const catalog = compatCrates();
  return [...text.slice(at, end < 0 ? text.length : end).matchAll(/^([\w-]+) = \{ path = /gm)].map((m) => m[1]).filter((name) => catalog.has(name)).sort();
}

/**
 * Where a binding crate rust-js's own crates use is, of Cargo's name,
 * `rust-js-builtins`, where the app has no npm package of it: its package
 * as Node finds it from here, `@rust-js/build`'s, or the checkout's own.
 * @param {string} cargo
 * @returns {string}
 */
function shippedCrate(cargo) {
  const shipped = shippedCrates.find((c) => c.cargo === cargo);
  if (!shipped) throw new Error(`rust-js ships no crate ${cargo}`);
  const here = dirname(fileURLToPath(import.meta.url));
  const found = lookup(here, shipped.npm);
  if (found) return found;
  const checkout = join(here, "..", shipped.dir);
  if (existsSync(join(checkout, "Cargo.toml"))) return realpathSync(checkout);
  throw new Error(`rust-js's ${[...compatCrates().keys()].join(", ")} map onto ${shipped.npm}'s crate: install ${shipped.npm}`);
}

/**
 * Write the patch of the app at `app`, and return the crates it names, each
 * with where it is, from the app, and its package: its name, which JS
 * imports it by, and its directory, its real path (ADR 0414). Throws,
 * writing nothing, if a crate is installed twice, which would be two of each
 * of its types, a package's crate is another version than it is, or a
 * package is installed under another name than its own. With `compat`, it
 * names those of rust-js's own crates; without, those it did (ADR 1112).
 * @param {string} app
 * @param {string[]} [compat]
 * @returns {{ crate: string, path: string, name: string, dir: string }[]}
 */
export function writePatch(app, compat) {
  const root = realpathSync(app);
  const crates = existsSync(join(root, "package.json")) ? installedCrates(root) : new Map();
  const config = join(root, ".cargo", "config.toml");
  const before = existsSync(config) ? readFileSync(config, "utf8") : "";
  // rust-js's own crates, as the patch had them, until Cargo's graph says
  // otherwise (`settleCompat`).
  const own = compat ?? patched(before);
  const lines = [...crates].sort(([x], [y]) => x.localeCompare(y)).map(([crate, { dir }]) => `${crate} = { path = ${JSON.stringify(shown(root, dir))} }`);
  if (own.length > 0) {
    const catalog = compatCrates();
    // And the binding crates they map onto, where no npm package has them.
    const bindings = [...new Set(own.flatMap((crate) => catalog.get(crate).bindings))].filter((cargo) => !crates.has(cargo)).sort();
    lines.push(
      COMPAT,
      ...own.map((crate) => `${crate} = { path = ${JSON.stringify(shown(root, catalog.get(crate).dir))} }`),
      ...bindings.map((cargo) => `${cargo} = { path = ${JSON.stringify(shown(root, shippedCrate(cargo)))} }`),
    );
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
 * it, made to name those of rust-js's own crates Cargo's graph has, of
 * `present`, the names of its packages, crates.io's or rust-js's, and each
 * of rust-js's those use, and no other, as Cargo warns of a patch it doesn't
 * use; and none unless they serve each of `dependents`, the graph's crates
 * that use one (`covered`): a crate that uses what they don't have yet
 * would fail to type-check, where crates.io's has it (#1129). Whether it
 * changed, so the graph is read again, and those it names. Of no patch
 * yet, the workspace's, `root`'s (ADR 1112).
 * @param {string} from
 * @param {string} root
 * @param {string[]} present
 * @param {{ name: string, version: string }[]} dependents
 * @returns {{ changed: boolean, patched: string[] }}
 */
export function settleCompat(from, root, present, dependents) {
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
  const needed = dependents.every(covered) ? withUsed(present) : [];
  if (patched(text).join() === needed.join()) return { changed: false, patched: needed };
  writePatch(dir, needed);
  return { changed: true, patched: needed };
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
