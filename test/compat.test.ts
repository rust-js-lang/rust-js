// rust-js's own wasm-bindgen, js-sys and web-sys (ADR 1112): an app's
// dependencies use them in place of crates.io's, which rust-js can't
// compile, by the patch `rust-js build` writes, which names them only while
// Cargo's graph has one and they serve each crate that uses one. An app's
// own crate that depends on one is refused, the patch there or not: an app
// binds JS by the js and webapi crates. A git repository stands in for
// crates.io, as in registry-crates.test.ts.
// Its graphs and locks here; the surfaces of the crates it serves, each a
// crate's own use of them, in compat-surfaces.test.ts (#2465).
import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { runSync } from "./child";
import { compatRefused } from "../tooling/cargo.js";
import { compatCrates, writePatch } from "../tooling/patch.js";
import { buildCompiler, fixture, root } from "./support";
import { answer, app, build, config, git, gitCrate, usesWasmBindgen } from "./compat";

beforeAll(buildCompiler, 600_000);

test("a dependency's wasm-bindgen is rust-js's own, named while the graph has one, and an app's own refused", () => {
  const dir = fixture("compat-patch");
  const crate = gitCrate(dir, usesWasmBindgen, answer);
  const at = app(dir, crate);
  // As an earlier build left it, which read crates.io's in the graph.
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  // Those the graph has, and what they use, and no other: web-sys isn't.
  const named = (name: string) => `${name} = { path = ${JSON.stringify(relative(realpathSync(at), realpathSync(compatCrates().get(name)!.dir)))} }`;
  for (const name of ["js-sys", "wasm-bindgen", "wasm-bindgen-macro"]) expect(config(at)).toContain(named(name));
  // The js crate they map onto, the checkout's here, an app's npm package's
  // where it has one; not webapi, which only web-sys uses.
  expect(config(at)).toContain(`rust-js-builtins = { path = ${JSON.stringify(relative(realpathSync(at), join(root, "builtins")))} }`);
  expect(config(at)).not.toContain("web-sys");
  expect(config(at)).not.toContain("rust-js-webapi");
  // Each of rust-js's, of no registry.
  const lock = readFileSync(join(at, "Cargo.lock"), "utf8");
  for (const name of ["js-sys", "wasm-bindgen", "wasm-bindgen-macro"]) expect(lock).toMatch(new RegExp(`name = "${name}"\\nversion = "[^"]*"\\n(?!source)`));
  expect(readFileSync(join(at, "src", "lib.js"), "utf8")).toContain("answer");
  // An app's own crate that depends on one: refused, the patch there.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace("\n\n[workspace]", '\nwasm-bindgen = "0.2"\n\n[workspace]'));
  const own = build(at);
  expect(own.code).not.toBe(0);
  expect(own.stderr).toContain("rust-js: app depends on wasm-bindgen, which rust-js gives an app's dependencies alone, in place of crates.io's: an app's own Rust binds JS with the js and webapi crates");
  // A crate as Cargo packaged it, `Cargo.toml.orig` beside its manifest, is
  // a dependency's source wherever it is, here a path's, vendored, as the
  // Crater's copy of a crate is: given rust-js's own.
  const gitLine = `uuid = { git = ${JSON.stringify(`file://${crate}`)} }`;
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('wasm-bindgen = "0.2"\n', "").replace(gitLine, 'uuid = { path = "vendored" }'));
  mkdirSync(join(at, "vendored", "src"), { recursive: true });
  const vendoredManifest = `[package]\nname = "uuid"\nversion = "1.18.1"\nedition = "2024"\n\n[dependencies]\n${usesWasmBindgen}`;
  writeFileSync(join(at, "vendored", "Cargo.toml"), vendoredManifest);
  writeFileSync(join(at, "vendored", "src", "lib.rs"), answer);
  const unpackaged = build(at);
  expect(unpackaged.code).not.toBe(0);
  expect(unpackaged.stderr).toContain("rust-js: uuid depends on");
  writeFileSync(join(at, "vendored", "Cargo.toml.orig"), vendoredManifest);
  const vendored = build(at);
  expect(vendored.stderr).toBe("");
  expect(vendored.code).toBe(0);
  // None in the graph: no patch of them, nor a word of it in the lockfile.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('uuid = { path = "vendored" }', gitLine));
  gitCrate(dir, "", "pub fn answer() -> u32 {\n    42\n}\n");
  expect(runSync(["cargo", "update", "-q", "--manifest-path", join(at, "Cargo.toml")], at, 120_000).code).toBe(0);
  const without = build(at);
  expect(without.stderr).toBe("");
  expect(without.code).toBe(0);
  expect(existsSync(join(at, ".cargo", "config.toml"))).toBe(false);
  expect(readFileSync(join(at, "Cargo.lock"), "utf8")).not.toContain("wasm-bindgen");
}, 600_000);

