// rust-js's own wasm-bindgen, js-sys and web-sys (ADR 1112): an app's
// dependencies use them in place of crates.io's, which rust-js can't
// compile, by the patch `rust-js build` writes, which names them only while
// Cargo's graph has one. An app's own crate that depends on one is refused,
// the patch there or not: an app binds JS by the js and webapi crates. A
// git repository stands in for crates.io, as in registry-crates.test.ts.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { runSync } from "./child";
import { compatRefused } from "../tooling/cargo.js";
import { compatCrates, writePatch } from "../tooling/patch.js";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

const cli = join(root, "tooling", "cli.js");
const build = (dir: string) => runSync([process.execPath, cli, "build"], dir, 300_000, { RUST_JS_COMPILER: compiler });
const git = (dir: string, ...args: string[]) => runSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], dir, 60_000);

/** A crate `dep` in a git repository of its own, at `dir/dep`, whose
 * dependencies are `deps` and root `source`, committed. */
function gitCrate(dir: string, deps: string, source: string) {
  const crate = join(dir, "dep");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), `[package]\nname = "dep"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${deps}`);
  writeFileSync(join(crate, "src", "lib.rs"), source);
  if (!existsSync(join(crate, ".git"))) expect(git(crate, "init", "-q").code).toBe(0);
  for (const args of [["add", "."], ["commit", "-q", "-m", "dep"]]) expect(git(crate, ...args).code).toBe(0);
  return crate;
}

/** An app at `dir/app` of no npm packages that depends on `dep`, and `own`. */
function app(dir: string, crate: string, own = "") {
  const at = join(dir, "app");
  mkdirSync(join(at, "src"), { recursive: true });
  writeFileSync(join(at, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\ndep = { git = ${JSON.stringify(`file://${crate}`)} }\n${own}\n[workspace]\n`);
  writeFileSync(join(at, "src", "lib.rs"), "pub fn answer() -> u32 {\n    dep::answer()\n}\n");
  return at;
}

const config = (at: string) => readFileSync(join(at, ".cargo", "config.toml"), "utf8");
const usesWasmBindgen = 'wasm-bindgen = "0.2.98"\njs-sys = "0.3"\n';
const answer = "use js_sys as _;\nuse wasm_bindgen as _;\n\npub fn answer() -> u32 {\n    42\n}\n";

test("a dependency's wasm-bindgen is rust-js's own, named while the graph has one, and an app's own refused", () => {
  const dir = fixture("compat-patch");
  const crate = gitCrate(dir, usesWasmBindgen, answer);
  const at = app(dir, crate);
  // As an earlier build left it, which read crates.io's in the graph.
  writePatch(at, true);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  for (const [name, { dir: compat }] of compatCrates()) expect(config(at)).toContain(`${name} = { path = ${JSON.stringify(relative(realpathSync(at), realpathSync(compat)))} }`);
  expect(readFileSync(join(at, "Cargo.lock"), "utf8")).not.toContain("registry+");
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toContain("answer");
  // An app's own crate that depends on one: refused, the patch there.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace("\n[workspace]", 'wasm-bindgen = "0.2"\n\n[workspace]'));
  const own = build(at);
  expect(own.code).not.toBe(0);
  expect(own.stderr).toContain("rust-js: app depends on wasm-bindgen, which rust-js gives an app's dependencies alone, in place of crates.io's: an app's own Rust binds JS with the js and webapi crates");
  // A crate as Cargo packaged it, `Cargo.toml.orig` beside its manifest, is
  // a dependency's source wherever it is, here a path's, vendored, as the
  // Crater's copy of a crate is: given rust-js's own.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('wasm-bindgen = "0.2"\n', 'vendored = { path = "vendored" }\n'));
  mkdirSync(join(at, "vendored", "src"), { recursive: true });
  const vendoredManifest = `[package]\nname = "vendored"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n${usesWasmBindgen}`;
  writeFileSync(join(at, "vendored", "Cargo.toml"), vendoredManifest);
  writeFileSync(join(at, "vendored", "src", "lib.rs"), answer);
  const unpackaged = build(at);
  expect(unpackaged.code).not.toBe(0);
  expect(unpackaged.stderr).toContain("rust-js: vendored depends on");
  writeFileSync(join(at, "vendored", "Cargo.toml.orig"), vendoredManifest);
  const vendored = build(at);
  expect(vendored.stderr).toBe("");
  expect(vendored.code).toBe(0);
  // None in the graph: no patch of them, nor a word of it in the lockfile.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('vendored = { path = "vendored" }\n', ""));
  gitCrate(dir, "", "pub fn answer() -> u32 {\n    42\n}\n");
  expect(runSync(["cargo", "update", "-q", "--manifest-path", join(at, "Cargo.toml")], at, 120_000).code).toBe(0);
  const without = build(at);
  expect(without.stderr).toBe("");
  expect(without.code).toBe(0);
  expect(existsSync(join(at, ".cargo", "config.toml"))).toBe(false);
  expect(readFileSync(join(at, "Cargo.lock"), "utf8")).not.toContain("wasm-bindgen");
}, 600_000);

// A dependency that asks for crates.io's at a version rust-js's isn't, an
// exact `=`, gets crates.io's, which rust-js can't compile: refused, naming
// the dependency, what it asks and rust-js's version.
test("a dependency pinned to another wasm-bindgen than rust-js's is refused, naming the pin", () => {
  const ours = compatCrates().get("wasm-bindgen")!.version;
  const registry = "registry+https://github.com/rust-lang/crates.io-index";
  const packages = [
    { id: "app", name: "app", version: "0.0.0", source: null, dependencies: [], metadata: null },
    { id: "pins", name: "pins", version: "1.0.0", source: registry, dependencies: [{ name: "wasm-bindgen", req: "=0.2.100" }], metadata: null },
    { id: "wb", name: "wasm-bindgen", version: "0.2.100", source: registry, dependencies: [], metadata: null },
  ];
  const graph = { nodes: [{ id: "app", deps: [{ pkg: "pins" }] }, { id: "pins", deps: [{ pkg: "wb" }] }, { id: "wb", deps: [] }] };
  expect(() => compatRefused(packages, graph, compatCrates())).toThrow(
    `rust-js: pins 1.0.0 asks for wasm-bindgen =0.2.100, and rust-js's own wasm-bindgen, which a dependency uses in place of crates.io's, is ${ours}, so Cargo takes crates.io's, which rust-js can't compile: use a release of pins that takes wasm-bindgen ${ours}`,
  );
  // rust-js's own, a path's, is what it gives.
  const patched = packages.map((pkg) => (pkg.id === "wb" ? { ...pkg, version: ours, source: null, metadata: { "rust-js": { bindings: true } } } : pkg));
  expect(() => compatRefused(patched, graph, compatCrates())).not.toThrow();
});
