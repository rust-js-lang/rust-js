// rust-js's own wasm-bindgen, js-sys and web-sys (ADR 1112): an app's
// dependencies use them in place of crates.io's, which rust-js can't
// compile, by the patch `rust-js build` writes, which names them only while
// Cargo's graph has one and they serve each crate that uses one. An app's
// own crate that depends on one is refused, the patch there or not: an app
// binds JS by the js and webapi crates. A git repository stands in for
// crates.io, as in registry-crates.test.ts.

import { beforeAll, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { runSync } from "./child";
import { compatRefused } from "../tooling/cargo.js";
import { compatCrates, writePatch } from "../tooling/patch.js";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root } from "./support";

beforeAll(buildCompiler, 600_000);

const cli = join(root, "tooling", "cli.js");
const build = (dir: string) => runSync([process.execPath, cli, "build"], dir, 300_000, { RUST_JS_COMPILER: compiler });
const git = (dir: string, ...args: string[]) => runSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], dir, 60_000);

/** A crate `name` in a git repository of its own, at `dir/name`, whose
 * dependencies are `deps` and root `source`, committed: by default a
 * stand-in for getrandom 0.3, which rust-js's crates serve
 * (`tooling/compat/covered.txt`). */
function gitCrate(dir: string, deps: string, source: string, name = "getrandom", version = "0.3.4") {
  const crate = join(dir, name);
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "${version}"\nedition = "2024"\n\n[dependencies]\n${deps}`);
  writeFileSync(join(crate, "src", "lib.rs"), source);
  if (!existsSync(join(crate, ".git"))) expect(git(crate, "init", "-q").code).toBe(0);
  for (const args of [["add", "."], ["commit", "-q", "-m", name]]) expect(git(crate, ...args).code).toBe(0);
  return crate;
}

/** An app at `dir/app` of no npm packages that depends on each git crate
 * of `crates`, its root `source`: the binding crates rust-js's own use are
 * the checkout's. */
function app(dir: string, crates: string | string[], source = "pub fn answer() -> u32 {\n    getrandom::answer()\n}\n") {
  const at = join(dir, "app");
  mkdirSync(join(at, "src"), { recursive: true });
  const deps = [crates].flat().map((crate) => `${crate.split("/").at(-1)} = { git = ${JSON.stringify(`file://${crate}`)} }`).join("\n");
  writeFileSync(join(at, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\n${deps}\n\n[workspace]\n`);
  writeFileSync(join(at, "src", "lib.rs"), source);
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
  const gitLine = `getrandom = { git = ${JSON.stringify(`file://${crate}`)} }`;
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('wasm-bindgen = "0.2"\n', "").replace(gitLine, 'getrandom = { path = "vendored" }'));
  mkdirSync(join(at, "vendored", "src"), { recursive: true });
  const vendoredManifest = `[package]\nname = "getrandom"\nversion = "0.3.4"\nedition = "2024"\n\n[dependencies]\n${usesWasmBindgen}`;
  writeFileSync(join(at, "vendored", "Cargo.toml"), vendoredManifest);
  writeFileSync(join(at, "vendored", "src", "lib.rs"), answer);
  const unpackaged = build(at);
  expect(unpackaged.code).not.toBe(0);
  expect(unpackaged.stderr).toContain("rust-js: getrandom depends on");
  writeFileSync(join(at, "vendored", "Cargo.toml.orig"), vendoredManifest);
  const vendored = build(at);
  expect(vendored.stderr).toBe("");
  expect(vendored.code).toBe(0);
  // None in the graph: no patch of them, nor a word of it in the lockfile.
  writeFileSync(join(at, "Cargo.toml"), readFileSync(join(at, "Cargo.toml"), "utf8").replace('getrandom = { path = "vendored" }', gitLine));
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
  writeFileSync(join(at, "Cargo.toml"), manifest.replace(/^getrandom = \{ (.*) \}$/m, 'getrandom = { $1, features = ["js"] }'));
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

