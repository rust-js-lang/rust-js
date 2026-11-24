// What rust-js's own wasm-bindgen, js-sys and web-sys serve of the crates
// that use them (ADR 1112), each as a crate writes it: inline JS imports,
// getrandom's backends, chrono's `wasmbind`, `#[wasm_bindgen_test]`s, the
// patch of a build's features and `unwrap_throw`. The graphs and locks that
// decide where they serve are compat.test.ts' (#2465).
import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compatCrates, writePatch } from "../tooling/patch.js";
import { node } from "./programs";
import { buildCompiler, compiler, fixture } from "./support";
import { answer, app, build, cli, config, git, gitCrate, usesWasmBindgen } from "./compat";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildCompiler, 600_000);

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
  const at = app(dir, crate, "pub use uuid::{flipped, random, refused, summed};\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = withoutBoundary(readFileSync(join(at, "rust-js", "crates", "uuid-1.18.1", "src", "lib.js"), "utf8"));
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

test("getrandom 0.1's backend, in a stand-in for chrono: imported types, methods, getters, its static `module`'s require, a Uint8Array", () => {
  const dir = fixture("compat-getrandom-01");
  const crate = gitCrate(dir, 'bindgen = { package = "wasm-bindgen", version = "0.2.29" }\njs-sys = "0.3"\n', getrandom01, "chrono", "0.4.42");
  const at = app(dir, crate, "pub use chrono::random;\n");
  writePatch(at, [...compatCrates().keys()]);
  const built = build(at);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  const js = withoutBoundary(readFileSync(join(at, "rust-js", "crates", "chrono-0.4.42", "src", "lib.js"), "utf8"));
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
  const js = withoutBoundary(readFileSync(join(at, "rust-js", "crates", "chrono-0.4.45", "src", "lib.js"), "utf8"));
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
  const crate = join(dir, "uuid");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "uuid"\nversion = "1.18.1"\nedition = "2024"\n\n[features]\nextra = []\n\n[dev-dependencies]\nwasm-bindgen-test = "0.3"\n');
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

// The patch is of the graph the build's features make: a feature as
// getrandom's `wasm_js` is, in a stand-in for uuid, brings wasm-bindgen in, which the patch names then,
// so its tests use rust-js's own, not crates.io's (ADR 1112).
test("the patch is of the graph of the features rust-js test is given", () => {
  const dir = fixture("compat-feature-graph");
  const crate = join(dir, "uuid");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "uuid"\nversion = "1.18.1"\nedition = "2024"\n\n[features]\nwasm_js = ["dep:wasm-bindgen"]\n\n[dependencies]\nwasm-bindgen = { version = "0.2", optional = true }\n');
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
  const at = app(dir, crate, "pub use uuid::{array, fine, none, thrown};\n");
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