// A lock of crates.io's of another release that the graph takes anew, put
// back under a patch an earlier build wrote, as the Crater puts a crate's
// own packed lock back for each of its builds: itertools' has criterion's
// wasm-bindgen 0.2.92, which getrandom 0.1 takes once the Crater turns its
// `wasm-bindgen` feature on, and Cargo takes the locked release, not the
// patch's. Moved onto rust-js's, though the patch didn't change (#1420).
test("a lock of crates.io's wasm-bindgen put back under a patch already written moves onto rust-js's", () => {
  const dir = fixture("compat-relocked");
  const optional = '[dependencies.wasm-bindgen]\nversion = "0.2.92"\noptional = true\n\n[features]\njs = ["dep:wasm-bindgen"]\n';
  const crate = gitCrate(dir, "", "#[cfg(feature = \"js\")]\nuse wasm_bindgen as _;\n\npub fn answer() -> u32 {\n    42\n}\n");
  writeFileSync(join(crate, "Cargo.toml"), readFileSync(join(crate, "Cargo.toml"), "utf8").replace("[dependencies]\n", optional));
  expect(git(crate, "commit", "-q", "-am", "js").code).toBe(0);
  const at = app(dir, crate);
  rmSync(join(at, ".cargo"), { recursive: true, force: true });
  const manifest = readFileSync(join(at, "Cargo.toml"), "utf8");
  // Its lock as the crate packed it: wasm-bindgen 0.2.92 of another
  // dependency's, which the app's own stands in for, and getrandom's
  // feature off.
  writeFileSync(join(at, "Cargo.toml"), manifest.replace("\n\n[workspace]", '\nwasm-bindgen = "=0.2.92"\n\n[workspace]'));
  rmSync(join(at, "Cargo.lock"), { force: true });
  expect(runSync(["cargo", "generate-lockfile", "-q", "--manifest-path", join(at, "Cargo.toml")], at, 120_000).code).toBe(0);
  const lock = readFileSync(join(at, "Cargo.lock"), "utf8");
  expect(lock).toMatch(/name = "wasm-bindgen"\nversion = "0\.2\.92"\nsource = "registry\+/);
  // The feature on, as the Crater turns it on, and the patch as builds
  // leave it, settled: a build that finds it so changes nothing of it.
  writeFileSync(join(at, "Cargo.toml"), manifest.replace(/^uuid = \{ (.*) \}$/m, 'uuid = { $1, features = ["js"] }'));
  rmSync(join(at, "Cargo.lock"), { force: true });
  for (let i = 0; i < 2; i++) expect(build(at).code).toBe(0);
  const settled = config(at);
  expect(settled).toContain("wasm-bindgen = { path =");
  writeFileSync(join(at, "Cargo.lock"), lock);
  const again = build(at);
  expect(again.stderr).toBe("");
  expect(again.code).toBe(0);
  expect(config(at)).toBe(settled);
  // Each of rust-js's, and none of crates.io's left beside it.
  const relocked = readFileSync(join(at, "Cargo.lock"), "utf8");
  for (const name of ["wasm-bindgen", "wasm-bindgen-macro"]) {
    expect(relocked).toMatch(new RegExp(`name = "${name}"\\nversion = "[^"]*"\\n(?!source)`));
    expect(relocked).not.toMatch(new RegExp(`name = "${name}"\\nversion = "[^"]*"\\nsource`));
  }
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
  expect(() => compatRefused(packages, graph, compatCrates(), ["wasm-bindgen"])).toThrow(
    `rust-js: pins 1.0.0 asks for wasm-bindgen =0.2.100, and rust-js's own wasm-bindgen, which a dependency uses in place of crates.io's, is ${ours}, so Cargo takes crates.io's, which rust-js can't compile: use a release of pins that takes wasm-bindgen ${ours}`,
  );
  // rust-js's own, a path's, is what it gives.
  const patched = packages.map((pkg) => (pkg.id === "wb" ? { ...pkg, version: ours, source: null, metadata: { "rust-js": { bindings: true } } } : pkg));
  expect(() => compatRefused(patched, graph, compatCrates(), ["wasm-bindgen"])).not.toThrow();
  // Where the patch doesn't name rust-js's, crates.io's is what's meant.
  expect(() => compatRefused(packages, graph, compatCrates(), [])).not.toThrow();
});

// What rust-js's wasm-bindgen doesn't know, an option or a type, is a
// compile error naming it, never a guess.
test("an import rust-js's wasm-bindgen doesn't know is a compile error naming what", () => {
  const dir = fixture("compat-unknown");
  for (const [source, said] of [
    ["#[wasm_bindgen(constructor)]\n    fn go(x: u32);", "rust-js's wasm-bindgen doesn't take `constructor` of an imported function yet"],
    ["#[wasm_bindgen(method)]\n    fn go(x: u32);", "rust-js's wasm-bindgen: a method's receiver is a name of a `&T`"],
    ["fn go(x: String);", "rust-js's wasm-bindgen doesn't pass `String` to JS yet"],
    ["#[wasm_bindgen(js_name = Other)]\n    type Thing;", "rust-js's wasm-bindgen doesn't bind an imported JS type's options yet"],
  ]) {
    const crate = gitCrate(dir, 'wasm-bindgen = "0.2.98"\n', `use wasm_bindgen::prelude::wasm_bindgen;\n\n#[wasm_bindgen]\nextern "C" {\n    ${source}\n}\n\npub fn answer() -> u32 {\n    42\n}\n`);
    const at = app(dir, crate);
    writePatch(at, [...compatCrates().keys()]);
    expect(runSync(["cargo", "update", "-q", "--manifest-path", join(at, "Cargo.toml")], at, 120_000).code).toBe(0);
    const built = build(at);
    expect(built.code).not.toBe(0);
    expect(built.stderr).toContain(said);
  }
}, 600_000);

// A crate of the graph that uses wasm-bindgen's family and isn't one
// rust-js's crates serve, here a stand-in for instant, whose `Date::now()`
// they don't have yet, would fail to type-check against them: its graph
// keeps crates.io's, as before ADR 1112, the crate types alone, and so does
// a graph beside it with a crate they serve too (#1129).
test("a graph with a crate rust-js's wasm-bindgen doesn't serve builds as before, of crates.io's", () => {
  const dir = fixture("compat-uncovered");
  const served = gitCrate(dir, usesWasmBindgen, answer);
  const instant = gitCrate(dir, usesWasmBindgen, "use js_sys::Date;\n\npub fn now() -> f64 {\n    Date::now()\n}\n", "instant", "0.1.13");
  for (const crates of [[instant], [served, instant]]) {
    const at = app(dir, crates, "pub fn answer() -> u32 {\n    42\n}\n");
    rmSync(join(at, ".cargo"), { recursive: true, force: true });
    rmSync(join(at, "Cargo.lock"), { force: true });
    const built = build(at);
    expect(built.code).toBe(0);
    expect(existsSync(join(at, ".cargo", "config.toml"))).toBe(false);
    expect(readFileSync(join(at, "Cargo.lock"), "utf8")).toMatch(/name = "js-sys"\nversion = "[^"]*"\nsource = "registry\+/);
  }
}, 600_000);