// getrandom 0.3's `wasm_js` backend, as it writes it: an inline `extern`
// block of a function of `globalThis.crypto`, which fills a slice of bytes
// not yet written. Under wasm-bindgen JS is given a view of wasm's memory;
// here a `Uint8Array` made once, what JS wrote copied back once (ADR 1112).
const imports = `use core::mem::MaybeUninit;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["globalThis", "crypto"], js_name = getRandomValues, catch)]
    fn get_random_values(buf: &mut [MaybeUninit<u8>]) -> Result<(), JsValue>;
    // As getrandom's own: a \`cfg\` leaves it out, its type unknown here.
    #[cfg(target_feature = "atomics")]
    #[wasm_bindgen(js_namespace = ["globalThis", "crypto"], js_name = getRandomValues, catch)]
    fn get_random_values(buf: &js_sys::Uint8Array) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = globalThis, js_name = flipBytes)]
    fn flip_bytes(buf: &mut [u8]);
    #[wasm_bindgen(js_namespace = globalThis, js_name = sumBytes)]
    fn sum_bytes(buf: &[u8]) -> u32;
    #[wasm_bindgen(js_namespace = globalThis, js_name = refuse, catch)]
    fn refuse(n: u32) -> Result<u32, JsValue>;
}

pub fn random(n: usize) -> Option<Vec<u8>> {
    let mut bytes = vec![MaybeUninit::<u8>::uninit(); n];
    get_random_values(&mut bytes).ok()?;
    // SAFETY: \`getRandomValues\` wrote each.
    Some(bytes.into_iter().map(|b| unsafe { b.assume_init() }).collect())
}

pub fn flipped(mut bytes: Vec<u8>) -> Vec<u8> {
    flip_bytes(&mut bytes);
    bytes
}

pub fn summed(bytes: &[u8]) -> u32 {
    sum_bytes(bytes)
}

pub fn refused(n: u32) -> Option<u32> {
    refuse(n).ok()
}
`;

test("a dependency's inline JS imports are rust-js bindings: a slice JS fills, one it reads, one it threw at", () => {
  const dir = fixture("compat-imports");
  const crate = gitCrate(dir, usesWasmBindgen, imports);
  const at = app(dir, crate, "pub use getrandom::{flipped, random, refused, summed};\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = readFileSync(join(at, "rust-js", "crates", "getrandom-0.3.4", "src", "lib.js"), "utf8");
  // One copy each way: a `Uint8Array` made, given, and read back once.
  expect(js).toContain("function getRandomValues(buf) {\n  const uint8Array = new Uint8Array(buf.length);\n  const value$1 = $try(() => globalThis.crypto.getRandomValues(uint8Array));");
  expect(js).toContain("function sumBytes(buf) {\n  return globalThis.sumBytes(Uint8Array.from(buf));\n}");
  // What JS threw, the value itself, not a box of it.
  expect(js).toContain("function refuse(n) {\n  return $try(() => globalThis.refuse(n));\n}");
  expect(js).not.toContain("atomics");
  const program = `const app = await import(${JSON.stringify(join(at, "src", "lib.js"))});
let seen;
globalThis.flipBytes = (view) => { seen = Array.from(view); view.forEach((b, i) => { view[i] = 255 - b; }); };
globalThis.sumBytes = (view) => view.reduce((a, b) => a + b, 0) + (view instanceof Uint8Array ? 0 : 1000);
globalThis.refuse = (n) => { if (n > 1) throw new RangeError("too many"); return n; };
const bytes = app.random(64);
console.log(JSON.stringify([bytes.length, bytes.some((b) => b !== 0), bytes.every((b) => Number.isInteger(b) && b >= 0 && b < 256), app.flipped([0, 1, 254]), seen, app.summed([1, 2, 3]), app.refused(1), app.refused(2)]));`;
  writeFileSync(join(at, "check.mjs"), program);
  const ran = runSync([process.execPath, join(at, "check.mjs")], at, 60_000);
  expect(ran.stderr).toBe("");
  expect(JSON.parse(ran.stdout)).toEqual([64, true, true, [255, 254, 1], [0, 1, 254], 6, 1, null]);
}, 600_000);

// getrandom 0.1's wasm-bindgen backend, as it writes it, wasm-bindgen named
// `bindgen` (#1404): imported types, a static getter that throws where
// there's no \`self\`, methods and getters, CommonJS's \`module\`, served its
// \`require\` (the user's decision on #1400), and js-sys's \`Uint8Array\`.
const getrandom01 = `extern crate bindgen as wasm_bindgen;

use std::cell::RefCell;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

const CHUNK_SIZE: usize = 256;

#[derive(Clone, Debug)]
enum RngSource {
    Node(NodeCrypto),
    Browser(BrowserCrypto, Uint8Array),
}

thread_local!(
    static RNG_SOURCE: RefCell<Option<RngSource>> = RefCell::new(None);
);

/// Which source filled \`dest\`, \`"node"\` or \`"browser"\`, or why none.
pub fn fill(dest: &mut [u8]) -> Result<String, String> {
    RNG_SOURCE.with(|f| {
        let mut source = f.borrow_mut();
        if source.is_none() {
            *source = Some(init()?);
        }
        match source.as_ref().unwrap() {
            RngSource::Node(n) => {
                n.random_fill_sync(dest);
                Ok("node".to_string())
            }
            RngSource::Browser(crypto, buf) => {
                for chunk in dest.chunks_mut(CHUNK_SIZE) {
                    let sub_buf = buf.subarray(0, chunk.len() as u32);
                    crypto.get_random_values(&sub_buf);
                    sub_buf.copy_to(chunk);
                }
                Ok("browser".to_string())
            }
        }
    })
}

fn init() -> Result<RngSource, String> {
    if let Ok(self_) = Global::get_self() {
        let crypto: BrowserCrypto = match (self_.crypto(), self_.ms_crypto()) {
            (crypto, _) if !crypto.is_undefined() => crypto.into(),
            (_, crypto) if !crypto.is_undefined() => crypto.into(),
            _ => return Err("no crypto".to_string()),
        };
        if crypto.get_random_values_fn().is_undefined() {
            return Err("no getRandomValues".to_string());
        }
        let buf = Uint8Array::new_with_length(CHUNK_SIZE as u32);
        return Ok(RngSource::Browser(crypto, buf));
    }
    Ok(RngSource::Node(MODULE.require("crypto")))
}

pub fn random(n: usize) -> (String, Vec<u8>) {
    let mut bytes = vec![0; n];
    let source = fill(&mut bytes).unwrap_or_else(|why| why);
    (source, bytes)
}

#[wasm_bindgen]
extern "C" {
    type Global;
    #[wasm_bindgen(getter, catch, static_method_of = Global, js_class = self, js_name = self)]
    fn get_self() -> Result<Self_, JsValue>;

    type Self_;
    #[wasm_bindgen(method, getter, js_name = "msCrypto", structural)]
    fn ms_crypto(me: &Self_) -> JsValue;
    #[wasm_bindgen(method, getter, structural)]
    fn crypto(me: &Self_) -> JsValue;

    #[derive(Clone, Debug)]
    type BrowserCrypto;

    #[wasm_bindgen(method, js_name = getRandomValues, structural, getter)]
    fn get_random_values_fn(me: &BrowserCrypto) -> JsValue;
    #[wasm_bindgen(method, js_name = getRandomValues, structural)]
    fn get_random_values(me: &BrowserCrypto, buf: &Uint8Array);

    #[derive(Clone, Debug)]
    type NodeCrypto;

    #[wasm_bindgen(method, js_name = randomFillSync, structural)]
    fn random_fill_sync(me: &NodeCrypto, buf: &mut [u8]);

    type NodeModule;

    #[wasm_bindgen(js_name = module)]
    static MODULE: NodeModule;

    #[wasm_bindgen(method)]
    fn require(this: &NodeModule, s: &str) -> NodeCrypto;
}
`;

test("getrandom 0.1's backend: imported types, methods, getters, its static `module`'s require, a Uint8Array", () => {
  const dir = fixture("compat-getrandom-01");
  const crate = gitCrate(dir, 'bindgen = { package = "wasm-bindgen", version = "0.2.29" }\njs-sys = "0.3"\n', getrandom01, "getrandom", "0.1.16");
  const at = app(dir, crate, "pub use getrandom::random;\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = readFileSync(join(at, "rust-js", "crates", "getrandom-0.1.16", "src", "lib.js"), "utf8");
  // The browser's \`self\` read where it's asked, as upstream's \`catch\`.
  expect(js).toContain("return $try(() => self.self);");
  const run = (prelude: string) => {
    const program = `${prelude}
const app = await import(${JSON.stringify(join(at, "src", "lib.js"))});
const [source, bytes] = app.random(300);
console.log(JSON.stringify([source, bytes.length, bytes.some((b) => b !== 0), bytes.every((b) => Number.isInteger(b) && b >= 0 && b < 256), bytes[0], bytes[299]]));`;
    writeFileSync(join(at, "check.mjs"), program);
    // Node's, whose global has no \`self\`, as Bun's has.
    const ran = runSync([node ?? "node", join(at, "check.mjs")], at, 60_000);
    expect(ran.stderr).toBe("");
    return JSON.parse(ran.stdout);
  };
  // In Node, no \`self\`: Node's crypto, by \`module.require\`.
  const [source, length, some, bytes] = run("");
  expect([source, length, some, bytes]).toEqual(["node", 300, true, true]);
  // In a browser's, \`self.crypto\`, each chunk of 256 through a view.
  expect(run("let n = 0;\nglobalThis.self = { crypto: { getRandomValues(view) { view.forEach((_, i) => { view[i] = (n++ % 250) + 1; }); } } };\nself.self = self;"))
    .toEqual(["browser", 300, true, true, 1, (299 % 250) + 1]);
}, 600_000);

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
// a graph beside it with getrandom too (#1129).
test("a graph with a crate rust-js's wasm-bindgen doesn't serve builds as before, of crates.io's", () => {
  const dir = fixture("compat-uncovered");
  const getrandom = gitCrate(dir, usesWasmBindgen, answer);
  const instant = gitCrate(dir, usesWasmBindgen, "use js_sys::Date;\n\npub fn now() -> f64 {\n    Date::now()\n}\n", "instant", "0.1.13");
  for (const crates of [[instant], [getrandom, instant]]) {
    const at = app(dir, crates, "pub fn answer() -> u32 {\n    42\n}\n");
    rmSync(join(at, ".cargo"), { recursive: true, force: true });
    rmSync(join(at, "Cargo.lock"), { force: true });
    const built = build(at);
    expect(built.code).toBe(0);
    expect(existsSync(join(at, ".cargo", "config.toml"))).toBe(false);
    expect(readFileSync(join(at, "Cargo.lock"), "utf8")).toMatch(/name = "js-sys"\nversion = "[^"]*"\nsource = "registry\+/);
  }
}, 600_000);

// chrono's `wasmbind`, as it writes it: `js_sys::Date`, made now, of
// milliseconds and of local parts, and its `getTime` and
// `getTimezoneOffset`, `js::Date`'s, which it derefs to. Its offsets in New
// York's zone, of a winter's and a summer's date, are native Rust's, `TZ`
// set. A crate of another platform's that uses js-sys, chrono's `cfg(unix)`
// iana-time-zone, isn't of rust-js's target's graph, nor keeps the patch
// off it (ADR 1112).
const dates = `pub struct Utc(pub f64);

impl From<Utc> for js_sys::Date {
    fn from(utc: Utc) -> js_sys::Date {
        let js_millis = wasm_bindgen::JsValue::from_f64(utc.0);
        js_sys::Date::new(&js_millis)
    }
}

pub fn now() -> f64 {
    js_sys::Date::new_0().get_time()
}

pub fn offset_of_utc(ms: f64) -> f64 {
    js_sys::Date::from(Utc(ms)).get_timezone_offset()
}

pub fn offset_of_local(year: i32, month0: i32) -> f64 {
    let js_date = js_sys::Date::new_with_year_month_day_hr_min_sec(year as u32, month0, 15, 12, 0, 0);
    js_date.get_timezone_offset()
}
`;

test("chrono's wasmbind surface is js::Date's, its offsets native Rust's in a zone with daylight saving time", () => {
  const dir = fixture("compat-dates");
  const unix = gitCrate(dir, 'js-sys = "0.3"\n', "use js_sys as _;\n", "iana-time-zone", "0.1.65");
  const chrono = gitCrate(dir, `wasm-bindgen = "0.2"\njs-sys = "0.3"\n\n[target.'cfg(unix)'.dependencies]\niana-time-zone = { git = ${JSON.stringify(`file://${unix}`)} }\n`, dates, "chrono", "0.4.45");
  const at = app(dir, chrono, "pub use chrono::{now, offset_of_local, offset_of_utc};\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(config(at)).toContain("js-sys = { path");
  const js = readFileSync(join(at, "rust-js", "crates", "chrono-0.4.45", "src", "lib.js"), "utf8");
  expect(js).toContain("export function offsetOfLocal(year, month0) {\n  const jsDate = new Date(year >>> 0, month0, 15, 12, 0, 0);\n  return jsDate.getTimezoneOffset();\n}");
  const program = `const app = await import(${JSON.stringify(join(at, "src", "lib.js"))});
console.log(JSON.stringify([Math.abs(app.now() - Date.now()) < 1000, app.offsetOfUtc(Date.UTC(2024, 0, 15, 12)), app.offsetOfUtc(Date.UTC(2024, 6, 15, 12)), app.offsetOfLocal(2024, 0), app.offsetOfLocal(2024, 6)]));`;
  writeFileSync(join(at, "check.mjs"), program);
  const ran = runSync([process.execPath, join(at, "check.mjs")], at, 60_000, { TZ: "America/New_York" });
  expect(ran.stderr).toBe("");
  // chrono's own, natively in New York: 300 minutes west in winter, 240 in summer.
  expect(JSON.parse(ran.stdout)).toEqual([true, 300, 240, 300, 240]);
}, 600_000);

// A crate's wasm tests, `#[wasm_bindgen_test]`, are `#[test]`s, which
// `rust-js test` runs as JS, of the features `--features` names, Cargo's
// own flag; an `async` one, which it runs none of yet, a compile error
// saying so, never a test left out. Upstream's own wasm-bindgen-test's
// dependencies, wasm-bindgen-futures, don't keep the patch off: the patch
// replaces them (ADR 1112).
test("a crate's #[wasm_bindgen_test]s are #[test]s rust-js test runs, of the features it's given", () => {
  const dir = fixture("compat-wasm-tests");
  const tests = (body: string) => `pub fn answer() -> u32 {\n    42\n}\n\n#[cfg(test)]\nmod tests {\n    use wasm_bindgen_test::*;\n\n    wasm_bindgen_test_configure!(run_in_browser);\n\n${body}}\n`;
  const sync = tests("    #[wasm_bindgen_test]\n    fn answers() {\n        assert_eq!(super::answer(), 42);\n    }\n\n    #[cfg(feature = \"extra\")]\n    #[wasm_bindgen_test]\n    fn extra() {\n        assert_eq!(super::answer() + 1, 43);\n    }\n");
  const crate = join(dir, "getrandom");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "getrandom"\nversion = "0.3.4"\nedition = "2024"\n\n[features]\nextra = []\n\n[dev-dependencies]\nwasm-bindgen-test = "0.3"\n');
  writeFileSync(join(crate, "Cargo.toml.orig"), readFileSync(join(crate, "Cargo.toml"), "utf8"));
  writeFileSync(join(crate, "src", "lib.rs"), sync);
  writePatch(crate, [...compatCrates().keys()]);
  const test = (...args: string[]) => runSync([process.execPath, cli, "test", ...args], crate, 300_000, { RUST_JS_COMPILER: compiler });
  const ran = test("--features", "extra");
  expect(ran.code).toBe(0);
  expect(ran.stdout + ran.stderr).toContain("answers");
  expect(ran.stdout + ran.stderr).toContain("extra");
  expect(config(crate)).toContain("wasm-bindgen-test = { path");
  writeFileSync(join(crate, "src", "lib.rs"), tests("    #[wasm_bindgen_test]\n    async fn later() {}\n"));
  const refused = test();
  expect(refused.code).not.toBe(0);
  expect(refused.stdout + refused.stderr).toContain("rust-js's wasm-bindgen-test doesn't run an `async` test yet");
  // Imported as `test`, getrandom's `use wasm_bindgen_test::wasm_bindgen_test
  // as test;`: the built-in `#[test]` it makes isn't this attribute again.
  writeFileSync(join(crate, "src", "lib.rs"), "#[cfg(test)]\nmod tests {\n    use wasm_bindgen_test::wasm_bindgen_test as test;\n\n    #[test]\n    fn imported() {\n        assert_eq!(1 + 1, 2);\n    }\n}\n");
  const imported = test();
  expect(imported.code).toBe(0);
  expect(imported.stdout + imported.stderr).toContain("imported");
}, 600_000);

// The patch is of the graph the build's features make: getrandom's
// `wasm_js` feature brings wasm-bindgen in, which the patch names then,
// so its tests use rust-js's own, not crates.io's (ADR 1112).
test("the patch is of the graph of the features rust-js test is given", () => {
  const dir = fixture("compat-feature-graph");
  const crate = join(dir, "getrandom");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "getrandom"\nversion = "0.3.4"\nedition = "2024"\n\n[features]\nwasm_js = ["dep:wasm-bindgen"]\n\n[dependencies]\nwasm-bindgen = { version = "0.2", optional = true }\n');
  writeFileSync(join(crate, "Cargo.toml.orig"), readFileSync(join(crate, "Cargo.toml"), "utf8"));
  writeFileSync(join(crate, "src", "lib.rs"), "#[cfg(feature = \"wasm_js\")]\npub fn answer() -> u32 {\n    format!(\"{:?}\", wasm_bindgen::JsValue::from_f64(42.0)).len() as u32\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    #[cfg(feature = \"wasm_js\")]\n    fn answers() {\n        assert_eq!(super::answer(), 11);\n    }\n}\n");
  const ran = runSync([process.execPath, cli, "test", "--features", "wasm_js"], crate, 300_000, { RUST_JS_COMPILER: compiler });
  expect(ran.stdout + ran.stderr).not.toContain("error");
  expect(config(crate)).toContain("wasm-bindgen = { path");
  expect(ran.code).toBe(0);
}, 600_000);

// wasm-bindgen's `unwrap_throw` and `expect_throw` are upstream's native
// branch, `expect`, a Rust panic of native Rust's message, which
// `catch_unwind` catches; a `JsValue`'s `{:?}` is what upstream's JS,
// `debugString`, shows: an `Error`'s `name: message\nstack`, an array's
// items (ADR 1112).
const throws = `use std::panic::{AssertUnwindSafe, catch_unwind};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = refuse, catch)]
    fn refuse(n: u32) -> Result<u32, JsValue>;
    #[wasm_bindgen(js_namespace = globalThis, js_name = listed, catch)]
    fn listed(n: u32) -> Result<u32, JsValue>;
}

fn caught(f: impl FnOnce() -> u32) -> String {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(n) => n.to_string(),
        Err(payload) => payload.downcast_ref::<String>().cloned().or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(),
    }
}

pub fn none() -> String {
    caught(|| None::<u32>.unwrap_throw())
}

pub fn thrown() -> String {
    caught(|| refuse(2).expect_throw("refused"))
}

pub fn array() -> String {
    caught(|| listed(0).unwrap_throw())
}

pub fn fine() -> String {
    caught(|| refuse(1).unwrap_throw())
}
`;

test("unwrap_throw is native Rust's expect, caught by catch_unwind, a JsValue shown as upstream's JS shows it", () => {
  const dir = fixture("compat-throws");
  const crate = gitCrate(dir, 'wasm-bindgen = "0.2.98"\n', throws);
  const at = app(dir, crate, "pub use getrandom::{array, fine, none, thrown};\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const program = `const app = await import(${JSON.stringify(join(at, "src", "lib.js"))});
globalThis.refuse = (n) => { if (n > 1) { const e = new RangeError("too many"); e.stack = "at refuse"; throw e; } return n; };
globalThis.listed = () => { throw [1, "a", null]; };
console.log(JSON.stringify([app.fine(), app.none(), app.thrown(), app.array()]));`;
  writeFileSync(join(at, "check.mjs"), program);
  const ran = runSync([process.execPath, join(at, "check.mjs")], at, 60_000);
  expect(JSON.parse(ran.stdout)).toEqual([
    "1",
    "called `Option::unwrap_throw()` on a `None` value",
    "refused: JsValue(RangeError: too many\nat refuse)",
    'called `Result::unwrap_throw()` on an `Err` value: JsValue([1, "a", null])',
  ]);
}, 600_000);
